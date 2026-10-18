import React from "react";
import { Film, Sparkles, Trash2 } from "lucide-react";
import { seekPlayback } from "../../lib/playbackControl";
import { assetById } from "../../lib/sequence";
import type { OpenedProject, Short } from "../../lib/types";
import { cleanTake, removeFootage, useShortsStore } from "../../stores/shortsStore";
import { InspectorSection } from "../inspector/InspectorSection";
import { Button } from "../ui";

function formatTime(us: number): string {
  const total = Math.max(0, us) / 1_000_000;
  const minutes = Math.floor(total / 60);
  const seconds = total - minutes * 60;
  return `${minutes}:${seconds.toFixed(1).padStart(4, "0")}`;
}

/** "Recording · Microphone" for a stream key, else the key. */
function streamName(project: OpenedProject, key: string | undefined): string {
  if (!key) return "none";
  const [assetId, streamId] = key.split(".");
  const asset = assetById(project, assetId);
  const stream = asset?.streams.find((s) => s.id === streamId);
  if (!asset) return key;
  return asset.kind === "recording" && stream ? `${asset.name} · ${stream.name}` : asset.name;
}

/// A take short's take and the footage under it: what plays in the screen part while the
/// take talks. The agent picks it (the take-shorts template); here it is listed and removed.
export const TakeFootage: React.FC<{ project: OpenedProject; short: Short; viewing: boolean }> = ({
  project,
  short,
  viewing,
}) => {
  const busy = useShortsStore((s) => s.busy);
  const take = short.take!;
  // While the short is on show, the store holds its own timeline.
  const footage = viewing
    ? project.sequence.tracks
        .filter((track) => track.kind === "video" && track.role === "screen")
        .flatMap((track) => track.clips)
        .sort((a, b) => a.startUs - b.startUs)
    : [];
  return (
    <InspectorSection id="short-take" title="Take and footage" icon={Film}>
      <dl className="grid grid-cols-[4.5rem_minmax(0,1fr)] gap-x-2 gap-y-1 text-meta">
        <dt className="text-studio-500">Camera</dt>
        <dd className="truncate text-studio-200">{streamName(project, take.camera)}</dd>
        <dt className="text-studio-500">Voice</dt>
        <dd className="truncate text-studio-200">{streamName(project, take.voice)}</dd>
      </dl>
      <ol className="list-decimal pl-4 space-y-0.5 text-meta text-studio-400">
        <li>Transcribe the take (Transcript, beside Shorts).</li>
        <li>Clean it: pauses and mouth noises, or by hand in Jump cuts and Mouth noises.</li>
        <li>Footage: the agent picks it from the video for each sentence.</li>
      </ol>
      <Button
        size="sm"
        variant="secondary"
        icon={Sparkles}
        className="w-full"
        disabled={!!busy || !viewing}
        onClick={() => void cleanTake(short.id, take.voice)}
        title="Cut the pauses between the take's words and mute its mouth noises (balanced settings; undoable). Needs the take's transcript."
      >
        {busy === "Cleaning the take" ? "Cleaning…" : "Clean the take"}
      </Button>
      {footage.length === 0 ? (
        <p className="text-meta text-studio-500 leading-relaxed">
          No footage yet: the camera fills the frame. Ask the agent to pick footage from the video for each sentence
          (the “take-shorts” template).
        </p>
      ) : (
        <ul className="space-y-1">
          {footage.map((clip) => (
            <li key={clip.id} className="flex items-center gap-1.5 text-meta">
              <button
                type="button"
                onClick={() => seekPlayback(clip.startUs)}
                className="font-mono tabular-nums text-studio-300 hover:text-white w-[7.5rem] text-left shrink-0"
                title="Go to the footage"
              >
                {formatTime(clip.startUs)}–{formatTime(clip.startUs + clip.durationUs)}
              </button>
              <span className="flex-1 truncate text-studio-200" title={`From ${formatTime(clip.inUs)} of it`}>
                {assetById(project, clip.asset)?.name ?? clip.asset} · {formatTime(clip.inUs)}
              </span>
              <button
                type="button"
                aria-label="Remove the footage"
                title="Remove (the camera fills the frame there)"
                disabled={!!busy}
                onClick={() => removeFootage(short.id, clip.startUs, clip.startUs + clip.durationUs)}
                className="h-6 w-6 shrink-0 inline-flex items-center justify-center rounded-control text-studio-500 hover:text-danger-fg hover:bg-danger/15 disabled:opacity-40"
              >
                <Trash2 className="w-3.5 h-3.5" />
              </button>
            </li>
          ))}
        </ul>
      )}
    </InspectorSection>
  );
};
