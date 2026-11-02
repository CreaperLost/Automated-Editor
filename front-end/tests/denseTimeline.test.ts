import test from "node:test";
import assert from "node:assert/strict";
import { groupDenseClips, groupDenseMarks, runClipAt, runMutes } from "../src/lib/sequence";
import type { Clip } from "../src/lib/types";

const clip = (id: string, startUs: number, durationUs: number, more: Partial<Clip> = {}): Clip => ({
  id,
  asset: "rec",
  stream: "mic",
  startUs,
  inUs: startUs,
  durationUs,
  ...more,
});
const drawn = (...clips: Clip[]) => clips.map((c, index) => ({ clip: c, index }));
const shape = (items: ReturnType<typeof groupDenseClips>) => items.map((d) => (d.run ? d.run.map((c) => c.id) : d.clip.id));

test("thin neighbours draw as one run; wide clips and single thin clips stay themselves", () => {
  // 1 px is 10 us here.
  const items = groupDenseClips(drawn(clip("a", 0, 10), clip("b", 10, 20), clip("c", 30, 100), clip("d", 130, 10)), 0.1, () => false);
  assert.deepEqual(shape(items), [["a", "b"], "c", "d"]);
});

test("a selected or missing clip, another stream or a visible gap breaks a run", () => {
  const thin = (id: string, at: number, more?: Partial<Clip>) => clip(id, at, 10, more);
  const apart = groupDenseClips(drawn(thin("a", 0), thin("b", 10), thin("c", 20), thin("d", 30), thin("e", 40)), 0.1, (c) => c.id === "c");
  assert.deepEqual(shape(apart), [["a", "b"], "c", ["d", "e"]]);
  const stream = groupDenseClips(drawn(thin("a", 0), thin("b", 10), thin("c", 20, { stream: "sys" }), thin("d", 30, { stream: "sys" })), 0.1, () => false);
  assert.deepEqual(shape(stream), [["a", "b"], ["c", "d"]]);
  const gap = groupDenseClips(drawn(thin("a", 0), thin("b", 10), thin("c", 30), thin("d", 40)), 0.1, () => false);
  assert.deepEqual(shape(gap), [["a", "b"], ["c", "d"]]);
});

test("a run keeps the place of its first clip", () => {
  const items = groupDenseClips(
    [
      { clip: clip("a", 0, 100), index: 7 },
      { clip: clip("b", 100, 10), index: 8 },
      { clip: clip("c", 110, 10), index: 9 },
    ],
    0.1,
    () => false,
  );
  assert.equal(items[1].index, 8);
});

test("a press on a run finds the clip under it, or the nearest across a gap", () => {
  const run = [clip("a", 0, 10), clip("b", 10, 10), clip("c", 30, 10)];
  assert.equal(runClipAt(run, 0).id, "a");
  assert.equal(runClipAt(run, 19).id, "b");
  assert.equal(runClipAt(run, 21).id, "b");
  assert.equal(runClipAt(run, 28).id, "c");
  assert.equal(runClipAt(run, 500).id, "c");
});

test("marks under a pixel apart become a band; the rest keep their own", () => {
  const marks = [{ atUs: 0 }, { atUs: 5 }, { atUs: 9 }, { atUs: 100 }, { atUs: 200 }, { atUs: 205 }];
  assert.deepEqual(groupDenseMarks(marks, 0.1), [
    { atUs: 0, toUs: 9, count: 3 },
    { atUs: 100 },
    { atUs: 200, toUs: 205, count: 2 },
  ]);
});

test("a run's muted moments land on the timeline, faster clips included, and join on screen", () => {
  const run = [clip("a", 0, 100, { inUs: 1000 }), clip("b", 100, 100, { inUs: 5000, speed: 2 })];
  const mutes = [
    { startUs: 5020, endUs: 5040 },
    { startUs: 1010, endUs: 1020 },
    { startUs: 1021, endUs: 1030 },
    { startUs: 9000, endUs: 9100 },
  ];
  assert.deepEqual(runMutes(run, mutes, 0.1), [{ startUs: 10, endUs: 30 }, { startUs: 110, endUs: 120 }]);
});

test("a dense timeline zoomed out comes down to a handful of runs", () => {
  const many: Clip[] = Array.from({ length: 8000 }, (_, i) => clip(`c${i}`, i * 450_000, 450_000));
  const pxPerUs = 1500 / (8000 * 450_000);
  assert.equal(groupDenseClips(drawn(...many), pxPerUs, () => false).length, 1);
  // Zoomed in far enough, each is itself again.
  assert.equal(groupDenseClips(drawn(...many), 100 / 450_000, () => false).length, 8000);
});
