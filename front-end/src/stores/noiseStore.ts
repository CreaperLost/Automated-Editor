import { create } from "zustand";
import type { FoundNoise, NoiseScan } from "../lib/types";

/** A noise's id in a scan: where it starts in its sound. */
export const noiseId = (noise: FoundNoise) => String(noise.sourceStartUs);

/// The mouth-noise pass of Jump cuts: what was found in which sound, and which are chosen.
interface NoiseStore {
  /** The pass is showing (instead of a cut pass). */
  open: boolean;
  /** 0 finds only obvious noises, 1 faint ones too. */
  sensitivity: number;
  /** Room muted either side of each noise, in ms. */
  paddingMs: number;
  /** The sound scanned (`<asset>.<stream>`); unset: the first speech on the timeline. */
  key?: string;
  scan: NoiseScan | null;
  selected: Record<string, boolean>;
  busy: boolean;
  error: string | null;
  notice: string | null;
  generation: number;
  setOpen: (open: boolean) => void;
  configure: (patch: Partial<Pick<NoiseStore, "sensitivity" | "paddingMs" | "key">>) => void;
  begin: () => number;
  finish: (generation: number, scan: NoiseScan) => void;
  fail: (generation: number, error: string) => void;
  toggle: (id: string) => void;
  selectAll: (selected: boolean) => void;
  /** After muting (or unmuting) these, at project revision `revision`. */
  applied: (ids: string[], muted: boolean, revision: number, notice: string) => void;
}

/** Chosen to start with: noises (breaths too) clear of speech, not yet muted. */
const preselected = (scan: NoiseScan) =>
  Object.fromEntries(scan.noises.map((noise) => [noiseId(noise), !noise.nearSpeech && !noise.muted]));

export const useNoiseStore = create<NoiseStore>((set, get) => ({
  open: false,
  sensitivity: 0.5,
  paddingMs: 20,
  scan: null,
  selected: {},
  busy: false,
  error: null,
  notice: null,
  generation: 0,
  setOpen: (open) => set({ open }),
  configure: (patch) => set({ ...patch, scan: null, selected: {}, error: null, notice: null, generation: get().generation + 1 }),
  begin: () => {
    const generation = get().generation + 1;
    set({ generation, busy: true, error: null, notice: null });
    return generation;
  },
  finish: (generation, scan) => {
    if (generation !== get().generation) return;
    set({ busy: false, scan, selected: preselected(scan) });
  },
  fail: (generation, error) => {
    if (generation !== get().generation) return;
    set({ busy: false, error });
  },
  toggle: (id) => set((state) => ({ selected: { ...state.selected, [id]: !state.selected[id] } })),
  selectAll: (selected) =>
    set((state) => ({
      selected: Object.fromEntries((state.scan?.noises ?? []).map((noise) => [noiseId(noise), selected && !noise.muted])),
    })),
  applied: (ids, muted, revision, notice) =>
    set((state) => {
      if (!state.scan) return {};
      const changed = new Set(ids);
      return {
        notice,
        scan: {
          ...state.scan,
          revision,
          noises: state.scan.noises.map((noise) => (changed.has(noiseId(noise)) ? { ...noise, muted } : noise)),
        },
        selected: Object.fromEntries(Object.entries(state.selected).map(([id, on]) => [id, on && !changed.has(id)])),
      };
    }),
}));
