import test from "node:test";
import assert from "node:assert/strict";
import { useProjectStore } from "../src/stores/projectStore";
import { api } from "../src/lib/ipc";
import type { OpenedProject, Short } from "../src/lib/types";

const short = { id: "s1", title: "One" } as unknown as Short;
const project = (revision: number, shorts: Short[], shortView?: string): OpenedProject => ({
  projectHandle: "tutorial", revision, name: "Tutorial", durationUs: 8_000_000, assets: [],
  sequence: { tracks: [], magnetic: true }, fps: 30, diagnostics: [], previewAvailable: false, undoAvailable: false, redoAvailable: false,
  shorts, shortView,
});
/** The Shorts workspace showing short s1 at revision 1. */
const reset = () => {
  useProjectStore.setState({ openedProject: project(1, [short], "s1"), viewShort: "s1", selectedClipIds: [] });
};

test("removing the short on show applies the project without it", async () => {
  reset();
  const asked: string[] = [];
  api.projectShortView = async (_handle, id) => {
    asked.push(id);
    throw new Error("That short no longer exists");
  };
  await useProjectStore.getState().applyOpenedProject(project(2, []));
  const state = useProjectStore.getState();
  assert.deepEqual(asked, []);
  assert.equal(state.viewShort, undefined);
  assert.equal(state.openedProject?.revision, 2);
  assert.deepEqual(state.openedProject?.shorts, []);
});

test("applying an edit to a short's view finishes once its view is there", async () => {
  reset();
  let release = () => {};
  api.projectShortView = (_handle, id) =>
    new Promise((resolve) => {
      release = () => resolve(project(2, [short], id));
    });
  const applied = useProjectStore.getState().applyOpenedProject(project(2, [short]));
  // The next queued save must not read revision 1.
  setTimeout(() => release(), 10);
  await applied;
  const state = useProjectStore.getState();
  assert.equal(state.openedProject?.revision, 2);
  assert.equal(state.openedProject?.shortView, "s1");
});
