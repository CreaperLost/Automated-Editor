import { api } from "./ipc";
import { useProjectStore } from "../stores/projectStore";

/** Undoes the last edit to the project (the header's Undo; the timeline has its own keys). */
export async function undoProject() {
  const { openedProject, applyOpenedProject } = useProjectStore.getState();
  if (!openedProject?.undoAvailable) return;
  applyOpenedProject(await api.projectUndo(openedProject.projectHandle, openedProject.revision));
}

/** Redoes the last undone edit. */
export async function redoProject() {
  const { openedProject, applyOpenedProject } = useProjectStore.getState();
  if (!openedProject?.redoAvailable) return;
  applyOpenedProject(await api.projectRedo(openedProject.projectHandle, openedProject.revision));
}

/** "Explorer" on Windows, "Finder" on a Mac, "file manager" elsewhere. */
export const FILE_MANAGER = /Windows/i.test(navigator.userAgent)
  ? "Explorer"
  : /Mac/i.test(navigator.userAgent)
    ? "Finder"
    : "file manager";
