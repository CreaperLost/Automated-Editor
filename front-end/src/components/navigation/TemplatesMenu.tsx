import React, { useEffect, useState } from "react";
import { Bookmark, BookmarkPlus, Check, Trash2 } from "lucide-react";
import { api } from "../../lib/ipc";
import type { Template } from "../../lib/types";
import { useProjectStore } from "../../stores/projectStore";
import { shortOp, useSelectedShort } from "../../stores/shortsStore";
import { useWorkspaceStore } from "../../stores/workspaceStore";
import { Button, Menu, MenuButton, type MenuEntry } from "../ui";

/// Saved settings any project can take on: the video's look and base layout, captions, zooms
/// and how layouts are picked, and in Shorts the picked short's look too. Applying one is one undo step.
export const TemplatesMenu: React.FC<{ onError: (message: string) => void }> = ({ onError }) => {
  const project = useProjectStore((s) => s.openedProject);
  const inShorts = useWorkspaceStore((s) => s.workspace === "shorts" || s.workspace === "clips");
  const short = useSelectedShort();
  const [templates, setTemplates] = useState<Template[]>([]);
  const [naming, setNaming] = useState(false);
  const [notice, setNotice] = useState<string>();

  useEffect(() => {
    void api.templatesList().then(setTemplates).catch(() => undefined);
  }, []);
  useEffect(() => {
    if (!notice) return;
    const timer = window.setTimeout(() => setNotice(undefined), 2500);
    return () => window.clearTimeout(timer);
  }, [notice]);

  if (!project) return null;
  // A short's look goes with the template only from Shorts, where it is picked.
  const shortId = inShorts ? short?.id ?? null : null;

  const apply = (template: Template) => {
    const withShort = shortId && template.shortLook ? shortId : null;
    const done = () => setNotice(`Applied "${template.name}"`);
    if (inShorts) {
      // After any look change of the short still waiting to be saved, so it can't undo this.
      shortOp("Applying a template", async (p) => {
        const next = await api.templateApply(p.projectHandle, p.revision, template.id, withShort);
        done();
        return next;
      });
      return;
    }
    const current = useProjectStore.getState().openedProject;
    if (!current) return;
    void api
      .templateApply(current.projectHandle, current.revision, template.id, null)
      .then((next) => {
        useProjectStore.getState().applyOpenedProject(next);
        done();
      })
      .catch((err) => onError(String(err)));
  };

  const remove = (template: Template) =>
    void api
      .templateDelete(template.id)
      .then(setTemplates)
      .catch((err) => onError(String(err)));

  const describe = (template: Template) =>
    template.shortLook ? "Video and short look" : "Video settings";

  const entries: MenuEntry[] = [
    {
      label: shortId ? "Save settings and this short's look…" : "Save current settings…",
      icon: BookmarkPlus,
      onSelect: () => setNaming(true),
    },
    { kind: "separator" },
    { kind: "heading", label: templates.length ? "Apply" : "No templates saved yet" },
    ...templates.map(
      (template): MenuEntry => ({
        label: template.name,
        icon: Bookmark,
        hint: describe(template),
        onSelect: () => apply(template),
      }),
    ),
    ...(templates.length
      ? ([
          { kind: "separator" },
          {
            kind: "submenu",
            label: "Delete",
            icon: Trash2,
            entries: templates.map(
              (template): MenuEntry => ({ label: template.name, danger: true, onSelect: () => remove(template) }),
            ),
          },
        ] as MenuEntry[])
      : []),
  ];

  return (
    <>
      <Menu
        label="Templates"
        entries={entries}
        width={300}
        trigger={({ open, toggle, ref }) => (
          <MenuButton ref={ref} open={open} onClick={toggle}>
            {notice ? (
              <span className="inline-flex items-center gap-1 text-audio-fg">
                <Check className="w-3.5 h-3.5" /> {notice}
              </span>
            ) : (
              "Templates"
            )}
          </MenuButton>
        )}
      />
      {naming && (
        <SaveTemplateDialog
          templates={templates}
          withShort={!!shortId}
          onClose={() => setNaming(false)}
          onSave={(name) => {
            setNaming(false);
            void api
              .templateSave(project.projectHandle, name, shortId)
              .then((list) => {
                setTemplates(list);
                setNotice(`Saved "${name}"`);
              })
              .catch((err) => onError(String(err)));
          }}
        />
      )}
    </>
  );
};

const SaveTemplateDialog: React.FC<{
  templates: Template[];
  withShort: boolean;
  onClose: () => void;
  onSave: (name: string) => void;
}> = ({ templates, withShort, onClose, onSave }) => {
  const [name, setName] = useState("");
  const trimmed = name.trim();
  const replaces = templates.some((t) => t.name.toLowerCase() === trimmed.toLowerCase());
  return (
    <div
      className="fixed inset-0 z-50 bg-black/70 flex items-center justify-center p-4"
      onPointerDown={(event) => {
        if (event.target === event.currentTarget) onClose();
      }}
    >
      <form
        role="dialog"
        aria-modal="true"
        aria-label="Save settings as a template"
        className="w-full max-w-sm flex flex-col gap-3 rounded-panel border border-studio-700 bg-studio-900 shadow-dialog p-4"
        onSubmit={(event) => {
          event.preventDefault();
          if (trimmed) onSave(trimmed);
        }}
        onKeyDown={(event) => {
          if (event.key === "Escape") onClose();
        }}
      >
        <h2 className="text-body font-semibold text-studio-100">Save as a template</h2>
        <p className="text-label text-studio-400 leading-relaxed">
          Keeps the canvas and camera style, the base layout, captions, zoom and layout picking settings
          {withShort ? ", and this short's look," : ""} for any project. Pick it from Templates to use it again.
        </p>
        <input
          autoFocus
          aria-label="Template name"
          placeholder="Name, e.g. Tutorial"
          maxLength={60}
          value={name}
          onChange={(e) => setName(e.target.value)}
          className="ui-field"
        />
        {replaces && <p className="text-meta text-studio-400">Replaces the template of that name.</p>}
        <div className="flex justify-end gap-2">
          <Button type="button" variant="ghost" onClick={onClose}>
            Cancel
          </Button>
          <Button type="submit" variant="primary" disabled={!trimmed}>
            Save
          </Button>
        </div>
      </form>
    </div>
  );
};
