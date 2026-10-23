import { useEffect } from "react";
import { api } from "./ipc";
import type { CleanupSettings, OpenedProject } from "./types";
import { useJumpCutStore } from "../stores/jumpCutStore";
import { useNoiseStore } from "../stores/noiseStore";
import { useProjectStore } from "../stores/projectStore";

/** The panels' settings as the project keeps them (what its preset gives unless changed). */
function fromPanels(current: CleanupSettings): CleanupSettings {
  const { silenceConfig, gapConfig } = useJumpCutStore.getState();
  const { sensitivity } = useNoiseStore.getState();
  return {
    silence: {
      thresholdDb: silenceConfig.thresholdDb,
      minDurationMs: silenceConfig.minDurationMs,
      paddingMs: silenceConfig.paddingMs,
    },
    gaps: {
      minDurationMs: gapConfig.minDurationMs,
      paddingMs: gapConfig.paddingMs,
      refineWordEdges: gapConfig.refineWordEdges ?? current.gaps.refineWordEdges,
      edgeThresholdDb: gapConfig.edgeThresholdDb ?? current.gaps.edgeThresholdDb,
    },
    mouthNoiseSensitivity: sensitivity,
  };
}

/** Puts the project's cleanup settings into the Cleanup panels whenever they change (a new
 *  project, its preset, undo). */
export function useCleanupFromProject(project: OpenedProject | null) {
  const cleanup = project?.cleanup;
  const key = cleanup ? JSON.stringify(cleanup) : "";
  useEffect(() => {
    if (!cleanup) return;
    const jumpCuts = useJumpCutStore.getState();
    const now = fromPanels(cleanup);
    if (JSON.stringify(now) === key) return;
    jumpCuts.configureSilence({ ...jumpCuts.silenceConfig, ...cleanup.silence, autoLevel: undefined });
    jumpCuts.configureGaps({ ...jumpCuts.gapConfig, ...cleanup.gaps });
    useNoiseStore.getState().configure({ sensitivity: cleanup.mouthNoiseSensitivity });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key]);
}

let timer: ReturnType<typeof setTimeout> | undefined;

/** Saves the panels' settings as the project's, a moment after the last change (one undo
 *  step); they then show as changed from the preset. */
export function saveCleanupSoon() {
  if (timer) clearTimeout(timer);
  timer = setTimeout(() => {
    const project = useProjectStore.getState().openedProject;
    if (!project?.cleanup) return;
    const next = fromPanels(project.cleanup);
    if (JSON.stringify(next) === JSON.stringify(project.cleanup)) return;
    void api
      .projectCleanupSet(project.projectHandle, project.revision, next)
      .then((updated) => useProjectStore.getState().applyOpenedProject(updated))
      .catch(() => undefined);
  }, 600);
}
