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
import { Badge } from "../ui";
import { releaseShortPlayback } from "../../lib/playbackControl";
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
  "ui-field !h-control-sm !text-meta !px-2 min-w-0 max-w-[15rem] truncate";

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
    <div className="flex items-center gap-1.5 shrink-0" title={error ?? "Preview quality. Export is not affected."}>
      <Gauge className={`w-3.5 h-3.5 ${error ? "text-danger" : "text-studio-500"}`} />
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
        <span className="font-mono text-meta text-accent-fg w-14" aria-label="Frames drawn per second">
          {measuredFps} fps
        </span>
      )}
    </div>
  );
};

/// The preview stage: project summary, the video preview, and track diagnostics.
export const StagePanel: React.FC = () => {
  const project = useProjectStore((s) => s.openedProject);
  const playbackShortId = useProjectStore((s) => s.playbackShortId);
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

  // Clicking back into the editor gives it the video again, at its place.
  useEffect(() => {
    const takeBack = () => releaseShortPlayback();
    window.addEventListener("focus", takeBack);
    return () => window.removeEventListener("focus", takeBack);
  }, []);

  if (!project) return null;
  const playingShort = project.shorts?.find((short) => short.id === playbackShortId);

  return (
    <div className="h-full flex flex-col min-w-0 min-h-0 overflow-hidden p-2 gap-2 bg-studio-900">
      {playbackShortId && (
        // The Shorts Studio is playing a short through this preview and its sound.
        <div className="flex items-center gap-2 rounded-md border border-accent/60 bg-accent/10 px-2 py-1 text-meta text-accent-fg">
          <span className="flex-1 truncate">
            Playing the short {playingShort ? `"${playingShort.title}"` : ""} from the Shorts Studio
          </span>
          <button
            type="button"
            onClick={() =>
              void api
                .playbackFocusShort(project.projectHandle, null)
                .then(useProjectStore.getState().applyPlaybackStatus)
                .catch(() => undefined)
            }
            className="px-2 py-0.5 rounded border border-accent/60 hover:bg-accent/15"
          >
            Back to the video
          </button>
        </div>
      )}
      <div className="flex items-center justify-between gap-3 px-1">
        <PreviewQualityControls />
        <Badge title="The canvas shape (change it in Inspector › Video)">{aspectRatio}</Badge>
      </div>

      {project.editedDurationUs === 0 && (
        <p className="shrink-0 rounded-lg border border-accent/40 bg-accent/10 px-3 py-2 text-label text-accent-fg">
          {project.recordingPath
            ? "Everything was cut. Undo, or put clips back from the timeline."
            : "This project starts empty. Import video, images or audio in the Media panel, then drag them onto the timeline."}
        </p>
      )}

      <div className="flex-1 min-h-0 overflow-hidden rounded-control bg-studio-950 flex items-center justify-center p-2">
        <NativePreviewHost key={project.projectHandle} fitAspectRatio={aspectValue(aspectRatio)} />
      </div>

      <details className="max-h-28 shrink-0 overflow-y-auto rounded-control border border-studio-800 bg-studio-900 px-3 py-1.5 text-label text-studio-400">
        <summary className="cursor-pointer text-meta text-studio-500 hover:text-studio-300">
          Recording details
          {project.diagnostics.length > 0 && (
            <span className="ml-1.5 text-suggest-fg">· {project.diagnostics.length} note{project.diagnostics.length === 1 ? "" : "s"}</span>
          )}
        </summary>
        <div className="space-y-2 pt-2">
          <p className="text-meta text-studio-400">
            {project.tracks.length} tracks · source {formatSeconds(project.sourceDurationUs)} · edited{" "}
            {formatSeconds(project.editedDurationUs)}
          </p>
          {project.diagnostics.length > 0 && (
            <ul className="text-label text-suggest-fg space-y-1 bg-suggest/10 border border-suggest/30 rounded p-2">
              {project.diagnostics.map((msg, i) => (
                <li key={i} className="flex gap-1.5 items-center">
                  <AlertTriangle className="w-3 h-3 shrink-0" />
                  <span>{msg}</span>
                </li>
              ))}
            </ul>
          )}
          <div className="flex items-center gap-2">
            <Clapperboard className="w-3.5 h-3.5 text-accent-hover" />
            <select
              aria-label="Track selector"
              value={trackId}
              onChange={(e) => setTrackId(e.target.value)}
              className={selectClass}
            >
              {project.tracks.map((t) => (
                <option key={t.descriptor.id} value={t.descriptor.id}>
                  {t.descriptor.id} ({t.descriptor.trackType}) — {t.availableSegmentCount}/{t.segmentCount} segments
                </option>
              ))}
            </select>
            {page && (
              <span className="text-studio-500 font-mono text-meta">{page.segments.length} segments loaded</span>
            )}
          </div>
          {error && <p role="alert" className="text-danger-fg">{error}</p>}
        </div>
      </details>
    </div>
  );
};
