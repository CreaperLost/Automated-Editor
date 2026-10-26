import React, { useState } from "react";
import { Layers, Trash2 } from "lucide-react";
import { InspectorSection } from "./InspectorSection";
import { useProjectStore } from "../../stores/projectStore";
import { useSettingsStore } from "../../stores/settingsStore";
import { api } from "../../lib/ipc";
import {
  MAX_SPEED,
  MIN_SPEED,
  assetById,
  clipName,
  clipRole,
  clipSpeed,
  findClip,
  formatSpeed,
  streamOf,
  trackNumber,
} from "../../lib/sequence";
import { layoutFromSettings } from "../../lib/types";
import type { Fit, LicenceKind, OpenedProject, Role, SequenceEdit } from "../../lib/types";
import { cropOffFor, setCropFor } from "../../lib/windowCrop";
import { Switch } from "../ui";

const FITS: { value: Fit; label: string }[] = [
  { value: "contain", label: "Fit inside the frame" },
  { value: "cover", label: "Fill the frame (crop edges)" },
];

/** Speeds one click away; any other is typed in. */
const SPEEDS = [0.5, 1, 1.5, 2, 4];

const PICTURE_ROLES: { value: Role; label: string }[] = [
  { value: "screen", label: "Screen: canvas layout, zooms, cursor" },
  { value: "webcam", label: "Camera: in the bubble" },
  { value: "overlay", label: "Overlay: over the whole canvas" },
];
const SOUND_ROLES: { value: Role; label: string }[] = [
  { value: "mic", label: "Speech: transcribed and captioned" },
  { value: "background", label: "Background: game, desktop" },
  { value: "music", label: "Music: kept under the speech" },
  { value: "effects", label: "Sound effects" },
];

const LICENCES: { value: LicenceKind | ""; label: string }[] = [
  { value: "", label: "Not known" },
  { value: "cc0", label: "CC0: no rights reserved" },
  { value: "publicDomain", label: "Public domain" },
  { value: "noCredit", label: "Free to use, no credit needed" },
  { value: "creditRequired", label: "Needs credit" },
  { value: "nonCommercial", label: "Not for monetised videos" },
  { value: "unknown", label: "Unclear" },
];

/** A number field committed on blur or Enter. */
const NumberField: React.FC<{
  label: string;
  value: number;
  min: number;
  max: number;
  step: number;
  disabled?: boolean;
  onCommit: (value: number) => void;
}> = ({ label, value, min, max, step, disabled, onCommit }) => (
  <label className="flex items-center justify-between text-label text-studio-300">
    <span>{label}</span>
    <input
      key={value}
      aria-label={label}
      type="number"
      min={min}
      max={max}
      step={step}
      defaultValue={value}
      disabled={disabled}
      onBlur={(event) => onCommit(Number(event.target.value))}
      onKeyDown={(event) => {
        if (event.key === "Enter") onCommit(Number((event.target as HTMLInputElement).value));
      }}
      className="w-24 bg-studio-950 px-2 py-1 rounded border border-studio-700 text-right"
    />
  </label>
);

/// The clip selected on the timeline: what its stream is, how it fills the frame, where it
/// starts, and removing it. Every change is one undoable edit.
export const TrackClipSection: React.FC = () => {
  const openedProject = useProjectStore((s) => s.openedProject);
  const applyOpenedProject = useProjectStore((s) => s.applyOpenedProject);
  const selectedIds = useProjectStore((s) => s.selectedClipIds);
  const setSelectedIds = useProjectStore((s) => s.setSelectedClipIds);
  const [error, setError] = useState<string>();
  const [busy, setBusy] = useState(false);
  const canvas = useSettingsStore((s) => s.canvas);
  const cameraBubble = useSettingsStore((s) => s.cameraBubble);

  const found =openedProject && selectedIds[0] ? findClip(openedProject.sequence, selectedIds[0]) : undefined;
  if (!openedProject || !found) return null;
  const { track, clip } = found;
  const asset = assetById(openedProject, clip.asset);
  const stream = streamOf(openedProject, clip);
  const role = clipRole(openedProject, track, clip);
  const partners = clip.link
    ? openedProject.sequence.tracks.flatMap((t) => t.clips).filter((c) => c.link === clip.link && c.id !== clip.id).length
    : 0;

  const run = async (work: () => Promise<OpenedProject>) => {
    if (busy) return;
    setBusy(true);
    setError(undefined);
    try {
      applyOpenedProject(await work());
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  };
  const edit = (change: SequenceEdit) =>
    run(() => api.projectSequenceEdit(openedProject.projectHandle, openedProject.revision, change));
  const setStart = (seconds: number) => {
    const startUs = Math.round(seconds * 1e6);
    if (!Number.isFinite(startUs) || startUs < 0 || startUs === clip.startUs) return;
    void edit({ kind: "moveClips", clipIds: [clip.id], deltaUs: startUs - clip.startUs });
  };
  const setRole = (next: Role) => {
    if (!stream || next === stream.role) return;
    void run(() =>
      api.projectMediaRoles(openedProject.projectHandle, openedProject.revision, clip.asset, [{ stream: stream.id, role: next }]),
    );
  };
  const roles = stream?.kind === "sound" ? SOUND_ROLES : PICTURE_ROLES;
  const speed = clipSpeed(clip);
  const setSpeed = (next: number) => {
    if (!Number.isFinite(next)) return;
    const clamped = Math.min(MAX_SPEED, Math.max(MIN_SPEED, next));
    if (Math.abs(clamped - speed) < 0.0005) return;
    void edit({ kind: "setSpeed", clipIds: selectedIds.length ? selectedIds : [clip.id], speed: clamped });
  };

  // The window crop for this clip's stretch of the recording.
  const firstRecording = openedProject.assets.find((a) => a.kind === "recording")?.id;
  const cropsHere = role === "screen" && asset?.kind === "recording" && canvas.windowCrop;
  const [clipFrom, clipTo] = [clip.inUs, clip.inUs + clip.durationUs];
  const cropOff = cropOffFor(canvas.windowCropOff, clip.asset, clipFrom, clipTo, firstRecording);
  const setClipCrop = (on: boolean) => {
    const windowCropOff = setCropFor(canvas.windowCropOff, clip.asset, clipFrom, clipTo, on, firstRecording);
    const layout = layoutFromSettings({ ...canvas, windowCropOff }, cameraBubble, openedProject.layout);
    void run(() => api.projectLayoutUpdate(openedProject.projectHandle, openedProject.revision, layout));
  };

  return (
    <InspectorSection id="track-clip" title={`Clip · ${trackNumber(openedProject.sequence, track.id)}`} icon={Layers}>
      <div className="text-label text-studio-200 truncate" title={asset?.path}>
        {clipName(openedProject, clip)}
        <span className="ml-1.5 text-studio-500 font-mono">
          {(clip.durationUs / 1e6).toFixed(2)}s{speed !== 1 && ` at ${formatSpeed(speed)}`}
        </span>
      </div>
      <div className="text-meta text-studio-400">
        {partners > 0
          ? `Linked with ${partners} other clip${partners === 1 ? "" : "s"}: they move and trim together (U unlinks).`
          : "Not linked: it moves and trims on its own."}
        {asset?.missing && " The media is missing."}
      </div>
      {stream && (
        <label className="block text-label text-studio-300 space-y-1">
          <span>{stream.kind === "sound" ? "This sound is" : "This picture is"}</span>
          <select
            aria-label="What the stream is"
            value={stream.role}
            disabled={busy}
            onChange={(event) => setRole(event.target.value as Role)}
            className="w-full min-w-0 truncate bg-studio-800 text-studio-100 text-label rounded px-2 py-1 border border-studio-700 focus:outline-none focus:border-accent-hover"
          >
            {roles.map((option) => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))}
          </select>
          {track.role && track.role !== stream.role && (
            <span className="block text-meta text-studio-500">
              Its track plays it as {track.role} (set on the track's header).
            </span>
          )}
        </label>
      )}
      {track.kind === "audio" && (
        <div className="space-y-1">
          <NumberField
            label="Clip gain (dB)"
            value={clip.mix?.gainDb ?? 0}
            min={-60}
            max={12}
            step={1}
            disabled={busy || track.locked}
            onCommit={(gainDb) =>
              Number.isFinite(gainDb) && gainDb !== (clip.mix?.gainDb ?? 0) && void edit({ kind: "setSound", clipIds: selectedIds.length ? selectedIds : [clip.id], gainDb })
            }
          />
          <NumberField
            label="Fade in (s)"
            value={(clip.mix?.fadeInUs ?? 0) / 1e6}
            min={0}
            max={clip.durationUs / 1e6}
            step={0.1}
            disabled={busy || track.locked}
            onCommit={(s) => Number.isFinite(s) && s >= 0 && void edit({ kind: "setSound", clipIds: [clip.id], fadeInUs: Math.round(s * 1e6) })}
          />
          <NumberField
            label="Fade out (s)"
            value={(clip.mix?.fadeOutUs ?? 0) / 1e6}
            min={0}
            max={clip.durationUs / 1e6}
            step={0.1}
            disabled={busy || track.locked}
            onCommit={(s) => Number.isFinite(s) && s >= 0 && void edit({ kind: "setSound", clipIds: [clip.id], fadeOutUs: Math.round(s * 1e6) })}
          />
          <p className="text-meta text-studio-500">On top of the track's volume (Audio page); fades start from silence at the clip's ends.</p>
        </div>
      )}
      {asset && stream?.kind === "sound" && (role === "music" || role === "effects" || asset.licence) && (
        <label className="block text-label text-studio-300 space-y-1">
          <span>Licence</span>
          <select
            aria-label="The file's licence"
            value={asset.licence?.kind ?? ""}
            disabled={busy}
            onChange={(event) => {
              const kind = event.target.value as LicenceKind | "";
              void run(() =>
                api.projectMediaLicence(
                  openedProject.projectHandle,
                  openedProject.revision,
                  asset.id,
                  kind ? { kind, source: asset.licence?.source } : null,
                ),
              );
            }}
            className="w-full min-w-0 truncate bg-studio-800 text-studio-100 text-label rounded px-2 py-1 border border-studio-700 focus:outline-none focus:border-accent-hover"
          >
            {LICENCES.map((option) => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))}
          </select>
          {role === "music" && !["cc0", "publicDomain", "noCredit"].includes(asset.licence?.kind ?? "") && (
            <span className="block text-meta text-suggest-fg">Music in the video must need no credit: the check before export flags this.</span>
          )}
        </label>
      )}
      {cropsHere && (
        <Switch
          checked={!cropOff}
          disabled={busy}
          onChange={setClipCrop}
          label="Crop to the active window"
          title="Off shows the whole screen for this clip, whatever window is in front. The switch for the whole video is on the Video page."
        />
      )}
      {track.kind === "video" && role === "overlay" && (
        <label className="block text-label text-studio-300 space-y-1">
          <span>Picture</span>
          <select
            aria-label="How the clip fills the frame"
            value={clip.fit ?? "contain"}
            disabled={busy || track.locked}
            onChange={(event) => void edit({ kind: "setClip", clipId: clip.id, fit: event.target.value as Fit })}
            className="w-full min-w-0 truncate bg-studio-800 text-studio-100 text-label rounded px-2 py-1 border border-studio-700 focus:outline-none focus:border-accent-hover"
          >
            {FITS.map((fit) => (
              <option key={fit.value} value={fit.value}>
                {fit.label}
              </option>
            ))}
          </select>
        </label>
      )}
      {asset?.kind !== "image" && asset?.kind !== "graphic" && (
        <div className="space-y-1">
          <label className="flex items-center justify-between text-label text-studio-300">
            <span>Speed (%)</span>
            <input
              key={`${clip.id}-${speed}`}
              aria-label="Clip speed in percent"
              type="number"
              min={MIN_SPEED * 100}
              max={MAX_SPEED * 100}
              step={5}
              defaultValue={Number((speed * 100).toFixed(1))}
              disabled={busy || track.locked}
              onBlur={(event) => setSpeed(Number(event.target.value) / 100)}
              onKeyDown={(event) => {
                if (event.key === "Enter") setSpeed(Number((event.target as HTMLInputElement).value) / 100);
              }}
              className="w-24 bg-studio-950 px-2 py-1 rounded border border-studio-700 text-right"
            />
          </label>
          <div className="flex gap-1" role="group" aria-label="Clip speed">
            {SPEEDS.map((preset) => (
              <button
                key={preset}
                aria-pressed={Math.abs(speed - preset) < 0.0005}
                disabled={busy || track.locked}
                onClick={() => setSpeed(preset)}
                className="flex-1 px-1 py-0.5 rounded border border-studio-700 text-label font-mono text-studio-200 hover:bg-studio-800 aria-pressed:bg-accent/25 aria-pressed:border-accent-hover aria-pressed:text-white disabled:opacity-40"
              >
                {formatSpeed(preset)}
              </button>
            ))}
          </div>
          <p className="text-meta text-studio-500">
            Plays the same media faster or slower{partners > 0 ? ", with what it is linked to" : ""}; sound keeps its pitch.{" "}
            {openedProject.sequence.magnetic
              ? "What follows moves up or along."
              : "Magnetic is off: faster leaves a gap, slower stops at the next clip."}
          </p>
        </div>
      )}
      <label className="flex items-center justify-between text-label text-studio-300">
        <span>Starts at (s)</span>
        <input
          key={`${clip.id}-${clip.startUs}`}
          aria-label="Clip start in seconds"
          type="number"
          min={0}
          step={0.1}
          defaultValue={(clip.startUs / 1e6).toFixed(2)}
          disabled={busy || track.locked}
          onBlur={(event) => setStart(Number(event.target.value))}
          onKeyDown={(event) => {
            if (event.key === "Enter") setStart(Number((event.target as HTMLInputElement).value));
          }}
          className="w-24 bg-studio-950 px-2 py-1 rounded border border-studio-700 text-right"
        />
      </label>
      <p className="text-meta text-studio-500">
        Drag the clip to move it along or between {track.kind} tracks, its edges to trim it. Alt+click picks it without what it is linked to.
      </p>
      <button
        disabled={busy || track.locked}
        onClick={() => {
          setSelectedIds([]);
          void edit({ kind: "delete", clipIds: selectedIds.length ? selectedIds : [clip.id] });
        }}
        className="flex items-center gap-1.5 px-2 py-1 rounded border border-danger/40 text-label text-danger-fg hover:bg-danger/10 disabled:opacity-40"
      >
        <Trash2 className="w-3.5 h-3.5" />
        Remove {selectedIds.length > 1 ? `${selectedIds.length} clips` : "clip"}
      </button>
      {error && (
        <p role="alert" className="text-meta text-danger-fg">
          {error}
        </p>
      )}
    </InspectorSection>
  );
};
