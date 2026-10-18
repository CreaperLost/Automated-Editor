import type { WindowCropOff } from "./types";

/** `ranges` with `[start, end)` of recording `media` taken out (unset media is `first`). */
export function withoutStretch(
  ranges: WindowCropOff[],
  media: string,
  start: number,
  end: number,
  first?: string,
): WindowCropOff[] {
  return ranges.flatMap((range) => {
    if ((range.media ?? first) !== media || range.sourceEndUs <= start || range.sourceStartUs >= end) return [range];
    const kept: WindowCropOff[] = [];
    if (range.sourceStartUs < start) kept.push({ ...range, sourceEndUs: start });
    if (range.sourceEndUs > end) kept.push({ ...range, sourceStartUs: end });
    return kept;
  });
}

/** Whether the window crop is off for all of `[start, end)` of recording `media`. */
export function cropOffFor(
  ranges: WindowCropOff[],
  media: string,
  start: number,
  end: number,
  first?: string,
): boolean {
  return ranges.some(
    (range) => (range.media ?? first) === media && range.sourceStartUs <= start && range.sourceEndUs >= end,
  );
}

/** `ranges` with the window crop switched on or off for `[start, end)` of recording `media`. */
export function setCropFor(
  ranges: WindowCropOff[],
  media: string,
  start: number,
  end: number,
  on: boolean,
  first?: string,
): WindowCropOff[] {
  const rest = withoutStretch(ranges, media, start, end, first);
  return on ? rest : [...rest, { media, sourceStartUs: start, sourceEndUs: end }];
}
