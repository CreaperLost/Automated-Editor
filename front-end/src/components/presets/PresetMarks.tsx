import React from "react";
import { RotateCcw } from "lucide-react";
import { useProjectStore } from "../../stores/projectStore";
import { api } from "../../lib/ipc";
import type { PresetPart, Short, ShortLayout } from "../../lib/types";
import { changeLayout } from "../../stores/shortsStore";
import { usePresets } from "./usePresets";

/** "fontSizePct" → "font size pct": a field's name as words. */
const words = (field: string) =>
  field
    .replace(/([a-z0-9])([A-Z])/g, "$1 $2")
    .toLowerCase()
    .replace(/\bpct\b/, "%")
    .replace(/\bms\b/, "(ms)");

const same = (a: unknown, b: unknown) => JSON.stringify(a) === JSON.stringify(b);

const chip =
  "inline-flex items-center gap-1 rounded-full border border-accent/40 bg-accent/10 px-1.5 py-0.5 text-accent-fg hover:bg-accent/20";

/// What a short's look changed from the look its preset starts shorts with (the shorts preset
/// for a short made from a take, the clips preset for a clip): each as a chip that takes it
/// back for this short only. Its hand framing and graphic places are its own and not listed.
export const ShortPresetMarks: React.FC<{ short: Short; layout: ShortLayout }> = ({ short, layout }) => {
  const openedProject = useProjectStore((s) => s.openedProject);
  const presets = usePresets(openedProject?.projectHandle, openedProject?.revision);
  const kind = short.take ? "shorts" : "clips";
  const look = presets?.shortLooks?.[kind];
  const preset = kind === "shorts" ? presets?.shortsPreset : presets?.clipsPreset;
  if (!openedProject || !look || !preset) return null;
  const current = layout as unknown as Record<string, unknown>;
  // A switch an older short never wrote is off.
  const changed = Object.keys(look).filter(
    (field) => !same(current[field] ?? (typeof look[field] === "boolean" ? false : undefined), look[field]),
  );
  if (changed.length === 0) {
    return <p className="text-meta text-studio-500">As the {preset} preset.</p>;
  }
  const back = (fields: string[]) =>
    changeLayout(Object.fromEntries(fields.map((f) => [f, look[f]])) as Partial<ShortLayout>);
  return (
    <div className="flex flex-wrap items-center gap-1 text-meta" aria-label="Changed from the preset">
      <span className="text-studio-400">Changed here:</span>
      {changed.map((field) => (
        <button
          key={field}
          type="button"
          onClick={() => back([field])}
          title={`Back to the ${preset} preset's ${words(field)} for this short: ${JSON.stringify(look[field])}`}
          className={chip}
        >
          {words(field)}
          <RotateCcw className="h-3 w-3" />
        </button>
      ))}
      {changed.length > 1 && (
        <button type="button" className="text-studio-400 hover:text-white hover:underline" onClick={() => back(changed)}>
          all back
        </button>
      )}
    </div>
  );
};

/// What this part of the edit changed from the project's preset: each value changed here as a
/// chip (click to take it back to the preset's), or "as the preset" when nothing is.
export const PresetMarks: React.FC<{ part: PresetPart; fields?: string[] }> = ({ part, fields }) => {
  const openedProject = useProjectStore((s) => s.openedProject);
  const applyOpenedProject = useProjectStore((s) => s.applyOpenedProject);
  const presets = usePresets(openedProject?.projectHandle, openedProject?.revision);
  if (!openedProject || !presets?.projectPreset) return null;
  const changed = Object.keys(presets.changes[part] ?? {}).filter((f) => !fields || fields.includes(f));
  const reset = (field?: string) => {
    const project = useProjectStore.getState().openedProject;
    if (!project) return;
    void api
      .projectPresetReset(project.projectHandle, project.revision, part, field)
      .then(applyOpenedProject)
      .catch(() => undefined);
  };
  if (changed.length === 0) {
    return <p className="text-meta text-studio-500">As the {presets.projectPreset} preset.</p>;
  }
  return (
    <div className="flex flex-wrap items-center gap-1 text-meta">
      <span className="text-studio-400">Changed here:</span>
      {changed.map((field) => (
        <button
          key={field}
          type="button"
          onClick={() => reset(field)}
          title={`Back to the ${presets.projectPreset} preset's ${words(field)}: ${JSON.stringify(
            presets.inherited[part]?.[field] ?? "the default",
          )}`}
          className={chip}
        >
          {words(field)}
          <RotateCcw className="h-3 w-3" />
        </button>
      ))}
      {changed.length > 1 && !fields && (
        <button type="button" className="text-studio-400 hover:text-white hover:underline" onClick={() => reset()}>
          all back
        </button>
      )}
    </div>
  );
};
