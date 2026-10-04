// Reading the timeline (src-tauri/src/sequence): tracks of clips of assets' streams.
import type { Asset, Clip, OpenedProject, Role, SeqTrack, Sequence, Stream } from "./types";

/** The name transcripts, waveforms and pauses use for a stream: `<asset>.<stream>`. */
export const streamKey = (asset: string, stream: string) => `${asset}.${stream}`;

export const clipEnd = (clip: Pick<Clip, "startUs" | "durationUs">) => clip.startUs + clip.durationUs;

export const videoTracks = (sequence: Sequence) => sequence.tracks.filter((t) => t.kind === "video");
export const audioTracks = (sequence: Sequence) => sequence.tracks.filter((t) => t.kind === "audio");

/** "V1", "A2": the track's number among its kind. */
export function trackNumber(sequence: Sequence, trackId: string): string {
  const track = sequence.tracks.find((t) => t.id === trackId);
  if (!track) return "";
  const index = sequence.tracks.filter((t) => t.kind === track.kind).findIndex((t) => t.id === trackId);
  return `${track.kind === "video" ? "V" : "A"}${index + 1}`;
}

/** The user's name for a track, else its number. */
export const trackLabel = (sequence: Sequence, track: SeqTrack) => track.name || trackNumber(sequence, track.id);

export const assetById = (project: OpenedProject | null | undefined, id: string): Asset | undefined =>
  project?.assets.find((a) => a.id === id);

export const streamOf = (project: OpenedProject | null | undefined, clip: Pick<Clip, "asset" | "stream">): Stream | undefined =>
  assetById(project, clip.asset)?.streams.find((s) => s.id === clip.stream);

/** What a clip plays as: its track's role (when it suits), else its stream's. */
export function clipRole(project: OpenedProject | null | undefined, track: SeqTrack, clip: Clip): Role | undefined {
  const stream = streamOf(project, clip);
  if (!stream) return undefined;
  const picture = (role: Role) => role === "screen" || role === "webcam" || role === "overlay";
  if (track.role && picture(track.role) === (stream.kind === "picture")) return track.role;
  return stream.role;
}

/** "Recording · Microphone", or just the file name for a file's only stream of a kind. */
export function clipName(project: OpenedProject | null | undefined, clip: Clip): string {
  const asset = assetById(project, clip.asset);
  if (!asset) return "Missing media";
  const stream = asset.streams.find((s) => s.id === clip.stream);
  const sameKind = asset.streams.filter((s) => s.kind === stream?.kind).length;
  if (!stream || (asset.kind !== "recording" && sameKind <= 1)) return asset.name;
  return `${asset.name} · ${stream.name}`;
}

export function findClip(sequence: Sequence, clipId: string): { track: SeqTrack; clip: Clip } | undefined {
  for (const track of sequence.tracks) {
    const clip = track.clips.find((c) => c.id === clipId);
    if (clip) return { track, clip };
  }
  return undefined;
}

/** `ids` with every clip linked to one of them (leaving out locked tracks: they never change). */
export function withPartners(sequence: Sequence, ids: Iterable<string>): string[] {
  const chosen = new Set(ids);
  const links = new Set<string>();
  for (const track of sequence.tracks) for (const clip of track.clips) if (chosen.has(clip.id) && clip.link) links.add(clip.link);
  for (const track of sequence.tracks) {
    if (track.locked) continue;
    for (const clip of track.clips) if (clip.link && links.has(clip.link)) chosen.add(clip.id);
  }
  return [...chosen];
}

export const clipAt = (track: SeqTrack, us: number) => track.clips.find((c) => c.startUs <= us && us < clipEnd(c));

/** Every clip edge, sorted and unique, with 0. */
export function sequenceEdges(sequence: Sequence): number[] {
  const edges = new Set<number>([0]);
  for (const track of sequence.tracks) for (const clip of track.clips) {
    edges.add(clip.startUs);
    edges.add(clipEnd(clip));
  }
  return [...edges].sort((a, b) => a - b);
}

/**
 * The time on `assetId`'s own clock at timeline time `editedUs`, through its topmost picture
 * clip there (its screen first): where zooms and webcam focus are anchored.
 */
export function sourceAt(project: OpenedProject, assetId: string, editedUs: number, role: Role = "screen"): number | null {
  const tracks = videoTracks(project.sequence).reverse();
  const pick = (want?: Role) => {
    for (const track of tracks) {
      const clip = clipAt(track, editedUs);
      if (clip && clip.asset === assetId && (!want || clipRole(project, track, clip) === want)) {
        return clip.inUs + (editedUs - clip.startUs);
      }
    }
    return null;
  };
  return pick(role) ?? pick();
}

/** Where cut time can be put back on a track: two clips of one source side by side with time missing between. */
export function cutJoins(project: OpenedProject, track: SeqTrack): { clipId: string; atUs: number; gapUs: number }[] {
  const joins = [];
  for (let i = 0; i + 1 < track.clips.length; i++) {
    const left = track.clips[i];
    const right = track.clips[i + 1];
    const still = assetById(project, left.asset)?.kind === "image";
    if (
      !still &&
      left.asset === right.asset &&
      left.stream === right.stream &&
      clipEnd(left) === right.startUs &&
      left.inUs + left.durationUs < right.inUs
    ) {
      joins.push({ clipId: left.id, atUs: right.startUs, gapUs: right.inUs - (left.inUs + left.durationUs) });
    }
  }
  return joins;
}

/** A sound that can be transcribed, scanned for pauses or captioned. */
export interface SoundSource {
  key: string;
  label: string;
  speech: boolean;
  /** Some clip plays it on the timeline. */
  placed: boolean;
}

/** Every sound stream in the project: speech first, then what is on the timeline. */
export function soundSources(project: OpenedProject | null | undefined): SoundSource[] {
  if (!project) return [];
  const placed = new Set(project.sequence.tracks.flatMap((t) => t.clips.map((c) => streamKey(c.asset, c.stream))));
  const sounds = project.assets.flatMap((asset) =>
    asset.streams
      .filter((s) => s.kind === "sound")
      .map((stream) => {
        const key = streamKey(asset.id, stream.id);
        const several = asset.streams.filter((s) => s.kind === "sound").length > 1;
        return {
          key,
          label: asset.kind === "recording" || several ? `${shortLabel(asset.name)} · ${stream.name}` : shortLabel(asset.name, 32),
          speech: stream.role === "mic",
          placed: placed.has(key),
        };
      }),
  );
  return sounds.sort((a, b) => Number(b.speech) - Number(a.speech) || Number(b.placed) - Number(a.placed));
}

/** `text` cut to `max` characters with an ellipsis, so lists and pickers keep their width. */
export function shortLabel(text: string, max = 24): string {
  return text.length <= max ? text : `${text.slice(0, max - 1).trimEnd()}…`;
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

/** How long a clip of this asset starts out (images show for 5 s). */
export const defaultClipUs = (asset: Asset) => (asset.kind === "image" ? Math.min(5_000_000, asset.durationUs) : asset.durationUs);

/** The kinds of track an asset's streams go on. */
export const assetKinds = (asset: Asset | undefined) => ({
  video: !!asset?.streams.some((s) => s.kind === "picture"),
  audio: !!asset?.streams.some((s) => s.kind === "sound"),
});

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
