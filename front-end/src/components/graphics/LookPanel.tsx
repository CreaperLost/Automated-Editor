import React, { useCallback, useEffect, useState } from "react";
import { Eye, Loader2, RotateCcw } from "lucide-react";
import { Button, IconButton, Segmented } from "../ui";
import { useProjectStore } from "../../stores/projectStore";
import { api, isTauriEnvironment } from "../../lib/ipc";
import { LOOK_CONTROLS, type GraphicsLook, type LookStill, type LookView } from "../../lib/types";

const same = (a: GraphicsLook, b: GraphicsLook) => JSON.stringify(a) === JSON.stringify(b);

/// The video's look: one style every graphic in it takes (type, corners, spacing, motion), with
/// the brand kit's colours. A preset and five controls change all of them at once; Preview draws
/// a few of the video's own graphics with the change before Apply makes it (one undo step).
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

  const load = useCallback(() => {
    if (!handle || !isTauriEnvironment()) return;
    void api
      .graphicsLook(handle)
      .then((loaded) => {
        setView(loaded);
        setDraft(loaded.look);
        setStills(undefined);
      })
      .catch(() => undefined);
  }, [handle]);
  // The look is in the edit, so it follows undo too.
  useEffect(load, [load, revision]);

  if (!openedProject || !view || !draft) return null;
  const changed = !same(draft, view.look);
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
  const preset = view.presets.find((p) => p.name === draft.preset);

  return (
    <section className="space-y-2">
      <div className="flex items-center gap-1 px-1">
        <h3 className="flex-1 text-meta font-semibold uppercase tracking-wide text-studio-500">Look</h3>
        <IconButton
          icon={RotateCcw}
          size="sm"
          label="Back to the preset as it comes"
          disabled={same(draft, { preset: draft.preset, size: 1, roundness: 1, speed: 1, energy: 1, density: 1 })}
          onClick={() => change({ size: 1, roundness: 1, speed: 1, energy: 1, density: 1, tokens: undefined })}
        />
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
          <label key={control.id} className="flex items-center gap-2 text-label text-studio-300" title={control.hint}>
            <span className="w-[4.5rem] shrink-0">{control.label}</span>
            <input
              type="range"
              aria-label={control.label}
              min={control.min}
              max={control.max}
              step={0.05}
              value={draft[control.id]}
              onChange={(e) => change({ [control.id]: Number(e.target.value) })}
              onDoubleClick={() => change({ [control.id]: 1 })}
              className="flex-1 min-w-0 h-1.5 cursor-pointer"
            />
            <span className="w-11 text-right font-mono text-meta tabular-nums text-studio-200">
              ×{draft[control.id].toFixed(2)}
            </span>
          </label>
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
    </section>
  );
};
