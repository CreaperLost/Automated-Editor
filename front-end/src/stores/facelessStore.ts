import { create } from "zustand";
import { api } from "../lib/ipc";
import type { FacelessProduction } from "../lib/types";

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
    if (!path) {
      set({ production: null, path: null });
      return;
    }
    const production = await api.facelessStatus().catch(() => null);
    set({ production, path });
  },
}));
