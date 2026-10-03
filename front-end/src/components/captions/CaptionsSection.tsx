import React, { useEffect, useRef, useState } from "react";
import { Captions } from "lucide-react";
import { InspectorSection, RangeRow } from "../inspector/InspectorSection";
import { transcribableSounds } from "../../lib/trackUtils";
import { useProjectStore } from "../../stores/projectStore";
import { api } from "../../lib/ipc";
import { CaptionSettings, DEFAULT_CAPTION_SETTINGS } from "../../lib/types";

const Toggle: React.FC<{ label: string; hint?: string; checked: boolean; onChange: (checked: boolean) => void }> = ({
  label,
  hint,
  checked,
  onChange,
}) => (
  <label className="flex items-center justify-between text-xs cursor-pointer">
    <span className="text-studio-300" title={hint}>
      {label}
    </span>
    <input
      type="checkbox"
      checked={checked}
      onChange={(e) => onChange(e.target.checked)}
      className="rounded bg-studio-800 border-studio-700 text-indigo-600 focus:ring-0 cursor-pointer"
    />
  </label>
);

const ColorRow: React.FC<{ label: string; value: string; onChange: (value: string) => void }> = ({
  label,
  value,
  onChange,
}) => (
  <label className="flex items-center justify-between text-xs text-studio-400">
    <span>{label}</span>
    <input
      type="color"
      value={value.toLowerCase()}
      onChange={(e) => onChange(e.target.value.toUpperCase())}
      className="w-12 h-6 bg-studio-850 border border-studio-800 rounded cursor-pointer"
    />
  </label>
);

/** Caption style. Saved to the project, so playback and export both draw it. */
export const CaptionsSection: React.FC = () => {
  const openedProject = useProjectStore((s) => s.openedProject);
  const applyOpenedProject = useProjectStore((s) => s.applyOpenedProject);
  const saved = { ...DEFAULT_CAPTION_SETTINGS, ...openedProject?.captions };
  const [draft, setDraft] = useState<CaptionSettings>(saved);
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

  const audioTracks = transcribableSounds(openedProject);

  const update = (patch: Partial<CaptionSettings>) => {
    const next = { ...draft, ...patch };
    setDraft(next);
    window.clearTimeout(timer.current);
    timer.current = window.setTimeout(() => {
      timer.current = undefined;
      const opened = openedRef.current;
      if (!opened) return;
      void api
        .projectCaptionsUpdate(opened.projectHandle, opened.revision, next)
        .then((updated) => {
          applyOpenedProject(updated);
          setError(undefined);
        })
        .catch((err) => setError(String(err)));
    }, 250);
  };

  return (
    <InspectorSection
      id="captions"
      title="Captions"
      icon={Captions}
      extra={
        <input
          type="checkbox"
          aria-label="Show captions"
          title="Show captions in the preview and export"
          checked={draft.enabled}
          onChange={(e) => update({ enabled: e.target.checked })}
          className="rounded bg-studio-800 border-studio-700 text-indigo-600 focus:ring-0 cursor-pointer"
        />
      }
    >
      {error && (
        <p role="alert" className="text-[11px] text-rose-300 bg-rose-950/40 border border-rose-900/40 rounded p-2">
          {error}
        </p>
      )}
      <p className="text-[11px] text-studio-500">
        Captions come from the transcript, so transcribe a track first. Cut words disappear from them and fixed words
        show corrected.
      </p>
      <fieldset disabled={!draft.enabled} className="space-y-4 disabled:opacity-50">
        {audioTracks.length > 1 && (
          <label className="flex items-center justify-between text-xs text-studio-400">
            <span>Track</span>
            <select
              value={draft.trackId ?? ""}
              onChange={(e) => update({ trackId: e.target.value || undefined })}
              className="min-w-0 max-w-[15rem] truncate bg-studio-800 text-studio-100 text-xs rounded px-1.5 py-0.5 border border-studio-700 focus:outline-none focus:border-teal-500"
            >
              <option value="">Automatic</option>
              {audioTracks.map((t) => (
                <option key={t.id} value={t.id}>
                  {t.label}
                </option>
              ))}
            </select>
          </label>
        )}
        <div className="grid grid-cols-3 gap-1">
          {(["top", "middle", "bottom"] as const).map((position) => (
            <button
              key={position}
              type="button"
              onClick={() => update({ position })}
              className={`py-1 rounded text-xs capitalize border ${
                draft.position === position
                  ? "bg-indigo-600/30 border-indigo-500 text-white"
                  : "bg-studio-850 border-studio-800 text-studio-400 hover:text-white"
              }`}
            >
              {position}
            </button>
          ))}
        </div>
        {draft.position !== "middle" && (
          <RangeRow
            label="Distance from edge"
            value={draft.offsetPct}
            min={0}
            max={45}
            unit="%"
            onChange={(offsetPct) => update({ offsetPct })}
          />
        )}
        <RangeRow
          label="Text size"
          value={draft.fontSizePct}
          min={2}
          max={15}
          step={0.5}
          unit="%"
          onChange={(fontSizePct) => update({ fontSizePct })}
        />
        <RangeRow
          label="Words at once"
          value={draft.maxWords}
          min={1}
          max={12}
          unit=""
          onChange={(maxWords) => update({ maxWords })}
        />
        <RangeRow
          label="Lines at most"
          value={draft.maxLines ?? 3}
          min={1}
          max={3}
          unit=""
          onChange={(maxLines) => update({ maxLines })}
        />
        <ColorRow label="Text color" value={draft.textColor} onChange={(textColor) => update({ textColor })} />
        <Toggle
          label="Highlight the spoken word"
          checked={draft.highlightWords}
          onChange={(highlightWords) => update({ highlightWords })}
        />
        {draft.highlightWords && (
          <ColorRow
            label="Highlight color"
            value={draft.highlightColor}
            onChange={(highlightColor) => update({ highlightColor })}
          />
        )}
        <Toggle label="Outline" checked={draft.outline} onChange={(outline) => update({ outline })} />
        <Toggle label="Background box" checked={draft.background} onChange={(background) => update({ background })} />
        {draft.background && (
          <>
            <ColorRow
              label="Box color"
              value={draft.backgroundColor}
              onChange={(backgroundColor) => update({ backgroundColor })}
            />
            <RangeRow
              label="Box opacity"
              value={Math.round(draft.backgroundOpacity * 100)}
              min={0}
              max={100}
              unit="%"
              onChange={(pct) => update({ backgroundOpacity: pct / 100 })}
            />
          </>
        )}
        <Toggle label="ALL CAPS" checked={draft.uppercase} onChange={(uppercase) => update({ uppercase })} />
      </fieldset>
    </InspectorSection>
  );
};
