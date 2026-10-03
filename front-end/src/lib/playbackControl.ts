import { api } from "./ipc";
import { useProjectStore } from "../stores/projectStore";

/**
 * Play/pause and seeking for whichever window calls them. The playback engine plays one thing
 * at a time: the video, or (while the Shorts Studio plays) one short. A window showing a short
 * keeps its own playhead while paused and borrows the engine only to play; the editor gets the
 * video back, at its old place, when the short stops.
 */
export function togglePlayback() {
  const { openedProject, viewShort, playbackShortId, isPlaying, currentTimeUs, applyPlaybackStatus } =
    useProjectStore.getState();
  if (!openedProject) return;
  const handle = openedProject.projectHandle;
  const done = (promise: Promise<Parameters<typeof applyPlaybackStatus>[0]>) =>
    void promise.then(applyPlaybackStatus).catch((err) => console.warn("[Playback] toggle failed:", err));
  if (viewShort) {
    if (isPlaying && playbackShortId === viewShort) {
      // Paused, the short stays in the engine: this window keeps its live preview.
      done(api.playbackPause(handle));
    } else {
      done(api.playbackFocusShort(handle, viewShort, currentTimeUs, true));
    }
    return;
  }
  // The editor: a short borrowed playback; take the video back and play it.
  if (playbackShortId) {
    done(api.playbackFocusShort(handle, null, 0, true));
    return;
  }
  done(isPlaying ? api.playbackPause(handle) : api.playbackPlay(handle));
}

export function seekPlayback(timeUs: number) {
  const { openedProject, viewShort, playbackShortId, durationUs, setCurrentTimeUs, applyPlaybackStatus } =
    useProjectStore.getState();
  const target = Math.max(0, Math.min(timeUs, durationUs));
  // A short's own playhead moves on its own while the engine plays the video.
  if (!openedProject || (viewShort && playbackShortId !== viewShort)) {
    setCurrentTimeUs(target);
    return;
  }
  void api
    .playbackSeek(openedProject.projectHandle, target)
    .then(applyPlaybackStatus)
    .catch((err) => console.warn("[Playback] seek failed:", err));
}

/**
 * The Shorts Studio takes the engine while it is the active window, so its preview is live
 * (paused frames, scrubbing, resolution and frame rate) like the editor's. The short keeps
 * its own playhead; the video's place is kept for when the editor takes the engine back.
 */
export function takeShortPlayback() {
  const { openedProject, viewShort, playbackShortId, currentTimeUs, applyPlaybackStatus } =
    useProjectStore.getState();
  if (!openedProject || !viewShort || playbackShortId === viewShort) return;
  void api
    .playbackFocusShort(openedProject.projectHandle, viewShort, currentTimeUs, false)
    .then(applyPlaybackStatus)
    .catch(() => undefined);
}

/** Hands playback back to the video if a short has it. */
export function releaseShortPlayback() {
  const { openedProject, playbackShortId, applyPlaybackStatus } = useProjectStore.getState();
  if (!openedProject || !playbackShortId) return;
  void api
    .playbackFocusShort(openedProject.projectHandle, null)
    .then(applyPlaybackStatus)
    .catch(() => undefined);
}
