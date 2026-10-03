import { useShallow } from "zustand/react/shallow";
import React, { useEffect, useState } from "react";
import { listenForProjects, listenForCaptionChanges } from "./lib/windowSync";
import { ChevronRight, Clock, FilePlus2, Film, Folder, FolderOpen, X } from "lucide-react";
import { EditorTopBar } from "./components/navigation/EditorTopBar";
import { StatusBar } from "./components/navigation/StatusBar";
import { Button, Notice } from "./components/ui";
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
  // The new-project dialog, opened on "edit a recording" or "start empty" (null: closed).
  const [newProject, setNewProject] = useState<null | { withRecording?: boolean }>(null);
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

  const showInFileManager = async (target: string | null | undefined) => {
    if (!target) return;
    try {
      await api.showInFinder(target);
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
      <EditorTopBar
        onOpenExport={() => setExportOpen(true)}
        exportJob={exportJob}
        busy={busy}
        onOpenFolder={handleOpenFolder}
        onNewProject={() => setNewProject({})}
        onOpenRecent={(recent) => void openPath(recent)}
        onCloseProject={handleCloseProject}
        onShowInFinder={() => void showInFileManager(path)}
        onError={setError}
      />

      {error && (
        <Notice tone="danger" onDismiss={() => setError(undefined)}>
          {error}
        </Notice>
      )}

      <main className="flex-1 flex flex-col min-h-0 overflow-hidden relative">
        {project ? (
          <div className="flex-1 min-h-0 overflow-hidden p-1.5">
            <DockWorkspace />
          </div>
        ) : (
          <Welcome
            busy={busy}
            recentProjects={recentProjects}
            onEditRecording={() => setNewProject({ withRecording: true })}
            onStartEmpty={() => setNewProject({ withRecording: false })}
            onOpenFolder={handleOpenFolder}
            onOpenRecent={(recent) => void openPath(recent)}
            onRemoveRecent={removeRecentProject}
            onClearRecent={clearRecentProjects}
          />
        )}
      </main>

      <StatusBar
        exportJob={exportJob}
        onOpenExport={() => setExportOpen(true)}
        onShowExport={(output) => void showInFileManager(output)}
      />

      {newProject && (
        <NewProjectDialog
          startWithRecording={newProject.withRecording}
          onClose={() => setNewProject(null)}
          onCreated={(opened) => {
            setNewProject(null);
            loadOpenedProject(opened, opened.projectPath ?? "");
            setExportDestination("");
          }}
        />
      )}

      {/* Jump Cuts */}
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

/// The start screen: edit a recording, start empty, or pick up a recent project.
const Welcome: React.FC<{
  busy: boolean;
  recentProjects: string[];
  onEditRecording: () => void;
  onStartEmpty: () => void;
  onOpenFolder: () => void;
  onOpenRecent: (path: string) => void;
  onRemoveRecent: (path: string) => void;
  onClearRecent: () => void;
}> = ({ busy, recentProjects, onEditRecording, onStartEmpty, onOpenFolder, onOpenRecent, onRemoveRecent, onClearRecent }) => (
  <div className="flex-1 overflow-y-auto bg-studio-950">
    <div className="mx-auto w-full max-w-3xl px-6 py-14">
      <div className="flex items-center gap-3">
        <img src="/aeroedits-icon.svg" alt="" className="w-11 h-11" draggable={false} />
        <div>
          <h1 className="font-display text-display text-studio-100">AeroEdits</h1>
          <p className="text-body text-studio-400">Turn screen recordings into polished videos.</p>
        </div>
      </div>

      <div className="mt-10 grid gap-3 sm:grid-cols-2">
        <StartCard
          icon={Film}
          title="Edit a recording"
          detail="Start from a .aero recording: zooms from your clicks, the camera and every sound track."
          onClick={onEditRecording}
          disabled={busy}
          primary
        />
        <StartCard
          icon={FilePlus2}
          title="Start an empty project"
          detail="Build a video from imported clips, images and audio."
          onClick={onStartEmpty}
          disabled={busy}
        />
      </div>
      <div className="mt-3">
        <Button variant="ghost" icon={FolderOpen} onClick={onOpenFolder} disabled={busy}>
          Open a project or recording folder…
        </Button>
      </div>

      <section className="mt-10">
        <div className="flex items-center justify-between border-b border-studio-800 pb-2">
          <h2 className="flex items-center gap-2 text-label font-semibold text-studio-300">
            <Clock className="w-4 h-4 text-studio-500" /> Recent projects
          </h2>
          {recentProjects.length > 0 && (
            <Button variant="ghost" size="sm" onClick={onClearRecent} disabled={busy}>
              Clear list
            </Button>
          )}
        </div>
        {recentProjects.length === 0 ? (
          <p className="py-6 text-label text-studio-500">Projects you open show up here.</p>
        ) : (
          <ul className="mt-2 divide-y divide-studio-800/70">
            {recentProjects.map((recentPath) => (
              <li key={recentPath} className="group flex items-center gap-3">
                <button
                  type="button"
                  disabled={busy}
                  onClick={() => onOpenRecent(recentPath)}
                  className="flex flex-1 min-w-0 items-center gap-3 rounded-control px-2 py-2.5 text-left hover:bg-studio-900"
                >
                  <Folder className="w-5 h-5 text-accent-fg shrink-0" />
                  <span className="min-w-0 flex-1">
                    <span className="block text-body font-medium text-studio-100 truncate">
                      {getProjectFolderName(recentPath)}
                    </span>
                    <span className="block text-meta text-studio-500 truncate">{shortenPath(recentPath)}</span>
                  </span>
                  <ChevronRight className="w-4 h-4 text-studio-600 group-hover:text-studio-300" />
                </button>
                <button
                  type="button"
                  aria-label={`Remove ${getProjectFolderName(recentPath)} from recent projects`}
                  title="Remove from the list (the project stays on disk)"
                  onClick={() => onRemoveRecent(recentPath)}
                  className="h-8 w-8 inline-flex items-center justify-center rounded-control text-studio-500 opacity-0 group-hover:opacity-100 focus:opacity-100 hover:text-studio-200 hover:bg-studio-800"
                >
                  <X className="w-4 h-4" />
                </button>
              </li>
            ))}
          </ul>
        )}
      </section>
    </div>
  </div>
);

const StartCard: React.FC<{
  icon: typeof Film;
  title: string;
  detail: string;
  onClick: () => void;
  disabled?: boolean;
  primary?: boolean;
}> = ({ icon: Icon, title, detail, onClick, disabled, primary }) => (
  <button
    type="button"
    onClick={onClick}
    disabled={disabled}
    className={
      "group flex items-start gap-3 rounded-panel border p-4 text-left transition-colors disabled:opacity-50 " +
      (primary
        ? "border-accent/50 bg-accent/10 hover:bg-accent/15 hover:border-accent-hover"
        : "border-studio-700 bg-studio-900 hover:bg-studio-850 hover:border-studio-600")
    }
  >
    <span
      className={
        "h-10 w-10 shrink-0 inline-flex items-center justify-center rounded-control " +
        (primary ? "bg-accent text-white" : "bg-studio-800 text-studio-200")
      }
    >
      <Icon className="w-5 h-5" />
    </span>
    <span className="min-w-0">
      <span className="block text-body font-semibold text-studio-100">{title}</span>
      <span className="mt-0.5 block text-label text-studio-400">{detail}</span>
    </span>
  </button>
);
