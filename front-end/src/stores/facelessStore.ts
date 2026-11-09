import { create } from "zustand";
import { api } from "../lib/ipc";
import type { FacelessProduction } from "../lib/types";

/** Counts refreshes: only the latest one's answer is kept (GAP-003). */
let generation = 0;

/** The open project's faceless production (null for an ordinary project), and where it is read from. */
export const useFacelessStore = create<{
  production: FacelessProduction | null;
  /** The project folder `production` was read for. */
  path: string | null;
  refresh: (path: string | null) => Promise<void>;
}>((set) => ({
  production: null,
  path: null,
  refresh: async (path) => {
    const mine = ++generation;
    if (!path) {
      set({ production: null, path: null });
      return;
    }
    // Another project's state never shows while this one is read.
    set((s) => (s.path === path ? s : { production: null, path }));
    const production = await api.facelessStatus().catch(() => null);
    // A later refresh (another project, a close, or a newer read of this one) wins.
    if (mine !== generation) return;
    set({ production, path });
  },
}));
