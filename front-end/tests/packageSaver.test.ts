import test from "node:test";
import assert from "node:assert/strict";
import { fakeTimers, mountHook, settle, elements } from "./hooks";
import { api } from "../src/lib/ipc";
import { packageSaver } from "../src/lib/packageSaver";
import { PackageEditor } from "../src/components/export/PackageEditor";
import type { PublishPackage, PublishPackageView } from "../src/lib/types";

const timers = fakeTimers();

const pkg = (title: string): PublishPackage =>
  ({ title, description: "About it", tags: ["a"], aiDisclosure: false }) as unknown as PublishPackage;
const view = (p: PublishPackage, problem?: string): PublishPackageView =>
  ({ target: "video", package: p, uploads: [], problem }) as unknown as PublishPackageView;

/** Saves answered by hand: each call waits until the test resolves or rejects it. */
function deferredSaves() {
  const calls: { handle: string; title: string; done: (fail?: string) => void }[] = [];
  api.publishPackageSave = (handle, _target, p) =>
    new Promise((resolve, reject) => {
      calls.push({ handle, title: p.title, done: (fail) => (fail ? reject(new Error(fail)) : resolve(view(p))) });
    });
  return calls;
}

// GAP-015
test("closing the package editor sends the last change to the project it was typed in", async () => {
  packageSaver.reset();
  const calls = deferredSaves();
  api.publishPackage = async () => view(pkg("Old title"));
  const editor = mountHook(() => PackageEditor({ projectHandle: "A", target: "video" }));
  await settle();
  const title = () =>
    elements(editor.render()).find((e) => e.type === "input" && (e.props as { value?: string }).value !== undefined)!;
  (title().props as { onChange: (e: unknown) => void }).onChange({ target: { value: "New title" } });
  timers.advance(100);
  editor.unmount();
  timers.advance(2_000);
  await settle();
  assert.deepEqual(
    calls.map((c) => [c.handle, c.title]),
    [["A", "New title"]],
    "one save, of the latest draft, to its own project",
  );
});

// GAP-016
test("upload waits for every save to land, and a failed save stops it", async () => {
  for (const outcome of ["saved", "failed"] as const) {
    packageSaver.reset();
    const calls = deferredSaves();
    api.publishPackage = async () => view(pkg("Second"));
    let uploads = 0;
    packageSaver.schedule("A", "video", pkg("First"));
    timers.advance(600);
    packageSaver.schedule("A", "video", pkg("Second"));
    timers.advance(100);
    // Upload clicked: the dialog flushes and uploads only after it resolves.
    const upload = packageSaver.flush("A", "video").then(() => uploads++);
    upload.catch(() => undefined);
    timers.advance(2_000);
    await settle();
    assert.equal(calls.length, 1, "the second save waits for the first");
    assert.equal(uploads, 0);
    calls[0].done();
    await settle();
    assert.deepEqual(
      calls.map((c) => c.title),
      ["First", "Second"],
    );
    assert.equal(uploads, 0, "still waiting for the last save");
    calls[1].done(outcome === "failed" ? "disk full" : undefined);
    if (outcome === "saved") {
      await upload;
      assert.equal(uploads, 1);
    } else {
      await assert.rejects(upload, /disk full/);
      assert.equal(uploads, 0, "nothing is uploaded when the save failed");
      assert.equal(packageSaver.unsaved("A", "video")?.draft.title, "Second", "the draft is kept");
    }
  }
});

// GAP-011
test("a save typed for one project never goes to another", async () => {
  packageSaver.reset();
  const calls = deferredSaves();
  packageSaver.schedule("A", "video", pkg("For A"));
  packageSaver.schedule("B", "video", pkg("For B"));
  timers.advance(600);
  await settle();
  assert.deepEqual(
    calls.map((c) => [c.handle, c.title]).sort(),
    [
      ["A", "For A"],
      ["B", "For B"],
    ],
  );
});
