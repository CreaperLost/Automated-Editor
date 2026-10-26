import React from "react";
import { Sparkles } from "lucide-react";
import { assetById } from "../../lib/sequence";
import type { OpenedProject } from "../../lib/types";
import { useProjectStore } from "../../stores/projectStore";
import { cleanTake, useSelectedShort, useShortsStore } from "../../stores/shortsStore";
import { Button } from "../ui";

/** "Recording · Microphone" for a stream key, else the key. */
export function streamName(project: OpenedProject, key: string | undefined): string {
  if (!key) return "none";
  const [assetId, streamId] = key.split(".");
  const asset = assetById(project, assetId);
  const stream = asset?.streams.find((s) => s.id === streamId);
  if (!asset) return key;
  return asset.kind === "recording" && stream ? `${asset.name} · ${stream.name}` : asset.name;
}

/// Over a take short's transcript (UI-8): the take's camera and voice, the steps that make it,
/// and "Clean the take" (pauses cut, mouth noises muted), since the transcript is where a take
/// is worked on. Nothing for the video or a clip cut from it.
export const TakeCleanBar: React.FC = () => {
  const project = useProjectStore((s) => s.openedProject);
  const short = useSelectedShort();
  const busy = useShortsStore((s) => s.busy);
  const take = short?.take;
  if (!project || !short || !take || project.shortView !== short.id) return null;
  return (
    <div className="shrink-0 space-y-1.5 px-3 py-2 border-b border-studio-800 bg-studio-950/40">
      <div className="flex items-center gap-2">
        <p className="flex-1 min-w-0 truncate text-meta text-studio-400" title="The take this short is made from">
          Take: <span className="text-studio-200">{streamName(project, take.camera)}</span>, voice{" "}
          <span className="text-studio-200">{streamName(project, take.voice)}</span>
        </p>
        <Button
          size="sm"
          variant="secondary"
          icon={Sparkles}
          disabled={!!busy}
          onClick={() => void cleanTake(short.id, take.voice)}
          title="Cut the pauses between the take's words and mute its mouth noises (balanced settings; undoable). Needs the take's transcript."
        >
          {busy === "Cleaning the take" ? "Cleaning…" : "Clean the take"}
        </Button>
      </div>
      <p className="text-meta text-studio-500">
        Transcribe the take, clean it (or by hand: Jump cuts, Mouth noises), then put footage under it in Look, or ask the
        agent.
      </p>
    </div>
  );
};
