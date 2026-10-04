import React, { useState } from "react";
import { AlertTriangle, Clapperboard, Film, FolderInput, Image as ImageIcon, Music, Plus, Search, Trash2, Upload } from "lucide-react";
import { Badge, Button, IconButton, Notice, Segmented } from "../ui";
import { useProjectStore } from "../../stores/projectStore";
import { api } from "../../lib/ipc";
import type { Asset } from "../../lib/types";

/** dataTransfer type for dragging a media bin item onto the timeline. */
export const MEDIA_DRAG_TYPE = "application/x-aeroedits-media";

let draggedMediaId: string | null = null;
/** The media being dragged from the bin; drag-over events cannot read the drag's data. */
export const currentMediaDrag = () => draggedMediaId;

const soundCount = (asset: Asset) => asset.streams.filter((s) => s.kind === "sound").length;

function formatLength(asset: Asset): string {
  if (asset.kind === "image") return "still image";
  const seconds = asset.durationUs / 1e6;
  const minutes = Math.floor(seconds / 60);
  return minutes > 0 ? `${minutes}:${String(Math.floor(seconds % 60)).padStart(2, "0")}` : `${seconds.toFixed(1)}s`;
}

const KIND_ICON = { recording: Clapperboard, video: Film, image: ImageIcon, audio: Music } as const;

/// The project's media: recordings, videos, images and audio. Drag one onto a track (or add it
/// at the playhead) to put its picture and sound on the timeline as linked clips.
export const MediaPanel: React.FC = () => {
  const openedProject = useProjectStore((s) => s.openedProject);
  const applyOpenedProject = useProjectStore((s) => s.applyOpenedProject);
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string>();
  const [search, setSearch] = useState("");
  const [kind, setKind] = useState<"all" | Asset["kind"]>("all");
  const assets = openedProject?.assets ?? [];
  const placed = new Set(openedProject?.sequence.tracks.flatMap((t) => t.clips.map((c) => c.asset)) ?? []);

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

  /** At the playhead, on the first track with room (or a new one). */
  const addAtPlayhead = (asset: Asset) =>
    run("Adding…", async () => {
      const project = useProjectStore.getState().openedProject;
      if (!project) return;
      applyOpenedProject(
        await api.projectSequenceEdit(project.projectHandle, project.revision, {
          kind: "placeAsset",
          assetId: asset.id,
          atUs: Math.round(useProjectStore.getState().currentTimeUs),
          trackId: null,
        }),
      );
    });

  const remove = (asset: Asset) =>
    run("Removing…", async () => {
      const project = useProjectStore.getState().openedProject;
      if (!project) return;
      applyOpenedProject(await api.projectMediaRemove(project.projectHandle, project.revision, asset.id));
    });

  const query = search.trim().toLowerCase();
  const shown = assets.filter(
    (asset) => (kind === "all" || asset.kind === kind) && (!query || asset.name.toLowerCase().includes(query)),
  );

  return (
    <div className="h-full flex flex-col min-h-0 bg-studio-900 text-label select-none">
      <div className="shrink-0 space-y-2 p-3 border-b border-studio-800">
        <div className="flex items-center gap-2">
          <Button
            variant="primary"
            icon={Upload}
            className="flex-1"
            disabled={!openedProject || busy !== null}
            onClick={() => void importFiles()}
            title="Add videos, images or audio. Files stay where they are; the project refers to them."
          >
            {busy ?? "Import media"}
          </Button>
          <IconButton
            icon={FolderInput}
            variant="secondary"
            label="Import every video, image and audio file in a folder"
            disabled={!openedProject || busy !== null}
            onClick={() => void importFolder()}
          />
        </div>
        {assets.length > 0 && (
          <>
            <div className="relative">
              <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 w-4 h-4 text-studio-500 pointer-events-none" />
              <input
                type="search"
                aria-label="Search media"
                placeholder="Search media…"
                value={search}
                onChange={(e) => setSearch(e.target.value)}
                className="ui-field w-full pl-8"
              />
            </div>
            <Segmented<"all" | Asset["kind"]>
              label="Show"
              size="sm"
              className="w-full"
              value={kind}
              onChange={setKind}
              options={[
                { value: "all", label: `All ${assets.length}` },
                ...(assets.some((a) => a.kind === "recording") ? [{ value: "recording" as const, label: "Recordings" }] : []),
                { value: "video", label: "Video" },
                { value: "image", label: "Images" },
                { value: "audio", label: "Audio" },
              ]}
            />
          </>
        )}
      </div>
      {missing.length > 0 && (
        <div className="mx-3 mt-3 flex items-center gap-2 rounded-control border border-suggest/40 bg-suggest/10 px-3 py-2 text-label text-suggest-fg">
          <AlertTriangle className="w-4 h-4 shrink-0" />
          <span className="flex-1">
            {missing.length} file{missing.length === 1 ? " was" : "s were"} moved or deleted. Their clips show nothing.
          </span>
          <Button
            variant="ghost"
            size="sm"
            disabled={busy !== null}
            onClick={() => void cleanUpMissing()}
            className="text-suggest-fg border-suggest/50"
            title="Remove the missing files and their clips from the project (undoable)"
          >
            Clean up
          </Button>
        </div>
      )}
      {error && (
        <Notice tone="danger" className="mx-3 mt-3 rounded-control border" onDismiss={() => setError(undefined)}>
          {error}
        </Notice>
      )}
      <div className="flex-1 min-h-0 overflow-y-auto p-2 space-y-1">
        {assets.length === 0 && (
          <div className="m-1 rounded-panel border border-dashed border-studio-700 p-4 text-center">
            <Upload className="mx-auto w-6 h-6 text-studio-500" />
            <p className="mt-2 text-label text-studio-300">Import intros, B-roll, images or music.</p>
            <p className="mt-1 text-meta text-studio-500 leading-relaxed">
              One by one or a whole folder; a recorder folder comes in as one recording with its camera, sound and mouse
              data. Drag an item onto a track, or use + to add it at the playhead. Files stay where they are.
            </p>
          </div>
        )}
        {assets.length > 0 && shown.length === 0 && (
          <p className="px-2 py-3 text-label text-studio-500">Nothing matches.</p>
        )}
        {shown.map((asset) => {
          const Icon = KIND_ICON[asset.kind];
          const sound = asset.kind === "audio";
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
              className={`group flex items-center gap-2.5 rounded-control border px-2 py-2 cursor-grab transition-colors ${
                asset.missing
                  ? "border-suggest/50 bg-suggest/5 opacity-75"
                  : "border-transparent hover:border-studio-700 hover:bg-studio-850"
              }`}
              title={`${asset.missing ? `MISSING: ${asset.path} is no longer there. ` : ""}${asset.name}${
                soundCount(asset) > 1 ? ` (sound: ${asset.streams.filter((s) => s.kind === "sound").map((s) => s.name).join(", ")})` : ""
              }. Drag onto a track: its picture and sound go on as linked clips.`}
            >
              <span
                className={`h-9 w-9 shrink-0 inline-flex items-center justify-center rounded-control ${
                  sound ? "bg-audio/15 text-audio-fg" : "bg-video/20 text-video-fg"
                }`}
              >
                <Icon className="w-4 h-4" />
              </span>
              <div className="min-w-0 flex-1">
                <div className="flex items-center gap-1.5 min-w-0">
                  {asset.kind === "recording" && (
                    <Badge tone="video" title="A recording: its screen, camera, sound and mouse data (for its own auto-zoom)">
                      Rec
                    </Badge>
                  )}
                  {asset.missing && <Badge tone="suggest">Missing</Badge>}
                  <span className="truncate text-label font-medium text-studio-100">{asset.name}</span>
                </div>
                <div className="text-meta text-studio-500 truncate">
                  {formatLength(asset)}
                  {asset.width > 0 && ` · ${asset.width}×${asset.height}`}
                  {asset.kind === "video" && soundCount(asset) === 0 && " · no sound"}
                  {soundCount(asset) > 1 && ` · ${soundCount(asset)} sounds`}
                  {!placed.has(asset.id) && " · not on the timeline"}
                </div>
              </div>
              <div className="flex items-center opacity-0 group-hover:opacity-100 focus-within:opacity-100 transition-opacity">
                <IconButton
                  icon={Plus}
                  size="sm"
                  label={`Add ${asset.name} at the playhead`}
                  disabled={busy !== null}
                  onClick={() => void addAtPlayhead(asset)}
                />
                <IconButton
                  icon={Trash2}
                  size="sm"
                  label={`Remove ${asset.name} from the project and the timeline (undoable)`}
                  disabled={busy !== null}
                  onClick={() => void remove(asset)}
                  className="hover:!text-danger-fg hover:!bg-danger/15"
                />
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
};
