import React, { useEffect, useState } from "react";
import { Captions } from "lucide-react";
import { InspectorSection, RangeRow } from "../inspector/InspectorSection";
import { PresetMarks } from "../presets/PresetMarks";
import { useDebouncedSave } from "../../hooks/useDebouncedSave";
import { soundSources } from "../../lib/sequence";
import { useProjectStore } from "../../stores/projectStore";
import { api } from "../../lib/ipc";
import { CaptionSettings, CaptionStyle, DEFAULT_CAPTION_SETTINGS } from "../../lib/types";

export const CAPTION_STYLES: [CaptionStyle, string, string][] = [
  ["pop", "Pop in", "Each word pops in as it is said"],
  ["karaoke", "Karaoke", "The whole caption shows dim, and each word lights up as it is said"],
  ["static", "Static", "The whole caption shows at once and stays still"],
];

export const CAPTION_WEIGHTS: [number, string][] = [
  [0, "The look's"],
  [500, "Medium"],
  [600, "Semibold"],
  [700, "Bold"],
  [800, "Extra bold"],
];

export const Toggle: React.FC<{ label: string; hint?: string; checked: boolean; onChange: (checked: boolean) => void }> = ({
  label,
  hint,
  checked,
  onChange,
}) => (
  <label className="flex items-center justify-between text-label cursor-pointer">
    <span className="text-studio-300" title={hint}>
      {label}
    </span>
    <input
      type="checkbox"
      checked={checked}
      onChange={(e) => onChange(e.target.checked)}
      className="rounded bg-studio-800 border-studio-700 focus:ring-0 cursor-pointer"
    />
  </label>
);

export const ColorRow: React.FC<{ label: string; value: string; onChange: (value: string) => void }> = ({
  label,
  value,
  onChange,
}) => (
  <label className="flex items-center justify-between text-label text-studio-400">
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
  const styleSave = useDebouncedSave(250);

  // Follow undo/redo and project switches while no edit is pending.
  const savedKey = JSON.stringify(saved);
  useEffect(() => {
    if (!styleSave.isPending()) setDraft(saved);
  }, [savedKey]);

  if (!openedProject) return null;

  const audioTracks = soundSources(openedProject);

  const update = (patch: Partial<CaptionSettings>) => {
    const next = { ...draft, ...patch };
    setDraft(next);
    styleSave.schedule((opened) => {
      void api
        .projectCaptionsUpdate(opened.projectHandle, opened.revision, next)
        .then((updated) => {
          applyOpenedProject(updated);
          setError(undefined);
        })
        .catch((err) => setError(String(err)));
    });
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
          className="rounded bg-studio-800 border-studio-700 focus:ring-0 cursor-pointer"
        />
      }
    >
      {error && (
        <p role="alert" className="text-meta text-danger-fg bg-danger/10 border border-danger/30 rounded p-2">
          {error}
        </p>
      )}
      <PresetMarks part="captions" />
      <p className="text-meta text-studio-500">
        Captions come from the transcript, so transcribe a track first. Cut words disappear from them and fixed words
        show corrected.
      </p>
      <fieldset disabled={!draft.enabled} className="space-y-4 disabled:opacity-50">
        {audioTracks.length > 1 && (
          <label className="flex items-center justify-between text-label text-studio-400">
            <span>Sound</span>
            <select
              value={draft.trackId ?? ""}
              onChange={(e) => update({ trackId: e.target.value || undefined })}
              className="min-w-0 max-w-[15rem] truncate bg-studio-800 text-studio-100 text-label rounded px-1.5 py-0.5 border border-studio-700 focus:outline-none focus:border-accent-hover"
            >
              <option value="">Automatic</option>
              {audioTracks.map((t) => (
                <option key={t.key} value={t.key}>
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
              className={`py-1 rounded text-label capitalize border ${
                draft.position === position
                  ? "bg-accent/30 border-accent-hover text-white"
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
          value={draft.maxLines ?? 1}
          min={1}
          max={3}
          unit=""
          onChange={(maxLines) => update({ maxLines })}
        />
        <p className="text-meta text-studio-500 leading-relaxed">
          How they look (pop, karaoke or still, colours, outline, box, font) is the project&apos;s style: the Style
          page.
        </p>
        <Toggle label="ALL CAPS" checked={draft.uppercase} onChange={(uppercase) => update({ uppercase })} />
      </fieldset>
    </InspectorSection>
  );
};
