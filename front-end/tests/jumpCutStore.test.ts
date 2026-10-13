import test from "node:test";
import assert from "node:assert/strict";
import { useJumpCutStore, JumpCutPass } from "../src/stores/jumpCutStore";
import { useProjectStore } from "../src/stores/projectStore";
import type { OpenedProject, SilenceDetectionResult } from "../src/lib/types";
import { gapSettings } from "../src/lib/jumpCutConfig";

const project: OpenedProject = {
  projectHandle: "tutorial", revision: 1, name: "Tutorial", durationUs: 8_000_000, assets: [],
  sequence: { tracks: [], magnetic: true }, fps: 30, diagnostics: [], previewAvailable: false, undoAvailable: false, redoAvailable: false,
};
const result: SilenceDetectionResult = {
  trackId: "mic", sampleRate: 8000, channels: 1, channelPolicy: "all", diagnostics: [], thresholds: [],
  transcriptDependencies: [{ trackId: "mic", wordStamp: "original" }],
  suggestions: [{ id: "gap", startUs: 1_000_000, endUs: 2_000_000, durationMs: 1000, selected: false, sourceStartUs: 1_000_000, sourceEndUs: 2_000_000 }],
};
const reset = () => {
  useProjectStore.setState({ openedProject: { ...project }, captionsVersion: 0 });
  useJumpCutStore.setState(useJumpCutStore.getInitialState(), true);
};
const start = (pass: JumpCutPass) => {
  const state = useProjectStore.getState();
  return { generation: useJumpCutStore.getState().beginScan(pass), snapshot: {
    projectHandle: state.openedProject!.projectHandle, revision: state.openedProject!.revision,
    captionsVersion: state.captionsVersion, transcriptDependencies: result.transcriptDependencies,
  }};
};
const complete = (pass: JumpCutPass) => {
  const scan = start(pass);
  useJumpCutStore.getState().finishScan(pass, scan.generation, result, scan.snapshot);
};

test("switching passes keeps separate settings, sources, reviews and selections", () => {
  reset();
  const store = useJumpCutStore.getState();
  store.chooseSources("silence", { sourceMode: "two", chosenTrack: "mic", secondTrack: "system" });
  store.chooseSources("nonSpeech", { sourceMode: "one", chosenTrack: "other" });
  complete("silence");
  store.select("silence", "gap", true);
  complete("nonSpeech");
  store.setActivePass("nonSpeech");
  store.configureGaps({ minDurationMs: 700, paddingMs: 150 });
  let state = useJumpCutStore.getState();
  assert.equal(state.reviews.silence.blocks[0].selected, true);
  assert.equal(state.reviews.silence.sourceMode, "two");
  assert.equal(state.reviews.nonSpeech.chosenTrack, "other");
  assert.equal(state.silenceConfig.minDurationMs, 250);
  complete("nonSpeech");
  store.configureSilence({ ...state.silenceConfig, minDurationMs: 900 });
  state = useJumpCutStore.getState();
  assert.equal(state.reviews.nonSpeech.blocks.length, 1);
  assert.equal(state.gapConfig.minDurationMs, 700);
  assert.equal(state.reviews.silence.blocks.length, 0);
  store.setActivePass("silence");
  assert.equal(useJumpCutStore.getState().gapConfig.minDurationMs, 700);
});

test("transcript updates discard both completed reviews and in-flight responses", () => {
  reset();
  complete("silence");
  const pending = start("nonSpeech");
  useProjectStore.getState().bumpCaptions(true);
  useJumpCutStore.getState().finishScan("nonSpeech", pending.generation, result, pending.snapshot);
  useJumpCutStore.getState().failScan("nonSpeech", pending.generation, "old failure");
  for (const pass of ["silence", "nonSpeech"] as const) {
    const review = useJumpCutStore.getState().reviews[pass];
    assert.equal(review.blocks.length, 0);
    assert.equal(review.busy, false);
    assert.equal(review.error, null);
    assert.match(review.notice!, /transcript changed/);
  }
});

test("timeline edits, undo, reopening and short views discard reviews while retaining settings", () => {
  for (const next of [{ ...project, revision: 2 }, { ...project, revision: 3 }, { ...project, projectHandle: "reopened" }, { ...project, shortView: "short" }]) {
    reset();
    useJumpCutStore.getState().configureGaps({ minDurationMs: 800, paddingMs: 120 });
    complete("silence");
    const pending = start("nonSpeech");
    useProjectStore.setState({ openedProject: next });
    useJumpCutStore.getState().finishScan("nonSpeech", pending.generation, result, pending.snapshot);
    const state = useJumpCutStore.getState();
    assert.equal(state.reviews.silence.blocks.length, 0);
    assert.equal(state.reviews.nonSpeech.blocks.length, 0);
    assert.equal(state.reviews.nonSpeech.busy, false);
    assert.equal(state.gapConfig.minDurationMs, 800);
  }
});

test("changed presets and sources reject late results without disrupting the other pass", () => {
  reset();
  complete("silence");
  const pending = start("nonSpeech");
  useJumpCutStore.getState().chooseSources("nonSpeech", { chosenTrack: "new-source" });
  useJumpCutStore.getState().finishScan("nonSpeech", pending.generation, result, pending.snapshot);
  assert.equal(useJumpCutStore.getState().reviews.nonSpeech.blocks.length, 0);
  assert.equal(useJumpCutStore.getState().reviews.silence.blocks.length, 1);
  const first = start("nonSpeech");
  const second = start("nonSpeech");
  useJumpCutStore.getState().finishScan("nonSpeech", second.generation, result, second.snapshot);
  useJumpCutStore.getState().finishScan("nonSpeech", first.generation, { ...result, suggestions: [] }, first.snapshot);
  assert.equal(useJumpCutStore.getState().reviews.nonSpeech.blocks.length, 1);
});

test("word-edge controls reach gap detection, retain silence review and reject a stale gap scan", () => {
  reset();
  const measured = { ...result, thresholds: [{ trackId: "mic", thresholdDb: -42 }] };
  const silence = start("silence");
  useJumpCutStore.getState().finishScan("silence", silence.generation, measured, silence.snapshot);
  const pending = start("nonSpeech");
  const settings = gapSettings({ thresholdDb: -30, autoLevel: 0.8, minDurationMs: 20, paddingMs: 0, refineWordEdges: true, edgeThresholdDb: -42 });
  assert.deepEqual(settings, { minDurationMs: 20, paddingMs: 0, refineWordEdges: true, edgeThresholdDb: -42 });
  useJumpCutStore.getState().configureGaps(settings);
  useJumpCutStore.getState().finishScan("nonSpeech", pending.generation, result, pending.snapshot);
  const state = useJumpCutStore.getState();
  assert.equal(state.reviews.nonSpeech.blocks.length, 0);
  assert.equal(state.gapConfig.paddingMs, 0);
  assert.equal(state.gapConfig.edgeThresholdDb, -42);
  assert.equal(state.reviews.silence.blocks.length, 1);
  assert.deepEqual(state.reviews.silence.thresholds, measured.thresholds);
});
