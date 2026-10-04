import React, { useState } from "react";
import { Eye, EyeOff, Volume2, VolumeX } from "lucide-react";
import { api } from "../../lib/ipc";
import { saveTrackMix } from "../../lib/trackMix";
import { isAudioTrack, layoutFromSettings, Track } from "../../lib/types";
import { useProjectStore } from "../../stores/projectStore";
import { useSettingsStore } from "../../stores/settingsStore";

/**
 * Per-track header buttons on the timeline: mute for audio tracks, show or hide for the
 * webcam. Both are saved to the project, so preview and export follow them.
 */
export const TrackHeaderButtons: React.FC<{ track: Track }> = ({ track }) => {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string>();
  const webcamVisible = useSettingsStore((s) => s.cameraBubble.enabled);

  const run = async (work: () => Promise<void>) => {
    if (busy) return;
    setBusy(true);
    try {
      await work();
      setError(undefined);
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  };

  const setWebcamVisible = (enabled: boolean) =>
    run(async () => {
      const opened = useProjectStore.getState().openedProject;
      const { canvas, cameraBubble, updateCameraBubble } = useSettingsStore.getState();
      updateCameraBubble({ enabled });
      if (!opened) return;
      try {
        const layout = layoutFromSettings(canvas, { ...cameraBubble, enabled }, opened.layout?.wallpaperAsset);
        useProjectStore
          .getState()
          .applyOpenedProject(await api.projectLayoutUpdate(opened.projectHandle, opened.revision, layout));
      } catch (err) {
        updateCameraBubble({ enabled: cameraBubble.enabled });
        throw err;
      }
    });

  const button =
    "h-7 w-7 inline-flex items-center justify-center rounded-control text-studio-400 hover:text-studio-100 hover:bg-studio-800 disabled:opacity-40 transition-colors";
  return (
    <div className="flex items-center gap-0.5" title={error}>
      {isAudioTrack(track.trackType) && (
        <button
          type="button"
          disabled={busy}
          aria-pressed={track.muted}
          aria-label={track.muted ? `Unmute ${track.id}` : `Mute ${track.id}`}
          title={error ?? (track.muted ? "Unmute this track" : "Mute this track in preview and export")}
          onClick={() => void run(() => saveTrackMix({ [track.id]: { muted: !track.muted } }))}
          className={`${button} ${track.muted ? "!text-danger-fg" : ""}`}
        >
          {track.muted ? <VolumeX className="w-4 h-4" /> : <Volume2 className="w-4 h-4" />}
        </button>
      )}
      {track.trackType === "webcam" && (
        <button
          type="button"
          disabled={busy}
          aria-pressed={!webcamVisible}
          aria-label={webcamVisible ? "Hide webcam" : "Show webcam"}
          title={error ?? (webcamVisible ? "Hide the webcam in preview and export" : "Show the webcam")}
          onClick={() => void setWebcamVisible(!webcamVisible)}
          className={`${button} ${webcamVisible ? "" : "!text-danger-fg"}`}
        >
          {webcamVisible ? <Eye className="w-4 h-4" /> : <EyeOff className="w-4 h-4" />}
        </button>
      )}
      {error && <span role="alert" className="sr-only">{error}</span>}
    </div>
  );
};
