export function editedToSourceUs(
  retained: { startUs: number; endUs: number }[],
  editedUs: number,
): number | null {
  let accumulated = 0;
  for (const interval of retained) {
    const duration = interval.endUs - interval.startUs;
    if (editedUs < accumulated + duration) {
      return interval.startUs + (editedUs - accumulated);
    }
    accumulated += duration;
  }
  return null;
}

type Range = { startUs: number; endUs: number };

/** A stretch of the edited timeline between two clip edges. */
export interface TimelineClip {
  startUs: number;
  endUs: number;
  sourceStartUs: number;
}

/** A removed source range, drawn against the clip it would grow back onto. */
export interface CutMarker {
  editedUs: number;
  sourceStartUs: number;
  sourceEndUs: number;
  /** Which clip grows on restore: the one ending where the cut starts, or starting where it ends. */
  grow: "end" | "start";
}

/** Splits retained media into clips at cut edges and at the user's split points. */
export function buildClips(retained: Range[], splitPointsUs: number[] = []): TimelineClip[] {
  const clips: TimelineClip[] = [];
  let edited = 0;
  for (const interval of retained) {
    const edges = [
      interval.startUs,
      ...splitPointsUs.filter((p) => p > interval.startUs && p < interval.endUs),
      interval.endUs,
    ];
    for (let i = 0; i + 1 < edges.length; i++) {
      const length = edges[i + 1] - edges[i];
      clips.push({ startUs: edited, endUs: edited + length, sourceStartUs: edges[i] });
      edited += length;
    }
  }
  return clips;
}

/**
 * Places each removed range at the end of the clip it follows in the recording, or else at
 * the start of the clip it precedes. Clips can be reordered, so this is found per clip
 * rather than by summing everything earlier in the recording.
 */
export function buildCutMarkers(retained: Range[], removed: Range[] = []): CutMarker[] {
  const placed: { interval: Range; editedStart: number }[] = [];
  let edited = 0;
  for (const interval of retained) {
    placed.push({ interval, editedStart: edited });
    edited += interval.endUs - interval.startUs;
  }
  return removed.map((cut) => {
    const before = placed.find((p) => p.interval.endUs === cut.startUs);
    const after = placed.find((p) => p.interval.startUs === cut.endUs);
    const base = { sourceStartUs: cut.startUs, sourceEndUs: cut.endUs };
    if (before) {
      return { ...base, grow: "end", editedUs: before.editedStart + before.interval.endUs - before.interval.startUs };
    }
    if (after) return { ...base, grow: "start", editedUs: after.editedStart };
    // Not next to any clip (e.g. bounded by recorder pauses): before the next clip in the recording.
    const next = placed.find((p) => p.interval.startUs > cut.startUs);
    return { ...base, grow: "end", editedUs: next ? next.editedStart : edited };
  });
}

/** Edit points on the edited timeline: 0, every clip edge, and the end. */
export function clipEdges(clips: TimelineClip[]): number[] {
  const edges = new Set<number>([0]);
  for (const clip of clips) {
    edges.add(clip.startUs);
    edges.add(clip.endUs);
  }
  return [...edges].sort((a, b) => a - b);
}

/** A clip keeps at least this much media when its edge is dragged inward. */
export const MIN_CLIP_US = 10_000;

/**
 * How far a clip edge may be dragged, in edited microseconds. Dragging inward
 * ripple-deletes media; dragging outward brings back removed media next to the
 * edge, which only exists where the edge is a cut rather than a split.
 */
export function clipTrimLimits(
  clip: TimelineClip,
  side: "start" | "end",
  retained: Range[],
  removed: Range[] = [],
): { minDeltaUs: number; maxDeltaUs: number } {
  const length = clip.endUs - clip.startUs;
  const inward = Math.max(0, length - MIN_CLIP_US);
  if (side === "start") {
    const atCut = retained.some((interval) => interval.startUs === clip.sourceStartUs);
    const before = atCut ? removed.find((range) => range.endUs === clip.sourceStartUs) : undefined;
    return { minDeltaUs: before ? -(before.endUs - before.startUs) : 0, maxDeltaUs: inward };
  }
  const sourceEndUs = clip.sourceStartUs + length;
  const atCut = retained.some((interval) => interval.endUs === sourceEndUs);
  const after = atCut ? removed.find((range) => range.startUs === sourceEndUs) : undefined;
  return { minDeltaUs: -inward, maxDeltaUs: after ? after.endUs - after.startUs : 0 };
}

const RULER_STEPS_US = [
  100_000, 200_000, 500_000, 1_000_000, 2_000_000, 5_000_000, 10_000_000, 15_000_000,
  30_000_000, 60_000_000, 120_000_000, 300_000_000, 600_000_000, 1_800_000_000, 3_600_000_000,
];

/** The smallest ruler step that keeps labels at least `minPx` apart. */
export function rulerStepUs(pxPerUs: number, minPx = 72): number {
  if (!(pxPerUs > 0)) return RULER_STEPS_US[RULER_STEPS_US.length - 1];
  return RULER_STEPS_US.find((step) => step * pxPerUs >= minPx) ?? RULER_STEPS_US[RULER_STEPS_US.length - 1];
}

/** `m:ss`, with tenths when the step is under a second. */
export function formatRulerLabel(timeUs: number, stepUs: number): string {
  const tenths = Math.round(timeUs / 100_000);
  const minutes = Math.floor(tenths / 600);
  const seconds = String(Math.floor((tenths % 600) / 10)).padStart(2, "0");
  return stepUs < 1_000_000 ? `${minutes}:${seconds}.${tenths % 10}` : `${minutes}:${seconds}`;
}
