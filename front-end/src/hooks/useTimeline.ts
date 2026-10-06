import { useEffect, useCallback } from "react";
import { useProjectStore } from "../stores/projectStore";
import { api } from "../lib/ipc";
import { seekPlayback, togglePlayback } from "../lib/playbackControl";

/** How often the playhead is read from the engine: while playing, and otherwise. */
const PLAYING_POLL_MS = 50;
const IDLE_POLL_MS = 200;

export function formatTimeUs(timeUs: number): string {
  const totalSeconds = timeUs / 1_000_000;
  const mins = Math.floor(totalSeconds / 60);
  const secs = Math.floor(totalSeconds % 60);
  const ms = Math.floor((totalSeconds % 1) * 100);

  const pad = (n: number) => n.toString().padStart(2, "0");
  return `${pad(mins)}:${pad(secs)}.${pad(ms)}`;
}

/**
 * Keeps the store's playhead in step with the playback engine (Rust's clock is the one that
 * counts) and gives the timeline its play and seek actions. It subscribes to nothing but the
 * project handle, so the component using it does not re-render as the playhead moves.
 */
export function useTimeline() {
  const handle = useProjectStore((s) => s.openedProject?.projectHandle);

  useEffect(() => {
    const { setIsPlaying, applyPlaybackStatus } = useProjectStore.getState();
    if (!handle) {
      setIsPlaying(false);
      return;
    }
    let cancelled = false;
    let timer: number | undefined;
    let inFlight = false;
    // One request at a time: the next waits for this one, however long the engine takes.
    const poll = async () => {
      window.clearTimeout(timer);
      inFlight = true;
      try {
        const status = await api.playbackStatus(handle);
        if (!cancelled) applyPlaybackStatus(status);
      } catch {
        if (!cancelled) setIsPlaying(false);
      }
      inFlight = false;
      if (!cancelled) {
        const playing = useProjectStore.getState().isPlaying;
        timer = window.setTimeout(() => void poll(), playing ? PLAYING_POLL_MS : IDLE_POLL_MS);
      }
    };
    void poll();
    // Playback starting here does not wait out the slower paused interval.
    const unsubscribe = useProjectStore.subscribe((state, previous) => {
      if (state.isPlaying && !previous.isPlaying && !inFlight && !cancelled) void poll();
    });
    return () => {
      cancelled = true;
      window.clearTimeout(timer);
      unsubscribe();
    };
  }, [handle]);

  // Play/pause and seeking know whether this window shows the video or a short.
  const togglePlayPause = useCallback(() => togglePlayback(), []);
  const seekToUs = useCallback((timeUs: number) => seekPlayback(timeUs), []);

  return { togglePlayPause, seekToUs };
}
