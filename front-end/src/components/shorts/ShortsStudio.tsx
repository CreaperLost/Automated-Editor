import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  ArrowDownToLine,
  ArrowUpToLine,
  Check,
  Download,
  FolderOpen,
  Loader2,
  Pause,
  Play,
  Plus,
  Smartphone,
  Sparkles,
  Trash2,
} from "lucide-react";
import { api } from "../../lib/ipc";
import { listenForProjects } from "../../lib/windowSync";
import { useProjectStore } from "../../stores/projectStore";
import type { ExportStatus, OpenedProject, Short, ShortLayout } from "../../lib/types";
import { transcribableSounds } from "../../lib/trackUtils";
import { TimelineStudio } from "../timeline/TimelineStudio";

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
/** Preview playback asks for frames this often; there is no audio in the preview. */
const PLAY_FRAME_MS = 100;

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
  const [offsetUs, setOffsetUs] = useState(0);
  const [playing, setPlaying] = useState(false);
  const [frameUrl, setFrameUrl] = useState<string>();
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string>();
  const [checked, setChecked] = useState<Set<string>>(new Set());
  const [exports, setExports] = useState<Record<string, ExportRow>>({});
  const saveTimer = useRef<number>();
  const frameRequest = useRef(0);

  // Load the project open in the editor, then follow its edits.
  useEffect(() => {
    document.title = "AeroEdits Shorts Studio";
    void api
      .projectCurrent()
      .then((current) => {
        if (current) loadOpenedProject(current);
      })
      .catch((err) => setError(errorMessage(err)))
      .finally(() => setLoaded(true));
    return listenForProjects((next) => {
      const state = useProjectStore.getState();
      if (state.openedProject?.projectHandle !== next.projectHandle) state.loadOpenedProject(next);
      else state.applyOpenedProject(next, { remote: true });
    });
  }, [loadOpenedProject]);

  const shorts = project?.shorts ?? [];
  const selected = shorts.find((s) => s.id === selectedId) ?? shorts[0];
  // Older shorts lack the newer settings: fill them from the defaults. Kept stable between
  // renders, since the preview reloads whenever the layout changes.
  const baseLayout = draft ?? selected?.layout;
  const layout = useMemo(() => ({ ...DEFAULT_LAYOUT, ...(baseLayout ?? {}) }), [baseLayout]);
  const playable = selected?.editedStartUs !== undefined && selected?.editedEndUs !== undefined;
  const lengthUs = playable ? selected!.editedEndUs! - selected!.editedStartUs! : 0;

  useEffect(() => {
    setDraft(null);
    setOffsetUs(0);
    setPlaying(false);
  }, [selected?.id]);

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
    saveTimer.current = window.setTimeout(() => patchSelected({ layout: next }), 400);
  };

  // Preview frames: the latest request wins.
  const showFrame = useCallback(
    async (at: number) => {
      const current = useProjectStore.getState().openedProject;
      if (!current || !selected || !playable) return;
      const request = ++frameRequest.current;
      try {
        const bytes = await api.shortPreviewFrame(current.projectHandle, selected.id, layout, at);
        if (request !== frameRequest.current) return;
        const url = URL.createObjectURL(new Blob([bytes], { type: "image/jpeg" }));
        setFrameUrl((old) => {
          if (old) URL.revokeObjectURL(old);
          return url;
        });
      } catch (err) {
        if (request === frameRequest.current) setError(errorMessage(err));
      }
    },
    [selected, playable, layout],
  );

  useEffect(() => {
    if (!playing) void showFrame(offsetUs);
  }, [showFrame, offsetUs, playing, project?.revision]);

  useEffect(() => {
    if (!playing) return;
    let at = offsetUs;
    let stopped = false;
    const started = performance.now();
    const tick = async () => {
      while (!stopped) {
        at = offsetUs + (performance.now() - started) * 1000;
        if (at >= lengthUs) {
          setPlaying(false);
          setOffsetUs(0);
          return;
        }
        await showFrame(at);
        await new Promise((resolve) => window.setTimeout(resolve, PLAY_FRAME_MS));
      }
    };
    void tick();
    return () => {
      stopped = true;
      setOffsetUs(Math.min(at, Math.max(0, lengthUs - 1)));
    };
    // Playback restarts only when started or stopped.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [playing]);

  /** A short's ends on the clock they play on: the recording's or one imported file's. */
  const anchors = (startEdited: number, endEdited: number) => {
    if (!project) return null;
    const start = clockAt(project, startEdited);
    const end = clockAt(project, endEdited - 1);
    if (!start || !end) return null;
    if (start.media !== end.media) {
      setError("A short starts and ends on the same recording or file; trim it so both ends are on one.");
      return null;
    }
    return { media: start.media, sourceStartUs: start.us, sourceEndUs: end.us + 1 };
  };

  const newShort = () => {
    if (!project) return;
    // From the playhead, or the start.
    const at = useProjectStore.getState().currentTimeUs;
    const startEdited = at + MIN_SHORT_US <= project.editedDurationUs ? at : 0;
    const ends = anchors(startEdited, Math.min(project.editedDurationUs, startEdited + NEW_SHORT_US));
    if (!ends) return;
    const id = `short-${Date.now().toString(36)}`;
    setSelectedId(id);
    void saveShorts([...shorts, { id, title: `Short ${shorts.length + 1}`, layout: DEFAULT_LAYOUT, ...ends }]);
  };

  /** Moves the short's start or end on the edited timeline. */
  const trim = (edge: "start" | "end", editedUs: number) => {
    if (!project || !selected || !playable) return;
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
    `flex-1 py-1.5 rounded-md text-xs ${active ? "bg-teal-600 text-white font-semibold" : "text-studio-300 hover:bg-studio-800"}`;
  const toExport = checked.size > 0 ? shorts.filter((s) => checked.has(s.id)) : shorts;

  return (
    <div className="h-screen bg-studio-950 text-studio-100 flex flex-col text-xs select-none">
      <header className="h-12 shrink-0 px-4 flex items-center gap-3 border-b border-studio-800 bg-studio-900">
        <Smartphone className="w-4 h-4 text-teal-400" />
        <span className="font-semibold text-sm">Shorts Studio</span>
        <span className="text-studio-500 truncate">{project.manifest.projectName}</span>
        <div className="ml-auto flex items-center gap-2">
          <button
            type="button"
            disabled={!!busy || !speechTrack}
            onClick={() =>
              speechTrack &&
              void run("Finding shorts", (p) => api.projectShortsGenerate(p.projectHandle, speechTrack.id))
            }
            title="Ask the AI provider from Transcription and AI settings for moments that work on their own. Replaces the list (undoable in the editor)."
            className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-violet-900/50 border border-violet-600/50 text-violet-100 hover:bg-violet-800/60 disabled:opacity-40"
          >
            {busy === "Finding shorts" ? <Loader2 className="w-3.5 h-3.5 animate-spin" /> : <Sparkles className="w-3.5 h-3.5" />}
            Find shorts with AI
          </button>
          <button
            type="button"
            disabled={!!busy}
            onClick={newShort}
            className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-studio-850 border border-studio-700 hover:bg-studio-800 disabled:opacity-40"
          >
            <Plus className="w-3.5 h-3.5" /> New short
          </button>
          <button
            type="button"
            disabled={toExport.length === 0 || Object.values(exports).some((e) => e.state === "running" || e.state === "waiting")}
            onClick={() => void exportShorts(toExport.map((s) => s.id))}
            className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-teal-600 hover:bg-teal-500 text-white font-semibold disabled:opacity-40"
          >
            <Download className="w-3.5 h-3.5" />
            Export {checked.size > 0 ? `${checked.size} selected` : `all ${shorts.length}`}
          </button>
        </div>
      </header>

      {error && (
        <p role="alert" className="px-4 py-1.5 bg-rose-950/50 border-b border-rose-900/60 text-rose-200">
          {error}
        </p>
      )}

      <div className="flex-1 min-h-0 flex">
        {/* Shorts list */}
        <aside className="w-72 shrink-0 border-r border-studio-800 overflow-y-auto p-2 space-y-1.5" aria-label="Shorts">
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
                className={`rounded-lg border p-2 cursor-pointer ${
                  s.id === selected?.id ? "border-teal-400/70 bg-teal-950/30" : "border-studio-800 bg-studio-900 hover:border-studio-600"
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
                    className="accent-teal-500"
                  />
                  <span className="flex-1 truncate font-medium">{s.title}</span>
                </div>
                <div className="mt-1 pl-5 font-mono text-[11px] text-studio-500">
                  {ok
                    ? `${formatTime(s.editedStartUs!)} · ${((s.editedEndUs! - s.editedStartUs!) / 1e6).toFixed(1)}s`
                    : "Part of it was cut; trim it again"}
                </div>
                {row && (
                  <div className="mt-1 pl-5 flex items-center gap-1 text-[11px]">
                    {row.state === "completed" ? (
                      <>
                        <Check className="w-3 h-3 text-emerald-300" />
                        <span className="text-emerald-300">Exported</span>
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
                      <span className="text-rose-300 truncate" title={row.message}>
                        Failed: {row.message}
                      </span>
                    ) : (
                      <span className="text-teal-300">
                        {row.state === "waiting" ? "Waiting" : `Exporting ${Math.round(row.progress * 100)}%`}
                      </span>
                    )}
                  </div>
                )}
              </div>
            );
          })}
        </aside>

        {/* Preview */}
        <main className="flex-1 min-w-0 flex flex-col items-center justify-center gap-3 p-4">
          {selected && playable ? (
            <>
              <div className="relative h-[min(70vh,720px)] aspect-[9/16] rounded-xl overflow-hidden border border-studio-700 bg-black">
                {frameUrl && <img src={frameUrl} alt="Short preview" className="w-full h-full object-contain" />}
              </div>
              <div className="w-[min(70vh*9/16,405px)] min-w-[280px] flex items-center gap-2">
                <button
                  type="button"
                  aria-label={playing ? "Pause" : "Play"}
                  onClick={() => setPlaying(!playing)}
                  className="p-1.5 rounded-md bg-studio-800 hover:bg-studio-700"
                >
                  {playing ? <Pause className="w-3.5 h-3.5" /> : <Play className="w-3.5 h-3.5" />}
                </button>
                <input
                  type="range"
                  aria-label="Position in the short"
                  min={0}
                  max={Math.max(0, lengthUs - 1)}
                  step={100_000}
                  value={Math.min(offsetUs, Math.max(0, lengthUs - 1))}
                  onChange={(e) => {
                    setPlaying(false);
                    setOffsetUs(Number(e.target.value));
                  }}
                  className="flex-1 accent-teal-500"
                />
                <span className="font-mono text-studio-400 w-20 text-right">
                  {formatTime(offsetUs)} / {formatTime(lengthUs)}
                </span>
              </div>
              <p className="text-[11px] text-studio-500">Preview without sound. The export has the audio.</p>
            </>
          ) : (
            <p className="text-studio-500">{selected ? "Part of this short was cut from the video; trim it again." : "Pick or create a short."}</p>
          )}
        </main>

        {/* Settings */}
        {selected && (
          <aside className="w-80 shrink-0 border-l border-studio-800 overflow-y-auto p-4 space-y-5" aria-label="Short settings">
            <label className="block space-y-1">
              <span className="text-studio-400">Title (also the file name)</span>
              <input
                key={selected.id + selected.title}
                defaultValue={selected.title}
                maxLength={100}
                onBlur={(e) => {
                  const title = e.target.value.trim();
                  if (title && title !== selected.title) patchSelected({ title });
                }}
                onKeyDown={(e) => e.key === "Enter" && e.currentTarget.blur()}
                className="w-full bg-studio-800 border border-studio-700 rounded px-2 py-1"
              />
            </label>
            {selected.reason && <p className="text-[11px] text-studio-500 italic">AI: {selected.reason}</p>}

            {playable && (
              <div className="space-y-2">
                <span className="font-semibold text-studio-300">Trim</span>
                {(["start", "end"] as const).map((edge) => {
                  const at = edge === "start" ? selected.editedStartUs! : selected.editedEndUs!;
                  return (
                    <div key={edge} className="flex items-center gap-2">
                      <span className="w-10 text-studio-400 capitalize">{edge}</span>
                      <button type="button" disabled={!!busy} onClick={() => trim(edge, Math.max(0, at - 1_000_000))} className="px-2 py-0.5 rounded bg-studio-800 hover:bg-studio-700 disabled:opacity-40">
                        −1s
                      </button>
                      <span className="flex-1 text-center font-mono">{formatTime(at)}</span>
                      <button type="button" disabled={!!busy} onClick={() => trim(edge, Math.min(project.editedDurationUs, at + 1_000_000))} className="px-2 py-0.5 rounded bg-studio-800 hover:bg-studio-700 disabled:opacity-40">
                        +1s
                      </button>
                    </div>
                  );
                })}
              </div>
            )}

            <div className="space-y-2">
              <span className="font-semibold text-studio-300">Camera</span>
              <div className="flex gap-1 bg-studio-900 border border-studio-800 rounded-lg p-1">
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
                  <span className="font-mono">{Math.round(layout.cameraPct)}%</span>
                </div>
                <input type="range" aria-label="Camera height" min={20} max={70} step={1} value={layout.cameraPct} onChange={(e) => changeLayout({ cameraPct: Number(e.target.value) })} className="w-full accent-teal-500" />
              </label>
            </div>

            <div className="space-y-2">
              <span className="font-semibold text-studio-300">Screen</span>
              <label className="block space-y-1">
                <div className="flex justify-between">
                  <span className="text-studio-400">Zoom</span>
                  <span className="font-mono">{layout.screenZoom.toFixed(1)}×</span>
                </div>
                <input type="range" aria-label="Screen zoom" min={1} max={3} step={0.1} value={layout.screenZoom} onChange={(e) => changeLayout({ screenZoom: Number(e.target.value) })} className="w-full accent-teal-500" />
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
                      className="font-mono text-studio-300 hover:text-white"
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
                    className="w-full accent-teal-500"
                  />
                </label>
              ))}
              <label className="flex items-center gap-2">
                <input type="checkbox" checked={layout.followZooms} onChange={(e) => changeLayout({ followZooms: e.target.checked })} className="accent-teal-500" />
                <span>Follow the video's zooms (clicks and manual zooms)</span>
              </label>
            </div>

            <div className="space-y-2">
              <label className="flex items-center gap-2 font-semibold text-studio-300">
                <input type="checkbox" checked={layout.captions} onChange={(e) => changeLayout({ captions: e.target.checked })} className="accent-teal-500" />
                Captions
              </label>
              {layout.captions && (
                <div className="flex gap-1 bg-studio-900 border border-studio-800 rounded-lg p-1">
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
                      <span className="font-mono">{layout.captionMaxWords || "editor's"}</span>
                    </div>
                    <input type="range" aria-label="Words per caption" min={0} max={12} step={1} value={layout.captionMaxWords} onChange={(e) => changeLayout({ captionMaxWords: Number(e.target.value) })} className="w-full accent-teal-500" />
                  </label>
                  <div className="space-y-1">
                    <span className="text-studio-400">Lines</span>
                    <div className="flex gap-1 bg-studio-900 border border-studio-800 rounded-lg p-1">
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
                      <span className="font-mono">{layout.captionSizePct ? `${layout.captionSizePct.toFixed(1)}%` : "editor's"}</span>
                    </div>
                    <input type="range" aria-label="Caption size" min={0} max={15} step={0.5} value={layout.captionSizePct} onChange={(e) => { const v = Number(e.target.value); changeLayout({ captionSizePct: v > 0 && v < 2 ? 2 : v }); }} className="w-full accent-teal-500" />
                  </label>
                </div>
              )}
              <p className="text-[11px] text-studio-500">Colours and font come from the editor's Captions settings; edit caption text on the timeline below.</p>
            </div>

            <div className="flex flex-col gap-2 pt-2 border-t border-studio-800">
              <button
                type="button"
                disabled={!!busy || shorts.length < 2}
                onClick={() => void saveShorts(shorts.map((s) => ({ ...s, layout })))}
                className="px-3 py-1.5 rounded-lg bg-studio-850 border border-studio-700 hover:bg-studio-800 disabled:opacity-40"
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
                className="flex items-center justify-center gap-1 px-3 py-1.5 rounded-lg text-rose-300 hover:bg-rose-950/40 disabled:opacity-40"
              >
                <Trash2 className="w-3.5 h-3.5" /> Remove this short
              </button>
            </div>
          </aside>
        )}
      </div>

      {/* The editor's timeline, opened on this short's stretch. Edits change the video itself,
          and the short follows them. */}
      {selected && playable && (
        <section className="h-64 shrink-0 border-t border-studio-800 flex flex-col min-h-0" aria-label="Short timeline">
          <div className="px-4 py-1 text-[11px] text-studio-500 border-b border-studio-800 bg-studio-900">
            Timeline of this short. Cuts, splits, captions and clips edited here change the main video too; the short
            keeps its start and end.
          </div>
          <div className="flex-1 min-h-0">
            <TimelineStudio scope={{ startUs: selected.editedStartUs!, endUs: selected.editedEndUs! }} />
          </div>
        </section>
      )}
    </div>
  );
};
