import React, { useEffect, useRef, useState } from "react";
import { ArrowDownToLine, Mic, MousePointerClick, Music, Sparkles, Volume2, VolumeX, Wand2, Wind } from "lucide-react";
import { InspectorSection, MoreSettings, RangeRow } from "../inspector/InspectorSection";
import { PresetMarks } from "../presets/PresetMarks";
import { Button, Notice, Switch } from "../ui";
import { useProjectStore } from "../../stores/projectStore";
import { api } from "../../lib/ipc";
import { saveTrackMix } from "../../lib/trackMix";
import { useDebouncedSave } from "../../hooks/useDebouncedSave";
import { clipName, clipRole, trackLabel } from "../../lib/sequence";
import {
  AudioSettings,
  DEFAULT_AUDIO_SETTINGS,
  DEFAULT_SOUND_EFFECTS,
  SOUND_EFFECTS_VOLUME_DB_RANGE,
  SoundEffects,
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
      <MoreSettings>
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
      </MoreSettings>
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
      <MoreSettings>
        <div className="pl-7">
          <RangeRow label="Noise reduction" value={denoiseDb} min={3} max={30} unit=" dB" onChange={onDenoise} />
        </div>
      </MoreSettings>
    )}
    {duckDb !== undefined && (
      <MoreSettings>
        <div className="pl-7">
          <RangeRow label="Lower under speech" value={duckDb} min={3} max={30} unit=" dB" onChange={onDuck} />
        </div>
      </MoreSettings>
    )}
  </div>
);

/** The sound effects lane: soft clicks and a few whooshes from the recording, with its own
 *  volume and mute, lowered under speech. */
const SoundEffectsRow: React.FC<{
  settings: SoundEffects;
  onChange: (patch: Partial<SoundEffects>) => void;
}> = ({ settings, onChange }) => {
  const silent = settings.muted || (!settings.clicks && !settings.whooshes);
  return (
    <div className="grid grid-cols-[auto_7rem_minmax(0,1fr)_auto_auto_auto_auto] items-center gap-1">
      <span className="h-7 w-7 inline-flex items-center justify-center text-studio-500" aria-hidden>
        <Sparkles className="w-4 h-4" />
      </span>
      <span
        className={`text-label truncate ${silent ? "text-studio-500 line-through" : "text-studio-200"}`}
        title="Soft clicks on your mouse clicks and a light whoosh on zooms and layout switches, thinned out so it never gets busy, and lowered under speech."
      >
        Sound effects
      </span>
      <input
        type="range"
        aria-label="Sound effects volume"
        min={SOUND_EFFECTS_VOLUME_DB_RANGE.min}
        max={SOUND_EFFECTS_VOLUME_DB_RANGE.max}
        step={1}
        value={settings.volumeDb}
        disabled={silent}
        onChange={(e) => onChange({ volumeDb: Number(e.target.value) })}
        onDoubleClick={() => onChange({ volumeDb: 0 })}
        title="Double-click to reset"
        className="w-full min-w-0 h-1.5 cursor-pointer disabled:opacity-40"
      />
      <span className="w-14 text-right font-mono text-meta tabular-nums text-studio-200">{formatDb(settings.volumeDb)}</span>
      <LaneToggle
        on={settings.clicks}
        label="Click sounds"
        title="A soft click on each mouse click (at most one every fifth of a second; a double click clicks once)"
        onClick={() => onChange({ clicks: !settings.clicks })}
      >
        <MousePointerClick className="w-4 h-4" />
      </LaneToggle>
      <LaneToggle
        on={settings.whooshes}
        label="Whooshes"
        title="A light whoosh when a zoom goes in or out and when the layout switches; none on small zooms or back-to-back moves"
        onClick={() => onChange({ whooshes: !settings.whooshes })}
      >
        <Wind className="w-4 h-4" />
      </LaneToggle>
      <button
        type="button"
        onClick={() => onChange({ muted: !settings.muted })}
        aria-pressed={settings.muted}
        aria-label={`${settings.muted ? "Unmute" : "Mute"} sound effects`}
        className={`h-7 w-7 inline-flex items-center justify-center rounded-control hover:bg-studio-800 ${settings.muted ? "text-danger-fg" : "text-studio-400 hover:text-studio-100"}`}
      >
        {settings.muted ? <VolumeX className="w-4 h-4" /> : <Volume2 className="w-4 h-4" />}
      </button>
    </div>
  );
};

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
    <MoreSettings>
      <RangeRow label="Cleanup strength" value={strength} min={1} max={100} unit="%" onChange={setStrength} />
      {enabled && strength !== savedStrength && <Button variant="secondary" size="sm" onClick={() => void save(true)}>Apply strength</Button>}
    </MoreSettings>
    {busy && <p role="status" className="text-meta text-studio-400">Preparing speech audio… Long recordings can take a few minutes.</p>}
    {error && <Notice tone="danger" onDismiss={() => setError(undefined)}>{error}</Notice>}
  </fieldset>;
};

/** Track mix and audio polish. Saved to the project, so playback and export both use them. */
export const AudioSection: React.FC = () => {
  const openedProject = useProjectStore((s) => s.openedProject);
  const [volumeDraft, setVolumeDraft] = useState<Record<string, number>>({});
  const volumeSave = useDebouncedSave(250);
  const applyOpenedProject = useProjectStore((s) => s.applyOpenedProject);
  const saved = openedProject?.audio ?? DEFAULT_AUDIO_SETTINGS;
  const [draft, setDraft] = useState<AudioSettings>(saved);
  const [error, setError] = useState<string>();
  const settingsSave = useDebouncedSave(250);

  // Follow undo/redo and project switches while no edit is pending.
  const savedKey = JSON.stringify(saved);
  useEffect(() => {
    if (!settingsSave.isPending()) setDraft(saved);
  }, [savedKey]);

  if (!openedProject) return null;

  // Every audio track is one lane of the mix; its settings are kept under its id.
  const lanes = soundLanes(openedProject);
  const soundEffects = draft.soundEffects ?? DEFAULT_SOUND_EFFECTS;
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
    settingsSave.schedule((opened) => {
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
    });
  };

  const saveMix = (patches: Parameters<typeof saveTrackMix>[0]) =>
    saveTrackMix(patches)
      .then(() => setError(undefined))
      .catch((err) => setError(String(err)));

  const setVolume = (trackId: string, volumeDb: number) => {
    const drafts = { ...volumeDraft, [trackId]: volumeDb };
    setVolumeDraft(drafts);
    volumeSave.schedule(() => {
      const patches = Object.fromEntries(
        Object.entries(drafts).map(([id, db]) => [id, { volumeDb: db }]),
      );
      void saveMix(patches).finally(() => setVolumeDraft({}));
    });
  };

  return (
    <InspectorSection id="audio" title="Audio" icon={Volume2}>
      <PresetMarks part="audio" />
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
          <SoundEffectsRow
            settings={soundEffects}
            onChange={(patch) => update({ soundEffects: { ...soundEffects, ...patch } })}
          />
        </div>
      )}
      {lanes.some((lane) => lane.role === "mic") && <MouthClickCleanup />}
      {lanes.some((lane) => lane.track.role === "music" || (openedProject && lane.track.clips.some((c) => clipRole(openedProject, lane.track, c) === "music"))) && (
        <EffectRow
          label="Music under speech"
          hint="Measures the music and the speech and keeps music tracks this far under the voice, on top of their volume. About 18 dB suits a voice-over."
          enabled={draft.musicUnderSpeechDb !== undefined}
          onToggle={(on) => update({ musicUnderSpeechDb: on ? 18 : undefined })}
          valueLabel={`${draft.musicUnderSpeechDb ?? 18} dB`}
          min={6}
          max={40}
          value={draft.musicUnderSpeechDb ?? 18}
          onValue={(musicUnderSpeechDb) => update({ musicUnderSpeechDb })}
        />
      )}
      <EffectRow
        label="Normalize loudness"
        hint="Scales the whole edit to a standard loudness without pushing peaks into the limiter. -16 LUFS suits speech."
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
