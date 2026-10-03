import React, { useEffect, useMemo, useRef, useState } from "react";
import {
  ArrowDownToLine,
  ArrowUpToLine,
  Check,
  Download,
  FolderOpen,
  Loader2,
  Plus,
  Sparkles,
  Trash2,
  RotateCcw,
} from "lucide-react";
import { api, setEditTarget } from "../../lib/ipc";
import { GAP } from "../../lib/projectUtils";
import { releaseShortPlayback, takeShortPlayback } from "../../lib/playbackControl";
import { useEngineFrames } from "../canvas/NativePreviewHost";
import { PreviewQualityControls } from "../layout/StagePanel";
import {
  DockviewDefaultTab,
  DockviewReact,
  type DockviewApi,
  type DockviewReadyEvent,
  type DockviewTheme,
  type IDockviewPanelHeaderProps,
  type IDockviewPanelProps,
} from "dockview-react";
import "dockview-react/dist/styles/dockview.css";
import "../layout/dockTheme.css";
import { listenForCaptionChanges, listenForProjects } from "../../lib/windowSync";
import { useProjectStore } from "../../stores/projectStore";
import type { ExportStatus, OpenedProject, Short, ShortLayout } from "../../lib/types";
import { BACKGROUND_PRESETS, presetBackgroundCss } from "../../lib/types";
import { transcribableSounds } from "../../lib/trackUtils";
import { TimelineStudio } from "../timeline/TimelineStudio";
import { Badge, Button, Notice, Switch } from "../ui";

const DEFAULT_LAYOUT: ShortLayout = {
  cameraPosition: "top",
  cameraPct: 35,
  screenZoom: 1,
  followZooms: true,
  screenPanX: 0,
  screenPanY: 0,
  captions: true,
  captionSpot: "seam",
  captionMaxWords: 0,
  captionLines: 2,
  captionSizePct: 0,
  backgroundType: "project",
  backgroundColorStart: "#000000",
  backgroundColorEnd: "#1e1b4b",
  backgroundPreset: "aurora",
};

/** Which clock edited time `editedUs` plays on (the recording, or an imported file) and
 *  where on it; `null` past the end. */
function clockAt(project: OpenedProject, editedUs: number): { media?: string; us: number } | null {
  let cursor = 0;
  for (const interval of project.retainedIntervals) {
    const length = interval.endUs - interval.startUs;
    if (editedUs < cursor + length) return { media: interval.media, us: interval.startUs + (editedUs - cursor) };
    cursor += length;
  }
  return null;
}
const NEW_SHORT_US = 30_000_000;
const MIN_SHORT_US = 3_000_000;
const MAX_SHORT_US = 180_000_000;

function formatTime(us: number): string {
  const total = Math.max(0, us) / 1_000_000;
  const minutes = Math.floor(total / 60);
  const seconds = total - minutes * 60;
  return `${minutes}:${seconds.toFixed(1).padStart(4, "0")}`;
}

function errorMessage(err: unknown): string {
  if (err instanceof Error && err.message) return err.message;
  if (typeof err === "string" && err.trim()) return err;
  return "Something went wrong.";
}

/** Each dock panel draws one part of the studio; the studio hands them over each render. */
type ShortsPanels = Record<ShortsPanelId, () => React.ReactNode>;
type ShortsPanelId = "list" | "preview" | "settings" | "timeline";
const ShortsPanelsContext = React.createContext<ShortsPanels | null>(null);

const SHORTS_PANELS: Record<ShortsPanelId, { title: string; minimumWidth: number; minimumHeight: number }> = {
  list: { title: "Shorts", minimumWidth: 200, minimumHeight: 120 },
  preview: { title: "Preview", minimumWidth: 260, minimumHeight: 260 },
  settings: { title: "Look", minimumWidth: 240, minimumHeight: 160 },
  timeline: { title: "Timeline", minimumWidth: 360, minimumHeight: 140 },
};
const SHORTS_LAYOUT_KEY = "aeroedits.shortsLayout.v1";

const ShortsPanel: React.FC<{ id: ShortsPanelId }> = ({ id }) => {
  const panels = React.useContext(ShortsPanelsContext);
  return <div className="h-full w-full min-h-0 min-w-0 overflow-hidden bg-studio-950">{panels?.[id]()}</div>;
};
const SHORTS_DOCK_COMPONENTS: Record<ShortsPanelId, React.FunctionComponent<IDockviewPanelProps>> = {
  list: () => <ShortsPanel id="list" />,
  preview: () => <ShortsPanel id="preview" />,
  settings: () => <ShortsPanel id="settings" />,
  timeline: () => <ShortsPanel id="timeline" />,
};
const SHORTS_DOCK_THEME: DockviewTheme = {
  name: "aeroedits",
  className: "dockview-theme-aero",
  colorScheme: "dark",
  gap: 6,
};
/** Panels stay open: there is no close button, so none can be lost. */
const ShortsTab: React.FC<IDockviewPanelHeaderProps> = (props) => <DockviewDefaultTab {...props} hideClose />;

function shortsPanel(id: ShortsPanelId) {
  const { title, minimumWidth, minimumHeight } = SHORTS_PANELS[id];
  return { id, component: id, title, minimumWidth, minimumHeight };
}

/** The default arrangement: list left, preview in the middle, look right, timeline below. */
function defaultShortsLayout(api: DockviewApi) {
  api.clear();
  api.addPanel(shortsPanel("preview"));
  api.addPanel({ ...shortsPanel("list"), position: { referencePanel: "preview", direction: "left" }, initialWidth: 260 });
  api.addPanel({ ...shortsPanel("settings"), position: { referencePanel: "preview", direction: "right" }, initialWidth: 320 });
  api.addPanel({ ...shortsPanel("timeline"), position: { direction: "below" }, initialHeight: 260 });
}

let shortsDockApi: DockviewApi | null = null;
function onShortsDockReady(event: DockviewReadyEvent) {
  shortsDockApi = event.api;
  try {
    const stored = window.localStorage.getItem(SHORTS_LAYOUT_KEY);
    if (stored) {
      event.api.fromJSON(JSON.parse(stored));
      const ids = new Set(event.api.panels.map((p) => p.id));
      if (!(Object.keys(SHORTS_PANELS) as ShortsPanelId[]).every((id) => ids.has(id))) defaultShortsLayout(event.api);
    } else {
      defaultShortsLayout(event.api);
    }
  } catch {
    defaultShortsLayout(event.api);
  }
  let timer = 0;
  event.api.onDidLayoutChange(() => {
    window.clearTimeout(timer);
    timer = window.setTimeout(() => {
      try {
        window.localStorage.setItem(SHORTS_LAYOUT_KEY, JSON.stringify(event.api.toJSON()));
      } catch {
        // Not remembering the layout is harmless.
      }
    }, 250);
  });
}

type ExportRow = { state: "waiting" | "running" | "completed" | "failed"; progress: number; message?: string; path?: string };

/// The Shorts Studio: its own window for picking vertical clips from the video, setting their
/// split-screen look (camera above or below the screen) and exporting them. Edits are saved in
/// the project and kept in step with the main editor.
export const ShortsStudio: React.FC = () => {
  const project = useProjectStore((s) => s.openedProject);
  const loadOpenedProject = useProjectStore((s) => s.loadOpenedProject);
  const applyOpenedProject = useProjectStore((s) => s.applyOpenedProject);
  const [loaded, setLoaded] = useState(false);
  const [selectedId, setSelectedId] = useState<string>();
  const [draft, setDraft] = useState<ShortLayout | null>(null);
  // The whole video (new shorts and trims are placed on it); the store holds the short's view.
  const [main, setMain] = useState<OpenedProject | null>(null);
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string>();
  const [checked, setChecked] = useState<Set<string>>(new Set());
  const [exports, setExports] = useState<Record<string, ExportRow>>({});
  const saveTimer = useRef<number>();

  // Load the project open in the editor, then follow its edits.
  useEffect(() => {
    document.title = "AeroEdits Shorts Studio";
    void api
      .projectCurrent()
      .then((current) => {
        if (current) {
          loadOpenedProject(current);
          setMain(current);
        }
      })
      .catch((err) => setError(errorMessage(err)))
      .finally(() => setLoaded(true));
    const stopCaptions = listenForCaptionChanges(() => useProjectStore.getState().bumpCaptions(true));
    const stopProjects = listenForProjects((next) => {
      const state = useProjectStore.getState();
      if (state.openedProject?.projectHandle !== next.projectHandle) state.loadOpenedProject(next);
      else state.applyOpenedProject(next, { remote: true });
    });
    return () => {
      stopCaptions();
      stopProjects();
    };
  }, [loadOpenedProject]);

  const shorts = project?.shorts ?? [];
  const selected = shorts.find((s) => s.id === selectedId) ?? shorts[0];
  // Older shorts lack the newer settings: fill them from the defaults. Kept stable between
  // renders, since the preview reloads whenever the layout changes.
  const baseLayout = draft ?? selected?.layout;
  const layout = useMemo(() => ({ ...DEFAULT_LAYOUT, ...(baseLayout ?? {}) }), [baseLayout]);
  // The store shows the selected short's own timeline once its view has arrived.
  const viewing = !!selected && project?.shortView === selected.id;
  const lengthUs = viewing ? project!.editedDurationUs : 0;
  const playable = viewing && lengthUs > 0;
  const ownEdit = !!selected?.edit;

  useEffect(() => {
    setDraft(null);
  }, [selected?.id]);

  // The selected short is what this window edits and what playback plays (with its sound).
  const selectedId_ = selected?.id;
  const handle_ = project?.projectHandle;
  useEffect(() => {
    if (!handle_) return;
    const shortId = selectedId_ ?? null;
    setEditTarget(shortId);
    useProjectStore.getState().setViewShort(shortId ?? undefined);
    if (shortId) {
      void api
        .projectShortView(handle_, shortId)
        .then((view) => useProjectStore.getState().applyOpenedProject(view, { remote: true }))
        .catch((err) => setError(errorMessage(err)));
    }
    // Switching shorts: the new one starts at 0, live in the preview while this window is active.
    useProjectStore.getState().setCurrentTimeUs(0);
    if (shortId && document.hasFocus()) {
      void api
        .playbackFocusShort(handle_, shortId, 0, false)
        .then(useProjectStore.getState().applyPlaybackStatus)
        .catch(() => undefined);
    }
  }, [selectedId_, handle_]);
  // Active, this window has the engine; the editor takes it back when it is clicked.
  useEffect(() => {
    const take = () => takeShortPlayback();
    window.addEventListener("focus", take);
    return () => window.removeEventListener("focus", take);
  }, []);
  // The short's own view arrives after selecting it: take the engine once it is there.
  const viewReady = project?.shortView;
  useEffect(() => {
    if (viewReady && document.hasFocus()) takeShortPlayback();
  }, [viewReady]);
  // Closing the window hands playback back to the video.
  useEffect(() => {
    const release = () => {
      setEditTarget(null);
      releaseShortPlayback();
    };
    window.addEventListener("beforeunload", release);
    return () => {
      window.removeEventListener("beforeunload", release);
      release();
    };
  }, []);
  // The whole video, refreshed with each revision.
  useEffect(() => {
    void api
      .projectCurrent()
      .then((current) => setMain(current ?? null))
      .catch(() => undefined);
  }, [project?.revision, project?.projectHandle]);

  // Speech first, the recording's or an imported file's.
  const speechTrack = transcribableSounds(project)[0];

  const run = async (label: string, work: (p: OpenedProject) => Promise<OpenedProject>) => {
    const current = useProjectStore.getState().openedProject;
    if (!current || busy) return;
    setBusy(label);
    setError(undefined);
    try {
      applyOpenedProject(await work(current));
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setBusy(null);
    }
  };

  const saveShorts = (next: Short[]) =>
    run("Saving", (p) => api.projectShortsSet(p.projectHandle, p.revision, next));

  const patchSelected = (patch: Partial<Short>) => {
    if (!selected) return;
    void saveShorts(shorts.map((s) => (s.id === selected.id ? { ...s, ...patch } : s)));
  };

  /** Layout changes show in the preview at once and are saved shortly after. */
  const changeLayout = (patch: Partial<ShortLayout>) => {
    if (!selected) return;
    const next = { ...layout, ...patch };
    setDraft(next);
    window.clearTimeout(saveTimer.current);
    // While it plays, the playing short picks the change up as soon as it is saved.
    saveTimer.current = window.setTimeout(
      () => patchSelected({ layout: next }),
      useProjectStore.getState().isPlaying ? 120 : 200,
    );
  };

  // The engine's frames while it has this short; the canvas keeps the last one otherwise.
  const playbackShortId = useProjectStore((s) => s.playbackShortId);
  const hasEngine = !!selected && playbackShortId === selected.id;
  useEngineFrames(canvasRef, playable, selected?.id, () => {
    const state = useProjectStore.getState();
    return !!state.viewShort && state.playbackShortId === state.viewShort;
  });


  /** A short's ends on the clock they play on: the recording's or one imported file's. */
  const anchors = (startEdited: number, endEdited: number) => {
    if (!main) return null;
    const start = clockAt(main, startEdited);
    const end = clockAt(main, endEdited - 1);
    if (!start || !end) return null;
    if (start.media === GAP || end.media === GAP) {
      setError("A short cannot start or end in a gap on V1.");
      return null;
    }
    if (start.media !== end.media) {
      setError("A short starts and ends on the same recording or file; trim it so both ends are on one.");
      return null;
    }
    return { media: start.media, sourceStartUs: start.us, sourceEndUs: end.us + 1 };
  };

  const newShort = () => {
    if (!main) return;
    // From the start of the video: the playhead here is in the selected short.
    const startEdited = 0;
    const ends = anchors(startEdited, Math.min(main.editedDurationUs, startEdited + NEW_SHORT_US));
    if (!ends) return;
    const id = `short-${Date.now().toString(36)}`;
    setSelectedId(id);
    void saveShorts([...shorts, { id, title: `Short ${shorts.length + 1}`, layout: DEFAULT_LAYOUT, ...ends }]);
  };

  /** Moves the short's start or end on the edited timeline. */
  const trim = (edge: "start" | "end", editedUs: number) => {
    if (!main || !selected || selected.editedStartUs === undefined || selected.editedEndUs === undefined) return;
    const start = edge === "start" ? editedUs : selected.editedStartUs!;
    const end = edge === "end" ? editedUs : selected.editedEndUs!;
    if (end - start < MIN_SHORT_US || end - start > MAX_SHORT_US) {
      setError("A short is between 3 seconds and 3 minutes long.");
      return;
    }
    const ends = anchors(start, end);
    if (!ends) return;
    setError(undefined);
    patchSelected(ends);
  };

  const exportShorts = async (ids: string[]) => {
    const current = useProjectStore.getState().openedProject;
    if (!current || ids.length === 0) return;
    setExports(Object.fromEntries(ids.map((id) => [id, { state: "waiting", progress: 0 } as ExportRow])));
    for (const id of ids) {
      const update = (row: ExportRow) => setExports((all) => ({ ...all, [id]: row }));
      try {
        let status: ExportStatus = await api.projectShortExport(current.projectHandle, id, {
          videoCodec: "h264",
          audioCodec: "aac",
          // A standard size: the backend turns it into 1080x1920 for the 9:16 canvas.
          width: 1920,
          height: 1080,
          fps: 30,
          quality: "high",
          bitrateKbps: null,
        });
        while (status.state === "queued" || status.state === "running") {
          update({
            state: "running",
            progress: status.progressDenominator > 0 ? status.progressNumerator / status.progressDenominator : 0,
          });
          await new Promise((resolve) => window.setTimeout(resolve, 500));
          status = await api.exportStatus(status.jobId);
        }
        update(
          status.state === "completed"
            ? { state: "completed", progress: 1, path: status.outputPath ?? undefined }
            : { state: "failed", progress: 0, message: status.failure?.message ?? status.state },
        );
      } catch (err) {
        update({ state: "failed", progress: 0, message: errorMessage(err) });
      }
    }
  };

  if (!loaded) {
    return <div className="h-screen bg-studio-950 flex items-center justify-center text-studio-400 text-sm">Loading…</div>;
  }
  if (!project) {
    return (
      <div className="h-screen bg-studio-950 flex items-center justify-center text-studio-400 text-sm">
        Open a project in the editor first.
      </div>
    );
  }

  const segmented = (active: boolean) =>
    `flex-1 h-7 inline-flex items-center justify-center gap-1 rounded-[5px] text-label font-medium transition-colors ${
      active
        ? "bg-accent/20 text-accent-fg shadow-[inset_0_0_0_1px_rgb(var(--accent-hover)/0.55)]"
        : "text-studio-400 hover:text-studio-100 hover:bg-studio-800"
    }`;
  const toExport = checked.size > 0 ? shorts.filter((s) => checked.has(s.id)) : shorts;

  // The studio's parts, drawn inside dockable panels (rearrange and resize like the editor's).
  const panels: ShortsPanels = {
    list: () => (
        <aside className="h-full overflow-y-auto p-2 space-y-1.5 text-label" aria-label="Shorts">
          {shorts.length === 0 && (
            <p className="p-2 text-studio-500 leading-relaxed">
              No shorts yet. Transcribe the video in the editor, then press Find shorts with AI, or start one with New
              short and trim it.
            </p>
          )}
          {shorts.map((s) => {
            const ok = s.editedStartUs !== undefined && s.editedEndUs !== undefined;
            const row = exports[s.id];
            return (
              <div
                key={s.id}
                onClick={() => setSelectedId(s.id)}
                className={`rounded-control border px-3 py-2.5 cursor-pointer transition-colors ${
                  s.id === selected?.id
                    ? "border-accent-hover/70 bg-accent/10 shadow-[inset_2px_0_0_rgb(var(--accent-hover))]"
                    : "border-studio-800 bg-studio-850 hover:border-studio-600"
                }`}
              >
                <div className="flex items-center gap-2">
                  <input
                    type="checkbox"
                    aria-label={`Select ${s.title} for export`}
                    checked={checked.has(s.id)}
                    onClick={(e) => e.stopPropagation()}
                    onChange={(e) => {
                      const next = new Set(checked);
                      if (e.target.checked) next.add(s.id);
                      else next.delete(s.id);
                      setChecked(next);
                    }}
                    className="w-4 h-4"
                  />
                  <span className="flex-1 truncate font-medium text-studio-100">{s.title}</span>
                </div>
                <div className="mt-1 pl-6 text-meta tabular-nums text-studio-500">
                  {ok
                    ? `${formatTime(s.editedStartUs!)} · ${((s.editedEndUs! - s.editedStartUs!) / 1e6).toFixed(1)}s`
                    : "Part of it was cut; trim it again"}
                </div>
                {row && (
                  <div className="mt-1 pl-6 flex items-center gap-1 text-meta">
                    {row.state === "completed" ? (
                      <>
                        <Check className="w-3 h-3 text-audio-fg" />
                        <span className="text-audio-fg">Exported</span>
                        {row.path && (
                          <button
                            type="button"
                            onClick={(e) => {
                              e.stopPropagation();
                              void api.showInFinder(row.path!);
                            }}
                            className="ml-auto flex items-center gap-1 text-studio-300 hover:text-white"
                          >
                            <FolderOpen className="w-3 h-3" /> Show
                          </button>
                        )}
                      </>
                    ) : row.state === "failed" ? (
                      <span className="text-danger-fg truncate" title={row.message}>
                        Failed: {row.message}
                      </span>
                    ) : (
                      <span className="text-accent-fg">
                        {row.state === "waiting" ? "Waiting" : `Exporting ${Math.round(row.progress * 100)}%`}
                      </span>
                    )}
                  </div>
                )}
              </div>
            );
          })}
        </aside>
    ),
    preview: () => (
        <main className="h-full min-w-0 min-h-0 flex flex-col gap-2 p-2 bg-studio-900">
          <div className="flex items-center justify-between gap-3 px-1 shrink-0">
            <PreviewQualityControls />
            <span className="truncate text-label text-studio-400">
              {selected ? `${selected.title} · ${formatTime(lengthUs)}` : "No short selected"}
            </span>
            <Badge title="Shorts are vertical: 1080 × 1920">9:16</Badge>
          </div>
          <div
            className="relative flex-1 min-h-0 overflow-hidden rounded-control bg-studio-950 flex items-center justify-center p-2"
            onPointerDown={() => takeShortPlayback()}
          >
            {selected && playable ? (
              <>
                <canvas
                  ref={canvasRef}
                  aria-label="Short preview"
                  className="h-full max-w-full aspect-[9/16] object-contain rounded-lg bg-black"
                />
                {!hasEngine && (
                  <button
                    type="button"
                    onClick={() => takeShortPlayback()}
                    className="absolute bottom-3 left-1/2 -translate-x-1/2 h-control px-3 rounded-control bg-studio-850 border border-studio-700 text-label text-studio-100 hover:bg-studio-800 shadow-popover"
                    title="The editor has the preview. Click to show this short here (the editor keeps its place)."
                  >
                    Show this short
                  </button>
                )}
              </>
            ) : (
              <p className="text-studio-500">{selected ? "Part of this short was cut from the video; trim it again." : "Pick or create a short."}</p>
            )}
          </div>
        </main>
    ),
    settings: () => (
      <>
        {selected && (
          <aside className="h-full overflow-y-auto p-4 space-y-6 text-label" aria-label="Short settings">
            <label className="block space-y-1.5">
              <span className="text-label text-studio-400">Title (also the file name)</span>
              <input
                key={selected.id + selected.title}
                defaultValue={selected.title}
                maxLength={100}
                onBlur={(e) => {
                  const title = e.target.value.trim();
                  if (title && title !== selected.title) patchSelected({ title });
                }}
                onKeyDown={(e) => e.key === "Enter" && e.currentTarget.blur()}
                className="ui-field w-full"
              />
            </label>
            {selected.reason && <p className="text-meta text-studio-500 italic">AI: {selected.reason}</p>}

            {ownEdit && (
              <div className="space-y-2 rounded-control border border-suggest/40 bg-suggest/10 p-3">
                <p className="text-meta text-suggest-fg">
                  Edited on its own: cuts and clips on its timeline below change only this short. Trim it by dragging
                  its first or last clip's edge.
                </p>
                <button
                  type="button"
                  disabled={!!busy}
                  onClick={() =>
                    void run("Re-syncing", (p) => api.projectShortResync(p.projectHandle, p.revision, selected.id))
                  }
                  className="h-7 px-2.5 rounded-control border border-suggest/50 text-suggest-fg hover:bg-suggest/15 disabled:opacity-40"
                  title="Drop this short's own edits so it follows the video again"
                >
                  Follow the video again
                </button>
              </div>
            )}
            {!ownEdit && selected.editedStartUs !== undefined && (
              <div className="space-y-2">
                <span className="text-label font-semibold text-studio-100">Trim</span>
                {(["start", "end"] as const).map((edge) => {
                  const at = edge === "start" ? selected.editedStartUs! : selected.editedEndUs!;
                  return (
                    <div key={edge} className="flex items-center gap-2">
                      <span className="w-10 text-studio-400 capitalize">{edge}</span>
                      <button type="button" disabled={!!busy} onClick={() => trim(edge, Math.max(0, at - 1_000_000))} className="h-7 px-2.5 rounded-control border border-studio-700 bg-studio-800 hover:bg-studio-700 disabled:opacity-40">
                        −1s
                      </button>
                      <span className="flex-1 text-center font-mono tabular-nums text-studio-100">{formatTime(at)}</span>
                      <button type="button" disabled={!!busy} onClick={() => trim(edge, Math.min(main?.editedDurationUs ?? at, at + 1_000_000))} className="h-7 px-2.5 rounded-control border border-studio-700 bg-studio-800 hover:bg-studio-700 disabled:opacity-40">
                        +1s
                      </button>
                    </div>
                  );
                })}
              </div>
            )}

            <div className="space-y-2">
              <span className="text-label font-semibold text-studio-100">Camera</span>
              <div className="flex gap-0.5 bg-studio-900 border border-studio-700 rounded-control p-0.5">
                <button type="button" aria-pressed={layout.cameraPosition === "top"} onClick={() => changeLayout({ cameraPosition: "top" })} className={segmented(layout.cameraPosition === "top")}>
                  <ArrowUpToLine className="inline w-3 h-3 mr-1" />
                  On top
                </button>
                <button type="button" aria-pressed={layout.cameraPosition === "bottom"} onClick={() => changeLayout({ cameraPosition: "bottom" })} className={segmented(layout.cameraPosition === "bottom")}>
                  <ArrowDownToLine className="inline w-3 h-3 mr-1" />
                  Below
                </button>
              </div>
              <label className="block space-y-1">
                <div className="flex justify-between">
                  <span className="text-studio-400">Camera height</span>
                  <span className="font-mono text-meta tabular-nums text-studio-200">{Math.round(layout.cameraPct)}%</span>
                </div>
                <input type="range" aria-label="Camera height" min={20} max={70} step={1} value={layout.cameraPct} onChange={(e) => changeLayout({ cameraPct: Number(e.target.value) })} className="w-full" />
              </label>
            </div>

            <div className="space-y-2">
              <span className="text-label font-semibold text-studio-100">Screen</span>
              <label className="block space-y-1">
                <div className="flex justify-between">
                  <span className="text-studio-400">Zoom</span>
                  <button
                    type="button"
                    onClick={() => changeLayout({ screenZoom: 1 })}
                    className="font-mono text-meta tabular-nums text-studio-200 hover:text-white"
                    title="1×: the screen fills its part of the frame. Below 1× it zooms out and the background shows around it."
                  >
                    {layout.screenZoom.toFixed(1)}×{layout.screenZoom < 1 ? " (out)" : ""}
                  </button>
                </div>
                <input type="range" aria-label="Screen zoom" min={0.3} max={3} step={0.05} value={layout.screenZoom} onChange={(e) => changeLayout({ screenZoom: Number(e.target.value) })} onDoubleClick={() => changeLayout({ screenZoom: 1 })} className="w-full" />
              </label>
              {(
                [
                  ["screenPanX", "Left / right"],
                  ["screenPanY", "Up / down"],
                ] as const
              ).map(([key, label]) => (
                <label key={key} className="block space-y-1">
                  <div className="flex justify-between">
                    <span className="text-studio-400">{label}</span>
                    <button
                      type="button"
                      onClick={() => changeLayout({ [key]: 0 })}
                      className="font-mono text-meta tabular-nums text-studio-200 hover:text-white"
                      title="Back to the centre"
                    >
                      {layout[key] === 0 ? "centre" : `${layout[key] > 0 ? "+" : ""}${Math.round(layout[key] * 100)}%`}
                    </button>
                  </div>
                  <input
                    type="range"
                    aria-label={`Screen ${label}`}
                    min={-1}
                    max={1}
                    step={0.02}
                    value={layout[key]}
                    onChange={(e) => changeLayout({ [key]: Number(e.target.value) })}
                    onDoubleClick={() => changeLayout({ [key]: 0 })}
                    className="w-full"
                  />
                </label>
              ))}
              <Switch
                checked={layout.followZooms}
                onChange={(followZooms) => changeLayout({ followZooms })}
                label="Follow the video's zooms"
                title="The screen part follows the zooms of the video (clicks and manual zooms)"
              />
            </div>

            <div className="space-y-2">
              <span className="text-label font-semibold text-studio-100">Background</span>
              <div className="grid grid-cols-5 gap-0.5 bg-studio-900 border border-studio-700 rounded-control p-0.5">
                {(
                  [
                    ["project", "Video's"],
                    ["solid", "Colour"],
                    ["gradient", "Gradient"],
                    ["preset", "Built-in"],
                    ["wallpaper", "Image"],
                  ] as const
                ).map(([kind, label]) => (
                  <button
                    key={kind}
                    type="button"
                    aria-pressed={layout.backgroundType === kind}
                    onClick={() => changeLayout({ backgroundType: kind })}
                    disabled={kind === "wallpaper" && !project.layout?.wallpaperAsset}
                    title={kind === "wallpaper" && !project.layout?.wallpaperAsset ? "Choose an image in the editor's Background settings first" : undefined}
                    className={`${segmented(layout.backgroundType === kind)} !text-meta disabled:opacity-40`}
                  >
                    {label}
                  </button>
                ))}
              </div>
              {(layout.backgroundType === "solid" || layout.backgroundType === "gradient") && (
                <div className="flex gap-2">
                  <input type="color" aria-label="Background colour" value={layout.backgroundColorStart} onChange={(e) => changeLayout({ backgroundColorStart: e.target.value })} className="h-control flex-1 bg-studio-850 border border-studio-700 rounded-control cursor-pointer" />
                  {layout.backgroundType === "gradient" && (
                    <input type="color" aria-label="Background end colour" value={layout.backgroundColorEnd} onChange={(e) => changeLayout({ backgroundColorEnd: e.target.value })} className="h-control flex-1 bg-studio-850 border border-studio-700 rounded-control cursor-pointer" />
                  )}
                </div>
              )}
              {layout.backgroundType === "preset" && (
                <div className="grid grid-cols-3 gap-1">
                  {BACKGROUND_PRESETS.map((preset) => (
                    <button
                      key={preset.key}
                      type="button"
                      aria-pressed={layout.backgroundPreset === preset.key}
                      onClick={() => changeLayout({ backgroundPreset: preset.key })}
                      className={`h-10 rounded-control border text-meta text-white/90 flex items-end p-1.5 ${
                        layout.backgroundPreset === preset.key ? "border-accent-fg shadow-[0_0_0_2px_rgb(var(--accent-hover)/0.7)]" : "border-studio-700"
                      }`}
                      style={{ background: presetBackgroundCss(preset.key) }}
                    >
                      {preset.label}
                    </button>
                  ))}
                </div>
              )}
            </div>

            <div className="space-y-2">
              <div className="flex items-center justify-between">
                <span className="text-label font-semibold text-studio-100">Captions</span>
                <Switch checked={layout.captions} onChange={(captions) => changeLayout({ captions })} title="Show captions in this short" />
              </div>
              {layout.captions && (
                <div className="flex gap-0.5 bg-studio-900 border border-studio-700 rounded-control p-0.5">
                  {(
                    [
                      ["seam", "Between"],
                      ["screen", "On screen"],
                      ["camera", "On camera"],
                    ] as const
                  ).map(([spot, label]) => (
                    <button key={spot} type="button" aria-pressed={layout.captionSpot === spot} onClick={() => changeLayout({ captionSpot: spot })} className={segmented(layout.captionSpot === spot)}>
                      {label}
                    </button>
                  ))}
                </div>
              )}
              {layout.captions && (
                <div className="space-y-2">
                  <label className="block space-y-1">
                    <div className="flex justify-between">
                      <span className="text-studio-400">Words at once</span>
                      <span className="font-mono text-meta tabular-nums text-studio-200">{layout.captionMaxWords || "editor's"}</span>
                    </div>
                    <input type="range" aria-label="Words per caption" min={0} max={12} step={1} value={layout.captionMaxWords} onChange={(e) => changeLayout({ captionMaxWords: Number(e.target.value) })} className="w-full" />
                  </label>
                  <div className="space-y-1">
                    <span className="text-studio-400">Lines</span>
                    <div className="flex gap-0.5 bg-studio-900 border border-studio-700 rounded-control p-0.5">
                      {[1, 2, 3].map((lines) => (
                        <button key={lines} type="button" aria-pressed={layout.captionLines === lines} onClick={() => changeLayout({ captionLines: lines })} className={segmented(layout.captionLines === lines)}>
                          {lines}
                        </button>
                      ))}
                    </div>
                  </div>
                  <label className="block space-y-1">
                    <div className="flex justify-between">
                      <span className="text-studio-400">Text size</span>
                      <span className="font-mono text-meta tabular-nums text-studio-200">{layout.captionSizePct ? `${layout.captionSizePct.toFixed(1)}%` : "editor's"}</span>
                    </div>
                    <input type="range" aria-label="Caption size" min={0} max={15} step={0.5} value={layout.captionSizePct} onChange={(e) => { const v = Number(e.target.value); changeLayout({ captionSizePct: v > 0 && v < 2 ? 2 : v }); }} className="w-full" />
                  </label>
                </div>
              )}
              <p className="text-meta text-studio-500">Colours and font come from the editor's Captions settings; edit caption text on the timeline below.</p>
            </div>

            <div className="flex flex-col gap-2 pt-2 border-t border-studio-800">
              <button
                type="button"
                disabled={!!busy || shorts.length < 2}
                onClick={() => void saveShorts(shorts.map((s) => ({ ...s, layout })))}
                className="h-control px-3 rounded-control bg-studio-800 border border-studio-700 hover:bg-studio-700 disabled:opacity-40"
              >
                Use this look for every short
              </button>
              <button
                type="button"
                disabled={!!busy}
                onClick={() => {
                  setSelectedId(undefined);
                  void saveShorts(shorts.filter((s) => s.id !== selected.id));
                }}
                className="h-control flex items-center justify-center gap-1.5 px-3 rounded-control text-danger-fg hover:bg-danger/15 disabled:opacity-40"
              >
                <Trash2 className="w-4 h-4" /> Remove this short
              </button>
            </div>
          </aside>
        )}
        {!selected && <p className="p-4 text-studio-500">Pick or create a short.</p>}
      </>
    ),
    timeline: () => (
      <>
      {selected && playable && (
        <section className="h-full flex flex-col min-h-0" aria-label="Short timeline">
          <div className="shrink-0 px-3 py-1.5 text-meta text-studio-400 border-b border-studio-800 bg-accent/5">
            This short's timeline. Cuts, splits, moves and clips here change only the short (the video stays as it
            is); caption text is shared with the video's transcript.
          </div>
          <div className="flex-1 min-h-0">
            <TimelineStudio />
          </div>
        </section>
      )}
        {!(selected && playable) && <p className="p-4 text-studio-500">Pick a short to edit its timeline.</p>}
      </>
    ),
  };


  return (
    <div className="h-screen bg-studio-950 text-studio-100 flex flex-col text-label select-none">
      <header className="h-header shrink-0 px-3 flex items-center gap-2 border-b border-studio-800 bg-studio-900">
        <img src="/aeroedits-icon.svg" alt="" className="w-7 h-7" draggable={false} />
        <div className="min-w-0 flex items-baseline gap-2 pr-2">
          <span className="font-display text-body font-semibold text-studio-100 shrink-0">Shorts Studio</span>
          <span className="text-label text-studio-500 truncate">{project.manifest.projectName}</span>
        </div>
        <div className="ml-auto flex items-center gap-2">
          <Button
            variant="ghost"
            icon={RotateCcw}
            onClick={() => {
              if (!shortsDockApi) return;
              defaultShortsLayout(shortsDockApi);
            }}
            title="Put the panels back where they started"
          >
            <span className="hidden lg:inline">Reset layout</span>
          </Button>
          <Button
            variant="secondary"
            icon={busy === "Finding shorts" ? Loader2 : Sparkles}
            className={busy === "Finding shorts" ? "[&>svg]:animate-spin" : undefined}
            disabled={!!busy || !speechTrack}
            onClick={() =>
              speechTrack &&
              void run("Finding shorts", (p) => api.projectShortsGenerate(p.projectHandle, speechTrack.id))
            }
            title="Ask the AI provider from Transcription and AI settings for moments that work on their own. Replaces the list (undoable in the editor)."
          >
            Find with AI
          </Button>
          <Button variant="secondary" icon={Plus} disabled={!!busy} onClick={newShort}>
            New short
          </Button>
          <Button
            variant="primary"
            icon={Download}
            disabled={toExport.length === 0 || Object.values(exports).some((e) => e.state === "running" || e.state === "waiting")}
            onClick={() => void exportShorts(toExport.map((s) => s.id))}
          >
            Export {checked.size > 0 ? `${checked.size} selected` : `all ${shorts.length}`}
          </Button>
        </div>
      </header>

      {error && (
        <Notice tone="danger" onDismiss={() => setError(undefined)}>
          {error}
        </Notice>
      )}

      <div className="flex-1 min-h-0 p-1.5">
        <ShortsPanelsContext.Provider value={panels}>
          <DockviewReact
            className="h-full w-full"
            theme={SHORTS_DOCK_THEME}
            components={SHORTS_DOCK_COMPONENTS}
            defaultTabComponent={ShortsTab}
            disableFloatingGroups
            onReady={onShortsDockReady}
          />
        </ShortsPanelsContext.Provider>
      </div>
    </div>
  );
};
