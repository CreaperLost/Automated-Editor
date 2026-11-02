import test from "node:test";
import assert from "node:assert/strict";
import { errorMessage, formatTime } from "../src/lib/format";

test("a time reads as minutes and seconds and never rounds up to :60", () => {
  assert.equal(formatTime(0), "0:00.0");
  assert.equal(formatTime(65_300_000), "1:05.3");
  assert.equal(formatTime(59_960_000), "1:00.0");
  assert.equal(formatTime(119_999_000, 2), "2:00.00");
  assert.equal(formatTime(3_250_000, 2), "0:03.25");
  assert.equal(formatTime(-5), "0:00.0");
});

test("a failure is told in its own words, else the fallback's", () => {
  assert.equal(errorMessage(new Error("Disk full")), "Disk full");
  assert.equal(errorMessage("Stale project handle"), "Stale project handle");
  assert.equal(errorMessage("  ", "Finding noises failed."), "Finding noises failed.");
  assert.equal(errorMessage(undefined), "Something went wrong.");
});
