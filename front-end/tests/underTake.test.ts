import test from "node:test";
import assert from "node:assert/strict";
import { placement, videoLines } from "../src/components/shorts/underTakeParts";
import type { TranscriptViewWord } from "../src/lib/types";

const S = 1_000_000;
const word = (text: string, start: number, end: number, cut = false): TranscriptViewWord => ({
  id: `${text}-${start}`,
  text,
  kind: "word",
  sourceStartUs: start * S,
  sourceEndUs: end * S,
  editedStartUs: cut ? null : start * S,
  editedEndUs: cut ? null : end * S,
});

test("the video's lines end at a sentence, a pause or 15 s, and skip cut words", () => {
  const lines = videoLines([
    word("Open", 0, 0.3),
    word("Standex.", 0.35, 0.8),
    word("Then", 0.9, 1.1),
    word("um", 1.15, 1.3, true),
    word("click", 1.4, 1.7),
    // A pause.
    word("Next", 3, 3.2),
    word("step", 3.25, 3.5),
  ]);
  assert.deepEqual(
    lines.map((l) => [l.text, l.startUs / S, l.endUs / S]),
    [
      ["Open Standex.", 0, 0.8],
      ["Then click", 0.9, 1.7],
      ["Next step", 3, 3.5],
    ],
  );
  const long = Array.from({ length: 40 }, (_, i) => word("and", i * 0.5, i * 0.5 + 0.4));
  assert.ok(videoLines(long).every((l) => l.endUs - l.startUs <= 15 * S));
});

test("placing goes on the selection, else from the playhead, kept inside the short", () => {
  assert.deepEqual(placement({ startUs: 2 * S, endUs: 5 * S }, 9 * S, 30 * S, 4 * S), { atUs: 2 * S, toUs: 5 * S, selected: true });
  assert.deepEqual(placement(null, 9 * S, 30 * S, 4 * S), { atUs: 9 * S, toUs: 13 * S, selected: false });
  // Near the end it is cut to fit; at the very end nothing fits.
  assert.deepEqual(placement(null, 28 * S, 30 * S, 4 * S), { atUs: 28 * S, toUs: 30 * S, selected: false });
  assert.equal(placement(null, 30 * S, 30 * S, 4 * S), null);
  // A click on the timeline (an empty selection) is the playhead.
  assert.deepEqual(placement({ startUs: 4 * S, endUs: 4 * S }, 4 * S, 30 * S, 2 * S), { atUs: 4 * S, toUs: 6 * S, selected: false });
});
