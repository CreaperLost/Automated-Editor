import React, { useCallback, useEffect, useState } from "react";
import { Download, Image as ImageIcon, Library, Loader2, Play, Plus, RotateCw, Sparkles, Trash2, X } from "lucide-react";
import { Badge, Button, IconButton, Notice, cn } from "../ui";
import { useProjectStore } from "../../stores/projectStore";
import { api, isTauriEnvironment } from "../../lib/ipc";
import { seekPlayback, togglePlayback } from "../../lib/playbackControl";
import type {
  GraphicCreated,
  GraphicLibraryItem,
  GraphicListed,
  GraphicTemplate,
  GraphicsInstallProgress,
  GraphicsRuntimeStatus,
} from "../../lib/types";
import { formatSeconds, libraryChanged, useGraphicsLibrary, useGraphicsUpdates } from "./graphicsEvents";

const MB = 1024 * 1024;

/// Motion graphics: the templates to add at the playhead, the graphics in the project with how
/// their renders are going, and the renderer itself (downloaded on first use).
export const GraphicsPanel: React.FC = () => {
  const openedProject = useProjectStore((s) => s.openedProject);
  const applyOpenedProject = useProjectStore((s) => s.applyOpenedProject);
  const setSelectedClipIds = useProjectStore((s) => s.setSelectedClipIds);
  const [runtime, setRuntime] = useState<GraphicsRuntimeStatus>();
  const [install, setInstall] = useState<GraphicsInstallProgress | null>(null);
  const [templates, setTemplates] = useState<GraphicTemplate[]>([]);
  const [graphics, setGraphics] = useState<GraphicListed[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string>();
  const handle = openedProject?.projectHandle;
  const revision = openedProject?.revision;

  useEffect(() => {
    if (!isTauriEnvironment()) return;
    void api.graphicsRuntimeStatus().then(setRuntime).catch((e) => setError(String(e)));
    void api.graphicsTemplates().then(setTemplates).catch((e) => setError(String(e)));
  }, []);

  const refreshGraphics = useCallback(() => {
    if (!handle) {
      setGraphics([]);
      return;
    }
    void api
      .projectGraphicsList(handle)
      .then(setGraphics)
      .catch(() => undefined);
  }, [handle]);
  // The list follows the project (graphics added, renamed, removed) and the renders.
  useEffect(refreshGraphics, [refreshGraphics, revision]);
  useGraphicsUpdates(refreshGraphics);

  useEffect(() => {
    if (!isTauriEnvironment()) return;
    let unlisten: (() => void) | undefined;
    let disposed = false;
    void import("@tauri-apps/api/event").then(({ listen }) =>
      listen<GraphicsInstallProgress>("graphics-runtime-progress", (event) => setInstall(event.payload)).then((fn) => {
        if (disposed) fn();
        else unlisten = fn;
      }),
    );
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  const installRuntime = async () => {
    setError(undefined);
    setInstall({ step: "Starting", bytesDone: 0, bytesTotal: null });
    try {
      setRuntime(await api.graphicsRuntimeInstall());
    } catch (err) {
      setError(String(err));
    } finally {
      setInstall(null);
    }
  };
  const removeRuntime = async () => {
    setError(undefined);
    try {
      setRuntime(await api.graphicsRuntimeRemove());
    } catch (err) {
      setError(String(err));
    }
  };

  const library = useGraphicsLibrary();

  /** Adds a graphic at the playhead and opens its form in the Inspector. */
  const addWith = async (make: (handle: string, revision: number, atUs: number) => Promise<GraphicCreated>) => {
    const project = useProjectStore.getState().openedProject;
    if (!project || busy) return;
    setBusy(true);
    setError(undefined);
    try {
      const created = await make(project.projectHandle, project.revision, useProjectStore.getState().currentTimeUs);
      applyOpenedProject(created.project);
      setSelectedClipIds([created.clipId]);
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  };
  const add = (template: GraphicTemplate) =>
    addWith((handle, revision, atUs) => api.projectGraphicCreate(handle, revision, template.name, atUs));
  const use = (item: GraphicLibraryItem) =>
    addWith((handle, revision, atUs) => api.projectGraphicFromLibrary(handle, revision, item.id, atUs));
  const removeFromLibrary = async (item: GraphicLibraryItem) => {
    if (!window.confirm(`Remove "${item.label}" from the library? Projects that use it keep their own copy.`)) return;
    try {
      await api.graphicsLibraryRemove(item.id);
      libraryChanged();
    } catch (err) {
      setError(String(err));
    }
  };

  const select = (graphic: GraphicListed) => {
    const first = graphic.clips[0];
    if (!first || !handle) return;
    setSelectedClipIds([first[0]]);
    void api
      .playbackSeek(handle, first[1])
      .then(useProjectStore.getState().applyPlaybackStatus)
      .catch(() => undefined);
  };

  /** Plays it from just before it starts, in the preview. */
  const play = (graphic: GraphicListed) => {
    const first = graphic.clips[0];
    if (!first) return;
    setSelectedClipIds([first[0]]);
    seekPlayback(Math.max(0, first[1] - 500_000));
    if (!useProjectStore.getState().isPlaying) window.setTimeout(() => togglePlayback(), 60);
  };

  const ready = runtime?.installed ?? false;
  const shown = templates.filter((t) => t.name !== "blank");

  return (
    <div className="flex-1 min-h-0 overflow-y-auto p-2 space-y-3">
      {error && (
        <Notice tone="danger" className="rounded-control border" onDismiss={() => setError(undefined)}>
          {error}
        </Notice>
      )}
      {runtime && !ready && (
        <div className="m-1 rounded-panel border border-studio-700 bg-studio-850 p-3 space-y-2">
          <div className="flex items-center gap-2 text-label font-medium text-studio-100">
            <Sparkles className="w-4 h-4 text-accent-fg" />
            {runtime.outdated ? "Update the graphics renderer" : "Install the graphics renderer"}
          </div>
          <p className="text-meta text-studio-400 leading-relaxed">
            Graphics are HyperFrames compositions the app renders with its own copy of Node and Chrome: a{" "}
            {runtime.downloadMb} MB download (about 500 MB installed) into the app's data folder. Nothing else on
            your computer changes. You can add graphics now; they render once it is installed.
          </p>
          {install ? (
            <div className="space-y-1">
              <div className="flex items-center gap-2 text-meta text-studio-300">
                <Loader2 className="w-3.5 h-3.5 animate-spin" />
                <span className="flex-1 truncate">
                  {install.step}
                  {install.bytesTotal
                    ? ` · ${Math.round(install.bytesDone / MB)} of ${Math.round(install.bytesTotal / MB)} MB`
                    : ""}
                </span>
                <IconButton icon={X} size="sm" label="Cancel the install" onClick={() => void api.graphicsRuntimeCancel()} />
              </div>
              <div className="h-1 rounded-full bg-studio-800 overflow-hidden">
                <div
                  className="h-full bg-accent-hover transition-[width]"
                  style={{
                    width: install.bytesTotal ? `${(100 * install.bytesDone) / install.bytesTotal}%` : "100%",
                    opacity: install.bytesTotal ? 1 : 0.4,
                  }}
                />
              </div>
            </div>
          ) : (
            <Button variant="primary" size="sm" icon={Download} onClick={() => void installRuntime()}>
              {runtime.outdated ? "Update" : "Install"} ({runtime.downloadMb} MB)
            </Button>
          )}
        </div>
      )}

      <section className="space-y-1">
        <h3 className="px-1 text-meta font-semibold uppercase tracking-wide text-studio-500">Templates</h3>
        <p className="px-1 text-meta text-studio-500 leading-relaxed">
          Full screen over the video, on top of everything. + adds one at the playhead on a Graphics track; edit its text and
          colours in the Inspector. Cuts never go through a graphic.
        </p>
        {shown.map((template) => (
          <div
            key={template.name}
            className="group flex items-center gap-2.5 rounded-control border border-transparent px-2 py-2 hover:border-studio-700 hover:bg-studio-850"
          >
            <span className="h-9 w-9 shrink-0 inline-flex items-center justify-center rounded-control bg-accent/15 text-accent-fg">
              <Sparkles className="w-4 h-4" />
            </span>
            <div className="min-w-0 flex-1">
              <div className="truncate text-label font-medium text-studio-100">{template.label}</div>
              <div className="text-meta text-studio-500 line-clamp-2" title={template.description}>
                {template.description} · {formatSeconds(template.lengthUs)}
              </div>
            </div>
            <IconButton
              icon={Plus}
              size="sm"
              label={`Add "${template.label}" at the playhead`}
              disabled={!openedProject || busy}
              onClick={() => void add(template)}
            />
          </div>
        ))}
      </section>

      <section className="space-y-1">
        <h3 className="px-1 text-meta font-semibold uppercase tracking-wide text-studio-500">Your library</h3>
        {library.length === 0 ? (
          <p className="px-1 text-meta text-studio-500 leading-relaxed">
            Graphics you save from a project (Inspector, Save to library) show here, to use in any project.
          </p>
        ) : (
          library.map((item) => (
            <LibraryRow
              key={`${item.id}-${item.updatedAt}`}
              item={item}
              canAdd={!!openedProject && !busy}
              onUse={() => void use(item)}
              onRemove={() => void removeFromLibrary(item)}
            />
          ))
        )}
      </section>

      {graphics.length > 0 && (
        <section className="space-y-1">
          <h3 className="px-1 text-meta font-semibold uppercase tracking-wide text-studio-500">In this project</h3>
          {graphics.map((graphic) => (
            <ProjectGraphicRow
              key={graphic.assetId}
              graphic={graphic}
              handle={handle}
              ready={ready}
              templateLabel={
                graphic.template
                  ? templates.find((t) => t.name === graphic.template)?.label ?? graphic.template
                  : "Custom"
              }
              onSelect={() => select(graphic)}
              onPlay={() => play(graphic)}
            />
          ))}
        </section>
      )}

      <p className="px-1 text-meta text-studio-500 leading-relaxed">
        How every graphic looks (colours, fonts, sizes, corners, motion) is the project&apos;s style: the
        Inspector&apos;s Style page.
      </p>

      {ready && runtime && (
        <div className="flex items-center gap-2 px-1 pt-2 text-meta text-studio-500 border-t border-studio-800">
          <span className="flex-1 truncate" title={runtime.dir}>
            Renderer: HyperFrames {runtime.hyperframes}
          </span>
          <IconButton
            icon={RotateCw}
            size="sm"
            label="Look at the graphics' files again and retry failed renders"
            onClick={() => void api.graphicsRenderAgain()}
          />
          <IconButton
            icon={Trash2}
            size="sm"
            label="Remove the renderer (graphics already rendered keep playing)"
            onClick={() => void removeRuntime()}
          />
        </div>
      )}
    </div>
  );
};

/// One library graphic: its label and whether it is a general template (it passed the gate) or
/// was saved as is, its version, when to use it, and the gate's contact sheet on demand.
const LibraryRow: React.FC<{
  item: GraphicLibraryItem;
  canAdd: boolean;
  onUse: () => void;
  onRemove: () => void;
}> = ({ item, canAdd, onUse, onRemove }) => {
  const [sheet, setSheet] = useState<string | null>();
  const [open, setOpen] = useState(false);
  const toggle = () => {
    setOpen((was) => !was);
    if (sheet === undefined)
      void api
        .graphicsLibrarySheet(item.id)
        .then(setSheet)
        .catch(() => setSheet(null));
  };
  const from = item.project ? ` · from ${item.project}` : "";
  return (
    <div className="group rounded-control border border-transparent px-2 py-2 hover:border-studio-700 hover:bg-studio-850">
      <div className="flex items-center gap-2.5">
        <span className="h-9 w-9 shrink-0 inline-flex items-center justify-center rounded-control bg-studio-800 text-studio-300">
          <Library className="w-4 h-4" />
        </span>
        <div className="min-w-0 flex-1">
          <div className="flex items-center gap-1.5 min-w-0">
            <span className="truncate text-label font-medium text-studio-100">{item.label}</span>
            {item.general ? (
              <Badge tone="success" title="Passed the gate: takes any text, fits 16:9 and 9:16, follows its length">
                Template
              </Badge>
            ) : (
              <Badge
                tone="suggest"
                title={`Saved as is, not general${item.failed?.length ? `: ${item.failed.join(", ")}` : ""}`}
              >
                As is
              </Badge>
            )}
            {item.version > 1 && <span className="shrink-0 text-meta text-studio-500">v{item.version}</span>}
          </div>
          <div className="text-meta text-studio-500 truncate" title={`${item.useWhen}${from}`}>
            {item.useWhen || item.name} · {formatSeconds(item.lengthUs)}
          </div>
        </div>
        {item.sheet && (
          <IconButton
            icon={ImageIcon}
            size="sm"
            active={open}
            label={open ? "Hide its check" : "Show its check: 16:9 and 9:16, with long text"}
            onClick={toggle}
          />
        )}
        <IconButton
          icon={Trash2}
          size="sm"
          label={`Remove "${item.label}" from the library`}
          className="opacity-0 group-hover:opacity-100 focus:opacity-100"
          onClick={onRemove}
        />
        <IconButton
          icon={Plus}
          size="sm"
          label={`Add a copy of "${item.label}" at the playhead`}
          disabled={!canAdd}
          onClick={onUse}
        />
      </div>
      {open && sheet && (
        <img
          src={sheet}
          alt={`The check of "${item.label}": 16:9 with its text and with long text, then 9:16`}
          className="mt-2 w-full rounded border border-studio-800"
        />
      )}
    </div>
  );
};

/** Thumbnails already made, by what they show (a graphic's place, its values, its render). */
const thumbnails = new Map<string, string>();
/** Each edit makes a new one (a PNG of some tens of kB): only the latest are kept. */
const KEPT_THUMBNAILS = 64;
function keepThumbnail(key: string, url: string) {
  thumbnails.delete(key);
  thumbnails.set(key, url);
  for (const oldest of thumbnails.keys()) {
    if (thumbnails.size <= KEPT_THUMBNAILS) break;
    thumbnails.delete(oldest);
  }
}

function formatClock(us: number) {
  const total = us / 1e6;
  const minutes = Math.floor(total / 60);
  return `${minutes}:${(total - minutes * 60).toFixed(1).padStart(4, "0")}`;
}

/** The words a graphic shows: its text values, colours and switches left out. */
function graphicText(graphic: GraphicListed): string {
  const values = graphic.declarations
    .filter((d) => d.type === "string")
    .map((d) => graphic.variables[d.id] ?? d.default)
    .filter((v): v is string => typeof v === "string" && v.trim() !== "" && !/^#[0-9a-f]{3,8}$/i.test(v));
  return values.join(" · ");
}

/// A graphic in the project: what it looks like where it plays, its template, when and how long
/// it plays, the words on it, and its render. Click to select it and go there; play to watch it.
const ProjectGraphicRow: React.FC<{
  graphic: GraphicListed;
  handle?: string;
  ready: boolean;
  templateLabel: string;
  onSelect: () => void;
  onPlay: () => void;
}> = ({ graphic, handle, ready, templateLabel, onSelect, onPlay }) => {
  const first = graphic.clips[0];
  const settled = graphic.render.state !== "rendering" && graphic.render.state !== "queued";
  const key = first
    ? `${handle}:${graphic.assetId}:${first[1]}:${first[2]}:${JSON.stringify(graphic.variables)}:${graphic.style}:${graphic.render.state}`
    : "";
  const [thumbnail, setThumbnail] = useState<string | undefined>(() => thumbnails.get(key));
  useEffect(() => {
    if (!first || !handle || !settled || !ready) return;
    const known = thumbnails.get(key);
    if (known) {
      setThumbnail(known);
      return;
    }
    let stale = false;
    // Halfway in: past the fade in, before the fade out.
    void api
      .projectFrameThumbnail(handle, first[1] + Math.floor(first[2] / 2), 224)
      .then((url) => {
        keepThumbnail(key, url);
        if (!stale) setThumbnail(url);
      })
      .catch(() => undefined);
    return () => {
      stale = true;
    };
    // `first` is part of `key`.
  }, [key, handle, settled, ready]);
  const words = graphicText(graphic);
  const more = graphic.clips.length - 1;
  return (
    <div
      role="button"
      tabIndex={0}
      onClick={onSelect}
      onKeyDown={(e) => (e.key === "Enter" || e.key === " ") && onSelect()}
      className={cn(
        "w-full text-left flex items-start gap-2.5 rounded-control border border-transparent p-1.5 hover:border-studio-700 hover:bg-studio-850 cursor-pointer",
        !first && "opacity-60",
      )}
      title={first ? "Select it and go to it" : "Not on the timeline"}
    >
      <div className="relative w-24 shrink-0 aspect-video rounded overflow-hidden bg-studio-950 border border-studio-800">
        {thumbnail ? (
          <img src={thumbnail} alt="" className="w-full h-full object-cover" />
        ) : (
          <span className="absolute inset-0 inline-flex items-center justify-center text-studio-600">
            <Sparkles className="w-4 h-4" />
          </span>
        )}
      </div>
      <div className="min-w-0 flex-1 space-y-0.5">
        <div className="line-clamp-2 text-label font-medium text-studio-100" title={graphic.name}>
          {graphic.name}
        </div>
        <div className="flex items-center gap-1.5 text-meta text-studio-400 min-w-0">
          <Badge tone="neutral">{templateLabel}</Badge>
          <span className="truncate font-mono tabular-nums">
            {first ? `${formatClock(first[1])} · ${formatSeconds(first[2])}${more > 0 ? ` · +${more} more` : ""}` : "Not on the timeline"}
          </span>
          <RenderBadge graphic={graphic} ready={ready} />
        </div>
        {words && (
          <p className="text-meta text-studio-500 line-clamp-2" title={words}>
            {words}
          </p>
        )}
      </div>
      {first && (
        <span onClick={(e) => e.stopPropagation()}>
          <IconButton icon={Play} size="sm" label="Play it in the preview" onClick={onPlay} />
        </span>
      )}
    </div>
  );
};

export const RenderBadge: React.FC<{ graphic: { render: GraphicListed["render"] }; ready: boolean }> = ({
  graphic,
  ready,
}) => {
  const { state, progress, error } = graphic.render;
  if (state === "rendering") return <Badge tone="accent">Rendering {Math.round(progress * 100)}%</Badge>;
  if (state === "queued") return <Badge tone="neutral">Waiting</Badge>;
  if (state === "failed")
    return (
      <Badge tone="danger" title={error}>
        Failed
      </Badge>
    );
  if (!ready) return <Badge tone="suggest">Needs renderer</Badge>;
  return null;
};
