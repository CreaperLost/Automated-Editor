import React, { useEffect, useRef, useState } from "react";
import { Volume2, VolumeX } from "lucide-react";
import { InspectorSection } from "../inspector/InspectorSection";
import { useProjectStore } from "../../stores/projectStore";
import { api } from "../../lib/ipc";
import { saveTrackMix } from "../../lib/trackMix";
import { audioStreamCount, audioTracks as audioTracksOf, trackLabel, videoTracks as videoTracksOf } from "../../lib/trackUtils";
import { buildClips } from "../../lib/projectUtils";
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

/** One timeline lane of imported sound, by the mix id the backend plays it under. */
interface SoundLane {
  id: string;
  label: string;
  /** An audio track mutes as a track; other lanes mute in the mix. */
  trackId?: string;
}

const LaneRow: React.FC<{
  lane: SoundLane;
  muted: boolean;
  volumeDb: number;
  onMute: () => void;
  onVolume: (volumeDb: number) => void;
}> = ({ lane, muted, volumeDb, onMute, onVolume }) => (
  <div className="grid grid-cols-[6.5rem_minmax(0,1fr)_auto_auto] items-center gap-2">
    <span className={`text-xs truncate ${muted ? "text-studio-500 line-through" : "text-studio-300"}`} title={lane.label}>
      {lane.label}
    </span>
    <input
      type="range"
      aria-label={`${lane.label} volume`}
      min={TRACK_VOLUME_DB_RANGE.min}
      max={TRACK_VOLUME_DB_RANGE.max}
      step={1}
      value={volumeDb}
      disabled={muted}
      onChange={(e) => onVolume(Number(e.target.value))}
      onDoubleClick={() => onVolume(0)}
      title="Double-click to reset"
      className="w-full min-w-0 accent-indigo-500 h-1.5 bg-studio-800 rounded-lg cursor-pointer disabled:opacity-40"
    />
    <span className="w-12 text-right font-mono text-[11px] text-studio-300">{formatDb(volumeDb)}</span>
    <button
      type="button"
      onClick={onMute}
      aria-pressed={muted}
      aria-label={`${muted ? "Unmute" : "Mute"} ${lane.label}`}
      className={`p-1 rounded hover:bg-studio-800 ${muted ? "text-rose-300" : "text-studio-400 hover:text-white"}`}
    >
      {muted ? <VolumeX className="w-3.5 h-3.5" /> : <Volume2 className="w-3.5 h-3.5" />}
    </button>
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

  // The lanes of imported sound, named as on the timeline (ids match src-tauri/src/media/audio.rs).
  const assetOf = (id: string) => openedProject.mediaAssets?.find((a) => a.id === id);
  const overlay = openedProject.overlayTracks ?? [];
  const mainStreams = Math.max(
    0,
    ...buildClips(openedProject.retainedIntervals)
      .filter((c) => c.media && !c.audioUnlinked)
      .map((c) => audioStreamCount(assetOf(c.media!))),
  );
  const soundLanes: SoundLane[] = [
    ...Array.from({ length: mainStreams }, (_, k) => ({ id: `main-sound-${k + 1}`, label: `V1 sound ${k + 1}` })),
    ...videoTracksOf(overlay).flatMap((track) => {
      const streams = Math.max(
        0,
        ...track.clips.filter((c) => !c.audioUnlinked).map((c) => audioStreamCount(assetOf(c.assetId))),
      );
      return Array.from({ length: streams }, (_, k) => ({
        id: `${track.id}-sound-${k + 1}`,
        label: `${trackLabel(overlay, track.id)} sound ${k + 1}`,
      }));
    }),
    ...audioTracksOf(overlay).map((track) => ({ id: track.id, label: trackLabel(overlay, track.id), trackId: track.id })),
  ];
  const laneMuted = (lane: SoundLane) =>
    lane.trackId
      ? !!overlay.find((t) => t.id === lane.trackId)?.muted
      : !!openedProject.audio?.tracks?.[lane.id]?.muted;
  const muteLane = (lane: SoundLane) => {
    const track = overlay.find((t) => t.id === lane.trackId);
    if (!track) {
      void saveMix({ [lane.id]: { muted: !laneMuted(lane) } });
      return;
    }
    void api
      .projectTracksEdit(openedProject.projectHandle, openedProject.revision, {
        kind: "setTrack",
        trackId: track.id,
        hidden: track.hidden,
        muted: !track.muted,
      })
      .then(applyOpenedProject)
      .catch((err) => setError(String(err)));
  };

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
      {soundLanes.length > 0 && (
        <div className="space-y-1.5 pb-2 border-b border-studio-800">
          <div className="text-[10px] font-semibold uppercase tracking-wider text-studio-500">Imported sound</div>
          {soundLanes.map((lane) => (
            <LaneRow
              key={lane.id}
              lane={lane}
              muted={laneMuted(lane)}
              volumeDb={volumeDraft[lane.id] ?? openedProject.audio?.tracks?.[lane.id]?.volumeDb ?? 0}
              onMute={() => muteLane(lane)}
              onVolume={(db) => setVolume(lane.id, db)}
            />
          ))}
        </div>
      )}
      {audioTracks.length === 0 ? (
        soundLanes.length === 0 && <p className="text-[11px] text-studio-500">No audio tracks yet.</p>
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
