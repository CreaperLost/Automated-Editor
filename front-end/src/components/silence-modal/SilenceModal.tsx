import { useShallow } from "zustand/react/shallow";
import React, { useEffect, useRef, useState } from "react";
import { X, Scissors, AlertCircle, ChevronDown, Play, Loader2 } from "lucide-react";
import { useProjectStore } from "../../stores/projectStore";
import { api } from "../../lib/ipc";
import { seekPlayback, togglePlayback } from "../../lib/playbackControl";
import { OpenedProject, SilenceConfig } from "../../lib/types";
import { soundSources } from "../../lib/sequence";
import { Button, IconButton, Segmented, cn } from "../ui";

/** The sounds worth scanning: those on the timeline (all of them when none is yet). */
function scannableSounds(project: OpenedProject | null) {
  const sounds = soundSources(project);
  const placed = sounds.filter((sound) => sound.placed);
  return placed.length > 0 ? placed : sounds;
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

/** How hard to cut: a gentle pass leaves breathing room, a tight one keeps the pace up. */
const PRESETS = {
  gentle: { label: "Gentle", hint: "Only long pauses, with room around words", config: { thresholdDb: -42, minDurationMs: 800, paddingMs: 100 } },
  balanced: { label: "Balanced", hint: "Pauses of half a second or more", config: { thresholdDb: -38, minDurationMs: 400, paddingMs: 50 } },
  tight: { label: "Tight", hint: "Short pauses too, for a fast pace", config: { thresholdDb: -34, minDurationMs: 250, paddingMs: 30 } },
} as const;
type Preset = keyof typeof PRESETS;

function presetOf(config: SilenceConfig): Preset | "custom" {
  return (
    (Object.keys(PRESETS) as Preset[]).find((key) => {
      const preset = PRESETS[key].config;
      return (
        preset.thresholdDb === config.thresholdDb &&
        preset.minDurationMs === config.minDurationMs &&
        preset.paddingMs === config.paddingMs
      );
    }) ?? "custom"
  );
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
    activeSilenceBlocks,
    setSilenceBlocks,
    toggleSilenceBlock,
    applySilenceCuts,
    openedProject,
    applyOpenedProject,
    silenceAnalysis,
  } = useProjectStore(
    useShallow((s) => ({
      isSilenceModalOpen: s.isSilenceModalOpen,
      setIsSilenceModalOpen: s.setIsSilenceModalOpen,
      activeSilenceBlocks: s.activeSilenceBlocks,
      setSilenceBlocks: s.setSilenceBlocks,
      toggleSilenceBlock: s.toggleSilenceBlock,
      applySilenceCuts: s.applySilenceCuts,
      openedProject: s.openedProject,
      applyOpenedProject: s.applyOpenedProject,
      silenceAnalysis: s.silenceAnalysis,
    })),
  );

  const [config, setConfig] = useState<SilenceConfig>({ ...PRESETS.balanced.config });
  const [advanced, setAdvanced] = useState(false);
  const [isDetecting, setIsDetecting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [diagnostics, setDiagnostics] = useState<string[]>([]);
  const detectGeneration = useRef(0);
  const [chosenTrack, setChosenTrack] = useState<string>();

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
  const audioTrackId =
    chosenTrack && sounds.some((sound) => sound.key === chosenTrack) ? chosenTrack : preferredAudioTrackId(openedProject);

  const handleRunDetection = async (settings: SilenceConfig = config) => {
    if (!openedProject) {
      setError("Open a project to detect silence.");
      return;
    }
    if (!audioTrackId) {
      setError("There is no sound to scan yet: import or record some first.");
      return;
    }
    const analyzedHandle = openedProject.projectHandle;
    const analyzedRevision = openedProject.revision;
    const generation = ++detectGeneration.current;
    setIsDetecting(true);
    setError(null);
    setDiagnostics([]);
    try {
      const result = await api.detectSilence(analyzedHandle, audioTrackId, settings);
      if (generation !== detectGeneration.current) return;
      const current = useProjectStore.getState().openedProject;
      if (
        !current ||
        current.projectHandle !== analyzedHandle ||
        current.revision !== analyzedRevision
      ) {
        return;
      }
      setSilenceBlocks(result.suggestions, {
        projectHandle: analyzedHandle,
        revision: analyzedRevision,
      });
      setDiagnostics(result.diagnostics ?? []);
      if (result.suggestions.length === 0 && (result.diagnostics?.length ?? 0) > 0) {
        setError(result.diagnostics.join(" · "));
      }
    } catch (err) {
      if (generation !== detectGeneration.current) return;
      setSilenceBlocks([]);
      setDiagnostics([]);
      setError(errorMessage(err));
    } finally {
      if (generation === detectGeneration.current) setIsDetecting(false);
    }
  };

  const preset = presetOf(config);
  const choosePreset = (next: Preset) => {
    const settings = { ...config, ...PRESETS[next].config };
    setConfig(settings);
    // Already scanned: show what this preset finds straight away.
    if (activeSilenceBlocks.length > 0 || silenceAnalysis) void handleRunDetection(settings);
  };

  const selectedCount = activeSilenceBlocks.filter((b) => b.selected).length;
  const totalTrimDurationMs = activeSilenceBlocks
    .filter((b) => b.selected)
    .reduce((acc, b) => acc + b.durationMs, 0);
  const allSelected = selectedCount === activeSilenceBlocks.length;
  const selectAll = (selected: boolean) =>
    setSilenceBlocks(
      activeSilenceBlocks.map((block) => ({ ...block, selected })),
      silenceAnalysis ?? undefined,
    );

  /** Plays from a moment before the pause, so you hear what the cut joins. */
  const audition = (startUs: number) => {
    seekPlayback(Math.max(0, startUs - 1_000_000));
    if (!useProjectStore.getState().isPlaying) window.setTimeout(() => togglePlayback(), 60);
  };

  const apply = () => {
    if (!openedProject || !silenceAnalysis) return;
    if (
      silenceAnalysis.projectHandle !== openedProject.projectHandle ||
      silenceAnalysis.revision !== openedProject.revision
    ) {
      setSilenceBlocks([]);
      setError("These pauses were found before a later edit. Scan again.");
      return;
    }
    const cuts = activeSilenceBlocks
      .filter((block) => block.selected)
      .map((block) => ({ startUs: block.startUs, endUs: block.endUs }));
    // Every track loses the same time and closes up, so pictures and other sound stay in step.
    void api
      .projectSequenceEdit(silenceAnalysis.projectHandle, silenceAnalysis.revision, {
        kind: "deleteRange",
        ranges: cuts,
        ripple: true,
      })
      .then((next) => {
        applyOpenedProject(next);
        applySilenceCuts();
        setError(null);
      })
      .catch((err) => setError(errorMessage(err)));
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
            <p className="text-label text-studio-400">Find the pauses and cut them out. Every track stays in step.</p>
          </div>
          <IconButton icon={X} label="Close (Esc)" onClick={() => setIsSilenceModalOpen(false)} />
        </div>

        <div className="px-5 py-4 space-y-4 overflow-y-auto flex-1">
          <div className="space-y-2">
            <Segmented<Preset | "custom">
              label="How much to cut"
              className="w-full"
              value={preset}
              onChange={(next) => next !== "custom" && choosePreset(next)}
              options={(Object.keys(PRESETS) as Preset[]).map((key) => ({
                value: key,
                label: PRESETS[key].label,
                title: PRESETS[key].hint,
              }))}
            />
            <p className="text-meta text-studio-500">
              {preset === "custom" ? "Your own settings (see Advanced)." : PRESETS[preset].hint}.
            </p>
          </div>

          {sounds.length > 1 && (
            <label className="flex items-center justify-between gap-3 text-label text-studio-300">
              <span>Sound to listen to</span>
              <select
                aria-label="Sound to scan"
                value={audioTrackId ?? ""}
                onChange={(e) => {
                  setChosenTrack(e.target.value);
                  setSilenceBlocks([]);
                }}
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
                {slider(
                  "Quieter than",
                  config.thresholdDb,
                  `${config.thresholdDb} dB`,
                  -60,
                  -20,
                  1,
                  (thresholdDb) => setConfig({ ...config, thresholdDb }),
                  "Lower finds only near-silence; higher also counts quiet background as a pause.",
                )}
                {slider(
                  "Pauses longer than",
                  config.minDurationMs,
                  `${(config.minDurationMs / 1000).toFixed(2)} s`,
                  200,
                  1500,
                  50,
                  (minDurationMs) => setConfig({ ...config, minDurationMs }),
                )}
                {slider(
                  "Room kept around words",
                  config.paddingMs,
                  `${config.paddingMs} ms`,
                  20,
                  150,
                  10,
                  (paddingMs) => setConfig({ ...config, paddingMs }),
                  "So the first and last sounds of words are never clipped.",
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
            disabled={isDetecting || !openedProject || !audioTrackId}
          >
            {isDetecting ? "Listening…" : activeSilenceBlocks.length > 0 ? "Find again" : "Find pauses"}
          </Button>

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
                  {selectedCount} of {activeSilenceBlocks.length} pauses selected · {(totalTrimDurationMs / 1000).toFixed(1)}s shorter
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
                      onChange={() => toggleSilenceBlock(block.id)}
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
              Pick how much to cut, then Find pauses. Nothing is cut until you apply.
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
              disabled={selectedCount === 0 || !openedProject || !silenceAnalysis}
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
