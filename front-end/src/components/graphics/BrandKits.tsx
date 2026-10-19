import React, { useState } from "react";
import { FolderOpen, Image as ImageIcon, Pencil, Plus, Trash2, Type, X } from "lucide-react";
import { Button, IconButton } from "../ui";
import { useProjectStore } from "../../stores/projectStore";
import { api } from "../../lib/ipc";
import type { BrandKit, BrandKitInput } from "../../lib/types";
import { brandsChanged, useBrands } from "./graphicsEvents";

const fieldClass =
  "w-full min-w-0 bg-studio-950 text-studio-100 text-label rounded px-2 py-1 border border-studio-700 focus:outline-none focus:border-accent-hover";

const fileName = (path: string | null) => (path ? (path.split(/[\\/]/).pop() ?? path) : null);

/// Brand kits: the app's kits (colours, a font, a logo), which one the project's graphics use by
/// default, and a form to make or change one. A kit fills the colours a graphic left alone.
export const BrandKits: React.FC<{ onError: (error: string) => void }> = ({ onError }) => {
  const openedProject = useProjectStore((s) => s.openedProject);
  const applyOpenedProject = useProjectStore((s) => s.applyOpenedProject);
  const brands = useBrands(openedProject?.projectHandle, openedProject?.revision);
  const [editing, setEditing] = useState<BrandKitInput | null>(null);

  const setDefault = async (brand: string | null) => {
    const project = useProjectStore.getState().openedProject;
    if (!project) return;
    try {
      applyOpenedProject(await api.projectGraphicsBrandSet(project.projectHandle, project.revision, brand));
      brandsChanged();
    } catch (err) {
      onError(String(err));
    }
  };
  const remove = async (kit: BrandKit) => {
    if (!window.confirm(`Remove the brand kit "${kit.name}"? Graphics that use it render without it.`)) return;
    try {
      await api.graphicsBrandRemove(kit.name);
      brandsChanged();
    } catch (err) {
      onError(String(err));
    }
  };

  const kits = brands?.kits ?? [];
  const projectDefault = brands?.projectDefault ?? null;
  const missingDefault = projectDefault !== null && !kits.some((k) => k.name === projectDefault);

  return (
    <section className="space-y-1">
      <div className="flex items-center gap-1 px-1">
        <h3 className="flex-1 text-meta font-semibold uppercase tracking-wide text-studio-500">Brand kits</h3>
        {brands && (
          <IconButton
            icon={FolderOpen}
            size="sm"
            label="Open the brand kits folder"
            onClick={() => void api.showInFinder(brands.folder)}
          />
        )}
        <IconButton
          icon={Plus}
          size="sm"
          label="Make a brand kit"
          disabled={editing !== null}
          onClick={() =>
            setEditing({
              name: "",
              background: "#0F172A",
              text: "#F8FAFC",
              accent: "#FACC15",
              font: "",
              fontFile: null,
              logo: null,
            })
          }
        />
      </div>
      {openedProject && (
        <label className="flex items-center gap-2 px-1 text-label text-studio-300">
          <span className="shrink-0">This project uses</span>
          <select
            aria-label="The project's brand kit"
            value={projectDefault ?? ""}
            onChange={(e) => void setDefault(e.target.value || null)}
            className={fieldClass}
          >
            <option value="">No brand kit</option>
            {kits.map((kit) => (
              <option key={kit.name} value={kit.name}>
                {kit.name}
              </option>
            ))}
            {missingDefault && <option value={projectDefault!}>{projectDefault} (removed)</option>}
          </select>
        </label>
      )}
      {kits.length === 0 && !editing && (
        <p className="px-1 text-meta text-studio-500 leading-relaxed">
          A kit holds your colours, font and logo. Graphics take its colours where you have not set your own, so every
          graphic matches without editing each one.
        </p>
      )}
      {kits.map((kit) =>
        editing?.previous === kit.name ? null : (
          <div
            key={kit.name}
            className="group flex items-center gap-2 rounded-control border border-transparent px-2 py-1.5 hover:border-studio-700 hover:bg-studio-850"
          >
            <Swatches kit={kit} />
            <div className="min-w-0 flex-1">
              <div className="truncate text-label text-studio-100">{kit.name}</div>
              <div className="truncate text-meta text-studio-500">
                {[kit.font || "Template fonts", kit.logo ? "logo" : null].filter(Boolean).join(" · ")}
              </div>
            </div>
            <IconButton
              icon={Pencil}
              size="sm"
              label={`Change "${kit.name}"`}
              disabled={editing !== null}
              onClick={() =>
                setEditing({
                  name: kit.name,
                  previous: kit.name,
                  background: kit.background,
                  text: kit.text,
                  accent: kit.accent,
                  font: kit.font,
                  fontFile: kit.fontFile,
                  logo: kit.logo,
                })
              }
            />
            <IconButton icon={Trash2} size="sm" label={`Remove "${kit.name}"`} onClick={() => void remove(kit)} />
          </div>
        ),
      )}
      {editing && <KitForm kit={editing} onDone={() => setEditing(null)} onError={onError} />}
    </section>
  );
};

const Swatches: React.FC<{ kit: Pick<BrandKit, "background" | "text" | "accent"> }> = ({ kit }) => (
  <span
    className="h-7 w-9 shrink-0 rounded-control border border-studio-700 overflow-hidden flex items-end"
    style={{ background: kit.background }}
    aria-hidden
  >
    <span className="h-2 flex-1" style={{ background: kit.accent }} />
    <span className="h-2 flex-1" style={{ background: kit.text }} />
  </span>
);

const KitForm: React.FC<{ kit: BrandKitInput; onDone: () => void; onError: (error: string) => void }> = ({
  kit: initial,
  onDone,
  onError,
}) => {
  const [kit, setKit] = useState(initial);
  const [saving, setSaving] = useState(false);
  const change = (patch: Partial<BrandKitInput>) => setKit((k) => ({ ...k, ...patch }));
  const pick = async (kind: "logo" | "font") => {
    try {
      const path = await api.pickBrandFile(kind);
      if (path) change(kind === "logo" ? { logo: path } : { fontFile: path });
    } catch (err) {
      onError(String(err));
    }
  };
  const save = async () => {
    setSaving(true);
    try {
      await api.graphicsBrandSave({ ...kit, name: kit.name.trim(), previous: initial.previous ?? null });
      brandsChanged();
      onDone();
    } catch (err) {
      onError(String(err));
    } finally {
      setSaving(false);
    }
  };
  const colour = (id: "background" | "text" | "accent", label: string) => (
    <label className="flex flex-col items-center gap-1 text-meta text-studio-400">
      <input
        type="color"
        aria-label={label}
        value={/^#[0-9a-f]{6}$/i.test(kit[id]) ? kit[id] : "#000000"}
        onChange={(e) => change({ [id]: e.target.value.toUpperCase() })}
        className="h-7 w-12 rounded border border-studio-700 bg-studio-950"
      />
      {label}
    </label>
  );
  return (
    <div className="m-1 space-y-2 rounded-panel border border-studio-700 bg-studio-850 p-2.5">
      <input
        aria-label="Kit name"
        placeholder="Kit name"
        value={kit.name}
        autoFocus
        onChange={(e) => change({ name: e.target.value })}
        className={fieldClass}
      />
      <div className="flex items-start justify-around">
        {colour("background", "Background")}
        {colour("text", "Text")}
        {colour("accent", "Accent")}
      </div>
      <input
        aria-label="Font family"
        placeholder="Font family (empty: each graphic's own)"
        value={kit.font}
        onChange={(e) => change({ font: e.target.value })}
        className={fieldClass}
      />
      <FileRow
        icon={Type}
        label="Font file"
        hint="Only if the font is not installed"
        value={kit.fontFile}
        onPick={() => void pick("font")}
        onClear={() => change({ fontFile: null })}
      />
      <FileRow
        icon={ImageIcon}
        label="Logo"
        hint="For graphics that show one"
        value={kit.logo}
        onPick={() => void pick("logo")}
        onClear={() => change({ logo: null })}
      />
      <div className="flex justify-end gap-1.5">
        <Button size="sm" variant="ghost" onClick={onDone}>
          Cancel
        </Button>
        <Button size="sm" variant="primary" disabled={saving || !kit.name.trim()} onClick={() => void save()}>
          {initial.previous ? "Save" : "Make kit"}
        </Button>
      </div>
    </div>
  );
};

const FileRow: React.FC<{
  icon: typeof Type;
  label: string;
  hint: string;
  value: string | null;
  onPick: () => void;
  onClear: () => void;
}> = ({ icon: Icon, label, hint, value, onPick, onClear }) => (
  <div className="flex items-center gap-2 text-label text-studio-300">
    <Icon className="w-3.5 h-3.5 shrink-0 text-studio-500" aria-hidden />
    <span className="min-w-0 flex-1 truncate" title={value ?? hint}>
      {value ? fileName(value) : <span className="text-studio-500">{hint}</span>}
    </span>
    <Button size="sm" variant="ghost" onClick={onPick}>
      {value ? "Change" : `Pick ${label.toLowerCase()}`}
    </Button>
    {value && <IconButton icon={X} size="sm" label={`No ${label.toLowerCase()}`} onClick={onClear} />}
  </div>
);
