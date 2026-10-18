import { create } from "zustand";
import type { FoundNoise, NoiseScan } from "../lib/types";

/** A place's id in a scan: where it starts in its sound. */
export const noiseId = (noise: FoundNoise) => String(noise.sourceStartUs);

/** Which places the list shows. */
export type NoiseFilter = "found" | "near" | "muted" | "all";

/// The Mouth noises panel: the last scan of a sound (kept with the project, also when an agent
/// ran it), which places are chosen, and the settings for the next scan.
interface NoiseStore {
  /** 0 finds only obvious noises, 1 faint ones too. */
  sensitivity: number;
  /** Room muted either side of each noise, in ms. */
  paddingMs: number;
  /** The sound shown (`<asset>.<stream>`); unset: the first speech on the timeline. */
  key?: string;
  scan: NoiseScan | null;
  selected: Record<string, boolean>;
  filter: NoiseFilter;
  /** Listening for noises (a new scan). */
  busy: boolean;
  error: string | null;
  notice: string | null;
  generation: number;
  configure: (patch: Partial<Pick<NoiseStore, "sensitivity" | "paddingMs">>) => void;
  chooseSound: (key: string) => void;
  setFilter: (filter: NoiseFilter) => void;
  begin: () => number;
  /** A new scan: what it chose to start with is chosen. */
  finish: (generation: number, scan: NoiseScan) => void;
  fail: (generation: number, error: string) => void;
  /** The kept scan read again (the project changed): choices of places still there stay. */
  reviewed: (scan: NoiseScan) => void;
  toggle: (id: string) => void;
  selectAll: (ids: string[], selected: boolean) => void;
  say: (notice: string | null) => void;
}

/** Chosen to start with: places (breaths too) clear of speech, not yet muted. */
const preselected = (scan: NoiseScan) =>
  Object.fromEntries(scan.noises.map((noise) => [noiseId(noise), !noise.nearSpeech && !noise.muted && noise.kind !== null]));

export const useNoiseStore = create<NoiseStore>((set, get) => ({
  sensitivity: 0.5,
  paddingMs: 20,
  scan: null,
  selected: {},
  filter: "found",
  busy: false,
  error: null,
  notice: null,
  generation: 0,
  configure: (patch) => set({ ...patch, notice: null }),
  // The scan still listening is of the last sound: its reply is ignored, so it is not waited on.
  chooseSound: (key) =>
    set({ key, scan: null, selected: {}, busy: false, error: null, notice: null, generation: get().generation + 1 }),
  setFilter: (filter) => set({ filter }),
  begin: () => {
    const generation = get().generation + 1;
    set({ generation, busy: true, error: null, notice: null });
    return generation;
  },
  finish: (generation, scan) => {
    if (generation !== get().generation) return;
    const unmuted = scan.noises.filter((n) => !n.muted && n.kind !== null);
    set({
      busy: false,
      scan,
      selected: preselected(scan),
      filter: unmuted.some((n) => !n.nearSpeech) ? "found" : unmuted.length ? "near" : "all",
    });
  },
  fail: (generation, error) => {
    if (generation !== get().generation) return;
    set({ busy: false, error });
  },
  reviewed: (scan) =>
    set((state) => {
      // Another sound's scan, or one first read: start from what the scan would choose.
      if (!state.scan || state.scan.key !== scan.key) {
        return {
          scan,
          selected: preselected(scan),
          ...(scan.config ? { sensitivity: scan.config.sensitivity, paddingMs: scan.config.paddingMs } : {}),
        };
      }
      const known = new Set(state.scan.noises.map(noiseId));
      const fresh = preselected(scan);
      return {
        scan,
        selected: Object.fromEntries(
          scan.noises.map((noise) => {
            const id = noiseId(noise);
            return [id, !noise.muted && (known.has(id) ? !!state.selected[id] : fresh[id])];
          }),
        ),
      };
    }),
  toggle: (id) => set((state) => ({ selected: { ...state.selected, [id]: !state.selected[id] } })),
  selectAll: (ids, selected) =>
    set((state) => ({ selected: { ...state.selected, ...Object.fromEntries(ids.map((id) => [id, selected])) } })),
  say: (notice) => set({ notice }),
}));
