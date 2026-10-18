import test from "node:test";
import assert from "node:assert/strict";
(globalThis as unknown as { window: unknown }).window ??= globalThis;
import { workspacePanels } from "../src/components/layout/dockLayout";
import { api } from "../src/lib/ipc";
import type { OpenedProject, Short } from "../src/lib/types";
import { useProjectStore } from "../src/stores/projectStore";
import { changeLayout, removeShort, resetShorts, selectShort, useShortsStore } from "../src/stores/shortsStore";

const shortOf = (id: string) => ({ id, title: id, sourceStartUs: 0, sourceEndUs: 1, layout: { cameraPct: 35 } }) as unknown as Short;
const project = (revision: number, shorts: Short[], shortView?: string): OpenedProject => ({
  projectHandle: "tutorial", revision, name: "Tutorial", durationUs: 8_000_000, assets: [],
  sequence: { tracks: [], magnetic: true }, fps: 30, diagnostics: [], previewAvailable: false, undoAvailable: false, redoAvailable: false,
  shorts, shortView,
});

test("Shorts has its own panels and the editor's workspaces don't show them", () => {
  const shorts = workspacePanels("shorts");
  assert.ok(shorts.includes("shorts") && shorts.includes("shortLook") && shorts.includes("preview") && shorts.includes("timeline"));
  assert.ok(!shorts.includes("jumpCuts") && !shorts.includes("transcript"));
  for (const workspace of ["edit", "cleanup"] as const) {
    const panels = workspacePanels(workspace);
    assert.ok(!panels.includes("shorts") && !panels.includes("shortLook"));
    assert.ok(panels.includes("jumpCuts") && panels.includes("transcript"));
  }
});

test("opening another project forgets the short on show", () => {
  useProjectStore.setState({ openedProject: project(1, [shortOf("s1")], "s1"), viewShort: "s1", playbackShortId: "s1" });
  useProjectStore.getState().loadOpenedProject(project(1, []), "C:/other");
  const state = useProjectStore.getState();
  assert.equal(state.viewShort, undefined);
  assert.equal(state.playbackShortId, undefined);
});

test("a look change lands on the short it was made on, even if another is picked meanwhile", async () => {
  resetShorts();
  useProjectStore.setState({ openedProject: project(1, [shortOf("a"), shortOf("b")]), viewShort: undefined, isPlaying: false });
  const saved: Short[][] = [];
  api.projectShortsSet = async (_handle, revision, shorts) => {
    saved.push(shorts);
    return project(revision + 1, shorts);
  };
  selectShort("a");
  changeLayout({ cameraPct: 50 });
  selectShort("b");
  await new Promise((resolve) => setTimeout(resolve, 260));
  await new Promise((resolve) => setTimeout(resolve, 0));
  assert.equal(saved.length, 1);
  assert.equal(saved[0].find((s) => s.id === "a")?.layout.cameraPct, 50);
  assert.equal(saved[0].find((s) => s.id === "b")?.layout.cameraPct, 35);
  assert.equal(useShortsStore.getState().draft, null);
});

test("removing the picked short clears the pick and saves the list without it", async () => {
  resetShorts();
  useProjectStore.setState({ openedProject: project(1, [shortOf("a"), shortOf("b")]), viewShort: undefined });
  let saved: Short[] = [];
  api.projectShortsSet = async (_handle, revision, shorts) => {
    saved = shorts;
    return project(revision + 1, shorts);
  };
  selectShort("a");
  removeShort("a");
  await new Promise((resolve) => setTimeout(resolve, 10));
  assert.equal(useShortsStore.getState().selectedId, undefined);
  assert.deepEqual(saved.map((s) => s.id), ["b"]);
});
