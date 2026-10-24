import React, { useEffect, useState } from "react";
import { createPortal } from "react-dom";
import { Keyboard, Plus, RotateCcw, X } from "lucide-react";
import {
  HOTKEY_ACTIONS,
  bindingFromEvent,
  formatBinding,
  useHotkeyStore,
} from "../../stores/hotkeyStore";
import { loadFavourites, useLayoutsStore } from "../../stores/layoutsStore";
import { BUILT_IN_LAYOUTS } from "../../lib/types";

/** One shortcut line: an action, or a layout to switch to. */
interface Row {
  id: string;
  label: string;
  list: string[];
  set: (list: string[]) => void;
  /** Back to the default; layouts have none. */
  reset?: () => void;
}

/** Which binding is waiting for a key press: a slot of a row, or a new one. */
type Capture = { id: string; index: number | "new" };

const LAYOUTS = "Layouts";
/** The categories, in the order the actions list them, then layouts: one tab each. */
const GROUPS = [...new Set(HOTKEY_ACTIONS.map((a) => a.group)), LAYOUTS];
/** Rows in the largest category: every tab is that tall, so switching never moves the dialog. */
const MOST_ROWS = Math.max(...GROUPS.map((g) => HOTKEY_ACTIONS.filter((a) => a.group === g).length));

/// Keyboard shortcut settings, a tab per category: click a shortcut and press the new keys, +
/// adds another key for the same action, × removes one. A key taken from another action moves
/// here. The Layouts tab binds keys that switch the layout from the playhead (LAY-2).
export const HotkeysDialog: React.FC<{ onClose: () => void }> = ({ onClose }) => {
  const bindings = useHotkeyStore((s) => s.bindings);
  const layoutKeys = useHotkeyStore((s) => s.layoutKeys);
  const setBindings = useHotkeyStore((s) => s.setBindings);
  const setLayoutKeys = useHotkeyStore((s) => s.setLayoutKeys);
  const reset = useHotkeyStore((s) => s.reset);
  const favourites = useLayoutsStore((s) => s.favourites);
  const [capture, setCapture] = useState<Capture | null>(null);
  const [notice, setNotice] = useState<string>();
  const [group, setGroup] = useState(GROUPS[0]);

  useEffect(() => {
    void loadFavourites();
  }, []);

  const rows: (Row & { group: string })[] = [
    ...HOTKEY_ACTIONS.map(({ action, label, group }) => ({
      id: action,
      label,
      group,
      list: bindings[action],
      set: (list: string[]) => setBindings(action, list),
      reset: () => reset(action),
    })),
    ...[
      ...BUILT_IN_LAYOUTS.map((b) => ({ id: `builtin:${b.name}`, label: b.name })),
      ...favourites.map((f) => ({ id: `fav:${f.id}`, label: `${f.name} (favourite)` })),
    ].map(({ id, label }) => ({
      id,
      label,
      group: LAYOUTS,
      list: layoutKeys[id] ?? [],
      set: (list: string[]) => setLayoutKeys(id, list),
    })),
  ];

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      // While the dialog is open, keys set shortcuts instead of editing the timeline.
      event.preventDefault();
      event.stopImmediatePropagation();
      if (!capture) {
        if (event.key === "Escape") onClose();
        return;
      }
      if (event.key === "Escape" && !event.ctrlKey && !event.metaKey && !event.shiftKey && !event.altKey) {
        setCapture(null);
        return;
      }
      const binding = bindingFromEvent(event);
      if (!binding) return; // A modifier on its own: wait for the key it goes with.
      const row = rows.find((r) => r.id === capture.id);
      if (!row) return;
      const owner = rows.find((r) => r.id !== row.id && r.list.includes(binding));
      const list = [...row.list];
      if (capture.index === "new") list.push(binding);
      else list[capture.index] = binding;
      row.set(list);
      setNotice(owner ? `${formatBinding(binding)} was moved here from “${owner.label}”.` : undefined);
      setCapture(null);
    };
    // Capture phase, so the timeline's own listener never sees these keys.
    window.addEventListener("keydown", onKeyDown, true);
    return () => window.removeEventListener("keydown", onKeyDown, true);
  });

  const chip = (active: boolean) =>
    `h-6 px-2 rounded border font-mono text-meta transition-colors ${
      active
        ? "border-accent-hover bg-accent/30 text-white animate-pulse"
        : "border-studio-700 bg-studio-850 text-studio-200 hover:border-studio-500"
    }`;

  const shown = rows.filter((r) => r.group === group);

  // On the body: an ancestor with a backdrop filter (the top bar) would otherwise become the
  // box `fixed` positions against, and the dialog would be pinned to it and cut off.
  return createPortal(
    <div
      className="fixed inset-0 z-50 bg-black/70 flex items-center justify-center p-4 select-none"
      onPointerDown={(event) => {
        if (event.target === event.currentTarget) onClose();
      }}
    >
      <div
        role="dialog"
        aria-modal="true"
        aria-label="Keyboard shortcuts"
        className="w-full max-w-xl flex flex-col rounded-panel border border-studio-700 bg-studio-900 shadow-dialog"
      >
        <div className="flex items-center justify-between px-4 py-3 border-b border-studio-800">
          <div className="flex items-center gap-2 text-heading text-studio-100">
            <Keyboard className="w-4 h-4 text-studio-400" />
            Keyboard shortcuts
          </div>
          <div className="flex items-center gap-1">
            <button
              type="button"
              onClick={() => {
                reset();
                setNotice("All shortcuts are back to their defaults.");
              }}
              className="flex items-center gap-1 px-2 py-1 rounded text-meta text-studio-400 hover:text-white hover:bg-studio-800"
            >
              <RotateCcw className="w-3 h-3" />
              Reset all
            </button>
            <button
              type="button"
              aria-label="Close"
              onClick={onClose}
              className="p-1 rounded text-studio-400 hover:text-white hover:bg-studio-800"
            >
              <X className="w-4 h-4" />
            </button>
          </div>
        </div>

        <p className="px-4 pt-3 text-meta text-studio-500">
          {capture
            ? "Press the new keys now. Esc cancels."
            : group === LAYOUTS
              ? "A layout key uses that layout from the playhead to the next change. Pressed while playing, it shows at once, and the presses undo as one step."
              : "Click a shortcut to change it, + to add another key for the same action."}
          {notice && <span className="block mt-1 text-suggest-fg">{notice}</span>}
        </p>

        <div role="tablist" aria-label="Shortcut categories" className="flex gap-1 px-4 pt-3">
          {GROUPS.map((g) => {
            const count = rows.filter((r) => r.group === g).length;
            return (
              <button
                key={g}
                type="button"
                role="tab"
                aria-selected={g === group}
                onClick={() => {
                  setGroup(g);
                  setCapture(null);
                }}
                className={`px-3 py-1.5 rounded-control text-label font-medium transition-colors ${
                  g === group ? "bg-accent text-white" : "text-studio-400 hover:text-white hover:bg-studio-800"
                }`}
              >
                {g}
                <span className={`ml-1.5 text-meta ${g === group ? "text-white/70" : "text-studio-600"}`}>{count}</span>
              </button>
            );
          })}
        </div>

        <div role="tabpanel" aria-label={group} className="px-4 py-3">
          <section>
            <div
              className="divide-y divide-studio-800/60 overflow-y-auto"
              style={{ minHeight: `${MOST_ROWS * 2.25}rem`, maxHeight: `${MOST_ROWS * 2.25}rem` }}
            >
              {shown.map(({ id, label, list, set, reset: resetRow }) => (
                <div key={id} className="h-9 flex items-center justify-between gap-3">
                  <span className="text-xs text-studio-300 truncate">{label}</span>
                  <div className="flex flex-wrap items-center justify-end gap-1 shrink-0">
                    {list.length === 0 && capture?.id !== id && <span className="text-meta text-studio-600">None</span>}
                    {list.map((binding, index) => {
                      const active = capture?.id === id && capture.index === index;
                      return (
                        <span key={binding} className="group relative">
                          <button
                            type="button"
                            onClick={() => setCapture({ id, index })}
                            className={chip(active)}
                            title="Click, then press the new keys"
                          >
                            {active ? "Press keys…" : formatBinding(binding)}
                          </button>
                          {!active && (
                            <button
                              type="button"
                              aria-label={`Remove ${formatBinding(binding)} from ${label}`}
                              onClick={() => set(list.filter((b) => b !== binding))}
                              className="absolute -top-1.5 -right-1.5 hidden group-hover:flex w-3.5 h-3.5 items-center justify-center rounded-full bg-studio-700 text-studio-200 hover:bg-danger"
                            >
                              <X className="w-2.5 h-2.5" />
                            </button>
                          )}
                        </span>
                      );
                    })}
                    {capture?.id === id && capture.index === "new" ? (
                      <span className={chip(true)}>Press keys…</span>
                    ) : (
                      <button
                        type="button"
                        aria-label={`Add a shortcut for ${label}`}
                        onClick={() => setCapture({ id, index: "new" })}
                        className="h-6 w-6 flex items-center justify-center rounded border border-dashed border-studio-700 text-studio-500 hover:text-white hover:border-studio-500"
                      >
                        <Plus className="w-3 h-3" />
                      </button>
                    )}
                    {resetRow ? (
                      <button
                        type="button"
                        aria-label={`Reset ${label}`}
                        title="Back to the default"
                        onClick={resetRow}
                        className="h-6 w-6 flex items-center justify-center rounded text-studio-600 hover:text-white hover:bg-studio-800"
                      >
                        <RotateCcw className="w-3 h-3" />
                      </button>
                    ) : (
                      <span className="h-6 w-6" aria-hidden />
                    )}
                  </div>
                </div>
              ))}
            </div>
          </section>
        </div>
      </div>
    </div>,
    document.body,
  );
};
