import { useEffect, useRef, useCallback } from "react";
import { useProjectStore } from "../stores/projectStore";
import { api } from "../lib/ipc";
import { seekPlayback, togglePlayback } from "../lib/playbackControl";


export function formatTimeUs(timeUs: number): string {
  const totalSeconds = timeUs / 1_000_000;
  const mins = Math.floor(totalSeconds / 60);
  const secs = Math.floor(totalSeconds % 60);
  const ms = Math.floor((totalSeconds % 1) * 100);

  const pad = (n: number) => n.toString().padStart(2, "0");
  return `${pad(mins)}:${pad(secs)}.${pad(ms)}`;
}

export function useTimeline() {
  const {
    openedProject,
    currentTimeUs,
    durationUs,
    isPlaying,
    setCurrentTimeUs,
    setIsPlaying,
    applyPlaybackStatus,
  } = useProjectStore();

  const handle = openedProject?.projectHandle;

  useEffect(() => {
    if (!handle) {
      setIsPlaying(false);
      return;
    }
    let cancelled = false;
    const poll = async () => {
      try {
        const status = await api.playbackStatus(handle);
        if (!cancelled) applyPlaybackStatus(status);
      } catch {
        if (!cancelled) setIsPlaying(false);
      }
    };
    void poll();
    const timer = window.setInterval(() => {
      void poll();
    }, 50);
    return () => {
      cancelled = true;
      window.clearInterval(timer);
    };
  }, [handle, setCurrentTimeUs, setIsPlaying, applyPlaybackStatus]);

  const interpolationRef = useRef<number | null>(null);
  useEffect(() => {
    if (interpolationRef.current) {
      cancelAnimationFrame(interpolationRef.current);
      interpolationRef.current = null;
    }
    // Display smoothing only. Rust playback_status is the media clock.
  }, [isPlaying, currentTimeUs]);

  // Play/pause and seeking know whether this window shows the video or a short.
  const togglePlayPause = useCallback(() => togglePlayback(), []);
  const seekToUs = useCallback((timeUs: number) => seekPlayback(timeUs), []);

  const seekRelativeUs = useCallback(
    (offsetUs: number) => {
      seekToUs(currentTimeUs + offsetUs);
    },
    [currentTimeUs, seekToUs],
  );

  return {
    currentTimeUs,
    durationUs,
    isPlaying,
    togglePlayPause,
    seekToUs,
    seekRelativeUs,
    formattedTime: formatTimeUs(currentTimeUs),
    formattedDuration: formatTimeUs(durationUs),
  };
}
