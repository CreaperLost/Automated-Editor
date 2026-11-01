import test from "node:test";
import assert from "node:assert/strict";
import React from "react";
import { fakeTimers, mountHook, settle } from "./hooks";
import { api } from "../src/lib/ipc";
import { useProjectStore } from "../src/stores/projectStore";
import { changeLayouts } from "../src/stores/layoutsStore";
import { useDebouncedSave } from "../src/hooks/useDebouncedSave";
import { ClipThumbs } from "../src/components/timeline/ClipThumbs";
import { SearchPanel } from "../src/components/review/SearchPanel";
import type { Clip, OpenedProject, SearchResults, Short } from "../src/lib/types";

const timers = fakeTimers();

const shorts = [{ id: "a", title: "A" }, { id: "b", title: "B" }] as unknown as Short[];
const project = (handle: string, revision: number, shortView?: string): OpenedProject => ({
  projectHandle: handle, revision, name: handle, durationUs: 8_000_000, assets: [],
  sequence: { tracks: [], magnetic: true }, fps: 30, diagnostics: [], previewAvailable: false, undoAvailable: false, redoAvailable: false,
  shorts, shortView,
});

/** Every element in a rendered tree (elements only: components are not called). */
function elements(node: unknown): React.ReactElement[] {
  if (Array.isArray(node)) return node.flatMap(elements);
  if (!React.isValidElement(node)) return [];
  const props = node.props as { children?: unknown };
  return [node, ...elements(props.children)];
}

// AUD-001
test("thumbnails wanted beyond the memory limit settle, and unwanted ones are reclaimed", async () => {
  const asked: string[] = [];
  api.projectClipThumbnails = async (_handle, asset, _stream, times) => {
    asked.push(asset);
    return times.map((t) => `data:${asset}@${t}`);
  };
  // 400 slots of ~0.24 s, rounded to 0.25 s steps: ~380 pictures a clip, ~2,700 for seven.
  const props = (asset: string) => ({
    projectHandle: "thumbs",
    clip: { id: asset, asset, stream: "v", startUs: 0, inUs: 0, durationUs: 400 * 239_532 } as Clip,
    stream: "v",
    still: false,
    aspect: 16 / 9,
    heightPx: 32,
    pxPerUs: 57 / 240_000,
    fromUs: 0,
    toUs: 400 * 239_532,
  });
  const inner = (ClipThumbs as unknown as { type: (p: ReturnType<typeof props>) => React.ReactNode }).type;
  const drain = async () => {
    for (let i = 0; i < 1_000 && timers.pending() > 0; i++) {
      timers.advance(60);
      await settle();
    }
  };
  const shown = (clip: { render: () => React.ReactNode }) => elements(clip.render()).filter((e) => e.type === "img").length;

  const clips = ["a1", "a2", "a3", "a4", "a5", "a6", "a7"].map((asset) => mountHook(() => inner(props(asset))));
  await drain();
  assert.equal(timers.pending(), 0, "nothing more is asked for once every wanted picture is made");
  const calls = asked.length;
  assert.ok(calls < 60, `asked ${calls} times`);
  timers.advance(10_000);
  await settle();
  assert.equal(asked.length, calls, "an idle timeline asks for nothing");
  for (const clip of clips) assert.ok(shown(clip) >= 390, "every wanted picture stays shown");

  // Scrolled away: another clip shows; the oldest unwanted pictures make room for it.
  clips.forEach((clip) => clip.unmount());
  const other = mountHook(() => inner(props("b1")));
  await drain();
  assert.ok(shown(other) >= 390);
  asked.length = 0;
  mountHook(() => inner(props("a1")));
  await drain();
  assert.ok(asked.includes("a1"), "the first clip's pictures were reclaimed");
});

// AUD-003
test("a debounced save goes to the timeline it was made on, never the one shown later", async () => {
  useProjectStore.setState({ openedProject: project("one", 1, "a"), viewShort: "a" });
  const saved: string[] = [];
  const hook = mountHook(() => useDebouncedSave(200));
  const save = (p: OpenedProject) => saved.push(`${p.projectHandle}/${p.shortView ?? "video"}`);

  // Another short asked for: the save goes out at once, on the short still shown.
  hook.result.schedule(save);
  useProjectStore.setState({ viewShort: "b" });
  assert.deepEqual(saved, ["one/a"]);
  timers.advance(500);
  assert.deepEqual(saved, ["one/a"], "and only once");

  // The other short shown in one step: dropped, never saved to it.
  useProjectStore.setState({ openedProject: project("one", 1, "a"), viewShort: "a" });
  hook.result.schedule(save);
  useProjectStore.setState({ openedProject: project("one", 1, "b"), viewShort: "b" });
  timers.advance(500);
  assert.deepEqual(saved, ["one/a"]);

  // Back to the video, then another project: neither gets the short's change.
  hook.result.schedule(save);
  useProjectStore.setState({ openedProject: project("one", 1), viewShort: undefined });
  hook.result.schedule(save);
  useProjectStore.setState({ openedProject: project("two", 1), viewShort: undefined });
  timers.advance(500);
  assert.deepEqual(saved, ["one/a"]);

  // Left alone, it saves after the pause.
  hook.result.schedule(save);
  timers.advance(199);
  assert.deepEqual(saved, ["one/a"]);
  timers.advance(1);
  assert.deepEqual(saved, ["one/a", "two/video"]);
  hook.unmount();
});

test("a base-layout change made on one short and a switch to another changes the first", async () => {
  useProjectStore.setState({ openedProject: project("one", 1, "a"), viewShort: "a" });
  const changed: (string | null)[] = [];
  api.projectLayoutsChange = async (handle, revision, shortId) => {
    changed.push(shortId);
    return project(handle, revision + 1, shortId ?? undefined);
  };
  api.projectShortView = async (handle, id) => project(handle, 3, id);
  const hook = mountHook(() => useDebouncedSave(200));
  hook.result.schedule(() => changeLayouts({ op: "base", layout: {} } as never));
  useProjectStore.setState({ viewShort: "b" });
  await settle();
  useProjectStore.setState({ openedProject: project("one", 3, "b") });
  timers.advance(500);
  await settle();
  assert.deepEqual(changed, ["a"]);
  hook.unmount();
});

// AUD-004
test("a search answered after its query was cleared or shortened shows nothing", async () => {
  useProjectStore.setState({ openedProject: project("one", 1), viewShort: undefined });
  const answers: { resolve: (r: SearchResults) => void; reject: (e: unknown) => void }[] = [];
  api.sourceSearch = () => new Promise((resolve, reject) => answers.push({ resolve, reject }));
  const hit = { stream: "s", firstWord: "w1", excerpt: "[hello]", status: "plays", editedUs: 0, sourceStartUs: 0, exact: true };
  const panel = mountHook(() => SearchPanel({}));
  const type = (value: string) => {
    const input = elements(panel.render()).find((e) => e.type === "input")!;
    (input.props as { onChange: (e: unknown) => void }).onChange({ target: { value } });
    return panel.render();
  };
  const listed = () => elements(panel.render()).filter((e) => e.type === "li").length;
  const alert = () => elements(panel.render()).some((e) => (e.props as { role?: string }).role === "alert");

  for (const [later, answer] of [["", "resolve"], ["h", "resolve"], ["", "reject"], ["h", "reject"]] as const) {
    type("hello");
    timers.advance(200);
    assert.equal(answers.length, 1);
    type(later);
    const { resolve, reject } = answers.pop()!;
    if (answer === "resolve") resolve({ total: 1, streams: 1, hits: [hit] } as unknown as SearchResults);
    else reject("Search failed");
    await settle();
    assert.equal(listed(), 0, `${answer} after "${later}"`);
    assert.equal(alert(), false, `${answer} after "${later}"`);
  }

  // The latest query's answer still shows.
  type("hello");
  timers.advance(200);
  answers.pop()!.resolve({ total: 1, streams: 1, hits: [hit] } as unknown as SearchResults);
  await settle();
  assert.equal(listed(), 1);
  panel.unmount();
});
