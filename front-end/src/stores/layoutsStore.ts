import { create } from "zustand";
import { api } from "../lib/ipc";
import type { FavouriteLayout, Layout, LayoutChange, OpenedProject, PickSettings } from "../lib/types";
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

/** From the playhead to the next layout change (or the end): what a layout key does. */
export function switchAt(atUs: number, layout: Layout | null, name?: string) {
  const project = useProjectStore.getState().openedProject;
  if (!project) return;
  const ends = (project.layouts?.sections ?? [])
    .flatMap((s) => s.editedRanges ?? [])
    .flatMap((r) => [r.startUs, r.endUs])
    .filter((t) => t > atUs + 1_000)
    .sort((a, b) => a - b);
  const endUs = ends[0] ?? project.durationUs;
  changeLayouts({ op: "add", startUs: atUs, endUs, layout, name: name ?? null }, "Switching the layout");
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
