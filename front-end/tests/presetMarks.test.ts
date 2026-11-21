import { closePage, dom } from "./dom";
import test, { after } from "node:test";
import assert from "node:assert/strict";
import React, { act } from "react";
import { createRoot } from "react-dom/client";
import { mockIPC, clearMocks } from "@tauri-apps/api/mocks";
import { ShortPresetMarks } from "../src/components/presets/PresetMarks";
import { useProjectStore } from "../src/stores/projectStore";
import { fullLayout, useShortsStore } from "../src/stores/shortsStore";
import type { OpenedProject, PresetsView, Short } from "../src/lib/types";

after(() => {
  clearMocks();
  closePage();
});

const look = { screenZoom: 1, followZooms: true, screenPanX: 0, screenPanY: 0, captions: true, captionSpot: "seam", captionMaxWords: 0, captionLines: 1, captionSizePct: 0, pushIns: false };
const view = {
  presets: [],
  projectPreset: "Tutorial",
  shortsPreset: "Shorts",
  clipsPreset: "Clips",
  changes: {},
  changed: 0,
  inherited: {},
  shortLooks: { shorts: { ...look, captionLines: 2 }, clips: look },
  folder: "",
} as unknown as PresetsView;

// A clip cut from the video with a bigger zoom and its own caption spot; a short from a take
// beside it as its preset has it.
const clip = { id: "c", title: "Clip", sourceStartUs: 0, sourceEndUs: 1, layout: { ...look, screenZoom: 1.5, captionSpot: "top", cameraClips: { x: {} } } } as unknown as Short;
const take = { id: "t", title: "Take", sourceStartUs: 0, sourceEndUs: 1, take: {}, layout: { ...look, captionLines: 2 } } as unknown as Short;
const project = (revision: number, shorts: Short[]): OpenedProject =>
  ({
    projectHandle: "p", revision, name: "p", durationUs: 1, assets: [], sequence: { tracks: [], magnetic: true }, fps: 30,
    diagnostics: [], previewAvailable: false, undoAvailable: false, redoAvailable: false, shorts,
  }) as unknown as OpenedProject;

const saved: Short[][] = [];
mockIPC((cmd, raw) => {
  const args = (raw ?? {}) as Record<string, unknown>;
  if (cmd === "presets") return view;
  if (cmd === "project_shorts_set") {
    saved.push(args.shorts as Short[]);
    return project((args.expectedRevision as number) + 1, args.shorts as Short[]);
  }
  return null;
});

const settle = (ms = 50) => act(async () => new Promise((done) => setTimeout(done, ms)));
const text = () => dom.document.body.textContent ?? "";
const chips = () => [...dom.document.querySelectorAll<HTMLButtonElement>("[aria-label='Changed from the preset'] button")];

test("a short shows what it changed from its own preset and takes one back for itself only", async () => {
  useProjectStore.setState({ openedProject: project(1, [clip, take]) });
  useShortsStore.setState({ selectedId: "c", kind: "clips", draft: null });
  dom.document.body.innerHTML = '<div id="app"></div>';
  const root = createRoot(dom.document.getElementById("app")!);
  const Panel = () => {
    const shorts = useProjectStore((s) => s.openedProject?.shorts) ?? [];
    const draft = useShortsStore((s) => s.draft);
    const id = useShortsStore((s) => s.selectedId);
    const short = shorts.find((s) => s.id === id)!;
    return React.createElement(ShortPresetMarks, { short, layout: fullLayout(draft ?? short.layout) });
  };
  await act(async () => root.render(React.createElement(Panel)));
  await settle();

  // The clip against the clips preset's look; its hand framing is its own and not listed.
  assert.deepEqual(
    chips().map((b) => b.textContent),
    ["screen zoom", "caption spot", "all back"],
  );
  assert.match(chips()[0].title, /Clips preset's screen zoom for this short: 1/);

  await act(async () => chips()[0].click());
  assert.deepEqual(chips().map((b) => b.textContent), ["caption spot"]);
  await settle(300);
  assert.equal(saved.length, 1);
  const [savedClip, savedTake] = saved[0];
  assert.equal(savedClip.layout.screenZoom, 1);
  assert.equal(savedClip.layout.captionSpot, "top", "only the field asked for");
  assert.deepEqual(savedClip.layout.cameraClips, { x: {} });
  assert.deepEqual(savedTake, take, "the other short is untouched");

  // The take short against the shorts preset's look (two caption lines): as its preset.
  await act(async () => useShortsStore.setState({ selectedId: "t", kind: "shorts", draft: null }));
  await settle();
  assert.equal(chips().length, 0);
  assert.match(text(), /As the Shorts preset\./);
  await act(async () => root.unmount());
});
