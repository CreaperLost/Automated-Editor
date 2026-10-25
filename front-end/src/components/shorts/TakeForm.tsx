import React, { useMemo, useState } from "react";
import { FolderOpen, Video } from "lucide-react";
import { api } from "../../lib/ipc";
import type { Asset, OpenedProject } from "../../lib/types";
import { useProjectStore } from "../../stores/projectStore";
import { newShortFromTake, setShortsError, useShortsStore } from "../../stores/shortsStore";
import { Button } from "../ui";

type Option = { key: string; label: string; asset: Asset; role: string };

/** Every picture (or sound) stream of the project's media that can be a take's camera (voice). */
function streams(project: OpenedProject, kind: "picture" | "sound"): Option[] {
  return project.assets
    .filter((asset) => !asset.missing && asset.kind !== "image" && asset.kind !== "graphic")
    .flatMap((asset) =>
      asset.streams
        .filter((stream) => stream.kind === kind)
        .map((stream) => ({
          key: `${asset.id}.${stream.id}`,
          label: asset.kind === "recording" ? `${asset.name} · ${stream.name}` : asset.name,
          asset,
          role: stream.role,
        })),
    );
}

/** The newest of `options`: media imported last comes last. */
const newest = (options: Option[]) => options[options.length - 1];

/// Makes a short from a take: a new camera recording (with its own microphone or a separate
/// one) that retells part of the video. The take talks; footage from the video goes under it.
export const TakeForm: React.FC<{ onDone: () => void }> = ({ onDone }) => {
  const project = useProjectStore((s) => s.openedProject);
  const busy = useShortsStore((s) => s.busy);
  const [importing, setImporting] = useState(false);
  const cameras = useMemo(() => (project ? streams(project, "picture") : []), [project]);
  // Music and sound effects (from the libraries) are never a take's voice.
  const sounds = useMemo(
    () => (project ? streams(project, "sound").filter((s) => s.role !== "music" && s.role !== "effects") : []),
    [project],
  );
  // Unset: worked out (see below).
  const [camera, setCamera] = useState<string>();
  const [voice, setVoice] = useState<string>();
  const [title, setTitle] = useState("");
  const [clean, setClean] = useState(true);
  if (!project) return null;

  // A recording's camera, or a video file (its picture comes in as a screen until it is a take).
  const takeCameras = cameras.filter((c) => c.asset.kind !== "recording" || c.role === "webcam");
  const cameraKey = camera ?? newest(takeCameras)?.key ?? "";
  const cameraAsset = cameraKey.split(".")[0];
  // The voice, as the app tells a take's files apart: a sound-only file imported with or after
  // the camera (a separate microphone records better), else a recording's microphone, else the
  // camera's own sound.
  const cameraIndex = project.assets.findIndex((a) => a.id === cameraAsset);
  const separate = newest(
    sounds.filter(
      (s) =>
        !s.asset.streams.some((x) => x.kind === "picture") &&
        project.assets.findIndex((a) => a.id === s.asset.id) > cameraIndex,
    ),
  );
  const ownSound =
    sounds.find((s) => s.asset.id === cameraAsset && s.role === "mic") ?? sounds.find((s) => s.asset.id === cameraAsset);
  const voiceKey = voice ?? separate?.key ?? ownSound?.key ?? newest(sounds)?.key ?? "";

  /** Imports what `pick` chooses (files, or a .aero recording folder) and makes it the take. */
  const importTake = async (pick: () => Promise<string[]>) => {
    setImporting(true);
    try {
      const paths = await pick();
      if (paths.length === 0) return;
      const before = new Set(project.assets.map((a) => a.id));
      const current = useProjectStore.getState().openedProject ?? project;
      const next = await api.projectMediaImport(current.projectHandle, current.revision, paths);
      await useProjectStore.getState().applyOpenedProject(next);
      // What was just imported becomes the take.
      const added = next.assets.filter((a) => !before.has(a.id));
      const video = added.find((a) => a.streams.some((s) => s.kind === "picture"));
      const audio = added.find((a) => !a.streams.some((s) => s.kind === "picture") && a.streams.some((s) => s.kind === "sound"));
      // A recording's camera is its webcam; a video file's its picture.
      const picture = video?.streams.find((s) => s.role === "webcam") ?? video?.streams.find((s) => s.kind === "picture");
      if (video && picture) setCamera(`${video.id}.${picture.id}`);
      if (audio) setVoice(`${audio.id}.${audio.streams.find((s) => s.kind === "sound")!.id}`);
      else setVoice(undefined);
    } catch (err) {
      setShortsError(err);
    } finally {
      setImporting(false);
    }
  };

  const make = async () => {
    if (await newShortFromTake(cameraKey || null, voiceKey || null, title.trim(), clean)) onDone();
  };

  const select = (value: string, onChange: (v: string) => void, options: Option[], label: string, none?: string) => (
    <select aria-label={label} value={value} onChange={(e) => onChange(e.target.value)} className="ui-field w-full !h-control-sm !text-meta">
      {none !== undefined && <option value="">{none}</option>}
      {options.map((o) => (
        <option key={o.key} value={o.key}>
          {o.label}
        </option>
      ))}
    </select>
  );

  return (
    <div className="space-y-2 rounded-control border border-accent/40 bg-accent/5 p-2.5 text-meta" aria-label="Short from a take">
      <p className="text-studio-300 leading-relaxed">
        A take you recorded for this short: the camera and your voice. Footage from the video plays under it.
      </p>
      <div className="flex gap-1.5">
        <Button
          size="sm"
          variant="ghost"
          icon={FolderOpen}
          className="flex-1"
          disabled={importing || !!busy}
          onClick={() => void importTake(() => api.pickMediaFiles())}
          title="A camera video, and a separate microphone file if you used one"
        >
          {importing ? "Importing…" : "Import files…"}
        </Button>
        <Button
          size="sm"
          variant="ghost"
          icon={Video}
          className="flex-1"
          disabled={importing || !!busy}
          onClick={() =>
            void importTake(async () => {
              const folder = await api.pickRecordingFolder();
              return folder ? [folder] : [];
            })
          }
          title="An AeroShoot recording (.aero folder): its camera and microphone together"
        >
          Import a recording…
        </Button>
      </div>
      <label className="block space-y-1">
        <span className="text-studio-400">Camera</span>
        {select(cameraKey, setCamera, cameras, "Take camera", "No camera (voice only)")}
      </label>
      <label className="block space-y-1">
        <span className="text-studio-400">Voice</span>
        {select(voiceKey, setVoice, sounds, "Take voice")}
        <span className="block text-studio-500">
          {voiceKey && voiceKey.split(".")[0] !== cameraAsset && cameraKey
            ? "A separate microphone: lined up with the camera's own sound."
            : "The camera's own sound."}
        </span>
      </label>
      <label className="flex items-start gap-2 text-studio-300">
        <input type="checkbox" checked={clean} onChange={(e) => setClean(e.target.checked)} className="mt-0.5 w-4 h-4" />
        <span>Clean it after transcribing: cut the pauses between words and mute mouth noises (undoable)</span>
      </label>
      <input
        aria-label="Short title (also the file name)"
        placeholder="Title (also the file name)"
        value={title}
        maxLength={100}
        onChange={(e) => setTitle(e.target.value)}
        className="ui-field w-full !h-control-sm !text-meta"
      />
      <div className="flex gap-1.5">
        <Button size="sm" variant="ghost" className="flex-1" onClick={onDone}>
          Cancel
        </Button>
        <Button
          size="sm"
          variant="primary"
          icon={Video}
          className="flex-1"
          disabled={!!busy || !voiceKey}
          onClick={() => void make()}
        >
          {busy === "Making the short" ? "Making…" : "Make short"}
        </Button>
      </div>
    </div>
  );
};
