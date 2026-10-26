import { create } from "zustand";
import { api } from "../lib/ipc";
import { GAP_PRESETS } from "../lib/jumpCutConfig";
import { clipEnd, clipSourceAt, soundSources } from "../lib/sequence";
import type { ExportStatus, Footage, GraphicSlot, OpenedProject, Short, ShortGraphicFrom, ShortLayout } from "../lib/types";
import { useProjectStore } from "./projectStore";

export const DEFAULT_SHORT_LAYOUT: ShortLayout = {
  cameraClips: {},
  screenZoom: 1,
  followZooms: true,
  screenPanX: 0,
  screenPanY: 0,
  captions: true,
  captionSpot: "seam",
  captionMaxWords: 0,
  captionLines: 1,
  captionSizePct: 0,
};
/** How long a new short lasts. */
const NEW_SHORT_US = 30_000_000;

export type ExportRow = { state: "waiting" | "running" | "completed" | "failed"; progress: number; message?: string; path?: string };

interface ShortsState {
  /** The short picked in the list; the first one when none is (or it was removed). */
  selectedId?: string;
  /** The tab showing: Shorts (made from a take) or Clips (cut from the video). */
  kind: ShortKind;
  /** A look change shown at once, saved shortly after. */
  draft: ShortLayout | null;
  /** The whole video (new shorts are placed on it); the project store holds the short's view. */
  main: OpenedProject | null;
  busy: string | null;
  error?: string;
  /** A line about what just happened (a take lined up, its words being transcribed). */
  notice?: string;
  /** Shorts ticked for export; none ticked exports all. */
  checked: Set<string>;
  exports: Record<string, ExportRow>;
  /** The short whose title is being edited. */
  renaming: string | null;
}

const INITIAL: ShortsState = {
  selectedId: undefined,
  kind: "shorts",
  draft: null,
  main: null,
  busy: null,
  error: undefined,
  notice: undefined,
  checked: new Set(),
  exports: {},
  renaming: null,
};

/** The Shorts workspace: which short is picked, its unsaved look, exports and the like. */
export const useShortsStore = create<ShortsState>(() => ({ ...INITIAL }));

const set = useShortsStore.setState;

function errorMessage(err: unknown): string {
  if (err instanceof Error && err.message) return err.message;
  if (typeof err === "string" && err.trim()) return err;
  return "Something went wrong.";
}

export const setShortsError = (err: unknown) => set({ error: err === undefined ? undefined : errorMessage(err) });

/** The picked short of `shorts`. */
export function pickShort(shorts: Short[] | undefined, selectedId: string | undefined, kind?: ShortKind): Short | undefined {
  const list = kind ? shorts?.filter((s) => ofKind(s, kind)) : shorts;
  return list?.find((s) => s.id === selectedId) ?? list?.[0];
}

/** Two tabs over one model: Shorts are made from a take, Clips are cut from the video. */
export type ShortKind = "shorts" | "clips";

export function ofKind(short: Short, kind: ShortKind): boolean {
  return kind === "shorts" ? !!short.take : !short.take;
}

/** The tab showing (the dock sets it when Shorts or Clips opens). */
export function setShortKind(kind: ShortKind) {
  if (useShortsStore.getState().kind !== kind) set({ kind });
}

/** The picked short of the tab showing, followed. */
export function useSelectedShort(): Short | undefined {
  const shorts = useProjectStore((s) => s.openedProject?.shorts);
  const selectedId = useShortsStore((s) => s.selectedId);
  const kind = useShortsStore((s) => s.kind);
  return pickShort(shorts, selectedId, kind);
}

/** A short's look with the newer settings older shorts lack filled from the defaults. */
export function fullLayout(layout: Partial<ShortLayout> | undefined): ShortLayout {
  return { ...DEFAULT_SHORT_LAYOUT, ...(layout ?? {}) };
}

let saveTimer: number | undefined;
let saveQueue: Promise<void> = Promise.resolve();

/** Forgets everything of the project before (another one opened, or none). */
export function resetShorts() {
  window.clearTimeout(saveTimer);
  saveTimer = undefined;
  // The tab showing stays.
  set({ ...INITIAL, kind: useShortsStore.getState().kind, checked: new Set(), exports: {} });
}

export function selectShort(id: string | undefined) {
  if (id === useShortsStore.getState().selectedId) return;
  set({ selectedId: id, draft: null });
}

/** Runs a backend change of the project and applies what it returns. */
export async function runShorts(label: string, work: (p: OpenedProject) => Promise<OpenedProject>) {
  const current = useProjectStore.getState().openedProject;
  if (!current || useShortsStore.getState().busy) return;
  set({ busy: label, error: undefined });
  try {
    await useProjectStore.getState().applyOpenedProject(await work(current));
  } catch (err) {
    setShortsError(err);
  } finally {
    set({ busy: null });
  }
}

/**
 * Saves a change to the shorts. The change is applied to the list as it is when the save
 * goes out, not as it was when it was asked for: a layout save waits a moment, and must not
 * undo what happened meanwhile (another short renamed, this one edited on its own). Saves go
 * out one at a time, each on the revision the one before left. `null` saves nothing.
 */
export function saveShorts(change: (current: Short[]) => Short[] | null) {
  // A save asked for in one project never lands in the next one opened.
  const handle = useProjectStore.getState().openedProject?.projectHandle;
  saveQueue = saveQueue.then(async () => {
    const current = useProjectStore.getState().openedProject;
    if (!current || current.projectHandle !== handle) return;
    const next = change(current.shorts ?? []);
    if (!next) return;
    set({ busy: "Saving", error: undefined });
    try {
      // Applied (the short's view of it in too) before the next save reads the revision.
      await useProjectStore
        .getState()
        .applyOpenedProject(await api.projectShortsSet(current.projectHandle, current.revision, next));
    } catch (err) {
      setShortsError(err);
    } finally {
      set({ busy: null });
    }
  });
  return saveQueue;
}

/** Renames a short (the title is also its file name). */
export const renameShort = (id: string, title: string) =>
  void saveShorts((list) => list.map((s) => (s.id === id ? { ...s, title } : s)));

function patchShort(id: string, patch: Partial<Short>) {
  // A short removed in the meantime is not brought back.
  void saveShorts((list) => (list.some((s) => s.id === id) ? list.map((s) => (s.id === id ? { ...s, ...patch } : s)) : null));
}

/** Drops a short's own caption breaks, hidden words and emphasis: it shows the video's again. */
export const dropShortCaptionMarks = (id: string) => patchShort(id, { captionMarks: {} });

function selected(): Short | undefined {
  const { selectedId, kind } = useShortsStore.getState();
  return pickShort(useProjectStore.getState().openedProject?.shorts, selectedId, kind);
}

/** The picked short's look, with any change not saved yet. */
export function currentLayout(): ShortLayout {
  return fullLayout(useShortsStore.getState().draft ?? selected()?.layout);
}

/** Look changes show in the preview at once and are saved shortly after. */
export function changeLayout(patch: Partial<ShortLayout>) {
  const short = selected();
  if (!short) return;
  const next = { ...currentLayout(), ...patch };
  set({ draft: next });
  window.clearTimeout(saveTimer);
  // While it plays, the playing short picks the change up as soon as it is saved.
  saveTimer = window.setTimeout(
    () => {
      saveTimer = undefined;
      patchShort(short.id, { layout: next });
    },
    useProjectStore.getState().isPlaying ? 120 : 200,
  );
}

/**
 * Runs a change of the short on the backend (its layouts, say) after any look change still
 * waiting to be saved, one at a time with the saves.
 */
export function shortOp(label: string, work: (p: OpenedProject) => Promise<OpenedProject>) {
  const { draft } = useShortsStore.getState();
  const short = selected();
  if (draft && short && saveTimer !== undefined) {
    window.clearTimeout(saveTimer);
    saveTimer = undefined;
    patchShort(short.id, { layout: draft });
  }
  set({ draft: null });
  // Asked for the short on show: never lands on another short or project shown by the time
  // the saves before it are done.
  const asked = useProjectStore.getState().openedProject;
  saveQueue = saveQueue.then(() => {
    const now = useProjectStore.getState().openedProject;
    if (!now || now.projectHandle !== asked?.projectHandle || now.shortView !== asked.shortView) return;
    return runShorts(label, work);
  });
}

/**
 * Where a new short goes: from the start of the video for up to `limitUs`, on the clock of a
 * recording or file playing at the start. Both ends must be on one asset's clock, so it ends
 * where that asset last plays within the limit (the time between keeps every clip). Of the
 * assets playing at the start, the one reaching furthest wins; the top picture on a tie.
 */
function newShortRange(project: OpenedProject, limitUs: number): { media: string; startUs: number; endUs: number } | null {
  const tracks = project.sequence.tracks.filter((t) => t.kind === "video").reverse()
    .concat(project.sequence.tracks.filter((t) => t.kind === "audio"));
  const moving = (asset: string) => {
    const kind = project.assets.find((a) => a.id === asset)?.kind;
    return kind !== "image" && kind !== "graphic";
  };
  const clips = tracks.flatMap((t) => t.clips).filter((c) => moving(c.asset));
  if (clips.length === 0) return null;
  const firstUs = Math.min(...clips.map((c) => c.startUs));
  let best: { media: string; startUs: number; endUs: number; reachUs: number } | null = null;
  for (const opening of clips.filter((c) => c.startUs <= firstUs && firstUs < clipEnd(c))) {
    const startUs = clipSourceAt(opening, firstUs);
    let reachUs = 0;
    let endUs = 0;
    for (const clip of clips.filter((c) => c.asset === opening.asset && c.startUs < firstUs + limitUs)) {
      const until = Math.min(clipEnd(clip), firstUs + limitUs);
      const sourceEnd = clipSourceAt(clip, until);
      if (until > reachUs && sourceEnd > startUs) {
        reachUs = until;
        endUs = sourceEnd;
      }
    }
    if (reachUs > (best?.reachUs ?? firstUs)) best = { media: opening.asset, startUs, endUs, reachUs };
  }
  return best && { media: best.media, startUs: best.startUs, endUs: best.endUs };
}

/** A short of the video's first 30 seconds, shaped on its timeline afterwards. */
export function newShort() {
  const { main } = useShortsStore.getState();
  if (!main) return;
  const range = newShortRange(main, NEW_SHORT_US);
  if (!range) {
    setShortsError("Nothing plays on the video's timeline to make a short from.");
    return;
  }
  const id = `short-${Date.now().toString(36)}`;
  selectShort(id);
  void saveShorts((list) => [
    ...list,
    { id, title: `Clip ${list.filter((s) => !s.take).length + 1}`, layout: DEFAULT_SHORT_LAYOUT, media: range.media, sourceStartUs: range.startUs, sourceEndUs: range.endUs },
  ]);
}

/** Asks the AI provider for moments that work on their own and adds them as clips (shorts from a take and the clips made stay). */
export function findShorts() {
  const project = useProjectStore.getState().openedProject;
  // Speech on the timeline first, the recording's or an imported file's.
  const speech = soundSources(project).find((sound) => sound.placed) ?? soundSources(project)[0];
  if (!speech) return;
  void runShorts("Finding clips", (p) => api.projectShortsGenerate(p.projectHandle, speech.key));
}

/**
 * A short from a take: the camera video (`camera`) and, when it was recorded apart, the
 * microphone (`voice`); media ids or stream keys. Its voice is transcribed next, for captions
 * and for the agent to find footage by.
 */
export async function newShortFromTake(camera: string | null, voice: string | null, title: string, clean = true) {
  const current = useProjectStore.getState().openedProject;
  if (!current || useShortsStore.getState().busy) return false;
  set({ busy: "Making the short", error: undefined, notice: undefined });
  let made;
  try {
    made = await api.projectShortFromTake(current.projectHandle, current.revision, title, camera, voice);
    await useProjectStore.getState().applyOpenedProject(made.project);
    selectShort(made.shortId);
  } catch (err) {
    setShortsError(err);
    return false;
  } finally {
    set({ busy: null });
  }
  const { offsetUs, score } = made.sync;
  const synced =
    score == null
      ? ""
      : offsetUs !== 0
        ? `Lined the camera up with the microphone (${(Math.abs(offsetUs) / 1e6).toFixed(2)} s). `
        : "The camera's sound didn't clearly match the microphone, so both start together: check the lip sync. ";
  const voiceKey = made.project.shorts?.find((s) => s.id === made.shortId)?.take?.voice;
  if (!voiceKey) return true;
  const handle = made.project.projectHandle;
  const transcribed = await api.transcriptGet(handle, voiceKey).catch(() => null);
  if (transcribed) {
    set({ notice: `${synced}Ask the agent to pick footage from the video for it.` });
    return true;
  }
  set({ notice: `${synced}Transcribing the take for its captions…` });
  void api
    .transcriptRun(handle, voiceKey)
    .then(async () => {
      useProjectStore.getState().bumpCaptions();
      if (clean) {
        await cleanTake(made.shortId, voiceKey, synced);
      } else {
        set({ notice: `${synced}Transcribed. Ask the agent to pick footage from the video for it.` });
      }
    })
    .catch((err) => set({ notice: undefined, error: `Couldn't transcribe the take: ${errorMessage(err)}` }));
  return true;
}

/**
 * Cleans a take on its short's own timeline, as Cleanup does for the video: cuts the gaps
 * between its words (the tight jump cuts) and mutes the mouth noises clear of speech.
 * Each is one undo step. Needs the take's transcript.
 */
export async function cleanTake(shortId: string, voice: string, before = "") {
  const start = useProjectStore.getState().openedProject;
  if (!start || useShortsStore.getState().busy) return;
  const handle = start.projectHandle;
  set({ busy: "Cleaning the take", error: undefined, notice: `${before}Cleaning the take…` });
  try {
    // Tight: a short keeps no pause at its cuts (Balanced leaves one you hear at each).
    const gaps = await api.detectNonSpeechGaps(handle, "speech", GAP_PRESETS.tight.config, undefined, shortId);
    const ranges = gaps.suggestions.map(({ startUs, endUs }) => ({ startUs, endUs }));
    const cutUs = ranges.reduce((sum, r) => sum + (r.endUs - r.startUs), 0);
    let revision = useProjectStore.getState().openedProject?.revision ?? start.revision;
    if (ranges.length > 0) {
      const cut = await api.applyJumpCuts(handle, revision, ranges, gaps.transcriptDependencies, shortId);
      await useProjectStore.getState().applyOpenedProject(cut);
      revision = cut.revision;
    }
    const scan = await api.detectMouthNoises(handle, voice, 0.5, 20, shortId);
    const noises = scan.noises.filter((n) => !n.nearSpeech && !n.muted && n.kind !== null);
    if (noises.length > 0) {
      const muted = await api.projectNoiseMutes(handle, revision, voice, noises.flatMap((n) => n.parts), false, shortId);
      await useProjectStore.getState().applyOpenedProject(muted);
    }
    set({
      notice:
        `${before}Cleaned the take: cut ${ranges.length} pause${ranges.length === 1 ? "" : "s"} ` +
        `(${(cutUs / 1e6).toFixed(1)} s) and muted ${noises.length} mouth noise${noises.length === 1 ? "" : "s"}; ` +
        "Undo takes either back. Next: put footage, graphics or files under it (below, or ask the agent).",
    });
  } catch (err) {
    setShortsError(`Couldn't clean the take: ${errorMessage(err)}`);
    set({ notice: undefined });
  } finally {
    set({ busy: null });
  }
}

/** Puts footage (the video's, or an imported file's) under a take, one undo step. */
export function addFootage(shortId: string, items: Footage[]) {
  void runShorts("Adding footage", (p) =>
    api.projectShortFootage(p.projectHandle, p.revision, shortId, { op: "add", items }),
  );
}

/** Makes a graphic and places it under a take at `atUs`, showing as `mode` (so it renders once,
 *  for that); `lengthUs` null keeps its own. */
export function addShortGraphic(
  shortId: string,
  from: ShortGraphicFrom,
  atUs: number,
  lengthUs: number | null,
  mode: GraphicSlot = "full",
) {
  void runShorts("Adding a graphic", (p) =>
    api
      .projectShortGraphicAdd(p.projectHandle, p.revision, shortId, from, atUs, lengthUs, mode)
      .then((made) => made.project),
  );
}

/** Takes clips off the short on show (its graphics), leaving the gap. */
export function removeShortClips(clipIds: string[]) {
  void runShorts("Removing", (p) =>
    api.projectSequenceEdit(p.projectHandle, p.revision, { kind: "delete", clipIds, ripple: false }),
  );
}

/** Brings files into the project, to go under a take. */
export function importForShort(paths: string[]) {
  if (paths.length === 0) return;
  void runShorts("Importing", (p) => api.projectMediaImport(p.projectHandle, p.revision, paths));
}

/** Takes the footage off `[fromUs, toUs)` of a take short's timeline. */
export function removeFootage(shortId: string, fromUs: number, toUs: number) {
  void runShorts("Removing footage", (p) =>
    api.projectShortFootage(p.projectHandle, p.revision, shortId, { op: "remove", fromUs, toUs }),
  );
}

export function removeShort(id: string) {
  if (useShortsStore.getState().selectedId === id) selectShort(undefined);
  void saveShorts((list) => list.filter((s) => s.id !== id));
}

/** Gives every short `layout` and this short's base layout; each keeps its own sections, graphic
 *  places and hand framing (they are of its own stretch). */
export function applyLookToAll(layout: ShortLayout) {
  const base = selected()?.layouts?.base;
  void saveShorts((list) =>
    list.map((s) => ({
      ...s,
      layout: { ...layout, graphicSlots: s.layout?.graphicSlots ?? {}, cameraClips: s.layout?.cameraClips ?? {} },
      layouts: s.layouts && base ? { ...s.layouts, base } : s.layouts,
    })),
  );
}

export async function exportShorts(ids: string[]) {
  const current = useProjectStore.getState().openedProject;
  if (!current || ids.length === 0) return;
  set({ exports: Object.fromEntries(ids.map((id) => [id, { state: "waiting", progress: 0 } as ExportRow])) });
  for (const id of ids) {
    const update = (row: ExportRow) => set((s) => ({ exports: { ...s.exports, [id]: row } }));
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
}
