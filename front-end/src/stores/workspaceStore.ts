import { create } from "zustand";
import type { Workspace } from "../components/layout/dockLayout";

const STORAGE_KEY = "aeroedits.workspace.v1";

function load(): Workspace {
  try {
    const stored = window.localStorage.getItem(STORAGE_KEY);
    if (stored === "edit" || stored === "cleanup") return stored;
  } catch {
    // Storage unavailable: start in Edit.
  }
  return "edit";
}

/** The editor window's workspace (Edit or Cleanup), remembered between sessions. */
export const useWorkspaceStore = create<{ workspace: Workspace; setWorkspace: (workspace: Workspace) => void }>(
  (set) => ({
    workspace: load(),
    setWorkspace: (workspace) => {
      try {
        window.localStorage.setItem(STORAGE_KEY, workspace);
      } catch {
        // Not remembering it is harmless.
      }
      set({ workspace });
    },
  }),
);
