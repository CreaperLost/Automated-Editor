import React from "react";
import { AlertCircle, Loader2, Play, VolumeX } from "lucide-react";
import { useProjectStore } from "../../stores/projectStore";
import { noiseId, useNoiseStore } from "../../stores/noiseStore";
import { api } from "../../lib/ipc";
import { seekPlayback, togglePlayback } from "../../lib/playbackControl";
import { soundSources } from "../../lib/sequence";
import type { FoundNoise } from "../../lib/types";
import { Button, IconButton, cn } from "../ui";

const KIND_LABELS: Record<FoundNoise["kind"], string> = { click: "Click", knock: "Pop", mouth: "Mouth noise", breath: "Breath" };

function formatTime(us: number): string {
  const total = us / 1_000_000;
  const minutes = Math.floor(total / 60);
  return `${minutes}:${(total - minutes * 60).toFixed(2).padStart(5, "0")}`;
}

function errorMessage(err: unknown): string {
  if (err instanceof Error && err.message) return err.message;
  if (typeof err === "string" && err.trim()) return err;
  return "Finding noises failed.";
}

/// Jump cuts' first pass: clicks, lip smacks and knocks found in a sound's own audio (word
/// timings miss them), reviewed and muted in place. Muted, they are silence to the passes after.
export const NoisePass: React.FC<{ onClose: () => void; onNext: () => void }> = ({ onClose, onNext }) => {
  const openedProject = useProjectStore((s) => s.openedProject);
  const applyOpenedProject = useProjectStore((s) => s.applyOpenedProject);
  const { sensitivity, paddingMs, key, scan, selected, busy, error, notice } = useNoiseStore();
  const [applying, setApplying] = React.useState(false);

  const sounds = soundSources(openedProject).filter((sound) => sound.placed);
  const soundKey = key && sounds.some((sound) => sound.key === key) ? key : (sounds.find((s) => s.speech) ?? sounds[0])?.key;
  const noises = scan && scan.key === soundKey ? scan.noises : [];
  const chosen = noises.filter((noise) => selected[noiseId(noise)] && !noise.muted);
  const mutedCount = noises.filter((noise) => noise.muted).length;
  const stale = !!scan && !!openedProject && scan.revision !== openedProject.revision;

  const find = async () => {
    if (!openedProject || !soundKey) return;
    const store = useNoiseStore.getState();
    const generation = store.begin();
    try {
      store.finish(generation, await api.detectMouthNoises(openedProject.projectHandle, soundKey, sensitivity, paddingMs));
    } catch (err) {
      store.fail(generation, errorMessage(err));
    }
  };

  const setMuted = async (list: FoundNoise[], muted: boolean) => {
    const project = useProjectStore.getState().openedProject;
    if (!project || !soundKey || list.length === 0 || applying) return;
    setApplying(true);
    try {
      const next = await api.projectNoiseMutes(
        project.projectHandle,
        project.revision,
        soundKey,
        list.map((noise) => ({ startUs: noise.sourceStartUs, endUs: noise.sourceEndUs })),
        !muted,
      );
      applyOpenedProject(next);
      const count = `${list.length} noise${list.length === 1 ? "" : "s"}`;
      useNoiseStore.getState().applied(
        list.map(noiseId),
        muted,
        next.revision,
        muted ? `Muted ${count}. Remove silence next: muted noises count as silence.` : `Unmuted ${count}.`,
      );
    } catch (err) {
      useNoiseStore.setState({ error: errorMessage(err) });
    } finally {
      setApplying(false);
    }
  };

  /** Plays from a moment before the noise, so you hear it in its place. */
  const audition = (noise: FoundNoise) => {
    seekPlayback(Math.max(0, noise.startUs - 700_000));
    if (!useProjectStore.getState().isPlaying) window.setTimeout(() => togglePlayback(), 60);
  };

  return (
    <>
      <div className="px-5 pb-4 space-y-4 overflow-y-auto flex-1">
        <p className="text-meta text-studio-400">
          Clicks, lip smacks, pops and breaths, found in the sound itself, even where a word's timing covers them. Muting silences just
          those moments of this sound: timing and every other track stay as they are. Then remove silence: muted noises count
          as silence, so the pauses around them are cut as one.
        </p>

        {sounds.length > 1 && (
          <label className="flex items-center justify-between gap-3 text-label text-studio-300">
            <span>Sound to clean</span>
            <select
              aria-label="Sound to find noises in"
              value={soundKey ?? ""}
              onChange={(e) => useNoiseStore.getState().configure({ key: e.target.value })}
              className="ui-field min-w-0 max-w-[16rem] truncate"
            >
              {sounds.map((sound) => (
                <option key={sound.key} value={sound.key}>
                  {sound.label}
                </option>
              ))}
            </select>
          </label>
        )}

        <label className="block space-y-1.5">
          <div className="flex justify-between text-label">
            <span className="text-studio-300">Sensitivity</span>
            <span className="font-mono text-meta tabular-nums text-studio-200">{Math.round(sensitivity * 100)}%</span>
          </div>
          <input
            type="range"
            aria-label="Noise sensitivity"
            min={0}
            max={1}
            step={0.05}
            value={sensitivity}
            onChange={(e) => useNoiseStore.getState().configure({ sensitivity: Number(e.target.value) })}
            className="w-full h-1.5 cursor-pointer"
          />
          <p className="text-meta text-studio-500">Higher also finds fainter noises; listen before muting them.</p>
        </label>

        <label className="block space-y-1.5">
          <div className="flex justify-between text-label">
            <span className="text-studio-300">Room around each noise</span>
            <span className="font-mono text-meta tabular-nums text-studio-200">{paddingMs} ms</span>
          </div>
          <input
            type="range"
            aria-label="Room muted around each noise"
            min={0}
            max={100}
            step={5}
            value={paddingMs}
            onChange={(e) => useNoiseStore.getState().configure({ paddingMs: Number(e.target.value) })}
            className="w-full h-1.5 cursor-pointer"
          />
          <p className="text-meta text-studio-500">
            Each noise is muted from where it rises out of the room to where it fades back, plus this. It never reaches into speech.
          </p>
        </label>

        <Button
          variant={noises.length > 0 ? "secondary" : "primary"}
          size="lg"
          icon={busy ? Loader2 : undefined}
          className={cn("w-full", busy && "[&>svg]:animate-spin")}
          onClick={() => void find()}
          disabled={busy || applying || !openedProject || !soundKey}
        >
          {busy ? "Listening…" : scan ? "Find again" : "Find noises"}
        </Button>

        {notice && (
          <div role="status" className="flex items-center justify-between gap-3 rounded-control bg-studio-850 px-3 py-2">
            <span className="text-meta text-studio-300">{notice}</span>
            <Button size="sm" variant="ghost" onClick={onNext}>
              Remove silence
            </Button>
          </div>
        )}
        {stale && <p className="text-meta text-studio-500">The timeline changed since: times may be off. Find again to update them.</p>}

        {error && (
          <div role="alert" className="px-3 py-2 rounded-control bg-danger/10 border border-danger/30 flex items-start gap-2 text-label text-danger-fg">
            <AlertCircle className="w-4 h-4 shrink-0 mt-0.5" />
            <span>{error}</span>
          </div>
        )}
        {scan && scan.diagnostics.length > 0 && !error && (
          <div className="px-3 py-2 rounded-control bg-suggest/10 border border-suggest/30 text-label text-suggest-fg space-y-1">
            {scan.diagnostics.map((item) => (
              <p key={item}>{item}</p>
            ))}
          </div>
        )}

        {noises.length > 0 && (
          <div className="space-y-2">
            <div className="flex items-center justify-between gap-3">
              <span className="text-label font-medium text-suggest-fg">
                {chosen.length} of {noises.length - mutedCount} chosen{mutedCount > 0 ? ` · ${mutedCount} muted` : ""}
              </span>
              <div className="flex gap-1">
                {mutedCount > 0 && (
                  <Button variant="ghost" size="sm" disabled={applying} onClick={() => void setMuted(noises.filter((n) => n.muted), false)}>
                    Unmute all
                  </Button>
                )}
                <Button variant="ghost" size="sm" onClick={() => useNoiseStore.getState().selectAll(chosen.length < noises.length - mutedCount)}>
                  {chosen.length < noises.length - mutedCount ? "Choose all" : "Choose none"}
                </Button>
              </div>
            </div>
            <ul className="space-y-1 max-h-64 overflow-y-auto pr-1">
              {noises.map((noise) => {
                const id = noiseId(noise);
                const on = !!selected[id] && !noise.muted;
                return (
                  <li
                    key={id}
                    className={cn(
                      "flex items-center gap-3 rounded-control border px-3 h-10 transition-colors",
                      on ? "bg-studio-850 border-studio-700" : "bg-transparent border-studio-800 text-studio-500",
                    )}
                  >
                    <input
                      type="checkbox"
                      checked={on}
                      disabled={noise.muted}
                      onChange={() => useNoiseStore.getState().toggle(id)}
                      aria-label={`Mute the ${KIND_LABELS[noise.kind].toLowerCase()} at ${formatTime(noise.startUs)}`}
                      className="w-4 h-4 cursor-pointer"
                    />
                    <span className="font-mono text-label tabular-nums text-studio-200">{formatTime(noise.startUs)}</span>
                    <span className="text-label text-studio-300">{KIND_LABELS[noise.kind]}</span>
                    <span className="font-mono text-meta tabular-nums text-studio-500">
                      {Math.round((noise.sourceEndUs - noise.sourceStartUs) / 1000)} ms
                    </span>
                    <span className="flex-1 text-meta text-studio-500 truncate">
                      {noise.muted ? "Muted" : noise.nearSpeech ? "May be speech: listen first" : ""}
                    </span>
                    <IconButton icon={Play} size="sm" label="Listen: play from just before it" onClick={() => audition(noise)} />
                  </li>
                );
              })}
            </ul>
          </div>
        )}

        {scan && noises.length === 0 && !busy && !error && (
          <p className="text-label text-studio-500">No noises found at this sensitivity.</p>
        )}
      </div>

      <div className="px-5 py-3 border-t border-studio-800 bg-studio-900 flex items-center justify-between gap-3">
        <span className="text-meta text-studio-500">Undo brings them back.</span>
        <div className="flex items-center gap-2">
          <Button variant="ghost" onClick={onClose}>
            Close
          </Button>
          <Button variant="primary" icon={VolumeX} disabled={busy || applying || chosen.length === 0} onClick={() => void setMuted(chosen, true)}>
            Mute {chosen.length} noise{chosen.length === 1 ? "" : "s"}
          </Button>
        </div>
      </div>
    </>
  );
};
