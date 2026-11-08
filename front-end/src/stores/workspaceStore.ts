import { create } from "zustand";
import { setWorkspaceSwitcher, type Workspace } from "../components/layout/dockLayout";

const STORAGE_KEY = "aeroedits.workspace.v1";

function load(): Workspace {
  try {
    const stored = window.localStorage.getItem(STORAGE_KEY);
    if (stored === "edit" || stored === "cleanup" || stored === "shorts" || stored === "clips" || stored === "faceless")
      return stored;
  } catch {
    // Storage unavailable: start in Edit.
  }
  return "edit";
}

/**
 * The editor window's workspace (Edit, Cleanup, Shorts, Clips or Faceless), remembered between
 * sessions. Faceless is only for a faceless production; the app moves to Edit for other projects.
 */
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

setWorkspaceSwitcher((workspace) => useWorkspaceStore.getState().setWorkspace(workspace));
