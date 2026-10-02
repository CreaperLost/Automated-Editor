import React, { useState } from "react";
import { Film, Image as ImageIcon, Music, Plus, Trash2, Upload } from "lucide-react";
import { useProjectStore } from "../../stores/projectStore";
import { api } from "../../lib/ipc";
import type { MediaAsset } from "../../lib/types";

/** dataTransfer type for dragging a media bin item onto the timeline. */
export const MEDIA_DRAG_TYPE = "application/x-aeroedits-media";

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

  const importFiles = () =>
    run("Importing…", async () => {
      const project = useProjectStore.getState().openedProject;
      if (!project) return;
      const paths = await api.pickMediaFiles();
      if (paths.length === 0) return;
      applyOpenedProject(await api.projectMediaImport(project.projectHandle, project.revision, paths));
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
        <button
          type="button"
          disabled={!openedProject || busy !== null}
          onClick={() => void importFiles()}
          className="flex items-center gap-1.5 px-2.5 py-1 rounded-md bg-fuchsia-600/25 hover:bg-fuchsia-600/35 border border-fuchsia-400/40 text-fuchsia-100 font-medium disabled:opacity-40"
          title="Copy videos, images or audio into the project"
        >
          <Upload className="w-3.5 h-3.5" />
          {busy ?? "Import…"}
        </button>
      </div>
      {error && (
        <p role="alert" className="mx-3 mt-2 text-[11px] text-rose-300 bg-rose-950/40 border border-rose-900/40 rounded p-2">
          {error}
        </p>
      )}
      <div className="flex-1 min-h-0 overflow-y-auto p-2 space-y-1">
        {assets.length === 0 && (
          <p className="px-1 py-2 text-[11px] text-studio-500 leading-relaxed">
            Import intros, B-roll, images or music. Drag an item onto the timeline to insert it at
            a clip edge, or use + to insert it at the playhead. Imported files are copied into the
            project folder.
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
              }}
              className="group flex items-center gap-2 rounded-md border border-studio-800 bg-studio-850 hover:border-fuchsia-400/50 px-2 py-1.5 cursor-grab"
              title={`${asset.name}. Drag onto the timeline to insert it.`}
            >
              <Icon className="w-3.5 h-3.5 shrink-0 text-fuchsia-300" />
              <div className="min-w-0 flex-1">
                <div className="truncate text-studio-100">{asset.name}</div>
                <div className="text-[10px] text-studio-500 font-mono">
                  {formatLength(asset)}
                  {asset.width > 0 && ` · ${asset.width}×${asset.height}`}
                  {asset.kind === "video" && !asset.audioPath && " · no audio"}
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
