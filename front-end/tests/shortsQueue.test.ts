import test from "node:test";
import assert from "node:assert/strict";
import { useProjectStore } from "../src/stores/projectStore";
import { saveShorts, shortOp } from "../src/stores/shortsStore";
import { api } from "../src/lib/ipc";
import type { OpenedProject, Short } from "../src/lib/types";

const shorts = [{ id: "a", title: "A" }, { id: "b", title: "B" }] as unknown as Short[];
const project = (handle: string, revision: number, shortView?: string): OpenedProject => ({
  projectHandle: handle, revision, name: handle, durationUs: 8_000_000, assets: [],
  sequence: { tracks: [], magnetic: true }, fps: 30, diagnostics: [], previewAvailable: false, undoAvailable: false, redoAvailable: false,
  shorts, shortView,
});

/** A save of short `a` in project `one` that finishes only when released. */
function slowSave() {
  let release = () => {};
  api.projectShortsSet = (handle, revision) =>
    new Promise((resolve) => {
      release = () => resolve(project(handle, revision + 1, "a"));
    });
  const saved = saveShorts((list) => list);
  return { saved, release: () => release() };
}

test("a short's change queued behind a save is dropped when another project opens", async () => {
  useProjectStore.setState({ openedProject: project("one", 1, "a"), viewShort: "a" });
  api.projectShortView = async (handle, id) => project(handle, 2, id);
  const { saved, release } = slowSave();
  const ran: string[] = [];
  shortOp("Changing layouts", async (p) => {
    ran.push(`${p.projectHandle}/${p.shortView}`);
    return p;
  });
  useProjectStore.setState({ openedProject: project("two", 5, "b"), viewShort: "b" });
  await new Promise((resolve) => setTimeout(resolve, 0));
  release();
  await saved;
  await new Promise((resolve) => setTimeout(resolve, 10));
  assert.deepEqual(ran, []);
});

test("a short's change queued behind a save is dropped when another short shows", async () => {
  useProjectStore.setState({ openedProject: project("one", 1, "a"), viewShort: "a" });
  const { saved, release } = slowSave();
  const ran: string[] = [];
  shortOp("Changing layouts", async (p) => {
    ran.push(`${p.projectHandle}/${p.shortView}`);
    return p;
  });
  useProjectStore.setState({ openedProject: project("one", 1, "b"), viewShort: "b" });
  await new Promise((resolve) => setTimeout(resolve, 0));
  release();
  await saved;
  await new Promise((resolve) => setTimeout(resolve, 10));
  assert.deepEqual(ran, []);
});

test("a short's change queued behind a save runs on the short it was asked for", async () => {
  useProjectStore.setState({ openedProject: project("one", 1, "a"), viewShort: "a" });
  api.projectShortView = async (handle, id) => project(handle, 2, id);
  const { saved, release } = slowSave();
  const ran: string[] = [];
  shortOp("Changing layouts", async (p) => {
    ran.push(`${p.projectHandle}/${p.shortView}@${p.revision}`);
    return p;
  });
  await new Promise((resolve) => setTimeout(resolve, 0));
  release();
  await saved;
  await new Promise((resolve) => setTimeout(resolve, 10));
  assert.deepEqual(ran, ["one/a@2"]);
});
