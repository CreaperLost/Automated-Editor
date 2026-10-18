import { create } from "zustand";
import { api } from "../lib/ipc";
import { clipAt, soundSources } from "../lib/sequence";
import type { ExportStatus, OpenedProject, Short, ShortLayout } from "../lib/types";
import { useProjectStore } from "./projectStore";

export const DEFAULT_SHORT_LAYOUT: ShortLayout = {
  cameraPosition: "top",
  cameraPct: 35,
  cameraClips: {},
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
  scenes: [],
  sceneTransitionMs: 350,
};
/** How long a new short lasts. */
const NEW_SHORT_US = 30_000_000;

export type ExportRow = { state: "waiting" | "running" | "completed" | "failed"; progress: number; message?: string; path?: string };

interface ShortsState {
  /** The short picked in the list; the first one when none is (or it was removed). */
  selectedId?: string;
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
export function pickShort(shorts: Short[] | undefined, selectedId: string | undefined): Short | undefined {
  return shorts?.find((s) => s.id === selectedId) ?? shorts?.[0];
}

/** The picked short, followed. */
export function useSelectedShort(): Short | undefined {
  const shorts = useProjectStore((s) => s.openedProject?.shorts);
  const selectedId = useShortsStore((s) => s.selectedId);
  return pickShort(shorts, selectedId);
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
  set({ ...INITIAL, checked: new Set(), exports: {} });
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

function selected(): Short | undefined {
  return pickShort(useProjectStore.getState().openedProject?.shorts, useShortsStore.getState().selectedId);
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
 * Runs a scene change on the backend after any look change still waiting to be saved, so
 * that save can't put back the scenes from before.
 */
export function sceneOp(label: string, work: (p: OpenedProject) => Promise<OpenedProject>) {
  const { draft } = useShortsStore.getState();
  const short = selected();
  if (draft && short && saveTimer !== undefined) {
    window.clearTimeout(saveTimer);
    saveTimer = undefined;
    patchShort(short.id, { layout: draft });
  }
  set({ draft: null });
  saveQueue = saveQueue.then(() => runShorts(label, work));
}

/** Which asset's clock timeline time `editedUs` plays on (its top picture, else any clip) and
 *  where on it; `null` where nothing plays. */
function clockAt(project: OpenedProject, editedUs: number): { media: string; us: number } | null {
  const tracks = project.sequence.tracks.filter((t) => t.kind === "video").reverse()
    .concat(project.sequence.tracks.filter((t) => t.kind === "audio"));
  for (const track of tracks) {
    const clip = clipAt(track, editedUs);
    if (clip) return { media: clip.asset, us: clip.inUs + (editedUs - clip.startUs) };
  }
  return null;
}

/** A short of the video's first 30 seconds, shaped on its timeline afterwards. */
export function newShort() {
  const { main } = useShortsStore.getState();
  if (!main) return;
  const start = clockAt(main, 0);
  const end = clockAt(main, Math.min(main.durationUs, NEW_SHORT_US) - 1);
  if (!start || !end) {
    setShortsError("A short starts and ends on a clip; nothing plays there.");
    return;
  }
  if (start.media !== end.media) {
    setShortsError("A short starts and ends on the same recording or file; trim it so both ends are on one.");
    return;
  }
  const id = `short-${Date.now().toString(36)}`;
  selectShort(id);
  void saveShorts((list) => [
    ...list,
    { id, title: `Short ${list.length + 1}`, layout: DEFAULT_SHORT_LAYOUT, media: start.media, sourceStartUs: start.us, sourceEndUs: end.us + 1 },
  ]);
}

/** Asks the AI provider for moments that work on their own; replaces the list. */
export function findShorts() {
  const project = useProjectStore.getState().openedProject;
  // Speech on the timeline first, the recording's or an imported file's.
  const speech = soundSources(project).find((sound) => sound.placed) ?? soundSources(project)[0];
  if (!speech) return;
  void runShorts("Finding shorts", (p) => api.projectShortsGenerate(p.projectHandle, speech.key));
}

/**
 * A short from a take: the camera video (`camera`) and, when it was recorded apart, the
 * microphone (`voice`); media ids or stream keys. Its voice is transcribed next, for captions
 * and for the agent to find footage by.
 */
export async function newShortFromTake(camera: string | null, voice: string | null, title: string) {
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
    .then(() => {
      useProjectStore.getState().bumpCaptions();
      set({ notice: `${synced}Transcribed. Ask the agent to pick footage from the video for it.` });
    })
    .catch((err) => set({ notice: undefined, error: `Couldn't transcribe the take: ${errorMessage(err)}` }));
  return true;
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

/** Gives every short `layout`; each keeps its own scenes and graphic places (they are of its
 *  own stretch). */
export function applyLookToAll(layout: ShortLayout) {
  void saveShorts((list) =>
    list.map((s) => ({
      ...s,
      layout: { ...layout, scenes: s.layout?.scenes ?? [], graphicSlots: s.layout?.graphicSlots ?? {} },
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
