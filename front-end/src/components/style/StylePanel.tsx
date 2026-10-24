import React, { useEffect, useRef, useState } from "react";
import { Captions, Copy, Eye, FolderOpen, Frame, Loader2, Palette, Pencil, Smartphone, Sparkles, Trash2, Wand2 } from "lucide-react";
import { InspectorSection, MoreSettings, RangeRow } from "../inspector/InspectorSection";
import { CAPTION_STYLES, CAPTION_WEIGHTS, ColorRow, Toggle } from "../captions/CaptionsSection";
import { Button, Field, IconButton, Notice, Segmented, cn } from "../ui";
import { useProjectStore } from "../../stores/projectStore";
import { api } from "../../lib/ipc";
import { stylesChanged, useStyles } from "../graphics/graphicsEvents";
import {
  BACKGROUND_PRESETS,
  LOOK_CONTROLS,
  SHORTS_LOOK_CONTROLS,
  presetBackgroundCss,
  type BackgroundPreset,
  type CaptionStyle,
  type InLinePlan,
  type LookStill,
  type Style,
  type StyleBackground,
  type StyleView,
} from "../../lib/types";

const fieldClass =
  "w-full min-w-0 bg-studio-950 text-studio-100 text-label rounded px-2 py-1 border border-studio-700 focus:outline-none focus:border-accent-hover";

const fileName = (path: string | null | undefined) => (path ? (path.split(/[\\/]/).pop() ?? path) : null);

const GRADIENTS = [
  { label: "Indigo Cosmic", start: "#312E81", end: "#0F172A" },
  { label: "Electric Violet", start: "#4C1D95", end: "#1E1B4B" },
  { label: "Deep Obsidian", start: "#27272A", end: "#09090B" },
  { label: "Emerald Matrix", start: "#064E3B", end: "#022C22" },
  { label: "Sunset Velvet", start: "#881337", end: "#1E1B4B" },
  { label: "Cyber Ocean", start: "#0C4A6E", end: "#0F172A" },
];

/** A name for a new style that no style has yet: "My style", "My style 2"... */
function freeName(styles: StyleView[], base: string): string {
  const taken = new Set(styles.map((s) => s.name.toLowerCase()));
  if (!taken.has(base.toLowerCase())) return base;
  for (let n = 2; ; n++) if (!taken.has(`${base} ${n}`.toLowerCase())) return `${base} ${n}`;
}

const Swatch: React.FC<{ label: string; background: string; selected: boolean; onClick: () => void }> = ({
  label,
  background,
  selected,
  onClick,
}) => (
  <button
    type="button"
    onClick={onClick}
    aria-pressed={selected}
    className={cn(
      "h-10 rounded-control border text-left p-1.5 flex flex-col justify-end transition-shadow",
      selected ? "border-accent-fg shadow-[0_0_0_2px_rgb(var(--accent-hover)/0.7)]" : "border-studio-700 hover:border-studio-500",
    )}
    style={{ background }}
  >
    <span className="text-meta font-medium text-white/90 drop-shadow truncate">{label}</span>
  </button>
);

/** A background's controls: built-in, gradient or a solid colour. */
const BackgroundEditor: React.FC<{ value: StyleBackground; onChange: (next: StyleBackground) => void }> = ({
  value,
  onChange,
}) => (
  <div className="space-y-2">
    <Segmented
      label="Background type"
      size="sm"
      className="w-full"
      value={value.type}
      onChange={(type) => onChange({ ...value, type })}
      options={[
        { value: "preset", label: "Built-in" },
        { value: "gradient", label: "Gradient" },
        { value: "solid", label: "Solid" },
      ]}
    />
    {value.type === "preset" && (
      <div className="grid grid-cols-3 gap-1.5">
        {BACKGROUND_PRESETS.map((preset) => (
          <Swatch
            key={preset.key}
            label={preset.label}
            background={presetBackgroundCss(preset.key)}
            selected={value.preset === preset.key}
            onClick={() => onChange({ ...value, preset: preset.key as BackgroundPreset })}
          />
        ))}
      </div>
    )}
    {value.type === "gradient" && (
      <>
        <div className="grid grid-cols-3 gap-1.5">
          {GRADIENTS.map((g) => (
            <Swatch
              key={g.label}
              label={g.label}
              background={`linear-gradient(135deg, ${g.start}, ${g.end})`}
              selected={value.colorStart === g.start && value.colorEnd === g.end}
              onClick={() => onChange({ ...value, colorStart: g.start, colorEnd: g.end })}
            />
          ))}
        </div>
        <ColorRow label="Start" value={value.colorStart} onChange={(colorStart) => onChange({ ...value, colorStart })} />
        <ColorRow label="End" value={value.colorEnd} onChange={(colorEnd) => onChange({ ...value, colorEnd })} />
      </>
    )}
    {value.type === "solid" && (
      <ColorRow label="Colour" value={value.colorStart} onChange={(colorStart) => onChange({ ...value, colorStart })} />
    )}
  </div>
);

/// Styles: how the video's graphics, captions and frame look. The project wears one (picking
/// another is one undo step); changing a style changes it for every project that wears it, so
/// changes save by themselves a moment after you make them. Built-in styles never change: the
/// first change makes your own copy and dresses the project in it.
export const StylePanel: React.FC = () => {
  const openedProject = useProjectStore((s) => s.openedProject);
  const applyOpenedProject = useProjectStore((s) => s.applyOpenedProject);
  const handle = openedProject?.projectHandle;
  const view = useStyles(handle, openedProject?.revision);
  const [error, setError] = useState<string>();
  const [notice, setNotice] = useState<string>();
  const [draft, setDraft] = useState<{ name: string; style: Style }>();
  const [renaming, setRenaming] = useState<string>();
  const [stills, setStills] = useState<LookStill[]>();
  const [previewing, setPreviewing] = useState(false);
  const [plans, setPlans] = useState<InLinePlan[]>([]);
  const timer = useRef<ReturnType<typeof setTimeout>>();
  // What is being saved, so a save in flight is not overtaken by an older one.
  const saving = useRef<Promise<unknown>>(Promise.resolve());

  const current = view?.styles.find((s) => s.name === view.projectStyle) ?? view?.styles[0];
  // The draft follows the project's style, unless it is the one being edited.
  useEffect(() => {
    if (current && current.name !== draft?.name) setDraft({ name: current.name, style: current.style });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [current?.name, current]);
  useEffect(() => {
    if (!handle) return;
    void api
      .graphicsInLinePlan(handle)
      .then(setPlans)
      .catch(() => setPlans([]));
  }, [handle, openedProject?.revision, view]);

  if (!openedProject || !view || !current || !draft) {
    return <p className="p-3 text-meta text-studio-500">Open a project to choose its style.</p>;
  }

  const useStyle = async (name: string) => {
    const project = useProjectStore.getState().openedProject;
    if (!project) return;
    try {
      applyOpenedProject(await api.projectStyleSet(project.projectHandle, project.revision, name));
      setStills(undefined);
    } catch (err) {
      setError(String(err));
    }
  };

  /** Saves `style` as the style being edited: a copy first when it is built in. */
  const save = (style: Style) => {
    const editing = view.styles.find((s) => s.name === draft.name);
    if (editing?.builtin) {
      const name = freeName(view.styles, `My ${editing.name.toLowerCase()}`);
      setDraft({ name, style });
      saving.current = saving.current
        .then(() => api.styleSave({ name, previous: editing.name, style }))
        .then(async () => {
          setNotice(`${editing.name} is built in, so your change went into a copy, "${name}", which this project now uses.`);
          await useStyle(name);
          stylesChanged();
        })
        .catch((err) => setError(String(err)));
      return;
    }
    if (timer.current) clearTimeout(timer.current);
    timer.current = setTimeout(() => {
      const name = draft.name;
      saving.current = saving.current
        .then(() => api.styleSave({ name, previous: name, style }))
        .then(() => stylesChanged())
        .catch((err) => setError(String(err)));
    }, 350);
  };
  const change = (next: (style: Style) => Style) => {
    const style = next(draft.style);
    setDraft({ ...draft, style });
    setStills(undefined);
    save(style);
  };

  const copy = async () => {
    const name = freeName(view.styles, `${draft.name} copy`);
    try {
      const style = structuredClone(draft.style);
      const own = view.styles.find((s) => s.name === draft.name);
      // Its files go with it, from the style's folder.
      if (own?.folder) {
        const at = (file?: string | null) => (file ? `${own.folder}/${file}` : file);
        style.brand.logo = at(style.brand.logo);
        style.brand.fonts.title.file = at(style.brand.fonts.title.file);
        style.brand.fonts.body.file = at(style.brand.fonts.body.file);
      }
      await api.styleSave({ name, style });
      await useStyle(name);
      stylesChanged();
    } catch (err) {
      setError(String(err));
    }
  };
  const rename = async () => {
    const name = renaming?.trim();
    setRenaming(undefined);
    if (!name || name === draft.name) return;
    try {
      await saving.current;
      await api.styleSave({ name, previous: draft.name, style: draft.style });
      setDraft({ ...draft, name });
      // The project follows a renamed style by its old name; store the new one.
      await useStyle(name);
      stylesChanged();
    } catch (err) {
      setError(String(err));
    }
  };
  const remove = async () => {
    if (!window.confirm(`Remove the style "${draft.name}" for every project? Projects that wear it take Light.`)) return;
    try {
      await api.styleRemove(draft.name);
      await useStyle("Light");
      stylesChanged();
    } catch (err) {
      setError(String(err));
    }
  };
  const pickFile = async (kind: "logo" | "title" | "body") => {
    const path = await api.pickBrandFile(kind === "logo" ? "logo" : "font").catch(() => null);
    if (!path) return;
    change((s) => {
      const next = structuredClone(s);
      if (kind === "logo") next.brand.logo = path;
      else next.brand.fonts[kind].file = path;
      return next;
    });
  };
  const preview = async () => {
    setPreviewing(true);
    try {
      setStills(await api.stylePreview(openedProject.projectHandle, draft.style));
    } catch (err) {
      setError(String(err));
    } finally {
      setPreviewing(false);
    }
  };
  const bringInLine = async () => {
    const project = useProjectStore.getState().openedProject;
    if (!project) return;
    try {
      applyOpenedProject(await api.projectGraphicsInLine(project.projectHandle, project.revision, plans));
    } catch (err) {
      setError(String(err));
    }
  };

  const s = draft.style;
  const editing = view.styles.find((v) => v.name === draft.name);
  const set = <K extends keyof Style>(key: K, value: Partial<Style[K]>) =>
    change((was) => ({ ...was, [key]: { ...was[key], ...value } }));
  const colors = s.brand.colors;
  const setColor = (role: keyof typeof colors, value: string) =>
    set("brand", { colors: { ...colors, [role]: value } });
  const fontRow = (which: "title" | "body", label: string) => {
    const font = s.brand.fonts[which];
    return (
      <div className="space-y-1">
        <span className="text-label text-studio-400">{label}</span>
        <div className="flex items-center gap-1">
          <input
            aria-label={`${label} family`}
            placeholder={which === "body" ? "Same as titles" : "System font"}
            value={font.family}
            onChange={(e) =>
              set("brand", { fonts: { ...s.brand.fonts, [which]: { ...font, family: e.target.value } } })
            }
            className={fieldClass}
          />
          <IconButton icon={FolderOpen} size="sm" label={`Choose a ${which} font file`} onClick={() => void pickFile(which)} />
          {font.file && (
            <IconButton
              icon={Trash2}
              size="sm"
              label={`Forget the font file ${fileName(font.file)}`}
              onClick={() => set("brand", { fonts: { ...s.brand.fonts, [which]: { ...font, file: null } } })}
            />
          )}
        </div>
        {font.file && <p className="text-meta text-studio-500 truncate">File: {fileName(font.file)}</p>}
      </div>
    );
  };

  return (
    <div className="pb-6">
      {error && (
        <Notice tone="danger" onDismiss={() => setError(undefined)}>
          {error}
        </Notice>
      )}
      {notice && <Notice onDismiss={() => setNotice(undefined)}>{notice}</Notice>}
      <div className="p-2 space-y-1.5 border-b border-studio-800">
        <div className="flex items-center gap-1">
          {renaming === undefined ? (
            <select
              aria-label="The project's style"
              value={view.projectStyle ?? ""}
              onChange={(e) => void useStyle(e.target.value)}
              className={fieldClass}
              title="The style this project wears (one undo step to change)"
            >
              {view.styles.map((v) => (
                <option key={v.name} value={v.name}>
                  {v.name}
                  {v.builtin ? " (built in)" : ""}
                </option>
              ))}
            </select>
          ) : (
            <input
              aria-label="The style's new name"
              value={renaming}
              autoFocus
              maxLength={40}
              onChange={(e) => setRenaming(e.target.value)}
              onBlur={() => void rename()}
              onKeyDown={(e) => {
                if (e.key === "Enter") void rename();
                if (e.key === "Escape") setRenaming(undefined);
              }}
              className={fieldClass}
            />
          )}
          <IconButton icon={Copy} size="sm" label="Make a copy of this style" onClick={() => void copy()} />
          {!editing?.builtin && (
            <>
              <IconButton icon={Pencil} size="sm" label="Rename this style" onClick={() => setRenaming(draft.name)} />
              <IconButton icon={Trash2} size="sm" label="Remove this style" onClick={() => void remove()} />
            </>
          )}
          <IconButton icon={FolderOpen} size="sm" label="Open the styles folder" onClick={() => void api.showInFinder(view.folder)} />
        </div>
        <p className="text-meta text-studio-500 leading-relaxed">
          {editing?.builtin
            ? "Built in: a change makes your own copy."
            : "Changes save as you make them, for every project that wears this style."}
        </p>
      </div>

      <InspectorSection id="style-brand" title="Brand" icon={Palette}>
        <div className="grid grid-cols-2 gap-x-3 gap-y-1.5">
          <ColorRow label="Background" value={colors.background} onChange={(v) => setColor("background", v)} />
          <ColorRow label="Text" value={colors.text} onChange={(v) => setColor("text", v)} />
          <ColorRow label="Accent" value={colors.accent} onChange={(v) => setColor("accent", v)} />
          <ColorRow label="Muted" value={colors.muted} onChange={(v) => setColor("muted", v)} />
        </div>
        {fontRow("title", "Titles and captions font")}
        {fontRow("body", "Body font")}
        <div className="flex items-center gap-2">
          <span className="flex-1 text-label text-studio-400 truncate">
            Logo{s.brand.logo ? `: ${fileName(s.brand.logo)}` : ""}
          </span>
          <Button size="sm" variant="ghost" onClick={() => void pickFile("logo")}>
            {s.brand.logo ? "Change" : "Choose…"}
          </Button>
          {s.brand.logo && (
            <IconButton icon={Trash2} size="sm" label="Forget the logo" onClick={() => set("brand", { logo: null })} />
          )}
        </div>
      </InspectorSection>

      <InspectorSection id="style-graphics" title="Graphics" icon={Sparkles}>
        <Segmented
          label="Graphics preset"
          size="sm"
          className="w-full"
          value={s.graphics.preset}
          onChange={(preset) => set("graphics", { preset })}
          options={view.lookPresets.map((p) => ({ value: p.name, label: p.label, title: p.description }))}
        />
        <p className="text-meta text-studio-500 leading-relaxed">
          {view.lookPresets.find((p) => p.name === s.graphics.preset)?.description}
        </p>
        <MoreSettings>
          {LOOK_CONTROLS.map((control) => (
            <RangeRow
              key={control.id}
              label={control.label}
              value={s.graphics[control.id]}
              min={control.min}
              max={control.max}
              step={0.05}
              unit="×"
              onChange={(value) => set("graphics", { [control.id]: value })}
            />
          ))}
        </MoreSettings>
        {stills && stills.length > 0 && (
          <div className="grid grid-cols-3 gap-1">
            {stills.map((still) => (
              <img key={still.assetId} src={still.image} alt={still.name} title={still.name} className="w-full rounded border border-studio-800" />
            ))}
          </div>
        )}
        <div className="flex items-center gap-1.5">
          <span className="flex-1 text-meta text-studio-500">
            {view.own.length > 0
              ? `Their own: ${view.own.map(([name, style, tokens]) => `${name} (${[style, ...tokens].filter(Boolean).join(", ")})`).join("; ")}`
              : "Every graphic follows the style"}
          </span>
          <Button size="sm" variant="ghost" icon={previewing ? Loader2 : Eye} disabled={previewing} onClick={() => void preview()}>
            Preview
          </Button>
        </div>
        {plans.length > 0 && (
          <div className="flex items-center gap-1.5 rounded-control border border-studio-800 bg-studio-850 px-2 py-1.5">
            <span className="flex-1 text-meta text-studio-400" title={plans.map((p) => `${p.name}: ${p.changes.join("; ")}`).join("\n")}>
              {plans.length === 1 ? "1 graphic doesn't" : `${plans.length} graphics don't`} follow the style
            </span>
            <Button size="sm" variant="ghost" icon={Wand2} onClick={() => void bringInLine()}>
              Bring in line
            </Button>
          </div>
        )}
      </InspectorSection>

      <InspectorSection id="style-captions" title="Captions" icon={Captions}>
        <Field label="Words appear">
          <Segmented<CaptionStyle>
            label="Words appear"
            size="sm"
            className="w-full"
            value={s.captions.style}
            onChange={(style) => set("captions", { style })}
            options={CAPTION_STYLES.map(([value, label, title]) => ({ value, label, title }))}
          />
        </Field>
        <MoreSettings>
          <label className="flex items-center justify-between text-label text-studio-400">
            <span>Weight</span>
            <select
              value={s.captions.weight}
              onChange={(e) => set("captions", { weight: Number(e.target.value) })}
              className="bg-studio-800 text-studio-100 text-label rounded px-1.5 py-0.5 border border-studio-700"
            >
              {CAPTION_WEIGHTS.map(([weight, label]) => (
                <option key={weight} value={weight}>
                  {weight === 0 ? "The graphics' titles" : label}
                </option>
              ))}
            </select>
          </label>
        </MoreSettings>
        <ColorRow label="Text colour" value={s.captions.textColor} onChange={(textColor) => set("captions", { textColor })} />
        <Toggle
          label="Emphasis word in colour"
          hint="Each caption's emphasis word, drawn in the emphasis colour (the brand's accent unless set)"
          checked={s.captions.emphasis}
          onChange={(emphasis) => set("captions", { emphasis })}
        />
        <MoreSettings>
          {s.captions.emphasis && (
            <label className="flex items-center justify-between text-label text-studio-400">
              <span>Emphasis colour</span>
              <span className="flex items-center gap-2">
                <button
                  type="button"
                  className="text-meta text-studio-400 hover:text-white hover:underline"
                  onClick={() => set("captions", { emphasisColor: s.captions.emphasisColor ? "" : "#FACC15" })}
                >
                  {s.captions.emphasisColor ? "Use the accent" : "The accent"}
                </button>
                {s.captions.emphasisColor && (
                  <input
                    type="color"
                    value={s.captions.emphasisColor.toLowerCase()}
                    onChange={(e) => set("captions", { emphasisColor: e.target.value.toUpperCase() })}
                    className="w-12 h-6 bg-studio-850 border border-studio-800 rounded cursor-pointer"
                  />
                )}
              </span>
            </label>
          )}
        </MoreSettings>
        <MoreSettings>
          <Toggle label="Outline" checked={s.captions.outline} onChange={(outline) => set("captions", { outline })} />
          <Toggle
            label="Box behind the text"
            checked={s.captions.box.on}
            onChange={(on) => set("captions", { box: { ...s.captions.box, on } })}
          />
          {s.captions.box.on && (
            <>
              <ColorRow label="Box colour" value={s.captions.box.color} onChange={(color) => set("captions", { box: { ...s.captions.box, color } })} />
              <RangeRow
                label="Box opacity"
                value={Math.round(s.captions.box.opacity * 100)}
                min={0}
                max={100}
                unit="%"
                onChange={(pct) => set("captions", { box: { ...s.captions.box, opacity: pct / 100 } })}
              />
            </>
          )}
        </MoreSettings>
      </InspectorSection>

      <InspectorSection id="style-frame" title="Frame" icon={Frame}>
        <BackgroundEditor value={s.frame.background} onChange={(background) => set("frame", { background })} />
        {openedProject.layout?.wallpaperAsset && (
          <p className="text-meta text-studio-500">This project shows its own image instead (Video page).</p>
        )}
        <MoreSettings>
          <RangeRow
            label="Screen corners"
            value={s.frame.screen.cornerRadiusPx}
            min={0}
            max={32}
            unit="px"
            onChange={(cornerRadiusPx) => set("frame", { screen: { ...s.frame.screen, cornerRadiusPx } })}
          />
          <RangeRow
            label="Screen shadow"
            value={s.frame.screen.shadowBlurPx}
            min={0}
            max={40}
            unit="px"
            onChange={(shadowBlurPx) => set("frame", { screen: { ...s.frame.screen, shadowBlurPx } })}
          />
          <RangeRow
            label="Camera border"
            value={s.frame.camera.borderWidth}
            min={0}
            max={8}
            unit="px"
            onChange={(borderWidth) => set("frame", { camera: { ...s.frame.camera, borderWidth } })}
          />
          {s.frame.camera.borderWidth > 0 && (
            <ColorRow
              label="Border colour"
              value={s.frame.camera.borderColor}
              onChange={(borderColor) => set("frame", { camera: { ...s.frame.camera, borderColor } })}
            />
          )}
          <Toggle
            label="Camera shadow"
            checked={s.frame.camera.shadow}
            onChange={(shadow) => set("frame", { camera: { ...s.frame.camera, shadow } })}
          />
        </MoreSettings>
      </InspectorSection>

      <InspectorSection id="style-portrait" title="At 9:16 (shorts)" icon={Smartphone}>
        <MoreSettings>
          <p className="text-meta text-studio-500">Graphics in a short, times the style&apos;s:</p>
          {SHORTS_LOOK_CONTROLS.map((control) => (
            <RangeRow
              key={control.id}
              label={control.label}
              value={s.portrait.graphics[control.id]}
              min={0.5}
              max={2}
              step={0.05}
              unit="×"
              onChange={(value) => set("portrait", { graphics: { ...s.portrait.graphics, [control.id]: value } })}
            />
          ))}
        </MoreSettings>
        <Field label="Words appear">
          <Segmented<"same" | CaptionStyle>
            label="Caption words at 9:16"
            size="sm"
            className="w-full"
            value={s.portrait.captionStyle ?? "same"}
            onChange={(v) => set("portrait", { captionStyle: v === "same" ? null : v })}
            options={[
              { value: "same", label: "Same", title: "As the captions above" },
              ...CAPTION_STYLES.map(([value, label, title]) => ({ value, label, title })),
            ]}
          />
        </Field>
        <Toggle
          label="A background of its own"
          checked={!!s.portrait.background}
          onChange={(own) => set("portrait", { background: own ? { ...s.frame.background } : null })}
        />
        {s.portrait.background && (
          <BackgroundEditor value={s.portrait.background} onChange={(background) => set("portrait", { background })} />
        )}
      </InspectorSection>
    </div>
  );
};
