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

/** A removed source range, drawn where it used to sit on the edited timeline. */
export interface CutMarker {
  editedUs: number;
  sourceStartUs: number;
  sourceEndUs: number;
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

export function buildCutMarkers(retained: Range[], removed: Range[] = []): CutMarker[] {
  return removed.map((cut) => ({
    editedUs: retained
      .filter((interval) => interval.startUs < cut.startUs)
      .reduce((sum, interval) => sum + Math.min(interval.endUs, cut.startUs) - interval.startUs, 0),
    sourceStartUs: cut.startUs,
    sourceEndUs: cut.endUs,
  }));
}
