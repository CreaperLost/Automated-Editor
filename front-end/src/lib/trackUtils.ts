import type { MediaAsset, OverlayClip, OverlayTrack } from "./types";

/** A row of the timeline a clip can be dropped on. */
export type TrackRow = { kind: "main" } | { kind: "new" } | { kind: "track"; trackId: string };

/** Rows carry `data-track-row`: "main", "new", or "track:<id>". */
export function rowFromElement(element: Element | null): TrackRow | null {
  const row = element?.closest("[data-track-row]")?.getAttribute("data-track-row");
  if (!row) return null;
  if (row === "main" || row === "new") return { kind: row };
  return row.startsWith("track:") ? { kind: "track", trackId: row.slice(6) } : null;
}

/** The row under a screen point, also while the pointer is captured by a dragged block. */
export function rowAtPoint(x: number, y: number): TrackRow | null {
  for (const element of document.elementsFromPoint(x, y)) {
    const row = rowFromElement(element);
    if (row) return row;
  }
  return null;
}

export function sameRow(a: TrackRow | null, b: TrackRow | null): boolean {
  if (!a || !b || a.kind !== b.kind) return false;
  return a.kind !== "track" || a.trackId === (b as { trackId: string }).trackId;
}

/** "V2" for the first track above the main sequence (V1). */
export function trackLabel(tracks: OverlayTrack[], trackId: string): string {
  return `V${tracks.findIndex((track) => track.id === trackId) + 2}`;
}

export function clipEndUs(clip: Pick<OverlayClip, "startUs" | "durationUs">): number {
  return clip.startUs + clip.durationUs;
}

/** Whether `[startUs, startUs + durationUs)` is free on the track, ignoring `ignoreId`. */
export function fitsOnTrack(
  track: OverlayTrack | undefined,
  startUs: number,
  durationUs: number,
  ignoreId?: string,
): boolean {
  if (!track || startUs < 0) return false;
  return track.clips.every(
    (clip) => clip.id === ignoreId || clipEndUs(clip) <= startUs || clip.startUs >= startUs + durationUs,
  );
}

/**
 * Snaps a block of `durationUs` starting near `startUs` so its start or end lands on a nearby
 * point (the playhead, clip edges), when within `snapUs`.
 */
export function snapStart(startUs: number, durationUs: number, points: number[], snapUs: number): number {
  let best = startUs;
  let bestDistance = snapUs;
  for (const point of points) {
    for (const candidate of [point, point - durationUs]) {
      const distance = Math.abs(candidate - startUs);
      if (distance < bestDistance) {
        best = candidate;
        bestDistance = distance;
      }
    }
  }
  return Math.max(0, Math.round(best));
}

/** How long a clip of this media starts out on a track (images show for 5 s). */
export function defaultClipUs(asset: MediaAsset): number {
  return asset.kind === "image" ? Math.min(5_000_000, asset.durationUs) : asset.durationUs;
}

/** The trimmed clip after moving one edge by `deltaUs`, kept within the media and the neighbours. */
export function trimClip(
  clip: OverlayClip,
  side: "start" | "end",
  deltaUs: number,
  asset: MediaAsset | undefined,
  track: OverlayTrack | undefined,
  minUs = 100_000,
): OverlayClip {
  const still = asset?.kind === "image";
  const maxLength = asset?.durationUs ?? clip.durationUs;
  const others = (track?.clips ?? []).filter((other) => other.id !== clip.id);
  const before = Math.max(0, ...others.filter((o) => clipEndUs(o) <= clip.startUs).map(clipEndUs));
  const after = Math.min(
    Number.MAX_SAFE_INTEGER,
    ...others.filter((o) => o.startUs >= clipEndUs(clip)).map((o) => o.startUs),
  );
  if (side === "end") {
    const room = still ? maxLength : maxLength - clip.inUs;
    const durationUs = Math.max(minUs, Math.min(clip.durationUs + deltaUs, room, after - clip.startUs));
    return { ...clip, durationUs: Math.round(durationUs) };
  }
  const end = clipEndUs(clip);
  // The start can move back only as far as the media (video) or the stretch limit (image) allows.
  const earliest = still ? Math.max(before, end - maxLength) : Math.max(before, clip.startUs - clip.inUs);
  const startUs = Math.round(Math.max(earliest, Math.min(clip.startUs + deltaUs, end - minUs)));
  return {
    ...clip,
    startUs,
    inUs: still ? 0 : clip.inUs + (startUs - clip.startUs),
    durationUs: end - startUs,
  };
}
