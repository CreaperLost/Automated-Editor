import React, { useEffect, useSyncExternalStore } from "react";
import { api } from "../../lib/ipc";
import { clipOut, clipSourceAt, clipSpeed } from "../../lib/sequence";
import type { Clip } from "../../lib/types";

/** Below this inner height a clip shows no pictures: its name needs the room. */
export const MIN_THUMB_LANE_PX = 32;
/** Times asked for in one call (the backend's limit too). */
const BATCH = 64;
/** Pictures kept in memory, the oldest going first (the backend keeps them on disk). */
const KEEP = 2_000;
/** Source times are rounded to one of these steps (seconds), so zooming a little or moving a
 * clip asks for pictures already made. */
const STEPS_S = [0.25, 0.5, 1, 2, 5, 10, 15, 30, 60, 120, 300, 600, 1200, 3600];

/** Made pictures (a data URL, or null where there is none) by key. */
const made = new Map<string, string | null>();
/** How many clips on show want each key: only those are asked for. */
const wanted = new Map<string, number>();
/** Keys asked for and not answered yet. */
const asked = new Set<string>();
let version = 0;
const listeners = new Set<() => void>();
let flushTimer: number | undefined;
let busy = false;

const notify = () => {
  version += 1;
  for (const listener of listeners) listener();
};
const subscribe = (listener: () => void) => {
  listeners.add(listener);
  return () => listeners.delete(listener);
};

const keyOf = (handle: string, asset: string, stream: string, height: number, atUs: number) =>
  `${handle}\n${asset}\n${stream}\n${height}\n${atUs}`;

/**
 * Asks for the wanted pictures not made yet, one call at a time per run. Pictures that
 * scrolled away before their turn are never asked for.
 */
function flush() {
  flushTimer = undefined;
  if (busy) return;
  const groups = new Map<string, { handle: string; asset: string; stream: string; height: number; keys: string[]; times: number[] }>();
  for (const [key, count] of wanted) {
    if (count <= 0 || made.has(key) || asked.has(key)) continue;
    const [handle, asset, stream, height, at] = key.split("\n");
    const group = `${handle}\n${asset}\n${stream}\n${height}`;
    let entry = groups.get(group);
    if (!entry) {
      entry = { handle, asset, stream, height: Number(height), keys: [], times: [] };
      groups.set(group, entry);
    }
    if (entry.keys.length < BATCH) {
      entry.keys.push(key);
      entry.times.push(Number(at));
    }
  }
  const next = groups.values().next();
  if (next.done) return;
  const { handle, asset, stream, height, keys, times } = next.value;
  busy = true;
  keys.forEach((k) => asked.add(k));
  void api
    .projectClipThumbnails(handle, asset, stream, times, height)
    .then((urls) => keys.forEach((k, i) => made.set(k, urls[i] ?? null)))
    .catch(() => keys.forEach((k) => made.set(k, null)))
    .finally(() => {
      keys.forEach((k) => asked.delete(k));
      while (made.size > KEEP) made.delete(made.keys().next().value as string);
      busy = false;
      notify();
      schedule();
    });
}

function schedule() {
  if (flushTimer === undefined) flushTimer = window.setTimeout(flush, 60);
}

interface ClipThumbsProps {
  projectHandle: string;
  clip: Clip;
  stream: string;
  /** A still: every slot shows the same picture. */
  still: boolean;
  /** Width over height of the picture. */
  aspect: number;
  /** The clip's inner height in pixels. */
  heightPx: number;
  pxPerUs: number;
  /** The part of the clip drawn (timeline time): only it gets pictures. */
  fromUs: number;
  toUs: number;
}

/**
 * Small pictures along a video clip (UI-1), made once per source and kept on disk by the
 * backend. Only the part near the view asks for them; a clip scrolled away stops asking.
 */
export const ClipThumbs = React.memo(function ClipThumbs({
  projectHandle,
  clip,
  stream,
  still,
  aspect,
  heightPx,
  pxPerUs,
  fromUs,
  toUs,
}: ClipThumbsProps) {
  useSyncExternalStore(subscribe, () => version);
  const dpr = typeof window === "undefined" ? 1 : Math.min(2, window.devicePixelRatio || 1);
  // Few sizes, so pictures are shared between lane heights.
  const height = Math.min(180, Math.ceil((heightPx * dpr) / 24) * 24);
  const slotPx = Math.max(16, heightPx * (aspect > 0 ? aspect : 16 / 9));
  const slotUs = pxPerUs > 0 ? slotPx / pxPerUs : 0;
  const sourceStepUs = slotUs * clipSpeed(clip);
  const roundUs = (STEPS_S.find((s) => s * 1e6 >= sourceStepUs) ?? STEPS_S[STEPS_S.length - 1]) * 1e6;
  const slots: { index: number; key: string }[] = [];
  if (slotUs > 0 && toUs > fromUs) {
    const first = Math.max(0, Math.floor((fromUs - clip.startUs) / slotUs));
    const last = Math.ceil((toUs - clip.startUs) / slotUs);
    const lastSourceUs = Math.max(clip.inUs, clipOut(clip) - 1);
    for (let index = first; index < last && slots.length < 400; index++) {
      const middle = clip.startUs + (index + 0.5) * slotUs;
      const sourceUs = Math.min(lastSourceUs, Math.max(clip.inUs, clipSourceAt(clip, Math.min(middle, clip.startUs + clip.durationUs))));
      const atUs = still ? 0 : Math.min(lastSourceUs, Math.round(sourceUs / roundUs) * roundUs);
      slots.push({ index, key: keyOf(projectHandle, clip.asset, stream, height, atUs) });
    }
  }
  const keys = slots.map((s) => s.key).join("|");

  useEffect(() => {
    const mine = keys ? [...new Set(keys.split("|"))] : [];
    for (const key of mine) wanted.set(key, (wanted.get(key) ?? 0) + 1);
    if (mine.some((key) => !made.has(key))) schedule();
    return () => {
      for (const key of mine) {
        const count = (wanted.get(key) ?? 1) - 1;
        if (count <= 0) wanted.delete(key);
        else wanted.set(key, count);
      }
    };
  }, [keys]);

  if (slots.length === 0) return null;
  return (
    <div aria-hidden className="absolute inset-0 pointer-events-none overflow-hidden">
      {slots.map(({ index, key }) => {
        const url = made.get(key);
        if (!url) return null;
        return (
          <img
            key={index}
            src={url}
            alt=""
            draggable={false}
            className="absolute inset-y-0 h-full object-cover opacity-60"
            style={{
              left: `${((index * slotUs) / clip.durationUs) * 100}%`,
              width: `${(slotUs / clip.durationUs) * 100}%`,
            }}
          />
        );
      })}
    </div>
  );
});
