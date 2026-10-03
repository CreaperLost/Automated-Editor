import { useShallow } from "zustand/react/shallow";
import React, { useEffect, useState } from "react";
import { listenForProjects, listenForCaptionChanges } from "./lib/windowSync";
import {
  Folder,
  FolderOpen,
  MonitorPlay,
  Clock,
  X,
  FilePlus2,
} from "lucide-react";
import { EditorTopBar } from "./components/navigation/EditorTopBar";
import { SilenceModal } from "./components/silence-modal/SilenceModal";
import { useProjectStore } from "./stores/projectStore";
import { useSettingsStore } from "./stores/settingsStore";
import { useWindowTitle } from "./hooks/useWindowTitle";
import { DockWorkspace } from "./components/layout/DockWorkspace";
import { api } from "./lib/ipc";
import { ExportSettings, ExportStatus } from "./lib/types";
import { ExportDialog } from "./components/export/ExportDialog";
import { NewProjectDialog } from "./components/project/NewProjectDialog";

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
  } = useProjectStore(
    useShallow((s) => ({
      openedProject: s.openedProject,
      projectPath: s.projectPath,
      recentProjects: s.recentProjects,
      loadOpenedProject: s.loadOpenedProject,
      clearProject: s.clearProject,
      removeRecentProject: s.removeRecentProject,
      clearRecentProjects: s.clearRecentProjects,
    })),
  );
  const { canvas } = useSettingsStore();

  const [error, setError] = useState<string>();
  const [busy, setBusy] = useState(false);
  const [exportOpen, setExportOpen] = useState(false);
  const [newProjectOpen, setNewProjectOpen] = useState(false);
  const [exportDestination, setExportDestination] = useState("");
  const [exportJob, setExportJob] = useState<ExportStatus>();

  // Edits made in the Shorts Studio window.
  useEffect(
    () =>
      listenForProjects((next) => {
        // The Shorts Studio may send a short's view: the editor shows the whole project.
        if (next.shortView) {
          void api
            .projectCurrent()
            .then((project) => project && useProjectStore.getState().applyOpenedProject(project, { remote: true }))
            .catch(() => undefined);
          return;
        }
        useProjectStore.getState().applyOpenedProject(next, { remote: true });
      }),
    [],
  );
  useEffect(() => listenForCaptionChanges(() => useProjectStore.getState().bumpCaptions(true)), []);

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

  const startExport = async (settings: ExportSettings) => {
    if (!project) return;
    setError(undefined);
    try {
      const next = await api.exportStart(project.projectHandle, {
        ...settings,
        destination: exportDestination.trim() || undefined,
      });
      setExportJob(next);
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
        onOpenExport={() => setExportOpen(true)}
        exportJob={exportJob}
        busy={busy}
        onOpenFolder={handleOpenFolder}
        onNewProject={() => setNewProjectOpen(true)}
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
          <div className="flex-1 min-h-0 overflow-hidden">
            <DockWorkspace />
          </div>
        ) : (
          /* Empty / Welcome State */
          <div className="flex-1 flex flex-col items-center justify-center p-8 text-center bg-studio-950 overflow-y-auto">
            <div className="w-16 h-16 rounded-2xl bg-teal-900/30 border border-teal-700/40 flex items-center justify-center text-teal-400 mb-4 shadow-xl shadow-teal-950/50">
              <MonitorPlay className="w-8 h-8" />
            </div>
            <h2 className="text-xl font-bold text-white mb-2">
              AeroEdits
            </h2>
            <p className="text-sm text-studio-400 max-w-md mb-6 leading-relaxed">
              Start a project from a <strong>.aero</strong> recording (smart zoom from mouse telemetry, dead-air trimming) or from nothing, with imported video, images and audio. Projects live in their own folders; recordings are never changed.
            </p>

            <div className="flex flex-wrap items-center justify-center gap-3">
              <button
                type="button"
                disabled={busy}
                onClick={() => setNewProjectOpen(true)}
                className="flex items-center gap-2 px-5 py-2.5 rounded-xl bg-gradient-to-r from-teal-600 to-emerald-600 hover:from-teal-500 hover:to-emerald-500 text-white font-semibold text-sm shadow-lg shadow-teal-900/40 transition-all hover:scale-102"
              >
                <FilePlus2 className="w-4 h-4" />
                <span>New Project</span>
              </button>
              <button
                type="button"
                disabled={busy}
                onClick={handleOpenFolder}
                title="Open a project folder, or a recording folder to edit it in place"
                className="flex items-center gap-2 px-5 py-2.5 rounded-xl border border-studio-700 bg-studio-900 hover:bg-studio-850 text-studio-100 font-semibold text-sm transition-colors"
              >
                <FolderOpen className="w-4 h-4" />
                <span>Open Project</span>
              </button>
            </div>

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

      {newProjectOpen && (
        <NewProjectDialog
          onClose={() => setNewProjectOpen(false)}
          onCreated={(opened) => {
            setNewProjectOpen(false);
            loadOpenedProject(opened, opened.projectPath ?? "");
            setExportDestination("");
          }}
        />
      )}

      {/* 4. Global Silence Cuts Modal */}
      <SilenceModal />

      {project && exportOpen && (
        <ExportDialog
          aspectRatio={canvas.aspectRatio}
          editedDurationUs={project.editedDurationUs}
          destination={exportDestination || defaultExportPath || ""}
          exportJob={exportJob}
          onChooseDestination={() => void chooseExportDestination()}
          onStart={(settings) => void startExport(settings)}
          onCancel={() => void cancelExport()}
          onClose={() => setExportOpen(false)}
        />
      )}
    </div>
  );
};
