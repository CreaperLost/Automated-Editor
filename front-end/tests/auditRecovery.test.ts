import test from "node:test";
import assert from "node:assert/strict";
import React from "react";
import { fakeTimers, mountHook, settle, elements } from "./hooks";
import { api } from "../src/lib/ipc";
import { ClipThumbs } from "../src/components/timeline/ClipThumbs";
import { changeLayouts, pressLayoutKey, useLayoutsStore } from "../src/stores/layoutsStore";
import { LayoutsSection } from "../src/components/layouts/LayoutsSection";
import { LayoutEditor } from "../src/components/layouts/LayoutEditor";
import { useProjectStore } from "../src/stores/projectStore";
import { useDebouncedSave } from "../src/hooks/useDebouncedSave";
import { BUILT_IN_LAYOUTS, DEFAULT_LAYOUT_TRACK, type Clip, type Layout, type LayoutChange, type OpenedProject } from "../src/lib/types";

const timers = fakeTimers();
const project = (handle = "one", revision = 1, shortView?: string): OpenedProject => ({
  projectHandle: handle, revision, shortView, name: handle, durationUs: 100_000_000, assets: [],
  sequence: { tracks: [], magnetic: true }, fps: 30, diagnostics: [], previewAvailable: false,
  undoAvailable: false, redoAvailable: false,
});
function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (error: unknown) => void;
  const promise = new Promise<T>((yes, no) => { resolve = yes; reject = no; });
  return { promise, resolve, reject };
}
const layout = (value: number) => ({ value }) as unknown as Layout;
const base = (value: number): LayoutChange => ({ op: "base", layout: layout(value) });
const props = (asset: string, many = false) => ({
  projectHandle: "recovery", clip: { id: asset, asset, startUs: 0, inUs: 0,
    durationUs: many ? 400 * 239_532 : 240_000 } as Clip,
  stream: "v", still: false, aspect: 16 / 9, heightPx: 32, pxPerUs: 57 / 240_000,
  fromUs: 0, toUs: many ? 400 * 239_532 : 240_000,
});
const inner = (ClipThumbs as unknown as { type: (p: ReturnType<typeof props>) => React.ReactNode }).type;
const images = (node: unknown): string[] =>
  elements(node).flatMap((e) => (e.type === "img" ? [(e.props as { src: string }).src] : []));
async function drain() {
  for (let i = 0; i < 1_000 && timers.pending(); i++) {
    timers.advance(60);
    await settle();
  }
  assert.equal(timers.pending(), 0, "queue settles without idle retry traffic");
}

test("AUD-006: a failed batch recovers on the same mounted keys after backoff", async () => {
  let calls = 0;
  api.projectClipThumbnails = async (_h, _a, _s, times) => {
    if (++calls === 1) throw new Error("temporarily unavailable");
    return times.map(() => "data:recovered");
  };
  const clip = mountHook(() => inner(props("retry")));
  timers.advance(60);
  await settle();
  timers.advance(499);
  await settle();
  assert.equal(calls, 1, "no immediate retry");
  await drain();
  assert.equal(calls, 2);
  assert.equal(images(clip.render()).length, 2);
  clip.unmount();
});

test("AUD-006: persistent failures stop after two retries and remount can recover", async () => {
  let calls = 0;
  api.projectClipThumbnails = async () => { calls++; throw new Error("offline"); };
  const clip = mountHook(() => inner(props("offline")));
  await drain();
  assert.equal(calls, 3);
  timers.advance(60_000);
  await settle();
  assert.equal(calls, 3);
  clip.unmount();
  api.projectClipThumbnails = async (_h, _a, _s, times) => {
    calls++;
    return times.map(() => "data:online");
  };
  const again = mountHook(() => inner(props("offline")));
  await drain();
  assert.equal(calls, 4);
  assert.equal(images(again.render()).length, 2);
  again.unmount();
});

test("AUD-006: genuine no-picture results stay cached without retries", async () => {
  let calls = 0;
  api.projectClipThumbnails = async (_h, _a, _s, times) => { calls++; return times.map(() => null); };
  const clip = mountHook(() => inner(props("gap")));
  await drain();
  clip.unmount();
  const again = mountHook(() => inner(props("gap")));
  await drain();
  assert.equal(calls, 1);
  assert.equal(images(again.render()).length, 0);
  again.unmount();
});

test("AUD-006: mixed replies retry only failed frames and keep pictures and gaps", async () => {
  const calls: number[][] = [];
  api.projectClipThumbnails = async (_h, _a, _s, times) => {
    calls.push(times);
    return calls.length === 1
      ? times.map((_t, i) => i === 0 ? "data:good" : i === 1 ? { error: "decode failed" } : null)
      : times.map(() => "data:recovered-frame");
  };
  const p = props("mixed");
  p.clip.durationUs = p.toUs = 3 * 239_532;
  const clip = mountHook(() => inner(p));
  timers.advance(60);
  await settle();
  assert.equal(images(clip.render()).length, 1, "successful frame is visible before retry");
  await drain();
  assert.deepEqual(calls[1], [calls[0][1]]);
  assert.equal(calls.length, 2);
  assert.equal(images(clip.render()).length, 2);
  clip.unmount();
});

test("AUD-008: releasing demand prunes to capacity without another thumbnail request", async () => {
  const fetched = new Set<string>();
  let calls = 0;
  api.projectClipThumbnails = async (_h, asset, _s, times) => {
    calls++;
    return times.map((t) => { const url = `data:${asset}@${t}`; fetched.add(url); return url; });
  };
  const assets = Array.from({ length: 7 }, (_, i) => `capacity-${i}`);
  const clips = assets.map((asset) => mountHook(() => inner(props(asset, true))));
  await drain();
  assert.ok(fetched.size > 2_000);
  assert.equal(new Set(clips.flatMap((c) => images(c.render()))).size, fetched.size,
    "wanted pictures remain resident above capacity (AUD-001)");
  clips.forEach((c) => c.unmount());
  const before = calls;
  timers.advance(60_000);
  await settle();
  assert.equal(calls, before);
  // Read retained pictures by remounting before any timer/API response can run.
  const again = assets.map((asset) => mountHook(() => inner(props(asset, true))));
  const retained = new Set(again.flatMap((c) => images(c.render())));
  assert.ok(retained.size <= 2_000, `retained ${retained.size} unused pictures`);
  again.forEach((c) => c.unmount());
  await drain();
  assert.equal(calls, before, "pruning needs no new load");
});

test("AUD-007: debounced edits queue on the new revision, coalescing only adjacent snapshots", async () => {
  useProjectStore.setState({ openedProject: project(), viewShort: undefined });
  const first = deferred<OpenedProject>();
  const calls: { revision: number; change: LayoutChange }[] = [];
  api.projectLayoutsChange = async (handle, revision, _short, change) => {
    calls.push({ revision, change });
    return calls.length === 1 ? first.promise : project(handle, revision + 1);
  };
  const hook = mountHook(() => useDebouncedSave(200));
  hook.result.schedule(() => changeLayouts(base(1)));
  timers.advance(200);
  hook.result.schedule(() => changeLayouts(base(2)));
  timers.advance(200);
  changeLayouts(base(3));
  const tile: LayoutChange = { op: "add", startUs: 0, endUs: 1_000_000, layout: layout(4) };
  changeLayouts(tile);
  changeLayouts(base(5));
  assert.equal(calls.length, 1);
  first.resolve(project("one", 2));
  await settle();
  assert.deepEqual(calls, [
    { revision: 1, change: base(1) }, { revision: 2, change: base(3) },
    { revision: 3, change: tile }, { revision: 4, change: base(5) },
  ]);
  assert.equal(useProjectStore.getState().openedProject?.revision, 5);
  assert.equal(useLayoutsStore.getState().busy, null);
  hook.unmount();
});

test("AUD-007: a failed save reports its error and releases the next queued edit", async () => {
  useProjectStore.setState({ openedProject: project(), viewShort: undefined });
  const first = deferred<OpenedProject>();
  const revisions: number[] = [];
  api.projectLayoutsChange = async (handle, revision) => {
    revisions.push(revision);
    return revisions.length === 1 ? first.promise : project(handle, revision + 1);
  };
  changeLayouts(base(1));
  changeLayouts(base(2));
  first.reject(new Error("save failed"));
  await settle();
  assert.deepEqual(revisions, [1, 1]);
  assert.equal(useLayoutsStore.getState().error, "save failed");
  assert.equal(useLayoutsStore.getState().busy, null);
});

test("AUD-007: picking settings and section snapshots coalesce without crossing other operations", async () => {
  useProjectStore.setState({ openedProject: project(), viewShort: undefined });
  const first = deferred<OpenedProject>();
  const changes: LayoutChange[] = [];
  api.projectLayoutsChange = async (handle, revision, _short, change) => {
    changes.push(change);
    return changes.length === 1 ? first.promise : project(handle, revision + 1);
  };
  const settings = (minRestMs: number): LayoutChange => ({
    op: "settings", pick: { ...DEFAULT_LAYOUT_TRACK.pick, minRestMs },
  });
  const section = (id: string, value: number): LayoutChange => ({ op: "update", id, layout: layout(value), name: null });
  changeLayouts(base(1));
  changeLayouts(settings(1));
  changeLayouts(settings(2));
  changeLayouts({ op: "settings", transitionMs: 100 });
  changeLayouts(section("a", 1));
  changeLayouts(section("a", 2));
  changeLayouts(section("b", 3));
  first.resolve(project("one", 2));
  await settle();
  assert.deepEqual(changes, [base(1), settings(2), { op: "settings", transitionMs: 100 }, section("a", 2), section("b", 3)]);
});

test("AUD-007: keyboard layout switches serialize with inspector saves", async () => {
  useProjectStore.setState({ openedProject: project(), viewShort: undefined, currentTimeUs: 123_000 });
  const first = deferred<OpenedProject>();
  const calls: { revision: number; change: LayoutChange }[] = [];
  api.projectLayoutsChange = async (handle, revision, _short, change) => {
    calls.push({ revision, change });
    return calls.length === 1 ? first.promise : project(handle, revision + 1);
  };
  changeLayouts(base(1));
  pressLayoutKey(`builtin:${BUILT_IN_LAYOUTS[0].name}`);
  await settle();
  assert.equal(calls.length, 1);
  first.resolve(project("one", 2));
  await settle();
  assert.equal(calls[1].revision, 2);
  assert.equal(calls[1].change.op, "add");
});

test("AUD-007: an earlier response keeps the editor's newer pending draft visible", async () => {
  useProjectStore.setState({ openedProject: project(), viewShort: undefined });
  api.layoutFavouritesList = async () => [];
  const first = deferred<OpenedProject>();
  const second = deferred<OpenedProject>();
  let calls = 0;
  api.projectLayoutsChange = () => ++calls === 1 ? first.promise : second.promise;
  const panel = mountHook(() => LayoutsSection({}));
  const editor = () => elements(panel.render()).find((e) => e.type === LayoutEditor)!.props as {
    layout: Layout; onChange: (value: Layout) => void;
  };
  editor().onChange(layout(1));
  timers.advance(200);
  editor().onChange(layout(2));
  timers.advance(200);
  first.resolve(project("one", 2));
  await settle();
  assert.equal(calls, 2);
  editor(); // Effects from the first response have run; render again to observe their result.
  assert.deepEqual(editor().layout, layout(2));
  second.resolve(project("one", 3));
  await settle();
  panel.unmount();
});

test("AUD-007: a project/timeline switch cancels queued work and ignores a stale response", async () => {
  for (const next of [project("two", 7), project("one", 7, "short-a")]) {
    useProjectStore.setState({ openedProject: project(), viewShort: undefined });
    const first = deferred<OpenedProject>();
    let calls = 0;
    api.projectLayoutsChange = () => { calls++; return first.promise; };
    changeLayouts(base(1));
    changeLayouts(base(2));
    useProjectStore.setState({ openedProject: next, viewShort: next.shortView });
    first.resolve(project("one", 2));
    await settle();
    assert.equal(calls, 1);
    assert.equal(useProjectStore.getState().openedProject, next);
    assert.equal(useLayoutsStore.getState().busy, null);
  }
});
