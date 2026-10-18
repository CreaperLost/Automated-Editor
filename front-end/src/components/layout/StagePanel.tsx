import React from "react";
import { NativePreviewHost } from "../canvas/NativePreviewHost";
import { useProjectStore } from "../../stores/projectStore";
import { useSettingsStore } from "../../stores/settingsStore";
import { api } from "../../lib/ipc";
import { useWorkspaceStore } from "../../stores/workspaceStore";
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

/// The preview stage: the video (in Shorts, the picked short, 9:16), and under it the playback
/// bar.
export const StagePanel: React.FC = () => {
  const project = useProjectStore((s) => s.openedProject);
  const playbackShortId = useProjectStore((s) => s.playbackShortId);
  const viewShort = useProjectStore((s) => s.viewShort);
  const inShorts = useWorkspaceStore((s) => s.workspace === "shorts");
  const aspectRatio = useSettingsStore((s) => s.canvas.aspectRatio);

  if (!project) return null;
  const playingShort = project.shorts?.find((short) => short.id === playbackShortId);
  const showsShort = !!project.shortView;
  // Shorts with none made yet: nothing to show but the way to make one.
  const noShorts = inShorts && (project.shorts ?? []).length === 0;

  return (
    <div className="h-full flex flex-col min-w-0 min-h-0 overflow-hidden bg-studio-900">
      {playbackShortId && playbackShortId !== viewShort && (
        // A short has the engine while the video is on show (an agent focused it).
        <div className="m-2 mb-0 flex items-center gap-2 rounded-control border border-accent/50 bg-accent/10 px-2 py-1 text-meta text-accent-fg">
          <span className="flex-1 truncate">
            Playing the short {playingShort ? `"${playingShort.title}"` : ""}
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
      {project.durationUs === 0 && !noShorts && (
        <p className="m-2 mb-0 shrink-0 rounded-control border border-accent/40 bg-accent/10 px-3 py-2 text-label text-accent-fg">
          {showsShort
            ? "Part of this short was cut from the video; adjust it on its timeline."
            : project.assets.length > 0
            ? "The timeline is empty. Drag media from the Media panel onto it, or Undo."
            : "This project starts empty. Import video, images or audio in the Media panel, then drag them onto the timeline."}
        </p>
      )}

      <div className="flex-1 min-h-0 overflow-hidden m-2 rounded-control bg-studio-950 flex items-center justify-center p-2">
        {noShorts ? (
          <p className="max-w-xs text-center text-label text-studio-500 leading-relaxed">
            No shorts yet. Make one in the Shorts panel: New short, or Find with AI once the video is transcribed.
          </p>
        ) : (
          <NativePreviewHost key={project.projectHandle} fitAspectRatio={showsShort ? 9 / 16 : aspectValue(aspectRatio)} />
        )}
      </div>

      <PreviewBar
        aspect={showsShort ? "9:16" : aspectRatio}
        aspectTitle={showsShort ? "Shorts are vertical: 1080 × 1920" : "The canvas shape (change it in Inspector › Video)"}
        notes={project.diagnostics}
      />
    </div>
  );
};
