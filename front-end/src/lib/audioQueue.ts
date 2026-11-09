import { api } from "./ipc";
import { useProjectStore } from "../stores/projectStore";
import { AudioSettings, DEFAULT_AUDIO_SETTINGS } from "./types";

/** The audio changes still to send, one after another. */
let chain: Promise<unknown> = Promise.resolve();

/**
 * Saves a change to the project's audio settings (GAP-010). Changes go one at a time; each is
 * applied, when its turn comes, to the settings and revision the last save left, so two quick
 * changes (a volume and a switch) both land instead of the second meeting a stale revision.
 * A change made for one project is dropped once another is open. Resolves when it is saved.
 */
export function updateAudio(change: (audio: AudioSettings) => AudioSettings): Promise<void> {
  const handle = useProjectStore.getState().openedProject?.projectHandle;
  if (!handle) return Promise.resolve();
  const run = chain
    .catch(() => undefined)
    .then(async () => {
      const { openedProject, applyOpenedProject } = useProjectStore.getState();
      if (!openedProject || openedProject.projectHandle !== handle) return;
      const audio = change({ ...DEFAULT_AUDIO_SETTINGS, ...openedProject.audio });
      const updated = await api.projectAudioUpdate(handle, openedProject.revision, audio);
      if (useProjectStore.getState().openedProject?.projectHandle !== handle) return;
      await applyOpenedProject(updated);
    });
  chain = run;
  return run;
}
