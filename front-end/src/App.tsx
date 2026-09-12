import React, { useEffect, useState } from "react";
import {
  Folder,
  FolderOpen,
  MonitorPlay,
  Clock,
  X,
  AlertTriangle,
  Clapperboard,
} from "lucide-react";
import { EditorTopBar } from "./components/navigation/EditorTopBar";
import { TimelineStudio } from "./components/timeline/TimelineStudio";
import { NativePreviewHost } from "./components/canvas/NativePreviewHost";
import { InspectorPanel } from "./components/inspector/InspectorPanel";
import { SilenceModal } from "./components/silence-modal/SilenceModal";
import { useProjectStore } from "./stores/projectStore";
import { useSettingsStore } from "./stores/settingsStore";
import { useWindowTitle } from "./hooks/useWindowTitle";
import { api } from "./lib/ipc";
import { ExportStatus, SegmentPage } from "./lib/types";

function formatSeconds(us: number): string {
  return `${(us / 1_000_000).toFixed(2)}s`;
}

function getProjectFolderName(fullPath: string): string {
  const parts = fullPath.split(/[/\\]/).filter(Boolean);
  return parts.length > 0 ? parts[parts.length - 1] : fullPath;
}

function shortenPath(fullPath: string): string {
  const parts = fullPath.split(/[/\\]/).filter(Boolean);
  if (parts.length <= 2) return fullPath;
  return `…/${parts.slice(-2).join("/")}`;
}

function getDefaultExportPath(bundlePath?: string | null, projectName?: string): string | undefined {
  if (!bundlePath) return undefined;
  const normalized = bundlePath.replace(/[/\\]+$/, "");
  const lastSlash = Math.max(normalized.lastIndexOf("/"), normalized.lastIndexOf("\\"));
  const parent = lastSlash >= 0 ? normalized.slice(0, lastSlash) : normalized;
  const name = (projectName || "Untitled").trim() || "Untitled";
  const safeName = name.replace(/[/\\:*?"<>|]/g, "").trim().replace(/^\.+/, "");
  const base = safeName.length > 0 ? safeName : "Untitled";
  const filename = base.toLowerCase().endsWith(".mp4") ? base : `${base}.mp4`;
  return `${parent}/${filename}`;
}

export const App: React.FC = () => {
  useWindowTitle();
  const {
    openedProject: project,
    projectPath: path,
    recentProjects,
    loadOpenedProject,
    clearProject,
    removeRecentProject,
    clearRecentProjects,
  } = useProjectStore();
  const { canvas } = useSettingsStore();

  const [error, setError] = useState<string>();
  const [busy, setBusy] = useState(false);
  const [trackId, setTrackId] = useState("");
  const [offset, setOffset] = useState(0);
  const [page, setPage] = useState<SegmentPage>();
  const [resolution, setResolution] = useState("1920x1080");
  const [exportFps, setExportFps] = useState(30);
  const [exportDestination, setExportDestination] = useState("");
  const [exportJob, setExportJob] = useState<ExportStatus>();

  useEffect(() => {
    setTrackId(project?.tracks[0]?.descriptor.id ?? "");
    setOffset(0);
  }, [project?.projectHandle]);

  useEffect(() => {
    let active = true;
    setPage(undefined);
    if (project && trackId) {
      void api
        .projectSegments(project.projectHandle, trackId, offset)
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
  }, [project?.projectHandle, trackId, offset]);

  useEffect(() => {
    if (!exportJob || (exportJob.state !== "queued" && exportJob.state !== "running")) {
      return;
    }
    let active = true;
    const timer = window.setInterval(() => {
      void api
        .exportStatus(exportJob.jobId)
        .then((next) => {
          if (active) setExportJob(next);
        })
        .catch((err) => {
          if (active) setError(String(err));
        });
    }, 250);
    return () => {
      active = false;
      window.clearInterval(timer);
    };
  }, [exportJob?.jobId, exportJob?.state]);

  const openPath = async (targetPath: string) => {
    setBusy(true);
    setError(undefined);
    try {
      const opened = await api.openProject(targetPath);
      loadOpenedProject(opened, targetPath);
      setExportDestination("");
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  };

  const handleOpenFolder = async () => {
    setBusy(true);
    setError(undefined);
    try {
      const nextPath = await api.pickProjectFolder();
      if (!nextPath) return;
      await openPath(nextPath);
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  };

  const handleCloseProject = async () => {
    if (!project) return;
    setBusy(true);
    setError(undefined);
    try {
      await api.closeProject(project.projectHandle);
      clearProject();
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  };

  const handleShowInFinder = async () => {
    if (!path) return;
    try {
      await api.showInFinder(path);
    } catch (err) {
      setError(String(err));
    }
  };

  const chooseExportDestination = async () => {
    if (!project) return;
    setError(undefined);
    try {
      const picked = await api.pickExportDestination(project.projectHandle);
      if (!picked) return;
      setExportDestination(picked);
    } catch (err) {
      setError(String(err));
    }
  };

  const startExport = async () => {
    if (!project) return;
    setError(undefined);
    try {
      const next = await api.exportStart(project.projectHandle, {
        videoCodec: "h264",
        audioCodec: "aac",
        width: Number(resolution.split("x")[0]),
        height: Number(resolution.split("x")[1]),
        fps: exportFps,
        destination: exportDestination.trim() || undefined,
      });
      setExportJob(next);
      if (next.failure) {
        setError(`${next.failure.kind}: ${next.failure.message}`);
      }
    } catch (err) {
      setError(String(err));
    }
  };

  const cancelExport = async () => {
    if (!exportJob?.jobId) return;
    try {
      setExportJob(await api.exportCancel(exportJob.jobId));
    } catch (err) {
      setError(String(err));
    }
  };

  const defaultExportPath = getDefaultExportPath(path, project?.manifest.projectName);

  return (
    <div className="flex flex-col h-screen w-screen overflow-hidden bg-studio-950 text-studio-100 select-none">
      {/* 1. Editor Header */}
      <EditorTopBar
        resolution={resolution}
        setResolution={setResolution}
        exportFps={exportFps}
        setExportFps={setExportFps}
        exportDestination={exportDestination || defaultExportPath || ""}
        chooseExportDestination={chooseExportDestination}
        startExport={startExport}
        cancelExport={cancelExport}
        exportJob={exportJob}
        busy={busy}
        onOpenFolder={handleOpenFolder}
        onCloseProject={handleCloseProject}
        onShowInFinder={handleShowInFinder}
      />

      {/* 2. Notification & Error Alerts */}
      {error && (
        <div role="alert" className="px-5 py-2 bg-rose-950/60 border-b border-rose-800/80 text-rose-200 text-xs flex items-center justify-between">
          <span>{error}</span>
          <button onClick={() => setError(undefined)} className="text-rose-400 hover:text-rose-100">
            <X className="w-3.5 h-3.5" />
          </button>
        </div>
      )}

      {exportJob && exportJob.state !== "idle" && (
        <div className="px-5 py-1.5 bg-teal-950/40 border-b border-teal-800/60 text-xs text-teal-300 flex items-center gap-3">
          <span className="font-semibold capitalize">Export {exportJob.state}:</span>
          {exportJob.progressDenominator > 0 && (
            <span>{Math.round((exportJob.progressNumerator / exportJob.progressDenominator) * 100)}%</span>
          )}
          {exportJob.outputPath && <span className="font-mono truncate">{exportJob.outputPath}</span>}
          {exportJob.state === "completed" && <span className="text-emerald-400">✓ Video saved successfully</span>}
        </div>
      )}

      {/* 3. Main Editor Workspace */}
      <main className="flex-1 flex flex-col min-h-0 overflow-hidden relative">
        {project ? (
          <div className="flex-1 flex flex-col min-h-0 overflow-hidden">
            {/* Top row: Canvas Stage + Inspector */}
            <div className="flex-1 flex min-h-0 overflow-hidden">
              {/* Center Canvas Stage */}
              <div className="flex-1 flex flex-col min-w-0 min-h-0 overflow-hidden p-4 gap-3 bg-studio-950">
                <div className="flex items-center justify-between text-xs text-studio-400 px-1">
                  <div>
                    {project.tracks.length} tracks · source {formatSeconds(project.sourceDurationUs)} · edited {formatSeconds(project.editedDurationUs)}
                  </div>
                  <div className="flex items-center gap-2">
                    <span className="font-mono text-[11px] text-studio-500">
                      Canvas: {canvas.aspectRatio}
                    </span>
                  </div>
                </div>

                <div className="flex-1 min-h-0 overflow-hidden border border-studio-800 rounded-xl bg-studio-900/40 flex items-center justify-center p-2">
                  <NativePreviewHost
                    key={project.projectHandle}
                    fitAspectRatio={
                      canvas.aspectRatio === "9:16"
                        ? 9 / 16
                        : canvas.aspectRatio === "4:3"
                          ? 4 / 3
                          : canvas.aspectRatio === "1:1"
                            ? 1
                            : 16 / 9
                    }
                  />
                </div>

                {/* Diagnostics details toggle */}
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
                        onChange={(e) => {
                          setTrackId(e.target.value);
                          setOffset(0);
                        }}
                        className="bg-studio-800 text-studio-100 rounded px-2 py-0.5"
                      >
                        {project.tracks.map((t) => (
                          <option key={t.descriptor.id} value={t.descriptor.id}>
                            {t.descriptor.id} ({t.descriptor.trackType}) — {t.availableSegmentCount}/{t.segmentCount} segments
                          </option>
                        ))}
                      </select>
                      {page && (
                        <span className="text-studio-500 font-mono text-[11px]">
                          {page.segments.length} segments loaded
                        </span>
                      )}
                    </div>
                  </div>
                </details>
              </div>

              {/* Right Inspector Panel */}
              <div className="w-80 shrink-0 h-full border-l border-studio-800">
                <InspectorPanel />
              </div>
            </div>

            {/* Bottom: Multi-Track Timeline Studio */}
            <div className="min-h-0 border-t border-studio-800 shrink-0">
              <TimelineStudio />
            </div>
          </div>
        ) : (
          /* Empty / Welcome State */
          <div className="flex-1 flex flex-col items-center justify-center p-8 text-center bg-studio-950 overflow-y-auto">
            <div className="w-16 h-16 rounded-2xl bg-teal-900/30 border border-teal-700/40 flex items-center justify-center text-teal-400 mb-4 shadow-xl shadow-teal-950/50">
              <MonitorPlay className="w-8 h-8" />
            </div>
            <h2 className="text-xl font-bold text-white mb-2">
              AeroShoot Video Editor
            </h2>
            <p className="text-sm text-studio-400 max-w-md mb-6 leading-relaxed">
              Open a recorded project folder produced by <strong>AeroShoot Recorder</strong> to edit video tracks, apply smart zoom from mouse telemetry, trim dead air, and export.
            </p>

            <button
              type="button"
              disabled={busy}
              onClick={handleOpenFolder}
              className="flex items-center gap-2 px-5 py-2.5 rounded-xl bg-gradient-to-r from-teal-600 to-emerald-600 hover:from-teal-500 hover:to-emerald-500 text-white font-semibold text-sm shadow-lg shadow-teal-900/40 transition-all hover:scale-102"
            >
              <FolderOpen className="w-4 h-4" />
              <span>Open Recording Folder</span>
            </button>

            {/* Recent Projects List */}
            {recentProjects.length > 0 && (
              <div className="w-full max-w-md mt-10 text-left border-t border-studio-800/80 pt-6">
                <div className="flex items-center justify-between mb-3">
                  <span className="text-xs font-semibold uppercase tracking-wider text-studio-400 flex items-center gap-1.5">
                    <Clock className="w-3.5 h-3.5 text-studio-400" />
                    Recent Projects
                  </span>
                  <button
                    type="button"
                    disabled={busy}
                    onClick={() => clearRecentProjects()}
                    className="text-[11px] text-studio-500 hover:text-studio-300 transition-colors"
                  >
                    Clear all
                  </button>
                </div>
                <ul className="space-y-2">
                  {recentProjects.map((recentPath) => {
                    const folderName = getProjectFolderName(recentPath);
                    const displayPath = shortenPath(recentPath);
                    return (
                      <li
                        key={recentPath}
                        className="flex items-center justify-between gap-3 px-3 py-2 rounded-lg bg-studio-900/80 hover:bg-studio-850 border border-studio-800 transition-colors group cursor-pointer"
                        onClick={() => void openPath(recentPath)}
                      >
                        <div className="flex items-center gap-2.5 min-w-0 flex-1">
                          <Folder className="w-4 h-4 text-teal-400 shrink-0" />
                          <div className="min-w-0 flex-1">
                            <p className="text-xs font-medium text-studio-200 group-hover:text-white truncate">
                              {folderName}
                            </p>
                            <p className="text-[10px] text-studio-500 font-mono truncate">
                              {displayPath}
                            </p>
                          </div>
                        </div>
                        <button
                          type="button"
                          onClick={(e) => {
                            e.stopPropagation();
                            removeRecentProject(recentPath);
                          }}
                          className="p-1 rounded text-studio-500 hover:text-studio-300 opacity-60 group-hover:opacity-100"
                        >
                          <X className="w-3 h-3" />
                        </button>
                      </li>
                    );
                  })}
                </ul>
              </div>
            )}
          </div>
        )}
      </main>

      {/* 4. Global Silence Cuts Modal */}
      <SilenceModal />
    </div>
  );
};
