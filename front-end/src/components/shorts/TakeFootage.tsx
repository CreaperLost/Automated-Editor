import React from "react";
import { Film, Shapes, Trash2 } from "lucide-react";
import { seekPlayback } from "../../lib/playbackControl";
import { assetById } from "../../lib/sequence";
import { saveTrackMix } from "../../lib/trackMix";
import { DEFAULT_AUDIO_SETTINGS, type OpenedProject, type Short } from "../../lib/types";
import {
  removeFootage,
  removeShortClips,
  setShortsError,
  useShortsStore,
} from "../../stores/shortsStore";
import { InspectorSection } from "../inspector/InspectorSection";
import { UnderTake } from "./UnderTake";
import { formatTime } from "./underTakeParts";

/// How loud the footage and B-roll sound plays under the take, and how far it is lowered while
/// the voice talks: both play on the short's one Footage sound track, so one setting does both.
const FootageSound: React.FC<{ project: OpenedProject }> = ({ project }) => {
  // While the short is on show the sequence is its own; the audio settings are the project's.
  const track = project.sequence.tracks.find((t) => t.kind === "audio" && t.role === "background");
  const mix = track ? project.audio?.tracks?.[track.id] : undefined;
  const saved = {
    volumeDb: mix?.volumeDb ?? 0,
    duckDb: mix?.duckDb ?? project.audio?.duckDb ?? DEFAULT_AUDIO_SETTINGS.duckDb,
  };
  const [draft, setDraft] = React.useState(saved);
  // Undo, or the agent, changes it too.
  React.useEffect(() => setDraft({ volumeDb: saved.volumeDb, duckDb: saved.duckDb }), [saved.volumeDb, saved.duckDb]);
  if (!track) return null;
  const commit = () => {
    if (draft.volumeDb === saved.volumeDb && draft.duckDb === saved.duckDb) return;
    void saveTrackMix({ [track.id]: draft }).catch((err) => setShortsError(String(err)));
  };
  const row = (label: string, title: string, key: "volumeDb" | "duckDb", min: number, max: number) => (
    <label className="flex items-center gap-2 text-label text-studio-300" title={title}>
      <span className="w-[6.5rem] shrink-0">{label}</span>
      <input
        type="range"
        aria-label={label}
        min={min}
        max={max}
        step={1}
        value={draft[key]}
        onChange={(e) => setDraft((was) => ({ ...was, [key]: Number(e.target.value) }))}
        onPointerUp={commit}
        onKeyUp={commit}
        onBlur={commit}
        className="flex-1 min-w-0 h-1.5 cursor-pointer"
      />
      <span className="w-14 shrink-0 whitespace-nowrap text-right font-mono text-meta tabular-nums text-studio-200">
        {key === "volumeDb" && draft[key] > 0 ? "+" : key === "duckDb" ? "−" : ""}
        {draft[key]} dB
      </span>
    </label>
  );
  return (
    <div className="space-y-1 rounded-control border border-studio-800 p-1.5">
      <p className="text-meta text-studio-500">Footage and B-roll sound</p>
      {row("Volume", "How loud the footage's and files' own sound plays (0 as recorded)", "volumeDb", -30, 12)}
      {row("Under the voice", "How far it is lowered while the take's voice talks", "duckDb", 3, 30)}
    </div>
  );
};

/// What plays under a take short while the take talks: footage from the video, graphics and
/// outside files. The agent places them (the "shorts" template), or the user here. The take
/// itself (cleaning it) is worked on in the Transcript panel.
export const TakeFootage: React.FC<{ project: OpenedProject; short: Short; viewing: boolean }> = ({
  project,
  short,
  viewing,
}) => {
  const busy = useShortsStore((s) => s.busy);
  const take = short.take!;
  // While the short is on show, the store holds its own timeline. A footage stretch is one or
  // two clips (picture and its sound): list the pictures.
  const placed = viewing
    ? project.sequence.tracks
        .filter((track) => track.kind === "video" && track.role !== "webcam")
        .flatMap((track) => track.clips)
        .filter((clip) => `${clip.asset}.${clip.stream}` !== take.camera && clip.asset !== take.camera)
        .map((clip) => ({ clip, asset: assetById(project, clip.asset) }))
        .sort((a, b) => a.clip.startUs - b.clip.startUs)
    : [];
  return (
    <InspectorSection id="short-take" title="What plays under the take" icon={Film}>
      {viewing && <UnderTake project={project} short={short} />}
      {viewing && <FootageSound project={project} />}
      {placed.length === 0 ? (
        <p className="text-meta text-studio-500 leading-relaxed">Nothing under the take yet: the camera fills the frame.</p>
      ) : (
        <ul className="space-y-1" aria-label="Under the take">
          {placed.map(({ clip, asset }) => {
            const graphic = asset?.kind === "graphic";
            const Icon = graphic ? Shapes : Film;
            return (
              <li key={clip.id} className="flex items-center gap-1.5 text-meta">
                <Icon className="w-3.5 h-3.5 shrink-0 text-studio-500" aria-hidden />
                <button
                  type="button"
                  onClick={() => seekPlayback(clip.startUs)}
                  className="font-mono tabular-nums text-studio-300 hover:text-white w-[7.5rem] text-left shrink-0"
                  title="Go there"
                >
                  {formatTime(clip.startUs)}–{formatTime(clip.startUs + clip.durationUs)}
                </button>
                <span
                  className="flex-1 truncate text-studio-200"
                  title={graphic ? asset?.name : `${asset?.name ?? clip.asset}, from ${formatTime(clip.inUs)} of it`}
                >
                  {asset?.name ?? clip.asset}
                  {graphic || asset?.kind === "image" ? "" : ` · ${formatTime(clip.inUs)}`}
                </span>
                <button
                  type="button"
                  aria-label={`Remove ${asset?.name ?? "it"}`}
                  title="Remove (the camera fills the frame there)"
                  disabled={!!busy}
                  onClick={() =>
                    graphic
                      ? removeShortClips([clip.id])
                      : removeFootage(short.id, clip.startUs, clip.startUs + clip.durationUs)
                  }
                  className="h-6 w-6 shrink-0 inline-flex items-center justify-center rounded-control text-studio-500 hover:text-danger-fg hover:bg-danger/15 disabled:opacity-40"
                >
                  <Trash2 className="w-3.5 h-3.5" />
                </button>
              </li>
            );
          })}
        </ul>
      )}
    </InspectorSection>
  );
};
