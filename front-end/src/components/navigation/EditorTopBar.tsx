import React, { useRef, useState } from "react";
import {
  Folder,
  FolderOpen,
  Film,
  Pencil,
  Scissors,
  Download,
  XCircle,
} from "lucide-react";
import { useProjectStore } from "../../stores/projectStore";
import { api } from "../../lib/ipc";
import { ExportStatus } from "../../lib/types";

interface EditorTopBarProps {
  resolution: string;
  setResolution: (res: string) => void;
  exportFps: number;
  setExportFps: (fps: number) => void;
  exportDestination: string;
  chooseExportDestination: () => void;
  startExport: () => void;
  cancelExport: () => void;
  exportJob?: ExportStatus;
  busy: boolean;
  onOpenFolder: () => void;
  onCloseProject: () => void;
  onShowInFinder: () => void;
}

export const EditorTopBar: React.FC<EditorTopBarProps> = ({
  resolution,
  setResolution,
  exportFps,
  setExportFps,
  exportDestination,
  chooseExportDestination,
  startExport,
  cancelExport,
  exportJob,
  busy,
  onOpenFolder,
  onCloseProject,
  onShowInFinder,
}) => {
  const {
    openedProject: project,
    projectPath,
    applyOpenedProject,
    setIsSilenceModalOpen,
  } = useProjectStore();

  const [projectNameInput, setProjectNameInput] = useState(
    project?.manifest.projectName ?? "",
  );
  const [isRenaming, setIsRenaming] = useState(false);
  const projectNameInputRef = useRef<HTMLInputElement>(null);

  React.useEffect(() => {
    setProjectNameInput(project?.manifest.projectName ?? "");
  }, [project?.projectHandle, project?.manifest.projectName]);

  const handleRename = async () => {
    if (!project || isRenaming) return;
    const trimmed = projectNameInput.trim();
    if (!trimmed || trimmed === project.manifest.projectName) {
      setProjectNameInput(project.manifest.projectName);
      return;
    }
    setIsRenaming(true);
    try {
      const updated = await api.projectRename(project.projectHandle, trimmed);
      applyOpenedProject(updated);
    } catch {
      setProjectNameInput(project.manifest.projectName);
    } finally {
      setIsRenaming(false);
    }
  };

  const exporting =
    exportJob?.state === "queued" || exportJob?.state === "running";

  return (
    <header className="h-14 border-b border-studio-800/80 bg-studio-900/90 backdrop-blur-xl px-4 flex items-center justify-between gap-4 select-none z-30 shrink-0">
      {/* 1. Left: Brand & Studio Name */}
      <div className="flex items-center space-x-3 shrink-0">
        <div className="flex items-center justify-center w-8 h-8 rounded-xl bg-gradient-to-tr from-teal-600 to-emerald-400 text-white font-black text-sm shadow-md shadow-teal-600/30">
          <Film className="w-4 h-4" />
        </div>
        <div className="flex items-center space-x-2">
          <span className="font-bold text-white tracking-tight text-sm">
            AeroShoot
          </span>
          <span className="text-[10px] px-2 py-0.5 rounded-full bg-teal-500/15 border border-teal-500/30 text-teal-300 font-mono font-medium">
            Video Editor
          </span>
        </div>
      </div>

      {/* 2. Center: Project Information & Rename */}
      <div className="flex items-center gap-3 min-w-0 flex-1 max-w-xl">
        {project ? (
          <div className="flex items-center gap-2 group min-w-0">
            <input
              ref={projectNameInputRef}
              type="text"
              aria-label="Project name"
              value={projectNameInput}
              disabled={isRenaming}
              maxLength={80}
              onChange={(e) => setProjectNameInput(e.target.value)}
              onBlur={() => void handleRename()}
              onKeyDown={(e) => {
                if (e.key === "Enter") e.currentTarget.blur();
                if (e.key === "Escape") {
                  setProjectNameInput(project.manifest.projectName);
                  e.currentTarget.blur();
                }
              }}
              className="text-sm text-white font-semibold bg-transparent hover:bg-studio-950/60 focus:bg-studio-950 border border-transparent hover:border-studio-700/60 focus:border-teal-500/80 rounded px-2 py-1 outline-none transition-colors truncate select-text"
              title="Click to rename project"
            />
            <button
              type="button"
              aria-label="Rename project"
              title="Rename project"
              disabled={isRenaming}
              onClick={() => {
                projectNameInputRef.current?.focus();
                projectNameInputRef.current?.select();
              }}
              className="p-1 rounded text-studio-500 hover:text-studio-300 hover:bg-studio-800/60 opacity-0 group-hover:opacity-100 focus:opacity-100 transition-opacity"
            >
              <Pencil className="w-3.5 h-3.5" />
            </button>
            <span className="text-[10px] font-mono px-1.5 py-0.5 rounded bg-studio-800 text-studio-400 shrink-0">
              rev {project.revision}
            </span>
          </div>
        ) : (
          <span className="text-xs text-studio-500 font-mono">
            No project open — pick a recording folder to start
          </span>
        )}
      </div>

      {/* 3. Right: Action Controls */}
      <div className="flex items-center gap-2 shrink-0">
        <button
          type="button"
          disabled={busy}
          onClick={onOpenFolder}
          className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-teal-600/20 border border-teal-500/40 text-teal-300 text-xs font-medium hover:bg-teal-600/30 disabled:opacity-40 transition-colors"
          title="Open a recorded project folder"
        >
          <FolderOpen className="w-3.5 h-3.5" />
          <span>Open Folder</span>
        </button>

        {project && projectPath && (
          <button
            type="button"
            disabled={busy}
            onClick={onShowInFinder}
            className="flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg bg-studio-850 hover:bg-studio-800 border border-studio-700 text-studio-300 hover:text-white text-xs font-medium transition-colors"
            title="Reveal project bundle in Finder"
          >
            <Folder className="w-3.5 h-3.5 text-indigo-400" />
            <span>Finder</span>
          </button>
        )}

        {project && (
          <>
            <button
              type="button"
              onClick={() => setIsSilenceModalOpen(true)}
              className="flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg bg-studio-850 hover:bg-studio-800 border border-studio-700 text-amber-300 hover:text-amber-200 text-xs font-medium transition-colors"
              title="AI Silence Cuts"
            >
              <Scissors className="w-3.5 h-3.5" />
              <span>Silence Cuts</span>
            </button>

            {/* Export Settings */}
            <div className="flex items-center bg-studio-950/60 border border-studio-800 rounded-lg p-0.5 text-[11px] font-mono">
              <select
                aria-label="Export resolution"
                disabled={exporting}
                value={resolution}
                onChange={(e) => setResolution(e.target.value)}
                className="bg-transparent text-studio-300 px-1.5 py-0.5 rounded outline-none"
              >
                <option value="1280x720">720p</option>
                <option value="1920x1080">1080p</option>
                <option value="3840x2160">4K</option>
              </select>
              <div className="h-3 w-px bg-studio-800 mx-0.5" />
              <select
                aria-label="Export frame rate"
                disabled={exporting}
                value={exportFps}
                onChange={(e) => setExportFps(Number(e.target.value))}
                className="bg-transparent text-studio-300 px-1.5 py-0.5 rounded outline-none"
              >
                <option value={24}>24fps</option>
                <option value={30}>30fps</option>
                <option value={60}>60fps</option>
              </select>
            </div>

            <button
              type="button"
              disabled={busy || exporting}
              onClick={chooseExportDestination}
              className="px-2 py-1.5 rounded-lg bg-studio-850 hover:bg-studio-800 border border-studio-700 text-studio-300 text-xs font-medium transition-colors"
              title={exportDestination || "Choose export save destination"}
            >
              Save as…
            </button>

            <button
              type="button"
              disabled={busy || exporting}
              onClick={startExport}
              className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-teal-600 hover:bg-teal-500 text-white text-xs font-semibold shadow-md shadow-teal-900/40 disabled:opacity-40 transition-all"
              title="Export high quality MP4 with H.264 video and AAC audio"
            >
              <Download className="w-3.5 h-3.5" />
              <span>{exporting ? "Exporting…" : "Export"}</span>
            </button>

            {exporting && (
              <button
                type="button"
                onClick={cancelExport}
                className="flex items-center gap-1 px-2 py-1.5 rounded-lg bg-rose-950/60 hover:bg-rose-900/60 border border-rose-800 text-rose-300 text-xs font-medium"
              >
                <XCircle className="w-3.5 h-3.5" />
                <span>Cancel</span>
              </button>
            )}

            <button
              type="button"
              disabled={busy}
              onClick={onCloseProject}
              className="text-xs text-studio-400 hover:text-studio-200 px-2 py-1 transition-colors"
              title="Close current project"
            >
              Close
            </button>
          </>
        )}
      </div>
    </header>
  );
};
