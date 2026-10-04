import React, { useRef, useState } from "react";
import { useShallow } from "zustand/react/shallow";
import {
  AlertCircle,
  Check,
  Clock,
  Download,
  FilePlus2,
  Folder,
  FolderOpen,
  Keyboard,
  LayoutGrid,
  Loader2,
  Redo2,
  RotateCcw,
  Scissors,
  Undo2,
  X,
} from "lucide-react";
import { useProjectStore } from "../../stores/projectStore";
import { useSaveStatusStore } from "../../stores/saveStatusStore";
import { useWorkspaceStore } from "../../stores/workspaceStore";
import { useHotkeyStore, formatBinding } from "../../stores/hotkeyStore";
import { api } from "../../lib/ipc";
import { ExportStatus } from "../../lib/types";
import { FILE_MANAGER, redoProject, undoProject } from "../../lib/projectActions";
import {
  LAYOUT_PRESETS,
  WORKSPACES,
  applyPresetToWorkspace,
  resetWorkspaceLayout,
  type LayoutPreset,
} from "../layout/dockLayout";
import { HotkeysDialog } from "../settings/HotkeysDialog";
import { Button, IconButton, Menu, MenuButton, Tabs, cn, type MenuEntry } from "../ui";

interface EditorTopBarProps {
  onOpenExport: () => void;
  exportJob?: ExportStatus;
  busy: boolean;
  onOpenFolder: () => void;
  onNewProject: () => void;
  onOpenRecent: (path: string) => void;
  onCloseProject: () => void;
  onShowInFinder: () => void;
  onError: (message: string) => void;
}

function folderName(path: string) {
  const parts = path.split(/[/\\]/).filter(Boolean);
  return parts[parts.length - 1] ?? path;
}

/// The window's header: the File menu, the project and whether it is saved, undo and redo,
/// the workspaces, and Export.
export const EditorTopBar: React.FC<EditorTopBarProps> = ({
  onOpenExport,
  exportJob,
  busy,
  onOpenFolder,
  onNewProject,
  onOpenRecent,
  onCloseProject,
  onShowInFinder,
  onError,
}) => {
  const { project, projectPath, recentProjects, setIsSilenceModalOpen } = useProjectStore(
    useShallow((s) => ({
      project: s.openedProject,
      projectPath: s.projectPath,
      recentProjects: s.recentProjects,
      setIsSilenceModalOpen: s.setIsSilenceModalOpen,
    })),
  );
  const workspace = useWorkspaceStore((s) => s.workspace);
  const setWorkspace = useWorkspaceStore((s) => s.setWorkspace);
  const bindings = useHotkeyStore((s) => s.bindings);
  const [hotkeysOpen, setHotkeysOpen] = useState(false);

  const shortcut = (action: "undo" | "redo") => (bindings[action][0] ? formatBinding(bindings[action][0]) : undefined);
  const run = (work: () => Promise<unknown>) => void work().catch((err) => onError(String(err)));

  const exporting = exportJob?.state === "queued" || exportJob?.state === "running";
  const exportPercent =
    exportJob && exportJob.progressDenominator > 0
      ? Math.round((exportJob.progressNumerator / exportJob.progressDenominator) * 100)
      : 0;

  const fileMenu: MenuEntry[] = [
    { label: "New project…", icon: FilePlus2, onSelect: onNewProject, disabled: busy },
    { label: "Open project…", icon: FolderOpen, onSelect: onOpenFolder, disabled: busy },
    {
      kind: "submenu",
      label: "Open recent",
      icon: Clock,
      disabled: recentProjects.length === 0,
      entries: recentProjects.slice(0, 10).map((path) => ({
        label: folderName(path),
        hint: undefined,
        onSelect: () => onOpenRecent(path),
      })),
    },
    { kind: "separator" },
    {
      label: `Show in ${FILE_MANAGER}`,
      icon: Folder,
      onSelect: onShowInFinder,
      disabled: !project || !projectPath,
      disabledReason: "Open a project first",
    },
    { label: "Close project", icon: X, onSelect: onCloseProject, disabled: !project || busy },
    { kind: "separator" },
    { kind: "heading", label: `Layout · ${WORKSPACES[workspace].label}` },
    ...(Object.keys(LAYOUT_PRESETS) as LayoutPreset[]).map(
      (preset): MenuEntry => ({
        label: LAYOUT_PRESETS[preset].label,
        icon: LayoutGrid,
        onSelect: () => applyPresetToWorkspace(preset),
        disabled: !project,
      }),
    ),
    { label: "Reset this workspace", icon: RotateCcw, onSelect: resetWorkspaceLayout, disabled: !project },
    { kind: "separator" },
    { label: "Keyboard shortcuts…", icon: Keyboard, onSelect: () => setHotkeysOpen(true) },
  ];

  return (
    <header className="h-header shrink-0 z-30 flex items-center gap-2 border-b border-studio-800 bg-studio-900 px-3 select-none">
      {/* Brand and File */}
      <div className="flex items-center gap-1 shrink-0">
        <div className="flex items-center gap-2 pr-2">
          <img src="/aeroedits-icon.svg" alt="" className="w-7 h-7" draggable={false} />
          <span className="hidden lg:inline font-display text-body font-semibold text-studio-100 tracking-tight">
            AeroEdits
          </span>
        </div>
        <Menu
          label="File"
          entries={fileMenu}
          width={280}
          trigger={({ open, toggle, ref }) => (
            <MenuButton ref={ref} open={open} onClick={toggle}>
              File
            </MenuButton>
          )}
        />
      </div>

      {project && (
        <>
          <div className="h-6 w-px bg-studio-800 mx-1 shrink-0" aria-hidden />
          <ProjectName />
          <SaveIndicator />
          <div className="h-6 w-px bg-studio-800 mx-1 shrink-0" aria-hidden />
          <div className="flex items-center gap-0.5 shrink-0">
            <IconButton
              icon={Undo2}
              label={`Undo${shortcut("undo") ? ` (${shortcut("undo")})` : ""}`}
              disabled={!project.undoAvailable}
              onClick={() => run(undoProject)}
            />
            <IconButton
              icon={Redo2}
              label={`Redo${shortcut("redo") ? ` (${shortcut("redo")})` : ""}`}
              disabled={!project.redoAvailable}
              onClick={() => run(redoProject)}
            />
          </div>
        </>
      )}

      {/* Workspaces, centred in the window */}
      <div className="flex-1 flex justify-center self-stretch">
        {project && (
          <Tabs
            label="Workspace"
            className="self-stretch"
            value={workspace}
            onChange={(next) => {
              if (next === "shorts") {
                run(() => api.openShortsWindow());
                return;
              }
              setWorkspace(next);
            }}
            items={[
              { value: "edit", label: "Edit", title: "Edit the video on the timeline" },
              { value: "cleanup", label: "Cleanup", title: "Edit by text: a tall transcript, the preview and the timeline" },
              { value: "shorts", label: "Shorts", title: "Open the Shorts Studio: vertical clips from this video, in their own window" },
            ]}
          />
        )}
      </div>

      {/* Actions */}
      <div className="flex items-center gap-2 shrink-0">
        {project ? (
          <>
            <Button
              variant="secondary"
              icon={Scissors}
              onClick={() => setIsSilenceModalOpen(true)}
              title="Find the silent pauses and cut them out (jump cuts)"
            >
              <span className="hidden xl:inline">Jump Cuts</span>
            </Button>
            <Button
              variant="primary"
              icon={exporting ? Loader2 : Download}
              disabled={busy}
              onClick={onOpenExport}
              className={cn(exporting && "[&>svg]:animate-spin", "min-w-[104px]")}
              title="Choose resolution, frame rate and quality, then export an MP4"
            >
              {exporting ? `Exporting ${exportPercent}%` : "Export"}
            </Button>
          </>
        ) : (
          <>
            <Button variant="secondary" icon={FolderOpen} onClick={onOpenFolder} disabled={busy}>
              Open
            </Button>
            <Button variant="primary" icon={FilePlus2} onClick={onNewProject} disabled={busy}>
              New project
            </Button>
          </>
        )}
      </div>
      {hotkeysOpen && <HotkeysDialog onClose={() => setHotkeysOpen(false)} />}
    </header>
  );
};

/** The project's name; click to rename. */
const ProjectName: React.FC = () => {
  const project = useProjectStore((s) => s.openedProject);
  const applyOpenedProject = useProjectStore((s) => s.applyOpenedProject);
  const [value, setValue] = useState(project?.manifest.projectName ?? "");
  const [renaming, setRenaming] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);

  React.useEffect(() => {
    setValue(project?.manifest.projectName ?? "");
  }, [project?.projectHandle, project?.manifest.projectName]);

  if (!project) return null;
  const rename = async () => {
    const trimmed = value.trim();
    if (renaming || !trimmed || trimmed === project.manifest.projectName) {
      setValue(project.manifest.projectName);
      return;
    }
    setRenaming(true);
    try {
      applyOpenedProject(await api.projectRename(project.projectHandle, trimmed));
    } catch {
      setValue(project.manifest.projectName);
    } finally {
      setRenaming(false);
    }
  };

  return (
    <input
      ref={inputRef}
      type="text"
      aria-label="Project name"
      title="Project name: click to rename"
      value={value}
      disabled={renaming}
      maxLength={80}
      size={Math.min(32, Math.max(8, value.length + 1))}
      onChange={(e) => setValue(e.target.value)}
      onBlur={() => void rename()}
      onKeyDown={(e) => {
        if (e.key === "Enter") e.currentTarget.blur();
        if (e.key === "Escape") {
          setValue(project.manifest.projectName);
          e.currentTarget.blur();
        }
      }}
      className="h-control min-w-0 max-w-[18rem] rounded-control border border-transparent bg-transparent px-2 text-body font-semibold text-studio-100 truncate select-text transition-colors hover:border-studio-700 hover:bg-studio-850 focus:border-accent-hover focus:bg-studio-950 focus:outline-none"
    />
  );
};

/** Saved, Saving… or Not saved, from the answers to the project's edits. */
const SaveIndicator: React.FC = () => {
  const pending = useSaveStatusStore((s) => s.pending);
  const failed = useSaveStatusStore((s) => s.failed);
  const clearFailure = useSaveStatusStore((s) => s.clearFailure);
  const revision = useProjectStore((s) => s.openedProject?.revision);

  if (pending > 0) {
    return (
      <span className="inline-flex items-center gap-1.5 text-label text-studio-400 shrink-0" role="status">
        <Loader2 className="w-4 h-4 animate-spin" /> <span className="hidden lg:inline">Saving…</span>
      </span>
    );
  }
  if (failed) {
    return (
      <button
        type="button"
        onClick={clearFailure}
        title={`The last change was not saved: ${failed}\nClick to dismiss.`}
        className="inline-flex items-center gap-1.5 h-control px-2 rounded-control text-label text-danger-fg hover:bg-danger/10 shrink-0"
      >
        <AlertCircle className="w-4 h-4" /> Not saved
      </button>
    );
  }
  return (
    <span
      className="inline-flex items-center gap-1.5 text-label text-studio-400 shrink-0"
      title={`Every change is saved as you make it (revision ${revision ?? 0}).`}
      role="status"
    >
      <Check className="w-4 h-4 text-success" /> <span className="hidden lg:inline">Saved</span>
    </span>
  );
};
