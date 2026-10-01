import React, { useEffect, useRef, useState } from "react";
import { Volume2 } from "lucide-react";
import { useProjectStore } from "../../stores/projectStore";
import { api } from "../../lib/ipc";
import { AudioSettings, DEFAULT_AUDIO_SETTINGS } from "../../lib/types";

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
          className="flex-1 accent-indigo-500 h-1.5 bg-studio-800 rounded-lg cursor-pointer"
        />
        <span className="w-16 text-right font-mono text-[11px] text-studio-300">{valueLabel}</span>
      </div>
    )}
  </div>
);

/** Audio polish switches. Saved to the project, so playback and export both use them. */
export const AudioPanel: React.FC = () => {
  const openedProject = useProjectStore((s) => s.openedProject);
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

  useEffect(() => () => window.clearTimeout(timer.current), []);

  if (!openedProject) return null;

  const update = (patch: Partial<AudioSettings>) => {
    const next = { ...draft, ...patch };
    setDraft(next);
    window.clearTimeout(timer.current);
    timer.current = window.setTimeout(() => {
      timer.current = undefined;
      const opened = openedRef.current;
      if (!opened) return;
      void api
        .projectAudioUpdate(opened.projectHandle, opened.revision, next)
        .then((updated) => {
          applyOpenedProject(updated);
          setError(undefined);
        })
        .catch((err) => setError(String(err)));
    }, 250);
  };

  return (
    <div className="shrink-0 border-t border-studio-800 bg-studio-900/95 p-5 space-y-4 select-none">
      <div className="flex items-center space-x-2 text-xs font-semibold uppercase tracking-wider text-studio-400">
        <Volume2 className="w-3.5 h-3.5 text-indigo-400" />
        <span>Audio Polish</span>
      </div>
      {error && (
        <p role="alert" className="text-[11px] text-rose-300 bg-rose-950/40 border border-rose-900/40 rounded p-2">
          {error}
        </p>
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
      <EffectRow
        label="Reduce mic noise"
        hint="Lowers steady background noise (fans, hum, hiss) on the microphone."
        enabled={draft.noiseReduction}
        onToggle={(noiseReduction) => update({ noiseReduction })}
        valueLabel={`-${draft.noiseReductionDb} dB`}
        min={3}
        max={30}
        value={draft.noiseReductionDb}
        onValue={(noiseReductionDb) => update({ noiseReductionDb })}
      />
      <EffectRow
        label="Lower system audio when speaking"
        hint="Ducks game or desktop audio while the microphone picks up speech."
        enabled={draft.duckSystemAudio}
        onToggle={(duckSystemAudio) => update({ duckSystemAudio })}
        valueLabel={`-${draft.duckDb} dB`}
        min={3}
        max={30}
        value={draft.duckDb}
        onValue={(duckDb) => update({ duckDb })}
      />
    </div>
  );
};
