import type { MediaAsset, OpenedProject, OverlayClip, OverlayTrack, SoundRole } from "./types";

/** A row of the timeline a clip can be dropped on; "new" rows add a video or audio track. */
export type TrackRow =
  | { kind: "main" }
  | { kind: "new"; audio: boolean }
  | { kind: "track"; trackId: string };

/** Rows carry `data-track-row`: "main", "new", "new-audio", or "track:<id>". */
export function rowFromElement(element: Element | null): TrackRow | null {
  const row = element?.closest("[data-track-row]")?.getAttribute("data-track-row");
  if (!row) return null;
  if (row === "main") return { kind: "main" };
  if (row === "new" || row === "new-audio") return { kind: "new", audio: row === "new-audio" };
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
  if (a.kind === "new") return a.audio === (b as { audio: boolean }).audio;
  return a.kind !== "track" || a.trackId === (b as { trackId: string }).trackId;
}

export const isAudioTrack = (track: OverlayTrack | undefined) => track?.kind === "audio";
export const videoTracks = (tracks: OverlayTrack[]) => tracks.filter((t) => !isAudioTrack(t));
export const audioTracks = (tracks: OverlayTrack[]) => tracks.filter(isAudioTrack);

/** "V2" for the first video track above the main sequence (V1); "A1" for the first audio track. */
export function trackLabel(tracks: OverlayTrack[], trackId: string): string {
  const track = tracks.find((t) => t.id === trackId);
  return isAudioTrack(track)
    ? `A${audioTracks(tracks).indexOf(track!) + 1}`
    : `V${videoTracks(tracks).findIndex((t) => t.id === trackId) + 2}`;
}

/** How many audio streams the media has. */
export function audioStreamCount(asset: MediaAsset | undefined): number {
  return asset?.audioPath ? 1 + (asset.extraAudioPaths?.length ?? 0) : 0;
}

/** What a stream is: as set, else a video's first stream is speech and the rest background. */
export function soundRole(asset: MediaAsset | undefined, stream: number): SoundRole {
  return asset?.soundRoles?.[stream] ?? (stream === 0 && asset?.kind === "video" ? "mic" : "background");
}

/** Sound that can be transcribed: the recording's mic and system tracks, then imported
 *  streams (speech first). Ids match the backend: `msound-<stream>-<asset>` for imports. */
export function transcribableSounds(project: OpenedProject | null): { id: string; label: string; speech: boolean }[] {
  if (!project) return [];
  const recorded = project.tracks
    .filter((t) => t.descriptor.trackType === "mic_audio" || t.descriptor.trackType === "system_audio")
    .map((t) => ({
      id: t.descriptor.id,
      label: `${t.descriptor.trackType === "mic_audio" ? "Microphone" : "System audio"} (${t.descriptor.id})`,
      speech: t.descriptor.trackType === "mic_audio",
    }));
  const imported = (project.mediaAssets ?? []).flatMap((asset) =>
    Array.from({ length: audioStreamCount(asset) }, (_, stream) => ({
      id: `msound-${stream}-${asset.id}`,
      label: `${shortLabel(asset.name)} · ${audioStreamName(asset, stream)}`,
      // V1's lane mark wins over the file's own role.
      speech: (project.audio?.tracks?.[`main-sound-${stream + 1}`]?.role ?? soundRole(asset, stream)) === "mic",
    })),
  );
  // Speech first: that is what transcripts and captions are for.
  return [...recorded, ...imported].sort((a, b) => Number(b.speech) - Number(a.speech));
}

/** A lane of sound in the mix, by the id the backend plays it under. */
export interface SoundLane {
  id: string;
  label: string;
  /** What it carries unless marked otherwise. */
  defaultRole: SoundRole;
  /** A recording's own track ("mic" or "system"); absent for imported sound. */
  recorded?: "mic" | "system";
  /** An audio track (A1, A2...) mutes as a track. */
  trackId?: string;
}

/** Every sound lane, as the timeline shows them: the recording's tracks, V1's and the video
 *  tracks' imported sound per stream, then the audio tracks. */
export function soundLanes(project: OpenedProject | null): SoundLane[] {
  if (!project) return [];
  const assetOf = (id: string) => project.mediaAssets?.find((a) => a.id === id);
  const overlay = project.overlayTracks ?? [];
  const recorded = project.tracks
    .filter((t) => t.descriptor.trackType === "mic_audio" || t.descriptor.trackType === "system_audio")
    .map((t): SoundLane => {
      const mic = t.descriptor.trackType === "mic_audio";
      return {
        id: t.descriptor.id,
        label: mic ? "Microphone" : "System audio",
        defaultRole: mic ? "mic" : "background",
        recorded: mic ? "mic" : "system",
      };
    });
  // V1's lanes: a lane per stream, named like the first file that has one.
  let cursor = 0;
  const firstOnV1: (MediaAsset | undefined)[] = [];
  for (const interval of project.retainedIntervals) {
    const asset = interval.media && !interval.audioUnlinked ? assetOf(interval.media) : undefined;
    for (let k = 0; k < audioStreamCount(asset); k++) firstOnV1[k] ??= asset;
    cursor += interval.endUs - interval.startUs;
  }
  const main = firstOnV1.map((asset, k): SoundLane => ({
    id: `main-sound-${k + 1}`,
    label: `V1 sound ${k + 1}`,
    defaultRole: soundRole(asset, k),
  }));
  const onTracks = videoTracks(overlay).flatMap((track) => {
    const linked = track.clips.filter((c) => !c.audioUnlinked);
    const streams = Math.max(0, ...linked.map((c) => audioStreamCount(assetOf(c.assetId))));
    return Array.from({ length: streams }, (_, k): SoundLane => ({
      id: `${track.id}-sound-${k + 1}`,
      label: `${trackLabel(overlay, track.id)} sound ${k + 1}`,
      defaultRole: soundRole(assetOf(linked.find((c) => audioStreamCount(assetOf(c.assetId)) > k)?.assetId ?? ""), k),
    }));
  });
  const audio = audioTracks(overlay).map((track): SoundLane => {
    const clip = track.clips[0];
    return {
      id: track.id,
      label: trackLabel(overlay, track.id),
      defaultRole: clip ? soundRole(assetOf(clip.assetId), clip.audioStream ?? 0) : "background",
      trackId: track.id,
    };
  });
  return [...recorded, ...main, ...onTracks, ...audio];
}

/** What a lane carries: as marked on the timeline, else its default. */
export function laneRole(project: OpenedProject | null, lane: SoundLane): SoundRole {
  return project?.audio?.tracks?.[lane.id]?.role ?? lane.defaultRole;
}

/** "Mic", or "Audio 2" when the stream has no real name (encoders' handler names don't count). */
export function audioStreamName(asset: MediaAsset | undefined, stream: number): string {
  const name = asset?.audioNames?.[stream];
  return name && !/handler/i.test(name) ? shortLabel(name, 18) : `Audio ${stream + 1}`;
}

/** `text` cut to `max` characters with an ellipsis, so lists and pickers keep their width. */
export function shortLabel(text: string, max = 24): string {
  return text.length <= max ? text : `${text.slice(0, max - 1).trimEnd()}…`;
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
