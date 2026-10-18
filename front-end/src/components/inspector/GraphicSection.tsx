import React, { useCallback, useEffect, useRef, useState } from "react";
import { FolderOpen, Library, RotateCw, Sparkles } from "lucide-react";
import { InspectorSection } from "./InspectorSection";
import { IconButton } from "../ui";
import { useProjectStore } from "../../stores/projectStore";
import { api } from "../../lib/ipc";
import { assetById, findClip } from "../../lib/sequence";
import type { GraphicVariable, GraphicView } from "../../lib/types";
import { RenderBadge } from "../graphics/GraphicsPanel";
import { LibrarySave } from "../graphics/LibrarySave";
import { useBrands, useGraphicsUpdates } from "../graphics/graphicsEvents";

/** The variables a brand kit fills where a graphic has not set them. */
const KIT_VARIABLES = ["background", "text", "accent"] as const;

const fieldClass =
  "w-full min-w-0 bg-studio-950 text-studio-100 text-label rounded px-2 py-1 border border-studio-700 focus:outline-none focus:border-accent-hover";

/// The selected graphic: its text and colours (its template's variables), its name and length,
/// and how its render is going. A change renders it again in the background.
export const GraphicSection: React.FC = () => {
  const openedProject = useProjectStore((s) => s.openedProject);
  const applyOpenedProject = useProjectStore((s) => s.applyOpenedProject);
  const selectedId = useProjectStore((s) => s.selectedClipIds[0]);
  const [loaded, setLoaded] = useState<GraphicView>();
  const [error, setError] = useState<string>();
  const [note, setNote] = useState<string>();
  const [ready, setReady] = useState(true);
  const [savingToLibrary, setSavingToLibrary] = useState(false);

  const found = openedProject && selectedId ? findClip(openedProject.sequence, selectedId) : undefined;
  const asset = openedProject && found ? assetById(openedProject, found.clip.asset) : undefined;
  const assetId = asset?.kind === "graphic" ? asset.id : undefined;
  const handle = openedProject?.projectHandle;
  const brands = useBrands(handle, openedProject?.revision);
  // Only the selected graphic's fields show, so an edit never lands on another graphic.
  const graphic = loaded?.assetId === assetId ? loaded : undefined;
  const showing = useRef<string>();
  showing.current = handle && assetId ? `${handle}:${assetId}` : undefined;
  /** A reply about a graphic selected before (or in another project) is dropped. */
  const take = useCallback((view: GraphicView, from: string) => {
    if (showing.current === from) setLoaded(view);
  }, []);
  useEffect(() => {
    setNote(undefined);
    setSavingToLibrary(false);
  }, [assetId]);

  const load = useCallback(() => {
    if (!handle || !assetId) {
      setLoaded(undefined);
      return;
    }
    void api
      .projectGraphicGet(handle, assetId)
      .then((view) => take(view, `${handle}:${assetId}`))
      .catch((e) => setError(String(e)));
  }, [handle, assetId, take]);
  useEffect(load, [load]);
  useGraphicsUpdates(load);
  useEffect(() => {
    void api
      .graphicsRuntimeStatus()
      .then((s) => setReady(s.installed))
      .catch(() => undefined);
  }, [assetId]);

  if (!openedProject || !found || !assetId) return null;
  const { clip, track } = found;

  const save = async (variables: Record<string, unknown>, name?: string, brand?: string) => {
    const project = useProjectStore.getState().openedProject;
    if (!project) return;
    setError(undefined);
    try {
      const updated = await api.projectGraphicSet(
        project.projectHandle,
        project.revision,
        assetId,
        variables,
        name,
        brand,
      );
      take(updated.graphic, `${project.projectHandle}:${assetId}`);
      if (updated.project) applyOpenedProject(updated.project);
    } catch (err) {
      setError(String(err));
    }
  };
  // The kit this graphic takes its colours from, if any.
  const kitName = graphic?.brand === "none" ? null : (graphic?.brand ?? brands?.projectDefault ?? null);
  const kit = brands?.kits.find((k) => k.name === kitName);
  const setLength = async (seconds: number) => {
    const lengthUs = Math.round(seconds * 1e6);
    if (!Number.isFinite(lengthUs) || lengthUs < 500_000 || lengthUs === clip.durationUs) return;
    const project = useProjectStore.getState().openedProject;
    if (!project) return;
    setError(undefined);
    try {
      applyOpenedProject(
        await api.projectSequenceEdit(project.projectHandle, project.revision, {
          kind: "trimClip",
          clipId: clip.id,
          edge: "end",
          toUs: clip.startUs + lengthUs,
        }),
      );
    } catch (err) {
      setError(String(err));
    }
  };

  return (
    <InspectorSection id="graphic" title="Graphic" icon={Sparkles}>
      {graphic && (
        <>
          <div className="flex items-center gap-2">
            <input
              key={`name-${graphic.assetId}-${graphic.name}`}
              aria-label="Graphic name"
              defaultValue={graphic.name}
              disabled={track.locked}
              onBlur={(e) => {
                if (e.target.value.trim() && e.target.value !== graphic.name) void save({}, e.target.value);
              }}
              onKeyDown={(e) => {
                if (e.key === "Enter") (e.target as HTMLInputElement).blur();
              }}
              className={fieldClass}
            />
            <RenderBadge graphic={graphic} ready={ready} />
          </div>
          <label className="flex items-center justify-between gap-2 text-label text-studio-300">
            <span className="shrink-0">Brand kit</span>
            <select
              aria-label="The graphic's brand kit"
              value={graphic.brand ?? "default"}
              disabled={track.locked}
              onChange={(e) => void save({}, undefined, e.target.value)}
              className={fieldClass}
            >
              <option value="default">
                Project's ({brands?.projectDefault ?? "none"})
              </option>
              <option value="none">No brand kit</option>
              {(brands?.kits ?? []).map((k) => (
                <option key={k.name} value={k.name}>
                  {k.name}
                </option>
              ))}
              {graphic.brand && graphic.brand !== "none" && !brands?.kits.some((k) => k.name === graphic.brand) && (
                <option value={graphic.brand}>{graphic.brand} (removed)</option>
              )}
            </select>
          </label>
          {graphic.declarations.map((decl) => {
            const own = decl.id in graphic.variables;
            const fromKit =
              !own && kit && (KIT_VARIABLES as readonly string[]).includes(decl.id)
                ? kit[decl.id as (typeof KIT_VARIABLES)[number]]
                : undefined;
            return (
              <VariableField
                key={`${graphic.assetId}-${decl.id}`}
                decl={decl}
                value={own ? graphic.variables[decl.id] : (fromKit ?? decl.default)}
                note={fromKit ? "from the kit" : undefined}
                onReset={
                  own && kit && (KIT_VARIABLES as readonly string[]).includes(decl.id)
                    ? () => void save({ [decl.id]: null })
                    : undefined
                }
                disabled={!!track.locked}
                onChange={(value) => void save({ [decl.id]: value })}
              />
            );
          })}
          <label className="flex items-center justify-between text-label text-studio-300">
            <span>Length (s)</span>
            <input
              key={`${clip.id}-${clip.durationUs}`}
              aria-label="Graphic length in seconds"
              type="number"
              min={0.5}
              step={0.5}
              defaultValue={(clip.durationUs / 1e6).toFixed(1)}
              disabled={track.locked}
              onBlur={(e) => void setLength(Number(e.target.value))}
              onKeyDown={(e) => {
                if (e.key === "Enter") void setLength(Number((e.target as HTMLInputElement).value));
              }}
              className="w-24 bg-studio-950 px-2 py-1 rounded border border-studio-700 text-right"
            />
          </label>
          <div className="flex items-center gap-1">
            <p className="flex-1 text-meta text-studio-500">
              Plays full screen over everything (in a short, pick its place in the Shorts studio). Cuts skip it;
              changes render it again.
            </p>
            <IconButton
              icon={Library}
              size="sm"
              active={savingToLibrary}
              label="Save to library: check it and keep it as a template for other projects"
              onClick={() => setSavingToLibrary((open) => !open)}
            />
            <IconButton
              icon={FolderOpen}
              size="sm"
              label="Open the graphic's folder (index.html) to edit it by hand"
              onClick={() => void api.showInFinder(graphic.folder)}
            />
            <IconButton
              icon={RotateCw}
              size="sm"
              label="Render it again (after editing its files by hand)"
              onClick={() => void api.graphicsRenderAgain()}
            />
          </div>
          {savingToLibrary && handle && (
            <LibrarySave
              key={graphic.assetId}
              projectHandle={handle}
              assetId={graphic.assetId}
              name={graphic.name}
              onClose={() => setSavingToLibrary(false)}
              onSaved={(saved) => {
                setSavingToLibrary(false);
                setNote(saved);
              }}
            />
          )}
          {graphic.render.state === "failed" && graphic.render.error && (
            <pre className="max-h-32 overflow-auto whitespace-pre-wrap text-meta text-danger-fg bg-danger/10 rounded p-2">
              {graphic.render.error}
            </pre>
          )}
        </>
      )}
      {note && <p className="text-meta text-studio-400">{note}</p>}
      {error && (
        <p role="alert" className="text-meta text-danger-fg">
          {error}
        </p>
      )}
    </InspectorSection>
  );
};

/// A colour, saved once the picker has settled (dragging in it fires many changes).
const ColorField: React.FC<{
  label: string;
  value: unknown;
  disabled: boolean;
  onChange: (value: unknown) => void;
  /** Where the colour comes from when the graphic has not set it ("from the kit"). */
  note?: string;
  /** Puts the colour back to the brand kit's. */
  onReset?: () => void;
}> = ({ label, value, disabled, onChange, note, onReset }) => {
  const hex = typeof value === "string" && /^#[0-9a-f]{6}$/i.test(value) ? value : "#000000";
  const timer = useRef<ReturnType<typeof setTimeout>>();
  useEffect(() => () => clearTimeout(timer.current), []);
  return (
    <label className="flex items-center justify-between gap-2 text-label text-studio-300">
      <span className="truncate flex-1">
        {label}
        {note && <span className="text-meta text-studio-500"> · {note}</span>}
      </span>
      {onReset && (
        <button
          type="button"
          disabled={disabled}
          onClick={(e) => {
            e.preventDefault();
            onReset();
          }}
          className="text-meta text-studio-500 hover:text-studio-200"
          title="Use the brand kit's colour"
        >
          Use kit
        </button>
      )}
      <input
        type="color"
        aria-label={label}
        defaultValue={hex}
        key={hex}
        disabled={disabled}
        onChange={(e) => {
          const next = e.target.value.toUpperCase();
          clearTimeout(timer.current);
          timer.current = setTimeout(() => {
            if (next !== hex.toUpperCase()) onChange(next);
          }, 500);
        }}
        className="h-7 w-12 rounded border border-studio-700 bg-studio-950"
      />
    </label>
  );
};

/// One variable's input, saved when it is left (or picked, for switches and lists).
const VariableField: React.FC<{
  decl: GraphicVariable;
  value: unknown;
  disabled: boolean;
  onChange: (value: unknown) => void;
  note?: string;
  onReset?: () => void;
}> = ({ decl, value, disabled, onChange, note, onReset }) => {
  const label = decl.label || decl.id;
  if (decl.type === "boolean") {
    return (
      <label className="flex items-center justify-between text-label text-studio-300">
        <span>{label}</span>
        <input type="checkbox" checked={Boolean(value)} disabled={disabled} onChange={(e) => onChange(e.target.checked)} />
      </label>
    );
  }
  if (decl.type === "color") {
    return (
      <ColorField label={label} value={value} disabled={disabled} onChange={onChange} note={note} onReset={onReset} />
    );
  }
  if (decl.type === "enum") {
    return (
      <label className="block text-label text-studio-300 space-y-1">
        <span>{label}</span>
        <select
          aria-label={label}
          value={JSON.stringify(value)}
          disabled={disabled}
          onChange={(e) => onChange(JSON.parse(e.target.value))}
          className={fieldClass}
        >
          {(decl.options ?? []).map((option) => (
            <option key={JSON.stringify(option.value)} value={JSON.stringify(option.value)}>
              {option.label || String(option.value)}
            </option>
          ))}
        </select>
      </label>
    );
  }
  if (decl.type === "number") {
    return (
      <label className="flex items-center justify-between gap-2 text-label text-studio-300">
        <span className="truncate">{label}</span>
        <input
          type="number"
          aria-label={label}
          key={String(value)}
          defaultValue={Number(value)}
          min={decl.min}
          max={decl.max}
          step={decl.step ?? "any"}
          disabled={disabled}
          onBlur={(e) => {
            const next = Number(e.target.value);
            if (Number.isFinite(next) && next !== Number(value)) onChange(next);
          }}
          onKeyDown={(e) => {
            if (e.key === "Enter") (e.target as HTMLInputElement).blur();
          }}
          className="w-24 bg-studio-950 px-2 py-1 rounded border border-studio-700 text-right"
        />
      </label>
    );
  }
  const text = typeof value === "string" ? value : String(value ?? "");
  // Lists ("one per line") and anything already on several lines get a box.
  const multiline = text.includes("\n") || /line/i.test(label);
  const commit = (next: string) => {
    if (next !== text) onChange(next);
  };
  return (
    <label className="block text-label text-studio-300 space-y-1">
      <span>{label}</span>
      {multiline ? (
        <textarea
          aria-label={label}
          key={text}
          defaultValue={text}
          rows={Math.min(6, Math.max(3, text.split("\n").length))}
          disabled={disabled}
          onBlur={(e) => commit(e.target.value)}
          className={fieldClass}
        />
      ) : (
        <input
          aria-label={label}
          key={text}
          defaultValue={text}
          disabled={disabled}
          onBlur={(e) => commit(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") (e.target as HTMLInputElement).blur();
          }}
          className={fieldClass}
        />
      )}
    </label>
  );
};
