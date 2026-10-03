import React, { useState } from "react";
import { Layers, Trash2 } from "lucide-react";
import { InspectorSection } from "./InspectorSection";
import { useProjectStore } from "../../stores/projectStore";
import { api } from "../../lib/ipc";
import { fitsOnTrack, trackLabel } from "../../lib/trackUtils";
import type { OverlayClip, OverlayFit, TrackEdit } from "../../lib/types";

const FITS: { value: OverlayFit; label: string }[] = [
  { value: "contain", label: "Fit inside the frame" },
  { value: "cover", label: "Fill the frame (crop edges)" },
];

/// The clip picked on a video track above the main sequence: how it fills the frame, where it
/// starts, and removing it. Every change is one undoable edit.
export const TrackClipSection: React.FC = () => {
  const openedProject = useProjectStore((s) => s.openedProject);
  const applyOpenedProject = useProjectStore((s) => s.applyOpenedProject);
  const selectedId = useProjectStore((s) => s.selectedOverlayClipId);
  const setSelectedId = useProjectStore((s) => s.setSelectedOverlayClipId);
  const [error, setError] = useState<string>();
  const [busy, setBusy] = useState(false);

  const tracks = openedProject?.overlayTracks ?? [];
  const track = tracks.find((t) => t.clips.some((clip) => clip.id === selectedId));
  const clip = track?.clips.find((c) => c.id === selectedId);
  if (!openedProject || !track || !clip) return null;
  const asset = openedProject.mediaAssets?.find((a) => a.id === clip.assetId);

  const edit = async (change: TrackEdit) => {
    if (busy) return;
    setBusy(true);
    setError(undefined);
    try {
      applyOpenedProject(await api.projectTracksEdit(openedProject.projectHandle, openedProject.revision, change));
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  };
  const update = (next: OverlayClip) => void edit({ kind: "updateClip", clip: next, trackId: track.id });
  const setStart = (seconds: number) => {
    const startUs = Math.round(seconds * 1e6);
    if (!Number.isFinite(startUs) || startUs < 0 || startUs === clip.startUs) return;
    if (!fitsOnTrack(track, startUs, clip.durationUs, clip.id)) {
      setError("Another clip on this track is in the way.");
      return;
    }
    update({ ...clip, startUs });
  };

  return (
    <InspectorSection id="track-clip" title={`Track clip · ${trackLabel(tracks, track.id)}`} icon={Layers}>
      <div className="text-xs text-studio-200 truncate" title={asset?.name}>
        {asset?.name ?? "Missing media"}
        <span className="ml-1.5 text-studio-500 font-mono">{(clip.durationUs / 1e6).toFixed(2)}s</span>
      </div>
      {asset?.kind !== "audio" && (
        <label className="block text-xs text-studio-300 space-y-1">
          <span>Picture</span>
          <select
            aria-label="How the clip fills the frame"
            value={clip.fit}
            disabled={busy}
            onChange={(event) => update({ ...clip, fit: event.target.value as OverlayFit })}
            className="w-full bg-studio-800 text-studio-100 rounded px-2 py-1 border border-studio-700"
          >
            {FITS.map((fit) => (
              <option key={fit.value} value={fit.value}>
                {fit.label}
              </option>
            ))}
          </select>
        </label>
      )}
      <label className="flex items-center justify-between text-xs text-studio-300">
        <span>Starts at (s)</span>
        <input
          key={`${clip.id}-${clip.startUs}`}
          aria-label="Clip start in seconds"
          type="number"
          min={0}
          step={0.1}
          defaultValue={(clip.startUs / 1e6).toFixed(2)}
          disabled={busy}
          onBlur={(event) => setStart(Number(event.target.value))}
          onKeyDown={(event) => {
            if (event.key === "Enter") setStart(Number((event.target as HTMLInputElement).value));
          }}
          className="w-24 bg-studio-950 px-2 py-1 rounded border border-studio-700 text-right"
        />
      </label>
      <p className="text-[11px] text-studio-500">
        Drag the clip on the timeline to move it, its edges to trim it, or down onto V1 to insert it into the main video.
      </p>
      <button
        disabled={busy}
        onClick={() => {
          setSelectedId(undefined);
          void edit({ kind: "removeClip", clipId: clip.id });
        }}
        className="flex items-center gap-1.5 px-2 py-1 rounded border border-rose-900/60 text-xs text-rose-300 hover:bg-rose-950/40 disabled:opacity-40"
      >
        <Trash2 className="w-3.5 h-3.5" />
        Remove clip
      </button>
      {error && (
        <p role="alert" className="text-[11px] text-rose-300">
          {error}
        </p>
      )}
    </InspectorSection>
  );
};
