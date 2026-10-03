import React, { useEffect, useState } from "react";
import { AlertTriangle, Clapperboard, Gauge } from "lucide-react";
import { NativePreviewHost } from "../canvas/NativePreviewHost";
import { useProjectStore } from "../../stores/projectStore";
import { useSettingsStore } from "../../stores/settingsStore";
import {
  PREVIEW_FPS,
  PREVIEW_RESOLUTIONS,
  usePreviewQualityStore,
} from "../../stores/previewQualityStore";
import { api } from "../../lib/ipc";
import type { SegmentPage } from "../../lib/types";

function formatSeconds(us: number): string {
  return `${(us / 1_000_000).toFixed(2)}s`;
}

function aspectValue(ratio: string): number {
  switch (ratio) {
    case "9:16":
      return 9 / 16;
    case "4:3":
      return 4 / 3;
    case "1:1":
      return 1;
    default:
      return 16 / 9;
  }
}

const selectClass =
  "bg-studio-800 text-studio-100 rounded px-1.5 py-0.5 text-[11px] border border-studio-700 focus:outline-none focus:border-teal-500";

/// Preview resolution and frame rate, changed on the fly, with the rate actually drawn.
const PreviewQualityControls: React.FC = () => {
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
    <div className="flex items-center gap-1.5 shrink-0" title={error ?? "Preview quality. Export is not affected."}>
      <Gauge className={`w-3.5 h-3.5 ${error ? "text-rose-400" : "text-studio-500"}`} />
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
        <span className="font-mono text-[11px] text-teal-300 w-14" aria-label="Frames drawn per second">
          {measuredFps} fps
        </span>
      )}
    </div>
  );
};

/// The preview stage: project summary, the video preview, and track diagnostics.
export const StagePanel: React.FC = () => {
  const project = useProjectStore((s) => s.openedProject);
  const aspectRatio = useSettingsStore((s) => s.canvas.aspectRatio);
  const [trackId, setTrackId] = useState("");
  const [page, setPage] = useState<SegmentPage>();
  const [error, setError] = useState<string>();

  useEffect(() => {
    setTrackId(project?.tracks[0]?.descriptor.id ?? "");
  }, [project?.projectHandle]);

  useEffect(() => {
    let active = true;
    setPage(undefined);
    setError(undefined);
    if (project && trackId) {
      void api
        .projectSegments(project.projectHandle, trackId, 0)
        .then((next) => {
          if (active) setPage(next);
        })
        .catch((err) => {
          if (active) setError(String(err));
        });
    }
    return () => {
      active = false;
    };
  }, [project?.projectHandle, trackId]);

  if (!project) return null;

  return (
    <div className="h-full flex flex-col min-w-0 min-h-0 overflow-hidden p-3 gap-2 bg-studio-950">
      <div className="flex items-center justify-between text-xs text-studio-400 px-1">
        <div className="truncate">
          {project.tracks.length} tracks · source {formatSeconds(project.sourceDurationUs)} · edited{" "}
          {formatSeconds(project.editedDurationUs)}
        </div>
        <div className="flex items-center gap-3 shrink-0">
          <PreviewQualityControls />
          <span className="font-mono text-[11px] text-studio-500">Canvas: {aspectRatio}</span>
        </div>
      </div>

      {project.editedDurationUs === 0 && (
        <p className="shrink-0 rounded-lg border border-teal-800/60 bg-teal-950/30 px-3 py-2 text-xs text-teal-200">
          {project.recordingPath
            ? "Everything was cut. Undo, or put clips back from the timeline."
            : "This project starts empty. Import video, images or audio in the Media panel, then drag them onto the timeline."}
        </p>
      )}

      <div className="flex-1 min-h-0 overflow-hidden border border-studio-800 rounded-xl bg-studio-900/40 flex items-center justify-center p-2">
        <NativePreviewHost key={project.projectHandle} fitAspectRatio={aspectValue(aspectRatio)} />
      </div>

      <details className="max-h-24 shrink-0 overflow-y-auto rounded-lg border border-studio-800/80 bg-studio-900/40 px-3 py-1.5 text-xs text-studio-400">
        <summary className="cursor-pointer font-medium text-studio-300">
          Track segments and recording diagnostics
        </summary>
        <div className="space-y-2 pt-2">
          {project.diagnostics.length > 0 && (
            <ul className="text-xs text-amber-300 space-y-1 bg-amber-950/20 border border-amber-900/40 rounded p-2">
              {project.diagnostics.map((msg, i) => (
                <li key={i} className="flex gap-1.5 items-center">
                  <AlertTriangle className="w-3 h-3 shrink-0" />
                  <span>{msg}</span>
                </li>
              ))}
            </ul>
          )}
          <div className="flex items-center gap-2">
            <Clapperboard className="w-3.5 h-3.5 text-teal-400" />
            <select
              aria-label="Track selector"
              value={trackId}
              onChange={(e) => setTrackId(e.target.value)}
              className="bg-studio-800 text-studio-100 rounded px-2 py-0.5"
            >
              {project.tracks.map((t) => (
                <option key={t.descriptor.id} value={t.descriptor.id}>
                  {t.descriptor.id} ({t.descriptor.trackType}) — {t.availableSegmentCount}/{t.segmentCount} segments
                </option>
              ))}
            </select>
            {page && (
              <span className="text-studio-500 font-mono text-[11px]">{page.segments.length} segments loaded</span>
            )}
          </div>
          {error && <p role="alert" className="text-rose-300">{error}</p>}
        </div>
      </details>
    </div>
  );
};
