import React, { useEffect } from "react";
import { NativePreviewHost } from "../canvas/NativePreviewHost";
import { useProjectStore } from "../../stores/projectStore";
import { useSettingsStore } from "../../stores/settingsStore";
import { api } from "../../lib/ipc";
import { releaseShortPlayback } from "../../lib/playbackControl";
import { PreviewBar } from "./PreviewBar";

export { PreviewQualityControls } from "./PreviewBar";

function aspectValue(ratio: string): number {
  switch (ratio) {
    case "9:16":
      return 9 / 16;
    case "4:3":
      return 4 / 3;
    case "1:1":
      return 1;
    default:
      return 16 / 9;
  }
}

/// The preview stage: the video, and under it the playback bar.
export const StagePanel: React.FC = () => {
  const project = useProjectStore((s) => s.openedProject);
  const playbackShortId = useProjectStore((s) => s.playbackShortId);
  const aspectRatio = useSettingsStore((s) => s.canvas.aspectRatio);

  // Clicking back into the editor gives it the video again, at its place.
  useEffect(() => {
    const takeBack = () => releaseShortPlayback();
    window.addEventListener("focus", takeBack);
    return () => window.removeEventListener("focus", takeBack);
  }, []);

  if (!project) return null;
  const playingShort = project.shorts?.find((short) => short.id === playbackShortId);

  return (
    <div className="h-full flex flex-col min-w-0 min-h-0 overflow-hidden bg-studio-900">
      {playbackShortId && (
        // The Shorts Studio is playing a short through this preview and its sound.
        <div className="m-2 mb-0 flex items-center gap-2 rounded-control border border-accent/50 bg-accent/10 px-2 py-1 text-meta text-accent-fg">
          <span className="flex-1 truncate">
            Playing the short {playingShort ? `"${playingShort.title}"` : ""} from the Shorts Studio
          </span>
          <button
            type="button"
            onClick={() =>
              void api
                .playbackFocusShort(project.projectHandle, null)
                .then(useProjectStore.getState().applyPlaybackStatus)
                .catch(() => undefined)
            }
            className="px-2 py-0.5 rounded border border-accent/60 hover:bg-accent/15"
          >
            Back to the video
          </button>
        </div>
      )}
      {project.durationUs === 0 && (
        <p className="m-2 mb-0 shrink-0 rounded-control border border-accent/40 bg-accent/10 px-3 py-2 text-label text-accent-fg">
          {project.assets.length > 0
            ? "The timeline is empty. Drag media from the Media panel onto it, or Undo."
            : "This project starts empty. Import video, images or audio in the Media panel, then drag them onto the timeline."}
        </p>
      )}

      <div className="flex-1 min-h-0 overflow-hidden m-2 rounded-control bg-studio-950 flex items-center justify-center p-2">
        <NativePreviewHost key={project.projectHandle} fitAspectRatio={aspectValue(aspectRatio)} />
      </div>

      <PreviewBar
        aspect={aspectRatio}
        aspectTitle="The canvas shape (change it in Inspector › Video)"
        notes={project.diagnostics}
      />
    </div>
  );
};
