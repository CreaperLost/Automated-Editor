import React, { useEffect, useState } from "react";
import { BookOpen, Check, Copy, FolderOpen, RotateCcw, Save, Smartphone, Trash2 } from "lucide-react";
import { api } from "../../lib/ipc";
import type { OpenedProject, PresetView } from "../../lib/types";
import { useProjectStore } from "../../stores/projectStore";
import { Button, Menu, MenuButton, type MenuEntry } from "../ui";
import { presetsChanged, usePresets } from "./usePresets";

/// The project's preset: what the app and the agent do for this kind of video. Following
/// another is one undo step (what was changed here stays changed); the project's settings can
/// be saved into its preset (every project following it follows) or as a new preset; the
/// method the agent follows and the user's own rules for it can be read and written.
export const PresetMenu: React.FC<{ onError: (message: string) => void }> = ({ onError }) => {
  const project = useProjectStore((s) => s.openedProject);
  const applyOpenedProject = useProjectStore((s) => s.applyOpenedProject);
  const view = usePresets(project?.projectHandle, project?.revision);
  const [naming, setNaming] = useState(false);
  const [editing, setEditing] = useState<PresetView>();
  const [notice, setNotice] = useState<string>();

  useEffect(() => {
    if (!notice) return;
    const timer = window.setTimeout(() => setNotice(undefined), 2500);
    return () => window.clearTimeout(timer);
  }, [notice]);

  if (!project || !view?.projectPreset) return null;
  const current = view.presets.find((p) => p.name === view.projectPreset);
  const run = (what: string, call: (handle: string, revision: number) => Promise<OpenedProject>) => {
    const now = useProjectStore.getState().openedProject;
    if (!now) return;
    void call(now.projectHandle, now.revision)
      .then((next) => {
        applyOpenedProject(next);
        presetsChanged();
        setNotice(what);
      })
      .catch((err) => onError(String(err)));
  };

  const entries: MenuEntry[] = [
    { kind: "heading", label: "This video follows" },
    ...view.presets.map(
      (p): MenuEntry => ({
        label: p.name,
        icon: p.name === view.projectPreset ? Check : undefined,
        hint: p.preset.description,
        onSelect: () => run(`Follows ${p.name}`, (h, r) => api.projectPresetSet(h, r, p.name)),
      }),
    ),
    { kind: "separator" },
    {
      kind: "submenu",
      label: `New shorts start from: ${view.shortsPreset ?? "Shorts"}`,
      icon: Smartphone,
      entries: view.presets.map(
        (p): MenuEntry => ({
          label: p.name,
          icon: p.name === view.shortsPreset ? Check : undefined,
          onSelect: () => run(`Shorts start from ${p.name}`, (h, r) => api.projectPresetSet(h, r, p.name, true)),
        }),
      ),
    },
    { kind: "separator" },
    {
      label: `Save changes to ${view.projectPreset}`,
      icon: Save,
      disabled: !current || current.builtin || view.changed === 0,
      hint: current?.builtin
        ? "Built in: save as a new preset"
        : view.changed === 0
          ? "Nothing changed here"
          : "Every project following it follows",
      onSelect: () => run(`Saved to ${view.projectPreset}`, (h, r) => api.presetSaveFromProject(h, r)),
    },
    { label: "Save as a new preset…", icon: Copy, onSelect: () => setNaming(true) },
    {
      label: "Take every change back",
      icon: RotateCcw,
      disabled: view.changed === 0,
      hint: `Back to ${view.projectPreset}'s values`,
      onSelect: () => run("Back to the preset", (h, r) => api.projectPresetReset(h, r)),
    },
    { kind: "separator" },
    { label: "Method and rules…", icon: BookOpen, onSelect: () => current && setEditing(current) },
    { label: "Open the presets folder", icon: FolderOpen, onSelect: () => void api.showInFinder(view.folder) },
    ...(current && !current.builtin
      ? ([
          {
            label: `Remove ${current.name}`,
            icon: Trash2,
            danger: true,
            onSelect: () => {
              if (!window.confirm(`Remove the preset "${current.name}" for every project? Projects following it follow Tutorial.`))
                return;
              void api
                .presetRemove(current.name)
                .then(() => {
                  presetsChanged();
                  const now = useProjectStore.getState().openedProject;
                  if (now) run("Removed", (h, r) => api.projectPresetSet(h, r, "Tutorial"));
                })
                .catch((err) => onError(String(err)));
            },
          },
        ] as MenuEntry[])
      : []),
  ];

  return (
    <>
      <Menu
        label="Preset"
        entries={entries}
        width={320}
        trigger={({ open, toggle, ref }) => (
          <MenuButton
            ref={ref}
            open={open}
            onClick={toggle}
            title="What the app and the agent do for this kind of video"
          >
            {notice ? (
              <span className="inline-flex items-center gap-1 text-audio-fg">
                <Check className="w-3.5 h-3.5" /> {notice}
              </span>
            ) : (
              <span>
                {view.projectPreset}
                {view.changed > 0 && <span className="text-studio-500"> · {view.changed} changed</span>}
              </span>
            )}
          </MenuButton>
        )}
      />
      {naming && (
        <NameDialog
          taken={view.presets.map((p) => p.name)}
          onClose={() => setNaming(false)}
          onSave={(name) => {
            setNaming(false);
            run(`Saved as ${name}`, (h, r) => api.presetSaveFromProject(h, r, name));
          }}
        />
      )}
      {editing && (
        <MethodDialog
          preset={editing}
          taken={view.presets.map((p) => p.name)}
          onClose={() => setEditing(undefined)}
          onSaved={(name) => {
            setEditing(undefined);
            presetsChanged();
            if (name !== view.projectPreset) run(`Follows ${name}`, (h, r) => api.projectPresetSet(h, r, name));
            else setNotice("Saved");
          }}
          onError={onError}
        />
      )}
    </>
  );
};

const Dialog: React.FC<{ label: string; wide?: boolean; onClose: () => void; children: React.ReactNode; onSubmit: () => void }> = ({
  label,
  wide,
  onClose,
  children,
  onSubmit,
}) => (
  <div
    className="fixed inset-0 z-50 bg-black/70 flex items-center justify-center p-4"
    onPointerDown={(event) => {
      if (event.target === event.currentTarget) onClose();
    }}
  >
    <form
      role="dialog"
      aria-modal="true"
      aria-label={label}
      className={`w-full ${wide ? "max-w-3xl" : "max-w-sm"} flex flex-col gap-3 rounded-panel border border-studio-700 bg-studio-900 shadow-dialog p-4`}
      onSubmit={(event) => {
        event.preventDefault();
        onSubmit();
      }}
      onKeyDown={(event) => {
        if (event.key === "Escape") onClose();
      }}
    >
      {children}
    </form>
  </div>
);

const NameDialog: React.FC<{ taken: string[]; onClose: () => void; onSave: (name: string) => void }> = ({
  taken,
  onClose,
  onSave,
}) => {
  const [name, setName] = useState("");
  const trimmed = name.trim();
  const clash = taken.some((t) => t.toLowerCase() === trimmed.toLowerCase());
  return (
    <Dialog label="Save as a new preset" onClose={onClose} onSubmit={() => trimmed && !clash && onSave(trimmed)}>
      <h2 className="text-body font-semibold text-studio-100">Save as a new preset</h2>
      <p className="text-label text-studio-400 leading-relaxed">
        This video&apos;s settings (canvas, captions, zooms, sound, layouts) become a preset this video then follows,
        with the method and rules of the one it follows now. Any project can follow it.
      </p>
      <input
        autoFocus
        aria-label="Preset name"
        placeholder="Name, e.g. Gaming"
        maxLength={40}
        value={name}
        onChange={(e) => setName(e.target.value)}
        className="ui-field"
      />
      {clash && <p className="text-meta text-danger-fg">There is already a preset of that name.</p>}
      <div className="flex justify-end gap-2">
        <Button type="button" variant="ghost" onClick={onClose}>
          Cancel
        </Button>
        <Button type="submit" variant="primary" disabled={!trimmed || clash}>
          Save
        </Button>
      </div>
    </Dialog>
  );
};

/** The agent's method and the user's rules for a preset, as Markdown. A built-in is saved as a copy. */
const MethodDialog: React.FC<{
  preset: PresetView;
  taken: string[];
  onClose: () => void;
  onSaved: (name: string) => void;
  onError: (message: string) => void;
}> = ({ preset, taken, onClose, onSaved, onError }) => {
  const [description, setDescription] = useState(preset.preset.description);
  const [method, setMethod] = useState(preset.method);
  const [rules, setRules] = useState(preset.rules);
  const [copyName, setCopyName] = useState(() => {
    const base = `My ${preset.name.toLowerCase()}`;
    let name = base;
    for (let n = 2; taken.some((t) => t.toLowerCase() === name.toLowerCase()); n++) name = `${base} ${n}`;
    return name;
  });
  const save = () => {
    const name = preset.builtin ? copyName.trim() : preset.name;
    void api
      .presetSave({
        name,
        previous: preset.builtin ? null : preset.name,
        preset: { ...preset.preset, description },
        method,
        rules,
      })
      .then(() => onSaved(name))
      .catch((err) => onError(String(err)));
  };
  const area = "w-full min-h-0 flex-1 resize-none ui-field font-mono text-meta leading-relaxed";
  return (
    <Dialog label={`${preset.name}: method and rules`} wide onClose={onClose} onSubmit={save}>
      <h2 className="text-body font-semibold text-studio-100">{preset.name}: method and rules</h2>
      <input
        aria-label="What kind of video it is for"
        value={description}
        onChange={(e) => setDescription(e.target.value)}
        className="ui-field"
      />
      <div className="grid grid-cols-[2fr_1fr] gap-3 h-[60vh]">
        <label className="flex flex-col gap-1 min-h-0">
          <span className="text-label text-studio-400">Method: the agent&apos;s steps and questions (Markdown)</span>
          <textarea aria-label="Method" value={method} onChange={(e) => setMethod(e.target.value)} className={area} />
        </label>
        <label className="flex flex-col gap-1 min-h-0">
          <span className="text-label text-studio-400">Your rules: they win over the method</span>
          <textarea
            aria-label="Rules"
            placeholder={"- Never cut the first sentence.\n- Keep pauses before a demo."}
            value={rules}
            onChange={(e) => setRules(e.target.value)}
            className={area}
          />
        </label>
      </div>
      {preset.builtin && (
        <label className="flex items-center gap-2 text-label text-studio-400">
          <span className="shrink-0">Built in: saved as</span>
          <input aria-label="Name of the copy" value={copyName} maxLength={40} onChange={(e) => setCopyName(e.target.value)} className="ui-field" />
        </label>
      )}
      <div className="flex justify-end gap-2">
        <Button type="button" variant="ghost" onClick={onClose}>
          Cancel
        </Button>
        <Button type="submit" variant="primary" disabled={preset.builtin && !copyName.trim()}>
          {preset.builtin ? "Save as a copy" : "Save"}
        </Button>
      </div>
    </Dialog>
  );
};
