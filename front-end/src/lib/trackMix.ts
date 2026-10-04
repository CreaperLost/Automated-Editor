import { api } from "./ipc";
import { useProjectStore } from "../stores/projectStore";
import { AudioSettings, DEFAULT_AUDIO_SETTINGS, DEFAULT_TRACK_MIX, TrackMix } from "./types";

/**
 * Saves volume, noise reduction and ducking for audio tracks, keyed by track id. Playback and
 * export read the same settings.
 */
export async function saveTrackMix(
  patches: Record<string, Partial<TrackMix>>,
  settings?: Partial<AudioSettings>,
): Promise<void> {
  const { openedProject, applyOpenedProject } = useProjectStore.getState();
  if (!openedProject) return;
  const audio = { ...(openedProject.audio ?? DEFAULT_AUDIO_SETTINGS), ...settings };
  const tracks = { ...audio.tracks };
  for (const [trackId, patch] of Object.entries(patches)) {
    const next = { ...DEFAULT_TRACK_MIX, ...tracks[trackId], ...patch };
    // Undefined clears a setting (noise reduction, ducking).
    for (const key of Object.keys(next) as (keyof TrackMix)[]) if (next[key] === undefined) delete next[key];
    const plain = next.volumeDb === 0 && next.denoiseDb === undefined && next.duckDb === undefined;
    if (plain) delete tracks[trackId];
    else tracks[trackId] = next;
  }
  const updated = await api.projectAudioUpdate(openedProject.projectHandle, openedProject.revision, {
    ...audio,
    tracks,
  });
  applyOpenedProject(updated);
}
