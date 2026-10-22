/**
 * Indexes for a long transcript, built once when the words change, so playback and scrolling
 * look things up by binary search instead of walking every word on every frame.
 */

/** Spans sorted by start time; `items[k]` is the caller's index of the k-th span. */
export interface TimeIndex {
  starts: Float64Array;
  ends: Float64Array;
  items: Int32Array;
}

/** Indexes the spans that have a time (`start` and `end` not null), sorted by start. */
export function buildTimeIndex(spans: { start: number | null; end: number | null }[]): TimeIndex {
  const kept: number[] = [];
  spans.forEach((s, i) => {
    if (s.start !== null && s.end !== null) kept.push(i);
  });
  // Playback order is nearly always time order already; sort only when it is not.
  let sorted = true;
  for (let k = 1; k < kept.length && sorted; k++) sorted = spans[kept[k - 1]].start! <= spans[kept[k]].start!;
  if (!sorted) kept.sort((a, b) => spans[a].start! - spans[b].start!);
  return {
    starts: Float64Array.from(kept, (i) => spans[i].start!),
    ends: Float64Array.from(kept, (i) => spans[i].end!),
    items: Int32Array.from(kept),
  };
}

/** The span playing at `t` (start ≤ t < end), as the caller's index; -1 when none is. */
export function findAt(index: TimeIndex, t: number): number {
  // The last span starting at or before t.
  let lo = 0;
  let hi = index.starts.length - 1;
  let found = -1;
  while (lo <= hi) {
    const mid = (lo + hi) >> 1;
    if (index.starts[mid] <= t) {
      found = mid;
      lo = mid + 1;
    } else {
      hi = mid - 1;
    }
  }
  return found >= 0 && t < index.ends[found] ? index.items[found] : -1;
}

/** Where each row starts, from the top, and the total height after: `count + 1` values. */
export function rowOffsets(count: number, heightOf: (row: number) => number): Float64Array {
  const offsets = new Float64Array(count + 1);
  for (let i = 0; i < count; i++) offsets[i + 1] = offsets[i] + heightOf(i);
  return offsets;
}

/** The rows `[first, last)` that show between `top` and `bottom` (pixels from the top). */
export function visibleRows(offsets: Float64Array, top: number, bottom: number): [number, number] {
  const count = offsets.length - 1;
  if (count <= 0) return [0, 0];
  // The first row ending below `top`, and the first row starting at or below `bottom`.
  const firstEndingAfter = (y: number) => {
    let lo = 0;
    let hi = count;
    while (lo < hi) {
      const mid = (lo + hi) >> 1;
      if (offsets[mid + 1] <= y) lo = mid + 1;
      else hi = mid;
    }
    return lo;
  };
  const first = Math.min(firstEndingAfter(top), count - 1);
  let last = first;
  while (last < count && offsets[last] < bottom) last++;
  return [first, Math.max(last, first + 1)];
}
