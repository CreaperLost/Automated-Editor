import React, { useEffect, useRef, useState } from "react";
import { ArrowDownToLine, Mic, Music, Volume2, VolumeX, Wand2 } from "lucide-react";
import { InspectorSection, RangeRow } from "../inspector/InspectorSection";
import { Button, Notice, Switch } from "../ui";
import { useProjectStore } from "../../stores/projectStore";
import { api } from "../../lib/ipc";
import { saveTrackMix } from "../../lib/trackMix";
import { clipName, clipRole, trackLabel } from "../../lib/sequence";
import {
  AudioSettings,
  DEFAULT_AUDIO_SETTINGS,
  TRACK_VOLUME_DB_RANGE,
  TrackMix,
  type OpenedProject,
  type Role,
  type SeqTrack,
} from "../../lib/types";

type SoundRole = Extract<Role, "mic" | "background">;

/** One audio track as the mix shows it. */
interface SoundLane {
  id: string;
  label: string;
  track: SeqTrack;
  /** Speech: as marked on the track, else what its clips are. */
  role: SoundRole;
}

function soundLanes(project: OpenedProject): SoundLane[] {
  return project.sequence.tracks
    .filter((t) => t.kind === "audio")
    .map((track) => {
      const names = [...new Set(track.clips.map((c) => clipName(project, c)))];
      const clipRoles = track.clips.map((c) => clipRole(project, track, c));
      const role: SoundRole =
        track.role === "mic" || track.role === "background" ? track.role : clipRoles.includes("mic") ? "mic" : "background";
      const name = trackLabel(project.sequence, track);
      return {
        id: track.id,
        track,
        role,
        label: names.length === 0 ? name : `${name} · ${names.slice(0, 2).join(", ")}${names.length > 2 ? "…" : ""}`,
      };
    });
}

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
  <div className="space-y-2">
    <div className="flex items-center justify-between gap-3">
      <span className="text-label text-studio-200" title={hint}>
        {label}
      </span>
      <Switch checked={enabled} onChange={onToggle} title={hint} />
    </div>
    {enabled && (
      <div className="flex items-center gap-3">
        <input
          type="range"
          aria-label={label}
          min={min}
          max={max}
          step={1}
          value={value}
          onChange={(e) => onValue(Number(e.target.value))}
          className="flex-1 h-1.5 cursor-pointer"
        />
        <span className="w-[4.5rem] text-right font-mono text-meta tabular-nums text-studio-200">{valueLabel}</span>
      </div>
    )}
    <p className="text-meta text-studio-500">{hint}</p>
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
    className={`h-7 w-7 inline-flex items-center justify-center rounded-control transition-colors disabled:opacity-30 ${
      on ? "bg-accent/20 text-accent-fg shadow-[inset_0_0_0_1px_rgb(var(--accent-hover)/0.5)]" : "text-studio-500 hover:text-studio-100 hover:bg-studio-800"
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
    <div className="grid grid-cols-[auto_7rem_minmax(0,1fr)_auto_auto_auto_auto] items-center gap-1">
      <LaneToggle
        on={role === "mic"}
        label={`${lane.label}: ${role === "mic" ? "speech" : "background"}`}
        title={
          role === "mic"
            ? "Speech: what background sound ducks under. Click to mark the track as background."
            : "Background (music, game, desktop): can duck under speech. Click to mark the track as speech."
        }
        onClick={() => onRole(role === "mic" ? "background" : "mic")}
      >
        {role === "mic" ? <Mic className="w-4 h-4" /> : <Music className="w-4 h-4" />}
      </LaneToggle>
      <span className={`text-label truncate ${muted ? "text-studio-500 line-through" : "text-studio-200"}`} title={lane.label}>
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
        className="w-full min-w-0 h-1.5 cursor-pointer disabled:opacity-40"
      />
      <span className="w-14 text-right font-mono text-meta tabular-nums text-studio-200">{formatDb(volumeDb)}</span>
      <LaneToggle
        on={denoiseDb !== undefined}
        label={`Reduce noise on ${lane.label}`}
        title="Reduce steady background noise (fans, hum, hiss)"
        onClick={() => onDenoise(denoiseDb === undefined ? DEFAULT_DENOISE_DB : undefined)}
      >
        <Wand2 className="w-4 h-4" />
      </LaneToggle>
      <LaneToggle
        on={duckDb !== undefined}
        disabled={role === "mic" && duckDb === undefined}
        label={`Lower ${lane.label} under speech`}
        title={role === "mic" ? "Speech tracks are what others duck under" : "Lower this track while speech plays on a speech track"}
        onClick={() => onDuck(duckDb === undefined ? DEFAULT_DUCK_DB : undefined)}
      >
        <ArrowDownToLine className="w-4 h-4" />
      </LaneToggle>
      <button
        type="button"
        onClick={onMute}
        aria-pressed={muted}
        aria-label={`${muted ? "Unmute" : "Mute"} ${lane.label}`}
        className={`h-7 w-7 inline-flex items-center justify-center rounded-control hover:bg-studio-800 ${muted ? "text-danger-fg" : "text-studio-400 hover:text-studio-100"}`}
      >
        {muted ? <VolumeX className="w-4 h-4" /> : <Volume2 className="w-4 h-4" />}
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

/** Cleanup is committed only after a reusable audio copy has been prepared. */
const MouthClickCleanup: React.FC = () => {
  const project = useProjectStore((s) => s.openedProject);
  const enabled = project?.audio?.mouthClicks ?? false;
  const savedStrength = project?.audio?.mouthClickStrength ?? 35;
  const [strength, setStrength] = useState(savedStrength);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string>();
  const generation = useRef(0);
  useEffect(() => { setStrength(savedStrength); }, [savedStrength, project?.projectHandle]);
  useEffect(() => {
    ++generation.current;
    setBusy(false);
    setError(undefined);
    return () => { ++generation.current; };
  }, [project?.projectHandle]);
  const save = async (mouthClicks: boolean) => {
    const current = useProjectStore.getState().openedProject;
    if (!current || busy) return;
    const run = ++generation.current;
    setBusy(true);
    setError(undefined);
    try {
      const next = await api.projectAudioUpdate(current.projectHandle, current.revision, {
        ...DEFAULT_AUDIO_SETTINGS, ...current.audio, mouthClicks, mouthClickStrength: strength,
      });
      if (run === generation.current && useProjectStore.getState().openedProject?.projectHandle === current.projectHandle)
        useProjectStore.getState().applyOpenedProject(next);
    } catch (err) {
      if (run === generation.current) setError(String(err));
    } finally {
      if (run === generation.current) setBusy(false);
    }
  };
  return <fieldset disabled={busy} className="space-y-2 py-3 border-b border-studio-800" aria-busy={busy}>
    <div className="flex items-center justify-between gap-3">
      <span className="text-label text-studio-200">Reduce mouth clicks</span>
      <Switch checked={enabled} onChange={(value) => void save(value)} title="Reduce mouth clicks on speech tracks" />
    </div>
    <p className="text-meta text-studio-500">Repairs short clicks on tracks marked Speech, keeping timing intact. Turn off to compare with the original. Strong settings can soften consonants.</p>
    <RangeRow label="Cleanup strength" value={strength} min={1} max={100} unit="%" onChange={setStrength} />
    {enabled && strength !== savedStrength && <Button variant="secondary" size="sm" onClick={() => void save(true)}>Apply strength</Button>}
    {busy && <p role="status" className="text-meta text-studio-400">Preparing speech audio… Long recordings can take a few minutes.</p>}
    {error && <Notice tone="danger" onDismiss={() => setError(undefined)}>{error}</Notice>}
  </fieldset>;
};

/** Track mix and audio polish. Saved to the project, so playback and export both use them. */
export const AudioSection: React.FC = () => {
  const openedProject = useProjectStore((s) => s.openedProject);
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

  // Every audio track is one lane of the mix; its settings are kept under its id.
  const lanes = soundLanes(openedProject);
  const mix = (lane: SoundLane) => openedProject.audio?.tracks?.[lane.id];
  const setTrack = (lane: SoundLane, change: Partial<Pick<SeqTrack, "muted" | "role">>) => {
    const next = { ...lane.track, ...change };
    void api
      .projectSequenceEdit(openedProject.projectHandle, openedProject.revision, {
        kind: "setTrack",
        trackId: lane.id,
        name: next.name ?? "",
        hidden: !!next.hidden,
        muted: !!next.muted,
        locked: !!next.locked,
        role: next.role ?? null,
      })
      .then(applyOpenedProject)
      .catch((err) => setError(String(err)));
  };
  // The project-wide switches apply to speech (noise reduction) and background (ducking).
  const denoiseOf = (lane: SoundLane) =>
    mix(lane)?.denoiseDb ?? (lane.role === "mic" && draft.noiseReduction ? draft.noiseReductionDb : undefined);
  const duckOf = (lane: SoundLane) =>
    mix(lane)?.duckDb ?? (lane.role === "background" && draft.duckSystemAudio ? draft.duckDb : undefined);
  /** Saves a lane's polish; the first per-lane change turns the project-wide switches into per-lane ones. */
  const setPolish = (lane: SoundLane, patch: { denoiseDb?: number | undefined; duckDb?: number | undefined }) => {
    const patches: Record<string, Partial<TrackMix>> = {};
    const migrate = draft.noiseReduction || draft.duckSystemAudio;
    if (migrate) {
      for (const other of lanes) {
        if (other.role === "mic" && draft.noiseReduction && mix(other)?.denoiseDb === undefined)
          patches[other.id] = { ...patches[other.id], denoiseDb: draft.noiseReductionDb };
        if (other.role === "background" && draft.duckSystemAudio && mix(other)?.duckDb === undefined)
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
      // Track volumes are saved on their own; keep the latest ones.
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

  return (
    <InspectorSection id="audio" title="Audio" icon={Volume2}>
      {error && (
        <Notice tone="danger" className="rounded-control border" onDismiss={() => setError(undefined)}>
          {error}
        </Notice>
      )}
      {lanes.length === 0 ? (
        <p className="text-label text-studio-500">No audio tracks yet.</p>
      ) : (
        <div className="space-y-2 pb-3 border-b border-studio-800">
          {lanes.map((lane) => (
            <LaneRow
              key={lane.id}
              lane={lane}
              role={lane.role}
              muted={!!lane.track.muted}
              volumeDb={volumeDraft[lane.id] ?? mix(lane)?.volumeDb ?? 0}
              denoiseDb={denoiseOf(lane)}
              duckDb={duckOf(lane)}
              onRole={(role) => setTrack(lane, { role })}
              onMute={() => setTrack(lane, { muted: !lane.track.muted })}
              onVolume={(db) => setVolume(lane.id, db)}
              onDenoise={(db) => setPolish(lane, { denoiseDb: db })}
              onDuck={(db) => setPolish(lane, { duckDb: db })}
            />
          ))}
        </div>
      )}
      {lanes.some((lane) => lane.role === "mic") && <MouthClickCleanup />}
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
