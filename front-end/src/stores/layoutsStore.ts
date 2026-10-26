import { create } from "zustand";
import { api } from "../lib/ipc";
import { BUILT_IN_LAYOUTS, type FavouriteLayout, type Layout, type LayoutChange, type OpenedProject, type PickSettings } from "../lib/types";
import { useProjectStore } from "./projectStore";
import { shortOp } from "./shortsStore";

interface LayoutsState {
  /** The layout section picked on the timeline's Layouts lane. */
  selectedId: string | null;
  /** The favourite layouts kept for the app. */
  favourites: FavouriteLayout[];
  busy: string | null;
  error?: string;
  /** What the last pick found. */
  notes: string[];
}

/** Layouts on the timeline shown (the video, or the short in view) and the favourites. */
export const useLayoutsStore = create<LayoutsState>(() => ({
  selectedId: null,
  favourites: [],
  busy: null,
  error: undefined,
  notes: [],
}));

const set = useLayoutsStore.setState;

function message(err: unknown): string {
  if (err instanceof Error && err.message) return err.message;
  return typeof err === "string" && err.trim() ? err : "Something went wrong.";
}

export const selectSection = (id: string | null) => set({ selectedId: id });
export const setLayoutsError = (error?: string) => set({ error });

/**
 * Runs `work` on the project as it is when it goes out. A short's changes queue behind its look
 * saves, so neither puts back what the other changed.
 */
function run(label: string, work: (project: OpenedProject) => Promise<OpenedProject>) {
  const project = useProjectStore.getState().openedProject;
  if (!project || useLayoutsStore.getState().busy) return;
  if (project.shortView) {
    shortOp(label, work);
    return;
  }
  set({ busy: label, error: undefined });
  void work(project)
    .then((next) => useProjectStore.getState().applyOpenedProject(next))
    .catch((err) => set({ error: message(err) }))
    .finally(() => set({ busy: null }));
}

/** One change to the shown timeline's layouts, as one undo step. */
export function changeLayouts(change: LayoutChange, label = "Changing layouts") {
  run(label, (p) => api.projectLayoutsChange(p.projectHandle, p.revision, p.shortView ?? null, change));
}

/** Picks sections from the recording with `settings` (else the timeline's own). */
export function pickLayouts(settings?: PickSettings) {
  run("Picking layouts", async (p) => {
    const picked = await api.projectLayoutsPick(p.projectHandle, p.revision, p.shortView ?? null, settings);
    set({
      notes: [
        picked.picked === 0
          ? "Nothing to pick: the base shows throughout."
          : `Picked ${picked.picked} section${picked.picked === 1 ? "" : "s"}.`,
        ...picked.diagnostics,
      ],
    });
    return picked.project;
  });
}

/** `layout` from `atUs` to the next layout change (or the end) of `project`'s timeline. */
function switchChange(project: OpenedProject, atUs: number, layout: Layout | null, name?: string): LayoutChange {
  const ends = (project.layouts?.sections ?? [])
    .flatMap((s) => s.editedRanges ?? [])
    .flatMap((r) => [r.startUs, r.endUs])
    .filter((t) => t > atUs + 1_000)
    .sort((a, b) => a - b);
  const endUs = ends[0] ?? project.durationUs;
  return { op: "add", startUs: atUs, endUs, layout, name: name ?? null };
}

/** From the playhead to the next layout change (or the end): what a layout key does. */
export function switchAt(atUs: number, layout: Layout | null, name?: string) {
  const project = useProjectStore.getState().openedProject;
  if (!project) return;
  changeLayouts(switchChange(project, atUs, layout, name), "Switching the layout");
}

/** What a layout key binds: `builtin:<name>` or `fav:<id>`. */
export function layoutKeyTarget(id: string): { name: string; layout: Layout } | null {
  if (id.startsWith("builtin:")) return BUILT_IN_LAYOUTS.find((b) => `builtin:${b.name}` === id) ?? null;
  const favourite = useLayoutsStore.getState().favourites.find((f) => `fav:${f.id}` === id);
  return favourite ? { name: favourite.name, layout: favourite.layout } : null;
}

/** Key presses go out one at a time, each on the project the one before left. */
let keyQueue: Promise<unknown> = Promise.resolve();
/** The presses of one stretch of playing, which undo as one step. */
let keyGroup: string | null = null;
useProjectStore.subscribe((state, previous) => {
  if (previous.isPlaying && !state.isPlaying) keyGroup = null;
});

/**
 * A layout key (LAY-2): the layout from the playhead to the next change. While playing it
 * shows at once (the engine swaps the look in) and every press until playback stops is one
 * undo step.
 */
export function pressLayoutKey(id: string) {
  const target = layoutKeyTarget(id);
  const { openedProject, currentTimeUs, isPlaying } = useProjectStore.getState();
  if (!target || !openedProject) return;
  const group = isPlaying ? (keyGroup ??= `keys-${Date.now()}`) : null;
  const work = (p: OpenedProject) =>
    api.projectLayoutsChange(
      p.projectHandle,
      p.revision,
      p.shortView ?? null,
      switchChange(p, currentTimeUs, target.layout, target.name),
      group,
    );
  if (openedProject.shortView) {
    shortOp("Switching the layout", work);
    return;
  }
  const handle = openedProject.projectHandle;
  keyQueue = keyQueue
    .then(async () => {
      const project = useProjectStore.getState().openedProject;
      if (!project || project.projectHandle !== handle || project.shortView) return;
      useProjectStore.getState().applyOpenedProject(await work(project));
    })
    .catch((err) => set({ error: message(err) }));
}

export async function loadFavourites() {
  try {
    set({ favourites: await api.layoutFavouritesList() });
  } catch (err) {
    set({ error: message(err) });
  }
}

export async function saveFavourite(name: string, layout: Layout) {
  try {
    set({ favourites: await api.layoutFavouriteSave(name, layout), error: undefined });
    return true;
  } catch (err) {
    set({ error: message(err) });
    return false;
  }
}

export async function removeFavourite(id: string) {
  try {
    set({ favourites: await api.layoutFavouriteRemove(id), error: undefined });
  } catch (err) {
    set({ error: message(err) });
  }
}
