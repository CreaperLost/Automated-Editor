import test from "node:test";
import assert from "node:assert/strict";
import { groupKey, modalOpen } from "../src/components/ui";

/** A key pressed in a group, with what the group did about it. */
function press(key: string, current: number, disabled: number[] = []) {
  const picked: string[] = [];
  let stopped = false;
  let prevented = false;
  const event = {
    key,
    preventDefault: () => (prevented = true),
    stopPropagation: () => (stopped = true),
    currentTarget: { querySelectorAll: () => [] },
  } as unknown as Parameters<typeof groupKey>[0];
  const handled = groupKey(event, ["a", "b", "c", "d"], (i) => !disabled.includes(i), current, (v) => picked.push(v));
  return { handled, picked, stopped, prevented };
}

// GAP-028
test("arrows, Home and End move within a group and skip what can't be chosen", () => {
  assert.deepEqual(press("ArrowRight", 0).picked, ["b"]);
  assert.deepEqual(press("ArrowRight", 0, [1]).picked, ["c"], "the disabled one is skipped");
  assert.deepEqual(press("ArrowLeft", 0).picked, ["d"], "wraps round");
  assert.deepEqual(press("Home", 2, [0]).picked, ["b"]);
  assert.deepEqual(press("End", 0, [3]).picked, ["c"]);
  assert.deepEqual(press("ArrowDown", 3).picked, ["a"]);
});

// GAP-019, GAP-028
test("a key the group handles never reaches the editor; others pass", () => {
  const arrow = press("ArrowLeft", 1);
  assert.ok(arrow.handled && arrow.stopped && arrow.prevented);
  const del = press("Delete", 1);
  assert.ok(!del.handled && !del.stopped && !del.prevented);
  assert.deepEqual(del.picked, []);
  assert.equal(modalOpen(), false, "no page, no dialog");
});
