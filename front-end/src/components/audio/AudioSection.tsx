import React, { useEffect, useRef, useState } from "react";
import { Volume2, VolumeX } from "lucide-react";
import { InspectorSection } from "../inspector/InspectorSection";
import { useProjectStore } from "../../stores/projectStore";
import { api } from "../../lib/ipc";
import { saveTrackMix } from "../../lib/trackMix";
import {
  AudioSettings,
  DEFAULT_AUDIO_SETTINGS,
  TRACK_VOLUME_DB_RANGE,
  Track,
  isAudioTrack,
} from "../../lib/types";

interface EffectRowProps {
  label: string;
  hint: string;
  enabled: boolean;
  onToggle: (enabled: boolean) => void;
  valueLabel: string;
  min: number;
  max: number;
  value: number;
  onValue: (value: number) => void;
}

const EffectRow: React.FC<EffectRowProps> = ({
  label,
  hint,
  enabled,
  onToggle,
  valueLabel,
  min,
  max,
  value,
  onValue,
}) => (
  <div className="space-y-1.5">
    <label className="flex items-center justify-between text-xs cursor-pointer">
      <span className="text-studio-300" title={hint}>
        {label}
      </span>
      <input
        type="checkbox"
        checked={enabled}
        onChange={(e) => onToggle(e.target.checked)}
        className="rounded bg-studio-800 border-studio-700 text-indigo-600 focus:ring-0 cursor-pointer"
      />
    </label>
    {enabled && (
      <div className="flex items-center gap-2">
        <input
          type="range"
          min={min}
          max={max}
          step={1}
          value={value}
          onChange={(e) => onValue(Number(e.target.value))}
          className="flex-1 accent-indigo-500 h-1.5 bg-studio-800 rounded-lg cursor-pointer"
        />
        <span className="w-16 text-right font-mono text-[11px] text-studio-300">{valueLabel}</span>
      </div>
    )}
  </div>
);

const formatDb = (db: number) => (db > 0 ? `+${db} dB` : `${db} dB`);

interface TrackRowProps {
  track: Track;
  volumeDb: number;
  onMute: () => void;
  onVolume: (volumeDb: number) => void;
}

const TrackRow: React.FC<TrackRowProps> = ({ track, volumeDb, onMute, onVolume }) => (
  <div className="space-y-1">
    <div className="flex items-center justify-between text-xs">
      <span className={track.muted ? "text-studio-500 line-through" : "text-studio-300"}>
        {track.trackType === "mic" ? "Microphone" : "System audio"}
        <span className="ml-1 font-mono text-[10px] text-studio-500">{track.id}</span>
      </span>
      <button
        type="button"
        onClick={onMute}
        aria-pressed={track.muted}
        title={track.muted ? "Unmute" : "Mute in preview and export"}
        className={`p-1 rounded hover:bg-studio-800 ${track.muted ? "text-rose-300" : "text-studio-400 hover:text-white"}`}
      >
        {track.muted ? <VolumeX className="w-3.5 h-3.5" /> : <Volume2 className="w-3.5 h-3.5" />}
      </button>
    </div>
    <div className="flex items-center gap-2">
      <input
        type="range"
        aria-label={`${track.id} volume`}
        min={TRACK_VOLUME_DB_RANGE.min}
        max={TRACK_VOLUME_DB_RANGE.max}
        step={1}
        value={volumeDb}
        disabled={track.muted}
        onChange={(e) => onVolume(Number(e.target.value))}
        onDoubleClick={() => onVolume(0)}
        title="Double-click to reset"
        className="flex-1 accent-indigo-500 h-1.5 bg-studio-800 rounded-lg cursor-pointer disabled:opacity-40"
      />
      <span className="w-16 text-right font-mono text-[11px] text-studio-300">{formatDb(volumeDb)}</span>
    </div>
  </div>
);

/** Track mix and audio polish. Saved to the project, so playback and export both use them. */
export const AudioSection: React.FC = () => {
  const openedProject = useProjectStore((s) => s.openedProject);
  const audioTracks = useProjectStore((s) => s.tracks).filter((t) => isAudioTrack(t.trackType));
  const [volumeDraft, setVolumeDraft] = useState<Record<string, number>>({});
  const volumeTimer = useRef<number>();
  const applyOpenedProject = useProjectStore((s) => s.applyOpenedProject);
  const saved = openedProject?.audio ?? DEFAULT_AUDIO_SETTINGS;
  const [draft, setDraft] = useState<AudioSettings>(saved);
  const [error, setError] = useState<string>();
  const timer = useRef<number>();
  const openedRef = useRef(openedProject);
  openedRef.current = openedProject;

  // Follow undo/redo and project switches while no edit is pending.
  const savedKey = JSON.stringify(saved);
  useEffect(() => {
    if (timer.current === undefined) setDraft(saved);
  }, [savedKey]);

  useEffect(
    () => () => {
      window.clearTimeout(timer.current);
      window.clearTimeout(volumeTimer.current);
    },
    [],
  );

  if (!openedProject) return null;

  const update = (patch: Partial<AudioSettings>) => {
    const next = { ...draft, ...patch };
    setDraft(next);
    window.clearTimeout(timer.current);
    timer.current = window.setTimeout(() => {
      timer.current = undefined;
      const opened = openedRef.current;
      if (!opened) return;
      // Track mutes and volumes are saved on their own; keep the latest ones.
      void api
        .projectAudioUpdate(opened.projectHandle, opened.revision, {
          ...next,
          tracks: opened.audio?.tracks,
        })
        .then((updated) => {
          applyOpenedProject(updated);
          setError(undefined);
        })
        .catch((err) => setError(String(err)));
    }, 250);
  };

  const saveMix = (patches: Parameters<typeof saveTrackMix>[0]) =>
    saveTrackMix(patches)
      .then(() => setError(undefined))
      .catch((err) => setError(String(err)));

  const setVolume = (trackId: string, volumeDb: number) => {
    const drafts = { ...volumeDraft, [trackId]: volumeDb };
    setVolumeDraft(drafts);
    window.clearTimeout(volumeTimer.current);
    volumeTimer.current = window.setTimeout(() => {
      volumeTimer.current = undefined;
      const patches = Object.fromEntries(
        Object.entries(drafts).map(([id, db]) => [id, { volumeDb: db }]),
      );
      void saveMix(patches).finally(() => setVolumeDraft({}));
    }, 250);
  };

  const allMuted = audioTracks.length > 0 && audioTracks.every((t) => t.muted);

  return (
    <InspectorSection id="audio" title="Audio" icon={Volume2}>
      {error && (
        <p role="alert" className="text-[11px] text-rose-300 bg-rose-950/40 border border-rose-900/40 rounded p-2">
          {error}
        </p>
      )}
      {audioTracks.length === 0 ? (
        <p className="text-[11px] text-studio-500">This recording has no audio tracks.</p>
      ) : (
        <div className="space-y-2.5 pb-2 border-b border-studio-800">
          {audioTracks.map((track) => (
            <TrackRow
              key={track.id}
              track={track}
              volumeDb={
                volumeDraft[track.id] ??
                openedProject.audio?.tracks?.[track.id]?.volumeDb ??
                0
              }
              onMute={() => void saveMix({ [track.id]: { muted: !track.muted } })}
              onVolume={(db) => setVolume(track.id, db)}
            />
          ))}
          <button
            type="button"
            onClick={() =>
              void saveMix(Object.fromEntries(audioTracks.map((t) => [t.id, { muted: !allMuted }])))
            }
            title={
              allMuted
                ? "Bring every audio track back"
                : "Mute every track. Export then writes a video with no audio stream."
            }
            className={`w-full py-1.5 rounded text-xs font-medium border transition-colors ${
              allMuted
                ? "border-teal-500/40 text-teal-300 hover:bg-teal-600/20"
                : "border-rose-500/30 text-rose-300 hover:bg-rose-950/40"
            }`}
          >
            {allMuted ? "Restore audio" : "Remove all audio"}
          </button>
        </div>
      )}
      <EffectRow
        label="Normalize loudness"
        hint="Scales the whole edit to a standard loudness. -14 LUFS matches YouTube."
        enabled={draft.normalize}
        onToggle={(normalize) => update({ normalize })}
        valueLabel={`${draft.targetLufs} LUFS`}
        min={-30}
        max={-8}
        value={draft.targetLufs}
        onValue={(targetLufs) => update({ targetLufs })}
      />
      <EffectRow
        label="Reduce mic noise"
        hint="Lowers steady background noise (fans, hum, hiss) on the microphone."
        enabled={draft.noiseReduction}
        onToggle={(noiseReduction) => update({ noiseReduction })}
        valueLabel={`-${draft.noiseReductionDb} dB`}
        min={3}
        max={30}
        value={draft.noiseReductionDb}
        onValue={(noiseReductionDb) => update({ noiseReductionDb })}
      />
      <EffectRow
        label="Lower system audio when speaking"
        hint="Ducks game or desktop audio while the microphone picks up speech."
        enabled={draft.duckSystemAudio}
        onToggle={(duckSystemAudio) => update({ duckSystemAudio })}
        valueLabel={`-${draft.duckDb} dB`}
        min={3}
        max={30}
        value={draft.duckDb}
        onValue={(duckDb) => update({ duckDb })}
      />
    </InspectorSection>
  );
};
