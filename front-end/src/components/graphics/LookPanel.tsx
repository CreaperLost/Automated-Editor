import React, { useCallback, useEffect, useState } from "react";
import { Eye, Loader2, RotateCcw, Save, Trash2, Wand2 } from "lucide-react";
import { Button, IconButton, Segmented } from "../ui";
import { useProjectStore } from "../../stores/projectStore";
import { api, isTauriEnvironment } from "../../lib/ipc";
import {
  DEFAULT_SHORTS_LOOK,
  LOOK_CONTROLS,
  SHORTS_LOOK_CONTROLS,
  type GraphicsLook,
  type InLinePlan,
  type LookStill,
  type LookView,
  type SavedLook,
} from "../../lib/types";

const same = (a: GraphicsLook, b: GraphicsLook) => JSON.stringify(a) === JSON.stringify(b);
const fieldClass =
  "w-full min-w-0 bg-studio-950 text-studio-100 text-label rounded px-2 py-1 border border-studio-700 focus:outline-none focus:border-accent-hover";

const Slider: React.FC<{
  label: string;
  hint: string;
  min: number;
  max: number;
  value: number;
  onChange: (value: number) => void;
}> = ({ label, hint, min, max, value, onChange }) => (
  <label className="flex items-center gap-2 text-label text-studio-300" title={hint}>
    <span className="w-[4.5rem] shrink-0">{label}</span>
    <input
      type="range"
      aria-label={label}
      min={min}
      max={max}
      step={0.05}
      value={value}
      onChange={(e) => onChange(Number(e.target.value))}
      onDoubleClick={() => onChange(1)}
      className="flex-1 min-w-0 h-1.5 cursor-pointer"
    />
    <span className="w-11 text-right font-mono text-meta tabular-nums text-studio-200">×{value.toFixed(2)}</span>
  </label>
);

/// The video's look: one style every graphic in it takes (type, corners, spacing, motion), with
/// the brand kit's colours. A preset and five controls change all of them at once; Preview draws
/// a few of the video's own graphics with the change before Apply makes it (one undo step).
/// Shorts take it times their own multiples; looks can be saved for every project; graphics
/// that decide their own styles can be brought in line.
export const LookPanel: React.FC<{ onError: (error: string) => void; hasGraphics: boolean; ready: boolean }> = ({
  onError,
  hasGraphics,
  ready,
}) => {
  const openedProject = useProjectStore((s) => s.openedProject);
  const applyOpenedProject = useProjectStore((s) => s.applyOpenedProject);
  const handle = openedProject?.projectHandle;
  const revision = openedProject?.revision;
  const [view, setView] = useState<LookView>();
  const [draft, setDraft] = useState<GraphicsLook>();
  const [stills, setStills] = useState<LookStill[]>();
  const [previewing, setPreviewing] = useState(false);
  const [saved, setSaved] = useState<SavedLook[]>([]);
  // The saved look picked, and the name being typed to save one.
  const [picked, setPicked] = useState("");
  const [saveName, setSaveName] = useState<string>();
  const [plans, setPlans] = useState<InLinePlan[]>([]);
  const [showPlans, setShowPlans] = useState(false);

  const load = useCallback(() => {
    if (!handle || !isTauriEnvironment()) return;
    void api
      .graphicsLook(handle)
      .then((loaded) => {
        setView(loaded);
        setDraft(loaded.look);
        setSaved(loaded.saved);
        setStills(undefined);
      })
      .catch(() => undefined);
    void api
      .graphicsInLinePlan(handle)
      .then(setPlans)
      .catch(() => setPlans([]));
  }, [handle]);
  // The look is in the edit, so it follows undo too.
  useEffect(load, [load, revision]);

  if (!openedProject || !view || !draft) return null;
  const changed = !same(draft, view.look);
  const shorts = draft.shorts ?? DEFAULT_SHORTS_LOOK;
  // From the latest draft: two changes before a render both count.
  const change = (patch: Partial<GraphicsLook>) => {
    setDraft((was) => (was ? { ...was, ...patch } : was));
    setStills(undefined);
  };

  const preview = async () => {
    setPreviewing(true);
    try {
      setStills(await api.graphicsLookPreview(openedProject.projectHandle, draft));
    } catch (err) {
      onError(String(err));
    } finally {
      setPreviewing(false);
    }
  };
  const apply = async () => {
    const project = useProjectStore.getState().openedProject;
    if (!project) return;
    try {
      applyOpenedProject(await api.projectGraphicsLookSet(project.projectHandle, project.revision, draft));
    } catch (err) {
      onError(String(err));
    }
  };
  const pickSaved = (name: string) => {
    setPicked(name);
    const look = saved.find((s) => s.name === name)?.look;
    if (look) change(look);
  };
  const save = async () => {
    const name = saveName?.trim();
    if (!name) return;
    try {
      const after = await api.graphicsLookSave(name, draft);
      setSaved(after);
      // Saving over one keeps its name as first written.
      setPicked(after.find((s) => s.name.toLowerCase() === name.toLowerCase())?.name ?? name);
      setSaveName(undefined);
    } catch (err) {
      onError(String(err));
    }
  };
  const removeSaved = async () => {
    try {
      setSaved(await api.graphicsLookSavedRemove(picked));
      setPicked("");
    } catch (err) {
      onError(String(err));
    }
  };
  const bringInLine = async () => {
    const project = useProjectStore.getState().openedProject;
    if (!project) return;
    try {
      applyOpenedProject(await api.projectGraphicsInLine(project.projectHandle, project.revision, plans));
      setShowPlans(false);
    } catch (err) {
      onError(String(err));
    }
  };
  const preset = view.presets.find((p) => p.name === draft.preset);
  const asItComes = {
    preset: draft.preset,
    size: 1,
    roundness: 1,
    speed: 1,
    energy: 1,
    density: 1,
    shorts: DEFAULT_SHORTS_LOOK,
  };

  return (
    <section className="space-y-2">
      <div className="flex items-center gap-1 px-1">
        <h3 className="flex-1 text-meta font-semibold uppercase tracking-wide text-studio-500">Look</h3>
        <IconButton
          icon={RotateCcw}
          size="sm"
          label="Back to the preset as it comes"
          disabled={same(draft, asItComes)}
          onClick={() => change({ ...asItComes, tokens: undefined })}
        />
      </div>
      <div className="flex items-center gap-1 px-1">
        {saveName === undefined ? (
          <>
            <select
              aria-label="Saved looks"
              value={picked}
              onChange={(e) => pickSaved(e.target.value)}
              className={fieldClass}
              title="Looks you saved, for every project"
            >
              <option value="">{saved.length ? "Saved looks…" : "No saved looks yet"}</option>
              {saved.map((s) => (
                <option key={s.name} value={s.name}>
                  {s.name}
                </option>
              ))}
            </select>
            <IconButton
              icon={Save}
              size="sm"
              label="Save this look for every project"
              onClick={() => setSaveName(picked)}
            />
            {picked && (
              <IconButton
                icon={Trash2}
                size="sm"
                label="Remove this saved look (projects keep the look they have)"
                onClick={() => void removeSaved()}
              />
            )}
          </>
        ) : (
          <>
            <input
              aria-label="Name of the saved look"
              placeholder="Name, like My channel"
              value={saveName}
              autoFocus
              maxLength={40}
              onChange={(e) => setSaveName(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") void save();
                if (e.key === "Escape") setSaveName(undefined);
              }}
              className={fieldClass}
            />
            <Button size="sm" variant="ghost" onClick={() => setSaveName(undefined)}>
              Cancel
            </Button>
            <Button size="sm" variant="primary" disabled={!saveName.trim()} onClick={() => void save()}>
              Save
            </Button>
          </>
        )}
      </div>
      <div className="px-1 space-y-1.5">
        <Segmented
          label="Look preset"
          size="sm"
          className="w-full"
          value={draft.preset}
          onChange={(name) => change({ preset: name })}
          options={view.presets.map((p) => ({ value: p.name, label: p.label, title: p.description }))}
        />
        {preset && <p className="text-meta text-studio-500 leading-relaxed">{preset.description}</p>}
      </div>
      <div className="px-1 space-y-1">
        {LOOK_CONTROLS.map((control) => (
          <Slider
            key={control.id}
            label={control.label}
            hint={control.hint}
            min={control.min}
            max={control.max}
            value={draft[control.id]}
            onChange={(value) => change({ [control.id]: value })}
          />
        ))}
      </div>
      <div className="px-1 space-y-1">
        <p className="text-meta text-studio-500" title="A short's graphics take the video's look times these">
          In shorts, times the video&apos;s
        </p>
        {SHORTS_LOOK_CONTROLS.map((control) => (
          <Slider
            key={control.id}
            label={control.label}
            hint={control.hint}
            min={0.5}
            max={2}
            value={shorts[control.id]}
            onChange={(value) => change({ shorts: { ...shorts, [control.id]: value } })}
          />
        ))}
      </div>
      {stills && stills.length > 0 && (
        <div className="grid grid-cols-3 gap-1 px-1">
          {stills.map((still) => (
            <img
              key={still.assetId}
              src={still.image}
              alt={still.name}
              title={still.name}
              className="w-full rounded border border-studio-800 bg-studio-950"
            />
          ))}
        </div>
      )}
      <div className="flex items-center gap-1.5 px-1">
        <span className="flex-1 text-meta text-studio-500">
          {changed ? "Not applied yet" : "Every graphic in this video uses it"}
        </span>
        <Button
          size="sm"
          variant="ghost"
          icon={previewing ? Loader2 : Eye}
          disabled={!hasGraphics || !ready || previewing}
          title={
            !hasGraphics ? "Add a graphic to preview the look on" : "Draw a few of this video's graphics with this look"
          }
          onClick={() => void preview()}
        >
          Preview
        </Button>
        <Button size="sm" variant="ghost" disabled={!changed} onClick={() => change(view.look)}>
          Cancel
        </Button>
        <Button size="sm" variant="primary" disabled={!changed} onClick={() => void apply()}>
          Apply
        </Button>
      </div>
      {view.overrides.length > 0 && (
        <p className="px-1 text-meta text-studio-500 leading-relaxed">
          Their own look:{" "}
          {view.overrides.map(([name, tokens]) => `${name} (${tokens.join(", ")})`).join("; ")}
        </p>
      )}
      {plans.length > 0 && (
        <div className="mx-1 space-y-1.5 rounded-control border border-studio-800 bg-studio-850 px-2 py-1.5">
          <div className="flex items-center gap-1.5">
            <span className="flex-1 text-meta text-studio-400">
              {plans.length === 1 ? "1 graphic doesn't" : `${plans.length} graphics don't`} follow the look
            </span>
            <Button size="sm" variant="ghost" icon={Wand2} onClick={() => setShowPlans((shown) => !shown)}>
              {showPlans ? "Hide" : "Bring in line"}
            </Button>
          </div>
          {showPlans && (
            <>
              <ul className="space-y-1 text-meta text-studio-400">
                {plans.map((plan) => (
                  <li key={plan.graphic}>
                    <span className="text-studio-200">{plan.name}</span>: {plan.changes.join("; ")}
                    {plan.left && plan.left.length > 0 && (
                      <span className="text-studio-500"> (stays its own: {plan.left.join(", ")})</span>
                    )}
                  </li>
                ))}
              </ul>
              <div className="flex justify-end">
                <Button size="sm" variant="primary" onClick={() => void bringInLine()}>
                  Bring {plans.length === 1 ? "it" : "them"} in line
                </Button>
              </div>
            </>
          )}
        </div>
      )}
    </section>
  );
};
