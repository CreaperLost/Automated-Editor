import { useCallback, useEffect, useRef } from "react";
import { useProjectStore } from "../stores/projectStore";
import type { OpenedProject } from "../lib/types";

/**
 * A project save made shortly after the last change, so dragging a slider saves once. The save
 * belongs to the project and timeline (the video, or a short) it was asked on: one still waiting
 * when the window moves to another timeline goes out at once, while its own still shows, and one
 * still waiting when another project or timeline is shown by then is dropped, never saved to
 * that one. One still waiting when the component goes away (its inspector page closed) is sent
 * at once rather than lost.
 */
export function useDebouncedSave(delayMs: number) {
  const timer = useRef<number | undefined>(undefined);
  const pending = useRef<(() => void) | undefined | undefined>(undefined);
  /** The project and timeline the waiting save is for. */
  const owner = useRef<{ handle: string; shortView?: string } | undefined>(undefined);

  const flush = useCallback(() => {
    window.clearTimeout(timer.current);
    timer.current = undefined;
    owner.current = undefined;
    const run = pending.current;
    pending.current = undefined;
    run?.();
  }, []);

  useEffect(() => flush, [flush]);

  // The short asked for (`viewShort`) is set before its view arrives, so the save still finds
  // its own timeline showing.
  useEffect(
    () =>
      useProjectStore.subscribe((state) => {
        const mine = owner.current;
        if (!mine) return;
        if (
          state.viewShort !== mine.shortView ||
          state.openedProject?.projectHandle !== mine.handle ||
          state.openedProject.shortView !== mine.shortView
        ) {
          flush();
        }
      }),
    [flush],
  );

  /** Saves with `save` after `delayMs` without another change, given the project as it is then. */
  const schedule = useCallback(
    (save: (project: OpenedProject) => void) => {
      const asked = useProjectStore.getState().openedProject;
      if (!asked) return;
      const { projectHandle: handle, shortView } = asked;
      window.clearTimeout(timer.current);
      owner.current = { handle, shortView };
      pending.current = () => {
        const opened = useProjectStore.getState().openedProject;
        if (opened?.projectHandle === handle && opened.shortView === shortView) save(opened);
      };
      timer.current = window.setTimeout(flush, delayMs);
    },
    [delayMs, flush],
  );

  /** Whether a change is waiting to be saved. */
  const isPending = useCallback(() => timer.current !== undefined, []);

  return { schedule, isPending };
}
