import React, { useEffect, useRef, useState } from "react";
import { ArrowDownToLine, Mic, Music, Volume2, VolumeX, Wand2 } from "lucide-react";
import { InspectorSection, RangeRow } from "../inspector/InspectorSection";
import { useProjectStore } from "../../stores/projectStore";
import { api } from "../../lib/ipc";
import { saveTrackMix } from "../../lib/trackMix";
import { laneRole, soundLanes, type SoundLane } from "../../lib/trackUtils";
import {
  AudioSettings,
  DEFAULT_AUDIO_SETTINGS,
  TRACK_VOLUME_DB_RANGE,
  TrackMix,
  SoundRole,
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
          className="flex-1 h-1.5 bg-studio-800 rounded-lg cursor-pointer"
        />
        <span className="w-16 text-right font-mono text-[11px] text-studio-300">{valueLabel}</span>
      </div>
    )}
  </div>
);

const formatDb = (db: number) => (db > 0 ? `+${db} dB` : `${db} dB`);

const DEFAULT_DENOISE_DB = 12;
const DEFAULT_DUCK_DB = 12;

/** A round icon toggle for a lane setting. */
const LaneToggle: React.FC<{
  on: boolean;
  disabled?: boolean;
  label: string;
  title: string;
  onClick: () => void;
  children: React.ReactNode;
}> = ({ on, disabled, label, title, onClick, children }) => (
  <button
    type="button"
    aria-pressed={on}
    aria-label={label}
    title={title}
    disabled={disabled}
    onClick={onClick}
    className={`p-1 rounded transition-colors disabled:opacity-30 ${
      on ? "bg-indigo-600/40 text-indigo-100" : "text-studio-500 hover:text-studio-200 hover:bg-studio-800"
    }`}
  >
    {children}
  </button>
);

/** One lane: its role, volume, noise reduction, ducking and mute; a setting's amount shows
 *  only while it is on. */
const LaneRow: React.FC<{
  lane: SoundLane;
  role: SoundRole;
  muted: boolean;
  volumeDb: number;
  denoiseDb?: number;
  duckDb?: number;
  onRole: (role: SoundRole) => void;
  onMute: () => void;
  onVolume: (volumeDb: number) => void;
  onDenoise: (db: number | undefined) => void;
  onDuck: (db: number | undefined) => void;
}> = ({ lane, role, muted, volumeDb, denoiseDb, duckDb, onRole, onMute, onVolume, onDenoise, onDuck }) => (
  <div className="space-y-1">
    <div className="grid grid-cols-[auto_5.5rem_minmax(0,1fr)_auto_auto_auto_auto] items-center gap-1.5">
      <LaneToggle
        on={role === "mic"}
        label={`${lane.label}: ${role === "mic" ? "speech" : "background"}`}
        title={
          role === "mic"
            ? "Speech: transcribed, captioned, and what background sound ducks under. Click for background."
            : "Background (music, game, desktop): can duck under speech. Click for speech."
        }
        onClick={() => onRole(role === "mic" ? "background" : "mic")}
      >
        {role === "mic" ? <Mic className="w-3.5 h-3.5" /> : <Music className="w-3.5 h-3.5" />}
      </LaneToggle>
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
        className="w-full min-w-0 h-1.5 bg-studio-800 rounded-lg cursor-pointer disabled:opacity-40"
      />
      <span className="w-11 text-right font-mono text-[10px] text-studio-300">{formatDb(volumeDb)}</span>
      <LaneToggle
        on={denoiseDb !== undefined}
        label={`Reduce noise on ${lane.label}`}
        title="Reduce steady background noise (fans, hum, hiss)"
        onClick={() => onDenoise(denoiseDb === undefined ? DEFAULT_DENOISE_DB : undefined)}
      >
        <Wand2 className="w-3.5 h-3.5" />
      </LaneToggle>
      <LaneToggle
        on={duckDb !== undefined}
        disabled={role === "mic" && duckDb === undefined}
        label={`Lower ${lane.label} under speech`}
        title={role === "mic" ? "Speech lanes are what others duck under" : "Lower this lane while speech plays on a speech lane"}
        onClick={() => onDuck(duckDb === undefined ? DEFAULT_DUCK_DB : undefined)}
      >
        <ArrowDownToLine className="w-3.5 h-3.5" />
      </LaneToggle>
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
    {denoiseDb !== undefined && (
      <div className="pl-7">
        <RangeRow label="Noise reduction" value={denoiseDb} min={3} max={30} unit=" dB" onChange={onDenoise} />
      </div>
    )}
    {duckDb !== undefined && (
      <div className="pl-7">
        <RangeRow label="Lower under speech" value={duckDb} min={3} max={30} unit=" dB" onChange={onDuck} />
      </div>
    )}
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

  // Every lane, named as on the timeline (ids match src-tauri/src/media/audio.rs).
  const overlay = openedProject.overlayTracks ?? [];
  const lanes = soundLanes(openedProject);
  const mix = (lane: SoundLane) => openedProject.audio?.tracks?.[lane.id];
  const laneMuted = (lane: SoundLane) =>
    lane.trackId ? !!overlay.find((t) => t.id === lane.trackId)?.muted : !!mix(lane)?.muted;
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
  // The older project-wide switches still apply to the recording's own tracks.
  const denoiseOf = (lane: SoundLane) =>
    mix(lane)?.denoiseDb ?? (lane.recorded === "mic" && draft.noiseReduction ? draft.noiseReductionDb : undefined);
  const duckOf = (lane: SoundLane) =>
    mix(lane)?.duckDb ?? (lane.recorded === "system" && draft.duckSystemAudio ? draft.duckDb : undefined);
  /** Saves a lane's polish; the first per-lane change turns the old switches into per-lane ones. */
  const setPolish = (lane: SoundLane, patch: { denoiseDb?: number | undefined; duckDb?: number | undefined }) => {
    const patches: Record<string, Partial<TrackMix>> = {};
    const migrate = draft.noiseReduction || draft.duckSystemAudio;
    if (migrate) {
      for (const other of lanes) {
        if (other.recorded === "mic" && draft.noiseReduction && mix(other)?.denoiseDb === undefined)
          patches[other.id] = { ...patches[other.id], denoiseDb: draft.noiseReductionDb };
        if (other.recorded === "system" && draft.duckSystemAudio && mix(other)?.duckDb === undefined)
          patches[other.id] = { ...patches[other.id], duckDb: draft.duckDb };
      }
    }
    patches[lane.id] = { ...patches[lane.id], ...patch };
    void saveTrackMix(patches, migrate ? { noiseReduction: false, duckSystemAudio: false } : undefined)
      .then(() => setError(undefined))
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
      {lanes.length === 0 ? (
        <p className="text-[11px] text-studio-500">No sound yet.</p>
      ) : (
        <div className="space-y-2 pb-2 border-b border-studio-800">
          {lanes.map((lane) => (
            <LaneRow
              key={lane.id}
              lane={lane}
              role={laneRole(openedProject, lane)}
              muted={laneMuted(lane)}
              volumeDb={volumeDraft[lane.id] ?? mix(lane)?.volumeDb ?? 0}
              denoiseDb={denoiseOf(lane)}
              duckDb={duckOf(lane)}
              onRole={(role) => void saveMix({ [lane.id]: { role } })}
              onMute={() => muteLane(lane)}
              onVolume={(db) => setVolume(lane.id, db)}
              onDenoise={(db) => setPolish(lane, { denoiseDb: db })}
              onDuck={(db) => setPolish(lane, { duckDb: db })}
            />
          ))}
          {audioTracks.length > 0 && (
            <button
              type="button"
              onClick={() =>
                void saveMix(Object.fromEntries(audioTracks.map((t) => [t.id, { muted: !allMuted }])))
              }
              title={
                allMuted
                  ? "Bring every recorded track back"
                  : "Mute every recorded track. With nothing else playing, export writes no audio stream."
              }
              className={`w-full py-1 rounded text-xs font-medium border transition-colors ${
                allMuted
                  ? "border-teal-500/40 text-teal-300 hover:bg-teal-600/20"
                  : "border-rose-500/30 text-rose-300 hover:bg-rose-950/40"
              }`}
            >
              {allMuted ? "Restore recorded audio" : "Mute recorded audio"}
            </button>
          )}
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
    </InspectorSection>
  );
};
