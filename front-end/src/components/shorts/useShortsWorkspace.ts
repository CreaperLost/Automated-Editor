import { useEffect, useRef } from "react";
import { api, setEditTarget } from "../../lib/ipc";
import { focusShortPlayback, releaseShortPlayback } from "../../lib/playbackControl";
import { switchStage } from "../../lib/switchTiming";
import { useProjectStore } from "../../stores/projectStore";
import { pickShort, resetShorts, setShortsError, useShortsStore } from "../../stores/shortsStore";

/**
 * While Shorts is the workspace, the picked short is what the window shows and edits: the
 * project store holds its view (so the preview, timeline and inspector work on it), edits go
 * to it and playback plays it. Leaving Shorts puts the whole video back, at its old place.
 */
export function useShortsWorkspace(active: boolean) {
  const handle = useProjectStore((s) => s.openedProject?.projectHandle);
  const revision = useProjectStore((s) => s.openedProject?.revision);
  const shorts = useProjectStore((s) => s.openedProject?.shorts);
  const selectedId = useShortsStore((s) => s.selectedId);
  const target = active ? pickShort(shorts, selectedId)?.id : undefined;

  // Another project opened, or none: nothing of the old one's shorts carries over.
  const shownHandle = useRef(handle);
  useEffect(() => {
    if (shownHandle.current === handle) return;
    shownHandle.current = handle;
    resetShorts();
  }, [handle]);

  useEffect(() => {
    const state = useProjectStore.getState();
    if (!handle) {
      setEditTarget(null);
      return;
    }
    setEditTarget(target ?? null);
    state.setViewShort(target);
    if (target) {
      switchStage("selection rendered", target);
      void api
        .projectShortView(handle, target)
        .then((view) => {
          switchStage("view arrived", target);
          return useProjectStore.getState().applyOpenedProject(view, { remote: true });
        })
        .then(() => requestAnimationFrame(() => switchStage("view painted", target)))
        .catch(setShortsError);
      // A short starts at 0, live in the preview. The list's click usually sent this already.
      state.setCurrentTimeUs(0);
      if (state.playbackShortId !== target) {
        void focusShortPlayback(handle, target, 0).then(() => switchStage("engine focused", target));
      }
    } else if (state.openedProject?.shortView) {
      // The video first, so its playhead isn't held to the short's length.
      void api
        .projectCurrent()
        .then((project) => project && useProjectStore.getState().applyOpenedProject(project, { remote: true }))
        .then(releaseShortPlayback)
        .catch(() => undefined);
    } else {
      releaseShortPlayback();
    }
  }, [handle, target]);

  // The whole video, for placing new shorts, refreshed with each revision.
  useEffect(() => {
    if (!active || !handle) return;
    void api
      .projectCurrent()
      .then((current) => useShortsStore.setState({ main: current ?? null }))
      .catch(() => undefined);
  }, [active, handle, revision]);
}
