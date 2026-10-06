import React, { useState } from "react";
import { Layers, Trash2 } from "lucide-react";
import { InspectorSection } from "./InspectorSection";
import { useProjectStore } from "../../stores/projectStore";
import { api } from "../../lib/ipc";
import { assetById, clipName, clipRole, findClip, streamOf, trackNumber } from "../../lib/sequence";
import type { Fit, OpenedProject, Role, SequenceEdit } from "../../lib/types";

const FITS: { value: Fit; label: string }[] = [
  { value: "contain", label: "Fit inside the frame" },
  { value: "cover", label: "Fill the frame (crop edges)" },
];

const PICTURE_ROLES: { value: Role; label: string }[] = [
  { value: "screen", label: "Screen: canvas layout, zooms, cursor" },
  { value: "webcam", label: "Camera: in the bubble" },
  { value: "overlay", label: "Overlay: over the whole canvas" },
];
const SOUND_ROLES: { value: Role; label: string }[] = [
  { value: "mic", label: "Speech: transcribed and captioned" },
  { value: "background", label: "Background: music, game, desktop" },
];

/// The clip selected on the timeline: what its stream is, how it fills the frame, where it
/// starts, and removing it. Every change is one undoable edit.
export const TrackClipSection: React.FC = () => {
  const openedProject = useProjectStore((s) => s.openedProject);
  const applyOpenedProject = useProjectStore((s) => s.applyOpenedProject);
  const selectedIds = useProjectStore((s) => s.selectedClipIds);
  const setSelectedIds = useProjectStore((s) => s.setSelectedClipIds);
  const [error, setError] = useState<string>();
  const [busy, setBusy] = useState(false);

  const found = openedProject && selectedIds[0] ? findClip(openedProject.sequence, selectedIds[0]) : undefined;
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

  return (
    <InspectorSection id="track-clip" title={`Clip · ${trackNumber(openedProject.sequence, track.id)}`} icon={Layers}>
      <div className="text-label text-studio-200 truncate" title={asset?.path}>
        {clipName(openedProject, clip)}
        <span className="ml-1.5 text-studio-500 font-mono">{(clip.durationUs / 1e6).toFixed(2)}s</span>
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
