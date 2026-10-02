import React, { useEffect, useState } from "react";
import { AlertTriangle, Clapperboard } from "lucide-react";
import { NativePreviewHost } from "../canvas/NativePreviewHost";
import { useProjectStore } from "../../stores/projectStore";
import { useSettingsStore } from "../../stores/settingsStore";
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
        <span className="font-mono text-[11px] text-studio-500 shrink-0">Canvas: {aspectRatio}</span>
      </div>

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
