import test from "node:test";
import assert from "node:assert/strict";
import { buildTimeIndex, findAt, rowOffsets, visibleRows } from "../src/lib/transcriptIndex";

test("the playing span is found by time, skipping cut ones and the gaps between", () => {
  const spans = [
    { start: 0, end: 100 },
    { start: null, end: null }, // cut
    { start: 150, end: 300 },
    { start: 300, end: 400 },
  ];
  const index = buildTimeIndex(spans);
  assert.equal(findAt(index, 0), 0);
  assert.equal(findAt(index, 99), 0);
  assert.equal(findAt(index, 120), -1, "between words");
  assert.equal(findAt(index, 150), 2);
  assert.equal(findAt(index, 300), 3, "a span's end is the next one's start");
  assert.equal(findAt(index, 400), -1, "after the last");
  assert.equal(findAt(index, -5), -1, "before the first");
  assert.equal(findAt(buildTimeIndex([]), 10), -1);
});

test("spans out of time order (reordered clips) are still found", () => {
  const index = buildTimeIndex([
    { start: 500, end: 600 },
    { start: 0, end: 100 },
    { start: 200, end: 300 },
  ]);
  assert.equal(findAt(index, 50), 1);
  assert.equal(findAt(index, 250), 2);
  assert.equal(findAt(index, 550), 0);
});

test("a long transcript's index matches walking every word", () => {
  const spans = Array.from({ length: 9500 }, (_, i) =>
    i % 7 === 3 ? { start: null, end: null } : { start: i * 68, end: i * 68 + 55 },
  );
  const index = buildTimeIndex(spans);
  for (let t = 0; t < 9500 * 68; t += 37) {
    const walked = spans.findIndex((s) => s.start !== null && s.end !== null && t >= s.start && t < s.end);
    assert.equal(findAt(index, t), walked, `at ${t}`);
  }
});

test("only the rows between the top and bottom are drawn", () => {
  const heights = [40, 60, 40, 80, 40];
  const offsets = rowOffsets(heights.length, (i) => heights[i]);
  assert.deepEqual([...offsets], [0, 40, 100, 140, 220, 260]);
  assert.deepEqual(visibleRows(offsets, 0, 50), [0, 2]);
  assert.deepEqual(visibleRows(offsets, 100, 150), [2, 4]);
  assert.deepEqual(visibleRows(offsets, 230, 1000), [4, 5]);
  assert.deepEqual(visibleRows(offsets, -500, 10), [0, 1], "above the list: the first row");
  assert.deepEqual(visibleRows(offsets, 900, 1000), [4, 5], "below the list: the last row");
  assert.deepEqual(visibleRows(rowOffsets(0, () => 10), 0, 100), [0, 0]);
});
