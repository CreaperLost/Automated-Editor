import React, { useState } from "react";
import { AlertTriangle, Film, FolderInput, Image as ImageIcon, Music, Plus, Trash2, Upload } from "lucide-react";
import { useProjectStore } from "../../stores/projectStore";
import { api } from "../../lib/ipc";
import type { MediaAsset } from "../../lib/types";
import { audioStreamCount } from "../../lib/trackUtils";

/** dataTransfer type for dragging a media bin item onto the timeline. */
export const MEDIA_DRAG_TYPE = "application/x-aeroedits-media";

let draggedMediaId: string | null = null;
/** The media being dragged from the bin; drag-over events cannot read the drag's data. */
export const currentMediaDrag = () => draggedMediaId;

function formatLength(asset: MediaAsset): string {
  if (asset.kind === "image") return "still image";
  const seconds = asset.durationUs / 1e6;
  const minutes = Math.floor(seconds / 60);
  return minutes > 0 ? `${minutes}:${String(Math.floor(seconds % 60)).padStart(2, "0")}` : `${seconds.toFixed(1)}s`;
}

const KIND_ICON = { video: Film, image: ImageIcon, audio: Music } as const;

/// The project's media bin: import videos, images and audio, then drag them onto the
/// timeline (or insert at the playhead) to play between parts of the recording.
export const MediaPanel: React.FC = () => {
  const openedProject = useProjectStore((s) => s.openedProject);
  const applyOpenedProject = useProjectStore((s) => s.applyOpenedProject);
  const currentTimeUs = useProjectStore((s) => s.currentTimeUs);
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string>();
  const assets = openedProject?.mediaAssets ?? [];

  const run = async (label: string, work: () => Promise<void>) => {
    if (busy) return;
    setBusy(label);
    setError(undefined);
    try {
      await work();
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(null);
    }
  };

  const importFrom = (pick: () => Promise<string[]>) =>
    run("Importing…", async () => {
      const project = useProjectStore.getState().openedProject;
      if (!project) return;
      const paths = await pick();
      if (paths.length === 0) return;
      applyOpenedProject(await api.projectMediaImport(project.projectHandle, project.revision, paths));
    });
  const importFiles = () => importFrom(() => api.pickMediaFiles());
  const importFolder = () =>
    importFrom(async () => {
      const folder = await api.pickMediaFolder();
      return folder ? [folder] : [];
    });
  const missing = assets.filter((asset) => asset.missing);
  /** Removes every file that is gone, and its clips, one undo step each. */
  const cleanUpMissing = () =>
    run("Cleaning up…", async () => {
      for (const asset of missing) {
        const project = useProjectStore.getState().openedProject;
        if (!project) return;
        applyOpenedProject(await api.projectMediaRemove(project.projectHandle, project.revision, asset.id));
      }
    });

  const insertAtPlayhead = (asset: MediaAsset) =>
    run("Inserting…", async () => {
      const project = useProjectStore.getState().openedProject;
      if (!project) return;
      applyOpenedProject(await api.projectMediaInsert(project.projectHandle, project.revision, asset.id, currentTimeUs));
    });

  const remove = (asset: MediaAsset) =>
    run("Removing…", async () => {
      const project = useProjectStore.getState().openedProject;
      if (!project) return;
      applyOpenedProject(await api.projectMediaRemove(project.projectHandle, project.revision, asset.id));
    });

  return (
    <div className="h-full flex flex-col min-h-0 bg-studio-900/95 text-xs select-none">
      <div className="flex items-center justify-between gap-2 px-3 py-2 border-b border-studio-800">
        <span className="text-studio-400">
          {assets.length === 0 ? "No media yet" : `${assets.length} file${assets.length === 1 ? "" : "s"}`}
        </span>
        <div className="flex items-center gap-1">
        <button
          type="button"
          disabled={!openedProject || busy !== null}
          onClick={() => void importFolder()}
          aria-label="Import a folder"
          className="p-1.5 rounded-md border border-studio-700 text-studio-300 hover:text-white hover:bg-studio-800 disabled:opacity-40"
          title="Import every video, image and audio file in a folder"
        >
          <FolderInput className="w-3.5 h-3.5" />
        </button>
        <button
          type="button"
          disabled={!openedProject || busy !== null}
          onClick={() => void importFiles()}
          className="flex items-center gap-1.5 px-2.5 py-1 rounded-md bg-fuchsia-600/25 hover:bg-fuchsia-600/35 border border-fuchsia-400/40 text-fuchsia-100 font-medium disabled:opacity-40"
          title="Add videos, images or audio. Files stay where they are; the project refers to them."
        >
          <Upload className="w-3.5 h-3.5" />
          {busy ?? "Import…"}
        </button>
        </div>
      </div>
      {missing.length > 0 && (
        <div className="mx-3 mt-2 flex items-center gap-2 rounded border border-amber-800/50 bg-amber-950/30 p-2 text-[11px] text-amber-200">
          <AlertTriangle className="w-3.5 h-3.5 shrink-0" />
          <span className="flex-1">
            {missing.length} file{missing.length === 1 ? " was" : "s were"} moved or deleted. Their clips show nothing.
          </span>
          <button
            type="button"
            disabled={busy !== null}
            onClick={() => void cleanUpMissing()}
            className="px-2 py-0.5 rounded border border-amber-700/60 hover:bg-amber-900/40 disabled:opacity-40"
            title="Remove the missing files and their clips from the project (undoable)"
          >
            Clean up
          </button>
        </div>
      )}
      {error && (
        <p role="alert" className="mx-3 mt-2 text-[11px] text-rose-300 bg-rose-950/40 border border-rose-900/40 rounded p-2">
          {error}
        </p>
      )}
      <div className="flex-1 min-h-0 overflow-y-auto p-2 space-y-1">
        {assets.length === 0 && (
          <p className="px-1 py-2 text-[11px] text-studio-500 leading-relaxed">
            Import intros, B-roll, images or music, one by one or a whole folder. A recorder
            folder comes in as one recording, with its camera, sound and mouse data. Drag an item onto
            the timeline to insert it at a clip edge, or use + to insert it at the playhead. Files
            stay where they are: moving or deleting one later shows it as missing here.
          </p>
        )}
        {assets.map((asset) => {
          const Icon = KIND_ICON[asset.kind];
          return (
            <div
              key={asset.id}
              draggable
              onDragStart={(event) => {
                event.dataTransfer.setData(MEDIA_DRAG_TYPE, asset.id);
                event.dataTransfer.effectAllowed = "copy";
                draggedMediaId = asset.id;
              }}
              onDragEnd={() => {
                draggedMediaId = null;
              }}
              className={`group flex items-center gap-2 rounded-md border bg-studio-850 px-2 py-1.5 cursor-grab ${
                asset.missing ? "border-amber-700/60 opacity-70" : "border-studio-800 hover:border-fuchsia-400/50"
              }`}
              title={`${asset.missing ? `MISSING: ${asset.sourcePath ?? asset.name} is no longer there. ` : ""}${asset.name}${
                audioStreamCount(asset) > 1 ? ` (audio: ${(asset.audioNames ?? []).join(", ")})` : ""
              }. Drag onto the main track to insert it, or onto a track above to lay it over the video.`}
            >
              <Icon className="w-3.5 h-3.5 shrink-0 text-fuchsia-300" />
              <div className="min-w-0 flex-1">
                <div className="truncate text-studio-100">
                  {asset.missing && <span className="mr-1 text-amber-300">Missing ·</span>}
                  {asset.recordingPath && (
                    <span
                      className="mr-1 px-1 rounded bg-teal-500/20 text-teal-200 text-[9px] font-semibold uppercase"
                      title="A recording: its screen, camera, sound and mouse data (for its own auto-zoom)"
                    >
                      Rec
                    </span>
                  )}
                  {asset.name}
                </div>
                <div className="text-[10px] text-studio-500 font-mono">
                  {formatLength(asset)}
                  {asset.width > 0 && ` · ${asset.width}×${asset.height}`}
                  {asset.kind === "video" && !asset.audioPath && " · no audio"}
                  {audioStreamCount(asset) > 1 && ` · ${audioStreamCount(asset)} audio tracks`}
                </div>
              </div>
              <button
                type="button"
                aria-label={`Insert ${asset.name} at the playhead`}
                title="Insert at the playhead"
                disabled={busy !== null}
                onClick={() => void insertAtPlayhead(asset)}
                className="p-1 rounded text-studio-400 hover:text-white hover:bg-studio-700 disabled:opacity-40"
              >
                <Plus className="w-3.5 h-3.5" />
              </button>
              <button
                type="button"
                aria-label={`Remove ${asset.name}`}
                title="Remove from the project and the timeline (undoable)"
                disabled={busy !== null}
                onClick={() => void remove(asset)}
                className="p-1 rounded text-studio-500 hover:text-rose-300 hover:bg-studio-700 disabled:opacity-40"
              >
                <Trash2 className="w-3.5 h-3.5" />
              </button>
            </div>
          );
        })}
      </div>
    </div>
  );
};
