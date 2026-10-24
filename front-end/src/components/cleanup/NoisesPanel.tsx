import React from "react";
import { AlertCircle, ChevronDown, Loader2, Play, Volume2, VolumeX } from "lucide-react";
import { useProjectStore } from "../../stores/projectStore";
import { noiseId, useNoiseStore, type NoiseFilter } from "../../stores/noiseStore";
import { useJumpCutStore } from "../../stores/jumpCutStore";
import { api } from "../../lib/ipc";
import { seekPlayback, togglePlayback } from "../../lib/playbackControl";
import { soundSources } from "../../lib/sequence";
import type { FoundNoise, NoiseKind } from "../../lib/types";
import { Button, IconButton, Segmented, cn } from "../ui";
import { requestPanel } from "../layout/dockLayout";
import { saveCleanupSoon, useCleanupFromProject } from "../../lib/cleanupSettings";

const KIND_LABELS: Record<NoiseKind, string> = { click: "Click", knock: "Pop", mouth: "Mouth noise", breath: "Breath" };

const kindLabel = (noise: FoundNoise) => (noise.kind ? KIND_LABELS[noise.kind] : "Muted moment");

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

const isFound = (n: FoundNoise) => !n.muted && n.kind !== null && !n.nearSpeech;
const isNear = (n: FoundNoise) => !n.muted && n.kind !== null && n.nearSpeech;
const FILTERS: Record<NoiseFilter, (n: FoundNoise) => boolean> = {
  found: isFound,
  near: isNear,
  muted: (n) => n.muted,
  all: () => true,
};

/// Mouth noises: clicks, lip smacks, pops and breaths found in a sound's own audio (word timings
/// miss them), reviewed and muted in place. Muted, they are silence to the jump cuts after. A
/// dock panel, so the preview and the timeline stay usable while reviewing; the last scan is kept
/// with the project, so what an agent found and muted shows here too.
export const NoisesPanel: React.FC = () => {
  const openedProject = useProjectStore((s) => s.openedProject);
  const applyOpenedProject = useProjectStore((s) => s.applyOpenedProject);
  const { sensitivity, paddingMs, key, scan, selected, filter, busy, error, notice } = useNoiseStore();
  // The sensitivity starts from the project's (its preset's unless changed there).
  useCleanupFromProject(openedProject);
  const [applying, setApplying] = React.useState(false);
  const [settingsOpen, setSettingsOpen] = React.useState(false);

  const sounds = soundSources(openedProject).filter((sound) => sound.placed);
  const soundKey = key && sounds.some((sound) => sound.key === key) ? key : (sounds.find((s) => s.speech) ?? sounds[0])?.key;
  const handle = openedProject?.projectHandle;
  const revision = openedProject?.revision;

  // The kept scan, where the timeline plays it now: read again whenever the project changes.
  React.useEffect(() => {
    if (!handle || !soundKey) return;
    let stale = false;
    api
      .noiseReview(handle, soundKey)
      .then((next) => {
        if (!stale) useNoiseStore.getState().reviewed(next);
      })
      .catch((err) => {
        if (!stale) useNoiseStore.setState({ error: errorMessage(err) });
      });
    return () => {
      stale = true;
    };
  }, [handle, soundKey, revision]);

  const noises = scan && scan.key === soundKey ? scan.noises : [];
  const shown = noises.filter(FILTERS[filter]);
  const counts = {
    found: noises.filter(isFound).length,
    near: noises.filter(isNear).length,
    muted: noises.filter((n) => n.muted).length,
  };
  const chosen = noises.filter((noise) => selected[noiseId(noise)] && !noise.muted && noise.kind !== null);
  const choosable = shown.filter((n) => !n.muted && n.kind !== null);
  const allShownChosen = choosable.length > 0 && choosable.every((n) => selected[noiseId(n)]);
  const mutedMs = noises
    .filter((n) => n.muted)
    .reduce((sum, n) => sum + n.parts.reduce((s, p) => s + (p.endUs - p.startUs), 0), 0) / 1000;

  // Only the place under the playhead re-renders the list as it plays.
  const playingId = useProjectStore((s) => {
    const at = s.currentTimeUs;
    const hit = noises.find((n) => n.startUs <= at && at < n.endUs);
    return hit ? noiseId(hit) : null;
  });

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
        list.flatMap((noise) => noise.parts),
        !muted,
      );
      applyOpenedProject(next);
      const count = `${list.length} place${list.length === 1 ? "" : "s"}`;
      useNoiseStore.getState().say(muted ? `Muted ${count}. Remove silence next: muted noises count as silence.` : `Unmuted ${count}.`);
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

  const slider = (label: string, aria: string, value: number, shownValue: string, min: number, max: number, step: number,
    onChange: (value: number) => void, hint: string) => (
    <label className="block space-y-1.5">
      <div className="flex justify-between text-label">
        <span className="text-studio-300">{label}</span>
        <span className="font-mono text-meta tabular-nums text-studio-200">{shownValue}</span>
      </div>
      <input type="range" aria-label={aria} min={min} max={max} step={step} value={value}
        onChange={(e) => onChange(Number(e.target.value))} className="w-full h-1.5 cursor-pointer" />
      <p className="text-meta text-studio-500">{hint}</p>
    </label>
  );

  if (!openedProject) return <p className="p-3 text-label text-studio-500">Open a project to find mouth noises.</p>;

  return (
    <div className="h-full flex flex-col min-h-0 bg-studio-900/95 select-none">
      <div className="shrink-0 p-3 space-y-3 border-b border-studio-800">
        {sounds.length > 1 && (
          <label className="flex items-center justify-between gap-3 text-label text-studio-300">
            <span>Sound</span>
            <select
              aria-label="Sound to find noises in"
              value={soundKey ?? ""}
              onChange={(e) => useNoiseStore.getState().chooseSound(e.target.value)}
              className="ui-field min-w-0 max-w-[14rem] truncate"
            >
              {sounds.map((sound) => (
                <option key={sound.key} value={sound.key}>
                  {sound.label}
                </option>
              ))}
            </select>
          </label>
        )}

        <div className="flex items-center gap-2">
          <Button
            variant={noises.length > 0 ? "secondary" : "primary"}
            icon={busy ? Loader2 : undefined}
            className={cn("flex-1", busy && "[&>svg]:animate-spin")}
            onClick={() => void find()}
            disabled={busy || applying || !soundKey}
            title="Listen to the whole sound for clicks, lip smacks, pops and breaths"
          >
            {busy ? "Listening…" : scan?.config ? "Find again" : "Find noises"}
          </Button>
          <IconButton
            icon={ChevronDown}
            label={settingsOpen ? "Hide the settings" : "Settings: sensitivity and room"}
            active={settingsOpen}
            onClick={() => setSettingsOpen(!settingsOpen)}
            className={cn("transition-transform", !settingsOpen && "-rotate-90")}
          />
        </div>

        {settingsOpen && (
          <div className="space-y-3">
            {slider("Sensitivity", "Noise sensitivity", sensitivity, `${Math.round(sensitivity * 100)}%`, 0, 1, 0.05,
              (value) => {
                useNoiseStore.getState().configure({ sensitivity: value });
                saveCleanupSoon();
              },
              "Higher also finds fainter noises; listen before muting them.")}
            {slider("Room around each noise", "Room muted around each noise", paddingMs, `${paddingMs} ms`, 0, 100, 5,
              (value) => useNoiseStore.getState().configure({ paddingMs: value }),
              "Each noise is muted from where it rises out of the room to where it fades, plus this. Never into a word.")}
            <p className="text-meta text-studio-500">Settings apply to the next Find.</p>
          </div>
        )}

        {scan?.config && (
          <p className="text-meta text-studio-500">
            Last scan at {Math.round(scan.config.sensitivity * 100)}% sensitivity, {scan.config.paddingMs} ms room
            {scan.guarded ? "; the transcript's words are never muted." : "."} Noises close together are one place.
          </p>
        )}
        {scan?.config && !scan.guarded && (
          <p className="text-meta text-suggest-fg">
            This sound had no transcript then, so nothing guarded the words. Transcribe it and find again.
          </p>
        )}
        {!scan?.config && noises.length === 0 && (
          <p className="text-meta text-studio-500">
            Found in the sound itself, even where a word's timing covers them, and muted in place: timing and every other track
            stay. Transcribe first: the transcript's words are never muted.
          </p>
        )}

        {notice && (
          <div role="status" className="flex items-center justify-between gap-2 rounded-control bg-studio-850 px-3 py-2">
            <span className="text-meta text-studio-300">{notice}</span>
            <Button
              size="sm"
              variant="ghost"
              onClick={() => {
                useJumpCutStore.getState().setActivePass("silence");
                requestPanel("jumpCuts");
              }}
            >
              Jump cuts
            </Button>
          </div>
        )}
        {error && (
          <div role="alert" className="px-3 py-2 rounded-control bg-danger/10 border border-danger/30 flex items-start gap-2 text-label text-danger-fg">
            <AlertCircle className="w-4 h-4 shrink-0 mt-0.5" />
            <span className="flex-1">{error}</span>
            <button type="button" className="text-meta underline" onClick={() => useNoiseStore.setState({ error: null })}>
              Dismiss
            </button>
          </div>
        )}
        {scan && scan.diagnostics.length > 0 && !error && (
          <div className="px-3 py-2 rounded-control bg-suggest/10 border border-suggest/30 text-meta text-suggest-fg space-y-1">
            {scan.diagnostics.map((item) => (
              <p key={item}>{item}</p>
            ))}
          </div>
        )}

        {noises.length > 0 && (
          <Segmented<NoiseFilter>
            label="Places shown"
            size="sm"
            className="w-full [&>button]:flex-1"
            value={filter}
            onChange={(next) => useNoiseStore.getState().setFilter(next)}
            options={[
              { value: "found", label: `Clear ${counts.found}`, title: "Clear of words: chosen to mute" },
              { value: "near", label: `Near words ${counts.near}`, title: "Right next to speech: may be part of a word, listen first" },
              { value: "muted", label: `Muted ${counts.muted}`, title: `Muted now (${(mutedMs / 1000).toFixed(1)} s in all)` },
              { value: "all", label: "All" },
            ]}
          />
        )}
      </div>

      <ul className="flex-1 min-h-0 overflow-y-auto p-2 space-y-1" aria-label="Mouth noises">
        {noises.length > 0 && shown.length === 0 && (
          <li className="px-1 text-meta text-studio-500">
            {filter === "muted" ? "Nothing is muted." : filter === "near" ? "None right next to words." : "Nothing left here."}
          </li>
        )}
        {scan?.config && noises.length === 0 && !busy && (
          <li className="px-1 text-meta text-studio-500">No noises found at this sensitivity.</li>
        )}
        {shown.map((noise) => {
          const id = noiseId(noise);
          const on = !!selected[id] && !noise.muted;
          const ms = Math.round((noise.sourceEndUs - noise.sourceStartUs) / 1000);
          return (
            <li
              key={id}
              onClick={() => seekPlayback(noise.startUs)}
              className={cn(
                "flex items-center gap-2 rounded-control border px-2 h-9 cursor-pointer transition-colors",
                playingId === id ? "border-accent/60 bg-accent/10" : on ? "bg-studio-850 border-studio-700" : "bg-transparent border-studio-800",
                noise.muted && "text-studio-500",
              )}
              title={noise.nearSpeech && !noise.muted ? "Right next to speech: it may be part of a word. Listen first." : undefined}
            >
              <input
                type="checkbox"
                checked={on}
                disabled={noise.muted || noise.kind === null}
                onClick={(e) => e.stopPropagation()}
                onChange={() => useNoiseStore.getState().toggle(id)}
                aria-label={`Choose the ${kindLabel(noise).toLowerCase()} at ${formatTime(noise.startUs)}`}
                className="w-4 h-4 cursor-pointer shrink-0"
              />
              <span className="font-mono text-meta tabular-nums text-studio-200">{formatTime(noise.startUs)}</span>
              <span className={cn("min-w-0 flex-1 truncate text-label", noise.muted ? "text-studio-500" : "text-studio-300")}>
                {kindLabel(noise)}
                {noise.count > 1 && <span className="text-studio-500"> ×{noise.count}</span>}
                {noise.nearSpeech && !noise.muted && <span className="text-suggest-fg"> · near words</span>}
              </span>
              <span className="font-mono text-meta tabular-nums text-studio-500 shrink-0">{ms} ms</span>
              <span className="flex shrink-0" onClick={(e) => e.stopPropagation()}>
                <IconButton icon={Play} size="sm" label="Listen: play from just before it" onClick={() => audition(noise)} />
                {noise.muted ? (
                  <IconButton icon={Volume2} size="sm" label="Unmute it" disabled={applying} onClick={() => void setMuted([noise], false)} />
                ) : (
                  <IconButton icon={VolumeX} size="sm" label="Mute it" disabled={applying} onClick={() => void setMuted([noise], true)} />
                )}
              </span>
            </li>
          );
        })}
      </ul>

      {noises.length > 0 && (
        <div className="shrink-0 px-3 py-2 border-t border-studio-800 flex items-center gap-2">
          {filter === "muted" ? (
            <Button variant="ghost" size="sm" disabled={applying || shown.length === 0} onClick={() => void setMuted(shown, false)}>
              Unmute all {shown.length}
            </Button>
          ) : (
            <Button
              variant="ghost"
              size="sm"
              disabled={choosable.length === 0}
              onClick={() => useNoiseStore.getState().selectAll(choosable.map(noiseId), !allShownChosen)}
            >
              {allShownChosen ? "Choose none" : "Choose all"}
            </Button>
          )}
          <span className="flex-1" />
          <Button variant="primary" size="sm" icon={VolumeX} disabled={busy || applying || chosen.length === 0} onClick={() => void setMuted(chosen, true)}>
            Mute {chosen.length}
          </Button>
        </div>
      )}
    </div>
  );
};
