import { useShallow } from "zustand/react/shallow";
import React, { useEffect, useState } from "react";
import { X, Scissors, AlertCircle, ChevronDown, Play, Loader2 } from "lucide-react";
import { useProjectStore } from "../../stores/projectStore";
import { useJumpCutStore } from "../../stores/jumpCutStore";
import { useWorkspaceStore } from "../../stores/workspaceStore";
import { api } from "../../lib/ipc";
import { seekPlayback, togglePlayback } from "../../lib/playbackControl";
import { OpenedProject } from "../../lib/types";
import { SILENCE_PRESETS as PRESETS, GAP_PRESETS, JumpCutPreset as Preset, JumpCutSettings, gapSettings, presetOf } from "../../lib/jumpCutConfig";
import { soundSources } from "../../lib/sequence";
import { Button, IconButton, Segmented, cn } from "../ui";

/** Scans every sound the timeline plays as speech at once (the backend's `ALL_SPEECH`). */
const ALL_SPEECH = "speech";

/**
 * The sounds worth scanning: those on the timeline (all of them when none is yet). With speech
 * from more than one source (a recording and an imported video), first all of it together: a
 * pause is where every speech playing is silent.
 */
function scannableSounds(project: OpenedProject | null) {
  const sounds = soundSources(project);
  const placed = sounds.filter((sound) => sound.placed);
  const list = placed.length > 0 ? placed : sounds;
  if (placed.filter((sound) => sound.speech).length < 2) return list;
  return [{ key: ALL_SPEECH, label: "All speech on the timeline", speech: true, placed: true }, ...list];
}

/** Speech on the timeline first, else any sound there. */
function preferredAudioTrackId(project: OpenedProject | null): string | undefined {
  return scannableSounds(project)[0]?.key;
}

function errorMessage(err: unknown): string {
  if (err instanceof Error && err.message) return err.message;
  if (typeof err === "string" && err.trim()) return err;
  return "Silence detection failed.";
}

function formatTime(us: number): string {
  const total = us / 1_000_000;
  const minutes = Math.floor(total / 60);
  const seconds = total - minutes * 60;
  return `${minutes}:${seconds.toFixed(1).padStart(4, "0")}`;
}

/// Jump cuts: find the pauses in a sound, review them, and cut the chosen ones from every
/// track at once (one undoable edit).
export const SilenceModal: React.FC = () => {
  const {
    isSilenceModalOpen,
    setIsSilenceModalOpen,
    openedProject,
    applyOpenedProject,
  } = useProjectStore(
    useShallow((s) => ({
      isSilenceModalOpen: s.isSilenceModalOpen,
      setIsSilenceModalOpen: s.setIsSilenceModalOpen,
      openedProject: s.openedProject,
      applyOpenedProject: s.applyOpenedProject,
    })),
  );

  const { activePass, review, silenceConfig, gapConfig, configureSilence, configureGaps, chooseSources, setActivePass } = useJumpCutStore(useShallow(s => ({
    activePass: s.activePass, review: s.reviews[s.activePass], silenceConfig: s.silenceConfig, gapConfig: s.gapConfig,
    configureSilence: s.configureSilence, configureGaps: s.configureGaps, chooseSources: s.chooseSources, setActivePass: s.setActivePass,
  })));
  const transcriptAssisted = activePass === "nonSpeech";
  const config: JumpCutSettings = transcriptAssisted ? { ...silenceConfig, ...gapConfig } : silenceConfig;
  const presets = transcriptAssisted ? GAP_PRESETS : PRESETS;
  const { blocks: activeSilenceBlocks, snapshot: silenceAnalysis, busy: isDetecting, error, diagnostics, thresholds, sourceMode, chosenTrack, secondTrack, notice } = review;
  const [advanced, setAdvanced] = useState(false);
  const [isApplying, setIsApplying] = useState(false);

  useEffect(() => {
    if (!isSilenceModalOpen) return;
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") setIsSilenceModalOpen(false);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [isSilenceModalOpen, setIsSilenceModalOpen]);

  if (!isSilenceModalOpen) return null;

  const sounds = scannableSounds(openedProject);
  const individualSounds = sounds.filter((sound) => sound.key !== ALL_SPEECH);
  const twoSources = sourceMode === "two" && individualSounds.length >= 2;
  const firstSounds = twoSources ? (transcriptAssisted ? [...individualSounds].sort((a, b) => Number(b.speech) - Number(a.speech)) : individualSounds) : sounds;
  const audioTrackId =
    chosenTrack && firstSounds.some((sound) => sound.key === chosenTrack) ? chosenTrack :
      twoSources ? firstSounds[0]?.key : preferredAudioTrackId(openedProject);
  const secondSounds = individualSounds.filter((sound) => sound.key !== audioTrackId);
  const secondAudioTrackId = secondSounds.some((sound) => sound.key === secondTrack) ? secondTrack : secondSounds[0]?.key;

  const changeConfig = (settings: JumpCutSettings) => {
    if (transcriptAssisted) configureGaps(gapSettings(settings));
    else configureSilence(settings);
  };

  const handleRunDetection = async (settings: JumpCutSettings = config) => {
    if (!openedProject || !audioTrackId) return;
    const analyzedHandle = openedProject.projectHandle;
    const analyzedRevision = openedProject.revision;
    const analyzedCaptionsVersion = useProjectStore.getState().captionsVersion;
    const store = useJumpCutStore.getState();
    const generation = store.beginScan(activePass);
    try {
      const ids = twoSources && secondAudioTrackId ? [audioTrackId, secondAudioTrackId] : undefined;
      const result = transcriptAssisted
        ? await api.detectNonSpeechGaps(analyzedHandle, audioTrackId, gapSettings(settings), ids)
        : await api.detectSilence(analyzedHandle, audioTrackId, settings, ids);
      store.finishScan(activePass, generation, result, {
        projectHandle: analyzedHandle,
        revision: analyzedRevision,
        captionsVersion: analyzedCaptionsVersion,
        transcriptDependencies: result.transcriptDependencies,
      });
    } catch (err) {
      store.failScan(activePass, generation, errorMessage(err));
    }
  };

  const preset = presetOf(config, transcriptAssisted);
  const choosePreset = (next: Preset) => {
    const settings = { ...config, ...presets[next].config, autoLevel: undefined };
    const rescan = activeSilenceBlocks.length > 0 || silenceAnalysis !== null || isDetecting;
    changeConfig(settings);
    // Already scanned: show what this preset finds straight away.
    if (rescan) void handleRunDetection(settings);
  };

  const selectedCount = activeSilenceBlocks.filter((b) => b.selected).length;
  const totalTrimDurationMs = activeSilenceBlocks
    .filter((b) => b.selected)
    .reduce((acc, b) => acc + b.durationMs, 0);
  const allSelected = selectedCount === activeSilenceBlocks.length;
  const selectAll = (selected: boolean) =>
    useJumpCutStore.getState().select(activePass, null, selected);

  /** Plays from a moment before the pause, so you hear what the cut joins. */
  const audition = (startUs: number) => {
    seekPlayback(Math.max(0, startUs - 1_000_000));
    if (!useProjectStore.getState().isPlaying) window.setTimeout(() => togglePlayback(), 60);
  };

  const apply = () => {
    if (!openedProject || !silenceAnalysis || isDetecting || isApplying) return;
    if (
      silenceAnalysis.projectHandle !== openedProject.projectHandle ||
      silenceAnalysis.revision !== openedProject.revision || silenceAnalysis.captionsVersion !== useProjectStore.getState().captionsVersion
    ) {
      useJumpCutStore.getState().invalidate("These suggestions were found before a later change. Find again.");
      return;
    }
    const cuts = activeSilenceBlocks
      .filter((block) => block.selected)
      .map((block) => ({ startUs: block.startUs, endUs: block.endUs }));
    // Every track loses the same time and closes up, so pictures and other sound stay in step.
    if (!cuts.length) return;
    setIsApplying(true);
    void api
      .applyJumpCuts(silenceAnalysis.projectHandle, silenceAnalysis.revision, cuts, silenceAnalysis.transcriptDependencies)
      .then((next) => {
        applyOpenedProject(next);
        setIsSilenceModalOpen(false);
      })
      .catch((err) => {
        const store = useJumpCutStore.getState();
        store.invalidate("Find suggestions again after the latest change.");
        store.failScan(activePass, store.reviews[activePass].generation, errorMessage(err));
      }).finally(() => setIsApplying(false));
  };

  const slider = (
    label: string,
    value: number,
    shown: string,
    min: number,
    max: number,
    step: number,
    onChange: (value: number) => void,
    hint?: string,
  ) => (
    <label className="block space-y-1.5">
      <div className="flex justify-between text-label">
        <span className="text-studio-300">{label}</span>
        <span className="font-mono text-meta tabular-nums text-studio-200">{shown}</span>
      </div>
      <input
        type="range"
        min={min}
        max={max}
        step={step}
        value={value}
        onChange={(e) => onChange(Number(e.target.value))}
        className="w-full h-1.5 cursor-pointer"
      />
      {hint && <p className="text-meta text-studio-500">{hint}</p>}
    </label>
  );

  return (
    <div
      className="fixed inset-0 z-50 bg-black/70 flex items-center justify-center p-4 select-none"
      onMouseDown={(event) => {
        if (event.target === event.currentTarget) setIsSilenceModalOpen(false);
      }}
    >
      <div
        role="dialog"
        aria-modal="true"
        aria-label="Jump cuts"
        className="w-full max-w-xl bg-studio-900 border border-studio-700 rounded-panel shadow-dialog overflow-hidden flex flex-col max-h-[88vh]"
      >
        <div className="px-5 py-4 border-b border-studio-800 flex items-start justify-between gap-4">
          <div>
            <h2 className="text-heading text-studio-100">Jump cuts</h2>
            <p className="text-label text-studio-400">Remove silence, then gaps without words. Each pass also works on its own.</p>
          </div>
          <IconButton icon={X} label="Close (Esc)" onClick={() => setIsSilenceModalOpen(false)} />
        </div>

        <div className="px-5 py-4 space-y-4 overflow-y-auto flex-1">
          <div className="space-y-2">
            <Segmented<"silence" | "nonSpeech">
              label="Cleanup pass"
              value={activePass}
              onChange={setActivePass}
              options={[
                { value: "silence", label: "1. Remove silence" },
                { value: "nonSpeech", label: "2. Gaps without words" },
              ]}
            />
            {transcriptAssisted && (
              <p className="text-meta text-studio-400">
                Use saved transcripts to find gaps containing breaths, mouth noises, or clicks, regardless of loudness.
                {twoSources ? " Transcribe the primary source first. The protected source is checked for any sound, even without a transcript." : " Transcribe the chosen source first."}
                {" Listen and select suggestions before applying."}
              </p>
            )}
            {!transcriptAssisted && <p className="text-meta text-studio-400">Find very quiet parts from the audio. Saved word timings add protection when available; transcription is optional.</p>}
            <div className="flex items-center justify-between gap-3 rounded-control bg-studio-850 px-3 py-2">
              <span className="text-meta text-studio-400">3. Edit words, fillers, and retakes in Transcript.</span>
              <Button size="sm" variant="ghost" onClick={() => { setIsSilenceModalOpen(false); useWorkspaceStore.getState().setWorkspace("cleanup"); }}>Edit words</Button>
            </div>
          </div>
          <div className="space-y-2">
            <Segmented<Preset | "custom">
              label="How much to cut"
              className="w-full"
              value={preset}
              onChange={(next) => next !== "custom" && choosePreset(next)}
              options={(Object.keys(presets) as Preset[]).map((key) => ({
                value: key,
                label: presets[key].label,
                title: presets[key].hint,
              }))}
            />
            <p className="text-meta text-studio-500">
              {preset === "custom" ? "Your own settings (see Advanced)" : presets[preset].hint}.
            </p>
          </div>

          {individualSounds.length > 1 && (
            <div className="space-y-2">
              <Segmented<"one" | "two">
                label="Sources to listen to"
                value={sourceMode}
                onChange={(sourceMode) => chooseSources(activePass, { sourceMode })}
                options={[{ value: "one", label: "One source" }, { value: "two", label: "Two sources" }]}
              />
              {twoSources && <p className="text-meta text-studio-400">
                {transcriptAssisted ? "Find gaps in the primary transcript. Keep speech, music, video audio, and other sounds on the protected source." : "Keep either voice. Cut only when both chosen sources are quiet."}
              </p>}
            </div>
          )}

          {sounds.length > 1 && (
            <label className="flex items-center justify-between gap-3 text-label text-studio-300">
              <span>{twoSources ? transcriptAssisted ? "Primary source (text)" : "First source" : "Sound to listen to"}</span>
              <select
                aria-label="Sound to scan"
                value={audioTrackId ?? ""}
                onChange={(e) => {
                  chooseSources(activePass, { chosenTrack: e.target.value });
                }}
                className="ui-field min-w-0 max-w-[16rem] truncate"
              >
                {firstSounds.map((sound) => (
                  <option key={sound.key} value={sound.key}>
                    {sound.label}
                  </option>
                ))}
              </select>
            </label>
          )}

          {twoSources && (
            <label className="flex items-center justify-between gap-3 text-label text-studio-300">
              <span>{transcriptAssisted ? "Protected source (audio)" : "Second source"}</span>
              <select aria-label="Second sound to scan" value={secondAudioTrackId ?? ""}
                onChange={(e) => chooseSources(activePass, { secondTrack: e.target.value })}
                className="ui-field min-w-0 max-w-[16rem] truncate">
                {secondSounds.map((sound) => <option key={sound.key} value={sound.key}>{sound.label}</option>)}
              </select>
            </label>
          )}

          <div className="rounded-control border border-studio-800">
            <button
              type="button"
              aria-expanded={advanced}
              onClick={() => setAdvanced(!advanced)}
              className="w-full flex items-center gap-2 h-10 px-3 text-label text-studio-300 hover:text-studio-100"
            >
              <ChevronDown className={cn("w-4 h-4 transition-transform", !advanced && "-rotate-90")} />
              Advanced
            </button>
            {advanced && (
              <div className="px-3 pb-3 space-y-4">
                {!transcriptAssisted && <label className="flex items-center gap-2 text-label text-studio-300">
                  <input type="checkbox" checked={config.autoLevel !== undefined}
                    onChange={(e) => changeConfig({ ...config, autoLevel: e.target.checked ? 0.8 : undefined })} />
                  Adapt to each source’s volume
                </label>}
                {!transcriptAssisted && (config.autoLevel !== undefined ? slider(
                  "Pause sensitivity", config.autoLevel, `${Math.round(config.autoLevel * 100)}%`,
                  0, 1, 0.05, (autoLevel) => changeConfig({ ...config, autoLevel }),
                  "Higher removes louder pauses and may cut soft speech. Saved words add protection when available.",
                ) : slider(
                  "Quieter than",
                  config.thresholdDb,
                  `${config.thresholdDb} dB`,
                  -60,
                  -20,
                  1,
                  (thresholdDb) => changeConfig({ ...config, thresholdDb }),
                  "Lower finds only near-silence; higher also counts quiet background as a pause.",
                ))}
                {transcriptAssisted && <label className="flex items-center gap-2 text-label text-studio-300">
                  <input type="checkbox" checked={config.refineWordEdges ?? false}
                    onChange={e => changeConfig({ ...config, refineWordEdges: e.target.checked })} />
                  Refine quiet word edges
                </label>}
                {transcriptAssisted && config.refineWordEdges && slider(
                  "Word edge threshold", config.edgeThresholdDb ?? -42, `${config.edgeThresholdDb ?? -42} dB`,
                  -70, -20, 1, edgeThresholdDb => changeConfig({ ...config, edgeThresholdDb }),
                  "Trims measured quiet tails at word boundaries for this pass. Higher may trim soft consonants. Saved transcript timings stay unchanged.",
                )}
                {slider(
                  transcriptAssisted ? "Gaps longer than" : "Quiet parts longer than",
                  config.minDurationMs,
                  `${(config.minDurationMs / 1000).toFixed(2)} s`,
                  20,
                  1500,
                  10,
                  (minDurationMs) => changeConfig({ ...config, minDurationMs }),
                )}
                {slider(
                  "Room kept around words",
                  config.paddingMs,
                  `${config.paddingMs} ms`,
                  0,
                  300,
                  5,
                  (paddingMs) => changeConfig({ ...config, paddingMs }),
                  "Less room makes tighter cuts. Review word endings, especially at zero.",
                )}
              </div>
            )}
          </div>

          <Button
            variant={activeSilenceBlocks.length > 0 ? "secondary" : "primary"}
            size="lg"
            icon={isDetecting ? Loader2 : undefined}
            className={cn("w-full", isDetecting && "[&>svg]:animate-spin")}
            onClick={() => void handleRunDetection()}
            disabled={isDetecting || isApplying || !openedProject || !audioTrackId}
          >
            {isDetecting ? "Analyzing…" : silenceAnalysis ? "Find again" : transcriptAssisted ? "Find gaps" : "Find silence"}
          </Button>

          {!transcriptAssisted && thresholds.length > 0 && <div className="text-meta text-studio-400 space-y-1">
            {thresholds.map(level => <p key={level.trackId}>
              {sounds.find(sound => sound.key === level.trackId)?.label ?? level.trackId}: measured cutoff {level.thresholdDb.toFixed(1)} dB
            </p>)}
          </div>}

          {notice && <p role="status" className="text-label text-studio-400">{notice}</p>}

          {error && (
            <div role="alert" className="px-3 py-2 rounded-control bg-danger/10 border border-danger/30 flex items-start gap-2 text-label text-danger-fg">
              <AlertCircle className="w-4 h-4 shrink-0 mt-0.5" />
              <span>{error}</span>
            </div>
          )}

          {diagnostics.length > 0 && !error && (
            <div className="px-3 py-2 rounded-control bg-suggest/10 border border-suggest/30 text-label text-suggest-fg space-y-1">
              {diagnostics.map((item) => (
                <p key={item}>{item}</p>
              ))}
            </div>
          )}

          {activeSilenceBlocks.length > 0 && (
            <div className="space-y-2">
              <div className="flex items-center justify-between gap-3">
                <span className="text-label font-medium text-suggest-fg">
                  {selectedCount} of {activeSilenceBlocks.length} {transcriptAssisted ? "gaps" : "quiet parts"} selected · {(totalTrimDurationMs / 1000).toFixed(1)}s shorter
                </span>
                <Button variant="ghost" size="sm" onClick={() => selectAll(!allSelected)}>
                  {allSelected ? "Select none" : "Select all"}
                </Button>
              </div>

              <ul className="space-y-1 max-h-64 overflow-y-auto pr-1">
                {activeSilenceBlocks.map((block) => (
                  <li
                    key={block.id}
                    className={cn(
                      "flex items-center gap-3 rounded-control border px-3 h-11 transition-colors",
                      block.selected ? "bg-studio-850 border-studio-700" : "bg-transparent border-studio-800 text-studio-500",
                    )}
                  >
                    <input
                      type="checkbox"
                      checked={block.selected}
                      onChange={() => useJumpCutStore.getState().select(activePass, block.id)}
                      aria-label={`Cut the pause at ${formatTime(block.startUs)}`}
                      className="w-4 h-4 cursor-pointer"
                    />
                    <span className="flex-1 font-mono text-label tabular-nums text-studio-200">
                      {formatTime(block.startUs)} – {formatTime(block.endUs)}
                    </span>
                    <span className="font-mono text-meta tabular-nums text-studio-400">
                      {(block.durationMs / 1000).toFixed(1)}s
                    </span>
                    <IconButton
                      icon={Play}
                      size="sm"
                      label="Listen: play from just before the pause"
                      onClick={() => audition(block.startUs)}
                    />
                  </li>
                ))}
              </ul>
            </div>
          )}

          {activeSilenceBlocks.length === 0 && !isDetecting && !error && (
            <p className="text-label text-studio-500">
              {silenceAnalysis ? "No matching intervals found with these settings." : "Choose settings and find suggestions. Nothing is cut until you apply."}
            </p>
          )}
        </div>

        <div className="px-5 py-3 border-t border-studio-800 bg-studio-900 flex items-center justify-between gap-3">
          <span className="text-meta text-studio-500">Undo brings everything back.</span>
          <div className="flex items-center gap-2">
            <Button variant="ghost" onClick={() => setIsSilenceModalOpen(false)}>
              Cancel
            </Button>
            <Button
              variant="primary"
              icon={Scissors}
              disabled={isDetecting || isApplying || selectedCount === 0 || !openedProject || !silenceAnalysis}
              onClick={apply}
            >
              Apply {selectedCount} cut{selectedCount === 1 ? "" : "s"}
            </Button>
          </div>
        </div>
      </div>
    </div>
  );
};
