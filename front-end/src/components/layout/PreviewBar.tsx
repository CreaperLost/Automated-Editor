import React, { useEffect } from "react";
import { AlertTriangle, Gauge, Pause, Play, SkipBack } from "lucide-react";
import { useProjectStore } from "../../stores/projectStore";
import { PREVIEW_FPS, PREVIEW_RESOLUTIONS, usePreviewQualityStore } from "../../stores/previewQualityStore";
import { formatBinding, useHotkeyStore } from "../../stores/hotkeyStore";
import { seekPlayback, togglePlayback } from "../../lib/playbackControl";
import { formatTimeUs } from "../../hooks/useTimeline";
import { Badge, IconButton, cn } from "../ui";

const selectClass = "ui-field !h-7 !text-meta !pl-2 !pr-1 w-auto";

/// Preview resolution and frame rate, changed on the fly, with the rate actually drawn.
export const PreviewQualityControls: React.FC = () => {
  const quality = usePreviewQualityStore((s) => s.quality);
  const measuredFps = usePreviewQualityStore((s) => s.measuredFps);
  const error = usePreviewQualityStore((s) => s.error);
  const init = usePreviewQualityStore((s) => s.init);
  const setQuality = usePreviewQualityStore((s) => s.setQuality);
  const isPlaying = useProjectStore((s) => s.isPlaying);

  useEffect(() => {
    void init();
  }, [init]);

  if (!quality) return null;
  return (
    <div className="flex items-center gap-1.5 shrink-0" title={error ?? "Preview quality. The export is not affected."}>
      <Gauge className={cn("w-4 h-4", error ? "text-danger" : "text-studio-500")} aria-hidden />
      <select
        aria-label="Preview resolution"
        value={quality.resolution}
        onChange={(e) => setQuality({ resolution: Number(e.target.value) })}
        className={selectClass}
      >
        {PREVIEW_RESOLUTIONS.map((option) => (
          <option key={option.value} value={option.value}>
            {option.label}
          </option>
        ))}
      </select>
      <select
        aria-label="Preview frame rate"
        value={quality.fps}
        onChange={(e) => setQuality({ fps: Number(e.target.value) })}
        className={selectClass}
      >
        {PREVIEW_FPS.map((option) => (
          <option key={option.value} value={option.value}>
            {option.label}
          </option>
        ))}
      </select>
      {isPlaying && measuredFps !== null && (
        <span className="font-mono text-meta tabular-nums text-accent-fg w-12" aria-label="Frames drawn per second">
          {measuredFps} fps
        </span>
      )}
    </div>
  );
};

/// The bar under a preview: go to start, play, where the playhead is, then the preview's
/// quality and the canvas shape. Used by the editor and the Shorts Studio alike.
export const PreviewBar: React.FC<{
  aspect: string;
  aspectTitle?: string;
  /** Recorder notes worth a look, shown as a warning icon. */
  notes?: string[];
}> = ({ aspect, aspectTitle, notes = [] }) => {
  const project = useProjectStore((s) => s.openedProject);
  const currentTimeUs = useProjectStore((s) => s.currentTimeUs);
  const durationUs = useProjectStore((s) => s.durationUs);
  const isPlaying = useProjectStore((s) => s.isPlaying);
  const bindings = useHotkeyStore((s) => s.bindings);
  const playKey = bindings.playPause[0] ? ` (${formatBinding(bindings.playPause[0])})` : "";
  const startKey = bindings.goToStart[0] ? ` (${formatBinding(bindings.goToStart[0])})` : "";

  return (
    <div className="h-11 shrink-0 flex items-center gap-1 px-2 border-t border-studio-800 bg-studio-900">
      <IconButton icon={SkipBack} size="sm" label={`Go to start${startKey}`} disabled={!project} onClick={() => seekPlayback(0)} />
      <button
        type="button"
        disabled={!project}
        onClick={() => togglePlayback()}
        aria-label={isPlaying ? "Pause" : "Play"}
        title={`${isPlaying ? "Pause" : "Play"}${playKey}`}
        className="h-8 w-8 shrink-0 inline-flex items-center justify-center rounded-full bg-accent hover:bg-accent-hover text-white disabled:opacity-40 transition-colors"
      >
        {isPlaying ? <Pause className="w-4 h-4 fill-white" /> : <Play className="w-4 h-4 fill-white ml-0.5" />}
      </button>
      <div className="px-2 font-mono text-label tabular-nums whitespace-nowrap" aria-label="Playhead time">
        <span className="font-semibold text-studio-100">{formatTimeUs(currentTimeUs)}</span>
        <span className="mx-1 text-studio-600">/</span>
        <span className="text-studio-400">{formatTimeUs(durationUs)}</span>
      </div>
      <span className="flex-1" />
      <PreviewQualityControls />
      <span className="mx-1 h-5 w-px bg-studio-800" aria-hidden />
      <Badge title={aspectTitle}>{aspect}</Badge>
      {notes.length > 0 && (
        <span className="ml-1 inline-flex text-suggest-fg" title={`Recorder notes:\n${notes.join("\n")}`} aria-label={`${notes.length} recorder notes`}>
          <AlertTriangle className="w-4 h-4" />
        </span>
      )}
    </div>
  );
};
