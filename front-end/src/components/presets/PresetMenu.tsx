import React, { useEffect, useState } from "react";
import { BadgeCheck, BookOpen, Check, Circle, AlertCircle, Copy, FolderOpen, History, Loader2, Pencil, Play, RotateCcw, Save, Smartphone, Trash2 } from "lucide-react";
import { api } from "../../lib/ipc";
import type { OpenedProject, PresetRunEvent, PresetRunPlan, PresetVersion, PresetView } from "../../lib/types";
import { useProjectStore } from "../../stores/projectStore";
import { Button, Menu, MenuButton, type MenuEntry, DialogBackdrop } from "../ui";
import { presetsChanged, usePresets } from "./usePresets";
import { listenTo } from "../../lib/windowSync";

/// The project's preset: what the app and the agent do for this kind of video. Following
/// another is one undo step (what was changed here stays changed); the project's settings can
/// be saved into its preset (every project following it follows) or as a new preset; the
/// method the agent follows and the user's own rules for it can be read and written.
export const PresetMenu: React.FC<{ onError: (message: string) => void }> = ({ onError }) => {
  const project = useProjectStore((s) => s.openedProject);
  const applyOpenedProject = useProjectStore((s) => s.applyOpenedProject);
  const view = usePresets(project?.projectHandle, project?.revision);
  const [naming, setNaming] = useState(false);
  const [renaming, setRenaming] = useState(false);
  const [editing, setEditing] = useState<PresetView>();
  const [runOpen, setRunOpen] = useState(false);
  const [workflowOpen, setWorkflowOpen] = useState(false);
  const [summaryOpen, setSummaryOpen] = useState(false);
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
    {
      label: `Run ${view.projectPreset}…`,
      icon: Play,
      hint: "The app runs its first steps (transcribe, clean up) up to where the agent takes over",
      onSelect: () => setRunOpen(true),
    },
    { kind: "separator" },
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
          onSelect: () => run(`Shorts start from ${p.name}`, (h, r) => api.projectPresetSet(h, r, p.name, "shorts")),
        }),
      ),
    },
    {
      kind: "submenu",
      label: `New clips start from: ${view.clipsPreset ?? "Clips"}`,
      icon: Smartphone,
      entries: view.presets.map(
        (p): MenuEntry => ({
          label: p.name,
          icon: p.name === view.clipsPreset ? Check : undefined,
          onSelect: () => run(`Clips start from ${p.name}`, (h, r) => api.projectPresetSet(h, r, p.name, "clips")),
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
      label: "Save this edit as a workflow…",
      icon: BadgeCheck,
      hint: "This edit's settings, with your exceptions and checks, for the next video of this kind",
      onSelect: () => setWorkflowOpen(true),
    },
    {
      label: "Take every change back",
      icon: RotateCcw,
      disabled: view.changed === 0,
      hint: `Back to ${view.projectPreset}'s values`,
      onSelect: () => run("Back to the preset", (h, r) => api.projectPresetReset(h, r)),
    },
    { kind: "separator" },
    { label: `What ${view.projectPreset} does…`, icon: History, hint: "A summary, and earlier versions to go back to", onSelect: () => setSummaryOpen(true) },
    { label: "Method and rules…", icon: BookOpen, onSelect: () => current && setEditing(current) },
    { label: "Open the presets folder", icon: FolderOpen, onSelect: () => void api.showInFinder(view.folder) },
    ...(current && !current.builtin
      ? ([
          { label: `Rename ${current.name}…`, icon: Pencil, onSelect: () => setRenaming(true) },
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
      {renaming && current && (
        <NameDialog
          title={`Rename ${current.name}`}
          initial={current.name}
          taken={view.presets.filter((p) => p.name !== current.name).map((p) => p.name)}
          onClose={() => setRenaming(false)}
          onSave={(name) => {
            setRenaming(false);
            void api
              .presetRename(current.name, name)
              .then(() => {
                presetsChanged();
                setNotice(`Renamed to ${name}`);
              })
              .catch((err) => onError(String(err)));
          }}
        />
      )}
      {runOpen && <RunDialog onClose={() => setRunOpen(false)} onError={onError} />}
      {workflowOpen && current && (
        <WorkflowDialog
          current={current}
          taken={view.presets.map((p) => p.name)}
          onClose={() => setWorkflowOpen(false)}
          onSave={(name, workflow) => {
            setWorkflowOpen(false);
            run(`Saved workflow ${name ?? current.name}`, (h, r) => api.presetSaveFromProject(h, r, name, workflow));
          }}
        />
      )}
      {summaryOpen && current && (
        <SummaryDialog
          current={current}
          onClose={() => setSummaryOpen(false)}
          onRestored={(version) => {
            setSummaryOpen(false);
            presetsChanged();
            setNotice(`Back to version ${version}`);
          }}
          onError={onError}
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
  <DialogBackdrop onClose={onClose} selectable>
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
  </DialogBackdrop>
);

const NameDialog: React.FC<{
  taken: string[];
  title?: string;
  initial?: string;
  onClose: () => void;
  onSave: (name: string) => void;
}> = ({ taken, title, initial, onClose, onSave }) => {
  const [name, setName] = useState(initial ?? "");
  const trimmed = name.trim();
  const clash = taken.some((t) => t.toLowerCase() === trimmed.toLowerCase());
  return (
    <Dialog label={title ?? "Save as a new preset"} onClose={onClose} onSubmit={() => trimmed && !clash && onSave(trimmed)}>
      <h2 className="text-body font-semibold text-studio-100">{title ?? "Save as a new preset"}</h2>
      <p className="text-label text-studio-400 leading-relaxed">
        {title
          ? "Projects that follow it keep following it under its new name."
          : "This video's settings (canvas, captions, zooms, sound, layouts, cleanup, export) become a preset this video then follows, with the method and rules of the one it follows now. Any project can follow it."}
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

const lines = (text: string) =>
  text
    .split("\n")
    .map((line) => line.trim())
    .filter(Boolean);

/** This edit's settings saved with the user's exceptions and checks, for the next video of its kind. */
const WorkflowDialog: React.FC<{
  current: PresetView;
  taken: string[];
  onClose: () => void;
  onSave: (name: string | undefined, workflow: { exceptions: string[]; checks: string[]; note: string }) => void;
}> = ({ current, taken, onClose, onSave }) => {
  const [name, setName] = useState(current.builtin ? "" : current.name);
  const [exceptions, setExceptions] = useState((current.preset.exceptions ?? []).join("\n"));
  const [checks, setChecks] = useState((current.preset.checks ?? []).join("\n"));
  const [note, setNote] = useState("");
  const trimmed = name.trim();
  const updating = !current.builtin && trimmed.toLowerCase() === current.name.toLowerCase();
  const clash = !updating && taken.some((t) => t.toLowerCase() === trimmed.toLowerCase());
  const ready = !!trimmed && !clash;
  return (
    <Dialog
      label="Save this edit as a workflow"
      onClose={onClose}
      onSubmit={() => {
        if (ready) onSave(updating ? undefined : trimmed, { exceptions: lines(exceptions), checks: lines(checks), note });
      }}
    >
      <h2 className="text-body font-semibold text-studio-100">Save this edit as a workflow</h2>
      <p className="text-label text-studio-400 leading-relaxed">
        This video&apos;s settings, plus {current.name}&apos;s steps and your rules, for the next video of this kind. Your agent follows
        the exceptions and goes through the checks before it asks about export.
        {updating ? " It becomes a new version; the one before it stays in the list." : ""}
      </p>
      <input
        autoFocus
        aria-label="Workflow name"
        placeholder="Name, e.g. My tutorials"
        maxLength={40}
        value={name}
        onChange={(e) => setName(e.target.value)}
        className="ui-field"
      />
      {clash && <p className="text-meta text-danger-fg">There is already a preset of that name.</p>}
      <textarea
        aria-label="Exceptions, one a line"
        placeholder={"Exceptions, one a line, e.g.\nA take under a minute gets no chapters"}
        value={exceptions}
        onChange={(e) => setExceptions(e.target.value)}
        className="ui-field h-20 resize-none"
      />
      <textarea
        aria-label="Checks before export, one a line"
        placeholder={"Checks before export, one a line, e.g.\nThe intro is under 20 s"}
        value={checks}
        onChange={(e) => setChecks(e.target.value)}
        className="ui-field h-20 resize-none"
      />
      <input
        aria-label="What you approved"
        placeholder="What you approved, in a line (optional)"
        maxLength={200}
        value={note}
        onChange={(e) => setNote(e.target.value)}
        className="ui-field"
      />
      <div className="flex justify-end gap-2">
        <Button type="button" variant="ghost" onClick={onClose}>
          Cancel
        </Button>
        <Button type="submit" variant="primary" disabled={!ready}>
          Save
        </Button>
      </div>
    </Dialog>
  );
};

/** What a preset does, in a few lines, with its earlier versions to read and go back to. */
const SummaryDialog: React.FC<{
  current: PresetView;
  onClose: () => void;
  onRestored: (version: number) => void;
  onError: (message: string) => void;
}> = ({ current, onClose, onRestored, onError }) => {
  const [versions, setVersions] = useState<PresetVersion[]>();
  const [showing, setShowing] = useState<PresetVersion>();
  useEffect(() => {
    void api
      .presetVersions(current.name)
      .then(setVersions)
      .catch((err) => onError(String(err)));
  }, [current.name, onError]);
  const restore = (version: number) => {
    void api
      .presetRestore(current.name, version)
      .then(() => onRestored(version))
      .catch((err) => onError(String(err)));
  };
  const earlier = (versions ?? []).filter((v) => !v.current);
  return (
    <Dialog label={`What ${current.name} does`} wide onClose={onClose} onSubmit={onClose}>
      <h2 className="text-body font-semibold text-studio-100">What {current.name} does</h2>
      <pre className="max-h-72 overflow-y-auto whitespace-pre-wrap rounded-control border border-studio-800 bg-studio-850 p-3 text-label text-studio-200 select-text font-sans">
        {showing ? showing.summary : current.summary}
      </pre>
      {earlier.length > 0 && (
        <ul className="max-h-32 overflow-y-auto flex flex-col gap-1" aria-label="Earlier versions">
          {earlier.map((version) => (
            <li key={version.version} className="flex items-center gap-2 text-label text-studio-300">
              <span className="flex-1">
                Version {version.version}
                {version.savedAt ? `, ${version.savedAt.slice(0, 10)}` : ""}
              </span>
              <Button
                type="button"
                size="sm"
                variant="ghost"
                onClick={() => setShowing(showing?.version === version.version ? undefined : version)}
              >
                {showing?.version === version.version ? "Show current" : "Read"}
              </Button>
              <Button type="button" size="sm" variant="secondary" onClick={() => restore(version.version)} title="Becomes a new version; the current one stays in this list">
                Go back to it
              </Button>
            </li>
          ))}
        </ul>
      )}
      <div className="flex justify-end">
        <Button type="submit" variant="primary">
          Done
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

type StepState = { status: "waiting" | "running" | "done" | "stopped"; line?: string; progress?: string };

/** Runs the project's preset up to where the agent takes over (AUTO-1): free steps on this
 *  computer only, unless the user allows the online ones for this run. Each step is one undo step. */
const RunDialog: React.FC<{ onClose: () => void; onError: (message: string) => void }> = ({ onClose, onError }) => {
  const [online, setOnline] = useState(false);
  const [plan, setPlan] = useState<PresetRunPlan>();
  const [steps, setSteps] = useState<StepState[]>([]);
  const [phase, setPhase] = useState<"ready" | "running" | "finished">("ready");

  useEffect(() => {
    if (phase !== "ready") return;
    let live = true;
    void api
      .presetRunPlan(online)
      .then((next) => {
        if (!live) return;
        setPlan(next);
        setSteps(next.steps.map(() => ({ status: "waiting" })));
      })
      .catch((err) => onError(String(err)));
    return () => {
      live = false;
    };
  }, [online, phase, onError]);

  useEffect(() => {
    return listenTo<PresetRunEvent>("aeroedits-preset-run", (event) => {
      if (event.kind === "finished") {
        setPhase("finished");
        return;
      }
      setSteps((now) =>
        now.map((step, i): StepState => {
          if (i !== event.index) return step;
          switch (event.kind) {
            case "started":
              return { status: "running" };
            case "progress":
              return { status: "running", progress: `${Math.round(event.fraction * 100)} % ${event.message}` };
            case "done":
              return { status: "done", line: event.line };
            case "stopped":
              return { status: "stopped", line: event.line };
          }
        }),
      );
    });
  }, []);

  const start = () => {
    setPhase("running");
    void api.presetRun(online).catch((err) => {
      onError(String(err));
      setPhase("finished");
    });
  };
  const stoppedAt = steps.findIndex((s) => s.status === "stopped");

  return (
    <Dialog label={`Run ${plan?.preset ?? "the preset"}`} wide onClose={phase === "running" ? () => {} : onClose} onSubmit={start}>
      <h2 className="text-body font-semibold text-studio-100">Run {plan?.preset ?? "the preset"}</h2>
      <p className="text-label text-studio-400 leading-relaxed">
        The app runs the preset&apos;s first steps itself, each one undo step, and stops where your agent takes over or
        a result looks wrong.
      </p>
      <label className="flex items-center gap-2 text-label text-studio-300">
        <input type="checkbox" checked={online} disabled={phase !== "ready"} onChange={(e) => setOnline(e.target.checked)} />
        Allow online steps for this run (transcription and the fillers&apos; review use your keys)
      </label>
      <ol className="flex flex-col gap-1.5 max-h-[50vh] overflow-y-auto">
        {plan?.steps.map((step, i) => {
          const state = steps[i] ?? { status: "waiting" };
          const Icon =
            state.status === "done" ? Check : state.status === "stopped" ? AlertCircle : state.status === "running" ? Loader2 : Circle;
          return (
            <li key={i} className="flex gap-2 text-label">
              <Icon
                className={`w-3.5 h-3.5 mt-0.5 shrink-0 ${state.status === "running" ? "animate-spin text-studio-300" : state.status === "done" ? "text-audio-fg" : state.status === "stopped" ? "text-danger-fg" : "text-studio-600"}`}
              />
              <div className="min-w-0">
                <div className="text-studio-200">
                  {step.label}
                  {step.online && <span className="text-studio-500"> · online</span>}
                </div>
                {state.progress && state.status === "running" && <div className="text-meta text-studio-500">{state.progress}</div>}
                {state.line && (
                  <div
                    title={state.line}
                    className={`text-meta whitespace-pre-wrap line-clamp-4 ${state.status === "stopped" ? "text-danger-fg" : "text-studio-400"}`}
                  >
                    {state.line}
                  </div>
                )}
              </div>
            </li>
          );
        })}
      </ol>
      {plan && plan.left.length > 0 && (
        <div className="text-label text-studio-400">
          <div className="text-studio-300">{phase === "finished" ? "Next, for your agent" : "Then, for your agent"}</div>
          <ul className="list-disc pl-5">
            {plan.left.map((line) => (
              <li key={line}>{line}</li>
            ))}
          </ul>
          {phase === "finished" && (
            <p className="mt-1">
              {stoppedAt >= 0
                ? "The run stopped at the step marked above: tell your agent what to do there, or run that pass yourself."
                : `Ask your agent to continue the ${plan.preset} preset on this project.`}
            </p>
          )}
        </div>
      )}
      <div className="flex justify-end gap-2">
        {phase === "running" ? (
          <Button type="button" variant="ghost" onClick={() => void api.presetRunStop()}>
            Stop
          </Button>
        ) : (
          <Button type="button" variant={phase === "finished" ? "primary" : "ghost"} onClick={onClose}>
            {phase === "finished" ? "Done" : "Cancel"}
          </Button>
        )}
        {phase === "ready" && (
          <Button type="submit" variant="primary" disabled={!plan}>
            Run
          </Button>
        )}
      </div>
    </Dialog>
  );
};
