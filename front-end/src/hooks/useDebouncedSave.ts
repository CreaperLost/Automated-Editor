import { useCallback, useEffect, useRef } from "react";
import { useProjectStore } from "../stores/projectStore";
import type { OpenedProject } from "../lib/types";

/**
 * A project save made shortly after the last change, so dragging a slider saves once. The save
 * belongs to the project it was asked in: one still waiting when another project opens is
 * dropped. One still waiting when the component goes away (its inspector page closed) is sent
 * at once rather than lost.
 */
export function useDebouncedSave(delayMs: number) {
  const timer = useRef<number>();
  const pending = useRef<(() => void) | undefined>();

  const flush = useCallback(() => {
    window.clearTimeout(timer.current);
    timer.current = undefined;
    const run = pending.current;
    pending.current = undefined;
    run?.();
  }, []);

  useEffect(() => flush, [flush]);

  /** Saves with `save` after `delayMs` without another change, given the project as it is then. */
  const schedule = useCallback(
    (save: (project: OpenedProject) => void) => {
      const handle = useProjectStore.getState().openedProject?.projectHandle;
      if (!handle) return;
      window.clearTimeout(timer.current);
      pending.current = () => {
        const opened = useProjectStore.getState().openedProject;
        if (opened?.projectHandle === handle) save(opened);
      };
      timer.current = window.setTimeout(flush, delayMs);
    },
    [delayMs, flush],
  );

  /** Whether a change is waiting to be saved. */
  const isPending = useCallback(() => timer.current !== undefined, []);

  return { schedule, isPending };
}
