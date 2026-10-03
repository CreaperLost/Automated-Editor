import React, { useRef, useState } from "react";
import {
  Folder,
  FolderOpen,
  Pencil,
  Scissors,
  Download,
  Smartphone,
  FilePlus2,
  Keyboard,
} from "lucide-react";
import { useProjectStore } from "../../stores/projectStore";
import { api } from "../../lib/ipc";
import { ExportStatus } from "../../lib/types";
import { LayoutMenu } from "../layout/LayoutMenu";
import { HotkeysDialog } from "../settings/HotkeysDialog";

interface EditorTopBarProps {
  onOpenExport: () => void;
  exportJob?: ExportStatus;
  busy: boolean;
  onOpenFolder: () => void;
  onNewProject: () => void;
  onCloseProject: () => void;
  onShowInFinder: () => void;
}

export const EditorTopBar: React.FC<EditorTopBarProps> = ({
  onOpenExport,
  exportJob,
  busy,
  onOpenFolder,
  onNewProject,
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
  const [hotkeysOpen, setHotkeysOpen] = useState(false);
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
        <img src="/aeroedits-icon.svg" alt="" className="w-9 h-9 drop-shadow-md" draggable={false} />
        <div className="hidden xl:flex items-center space-x-2">
          <span className="font-bold text-white tracking-tight text-sm">
            AeroEdits
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
            No project open: start a new one or open an existing one
          </span>
        )}
      </div>

      {/* 3. Right: Action Controls */}
      <div className="flex items-center gap-2 shrink-0">
        <button
          type="button"
          disabled={busy}
          onClick={onNewProject}
          className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-studio-850 hover:bg-studio-800 border border-studio-700 text-studio-200 text-xs font-medium disabled:opacity-40 transition-colors"
          title="Start a new project, empty or from a recording"
        >
          <FilePlus2 className="w-3.5 h-3.5" />
          <span className="hidden xl:inline">New</span>
        </button>
        <button
          type="button"
          disabled={busy}
          onClick={onOpenFolder}
          className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-teal-600/20 border border-teal-500/40 text-teal-300 text-xs font-medium hover:bg-teal-600/30 disabled:opacity-40 transition-colors"
          title="Open a project folder, or a recording folder to edit it in place"
        >
          <FolderOpen className="w-3.5 h-3.5" />
          <span className="hidden xl:inline">Open</span>
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
            <span className="hidden xl:inline">Finder</span>
          </button>
        )}

        <button
          type="button"
          onClick={() => setHotkeysOpen(true)}
          aria-label="Keyboard shortcuts"
          className="p-1.5 rounded-lg bg-studio-850 hover:bg-studio-800 border border-studio-700 text-studio-300 hover:text-white transition-colors"
          title="Keyboard shortcuts: see and change them"
        >
          <Keyboard className="w-3.5 h-3.5" />
        </button>
        {hotkeysOpen && <HotkeysDialog onClose={() => setHotkeysOpen(false)} />}

        {project && (
          <>
            <LayoutMenu />

            <button
              type="button"
              onClick={() => setIsSilenceModalOpen(true)}
              className="flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg bg-studio-850 hover:bg-studio-800 border border-studio-700 text-amber-300 hover:text-amber-200 text-xs font-medium transition-colors"
              title="Find the silent pauses and cut them out (jump cuts)"
            >
              <Scissors className="w-3.5 h-3.5" />
              <span>Jump Cuts</span>
            </button>

            <button
              type="button"
              onClick={() => void api.openShortsWindow().catch(() => undefined)}
              className="flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg bg-studio-850 hover:bg-studio-800 border border-studio-700 text-studio-200 text-xs font-medium transition-colors"
              title="Open the Shorts Studio: vertical split-screen clips from this video"
            >
              <Smartphone className="w-3.5 h-3.5" />
              <span>Shorts</span>
            </button>

            {/* Export: settings, destination and progress live in the export dialog. */}
            <button
              type="button"
              disabled={busy}
              onClick={onOpenExport}
              className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-teal-600 hover:bg-teal-500 text-white text-xs font-semibold shadow-md shadow-teal-900/40 disabled:opacity-40 transition-all"
              title="Choose resolution, frame rate and quality, then export an MP4"
            >
              <Download className="w-3.5 h-3.5" />
              <span>
                {exporting
                  ? `Exporting ${
                      exportJob && exportJob.progressDenominator > 0
                        ? Math.round((exportJob.progressNumerator / exportJob.progressDenominator) * 100)
                        : 0
                    }%`
                  : "Export…"}
              </span>
            </button>

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
