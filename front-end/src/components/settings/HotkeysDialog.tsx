import React, { useEffect, useState } from "react";
import { createPortal } from "react-dom";
import { Keyboard, Plus, RotateCcw, X } from "lucide-react";
import {
  HOTKEY_ACTIONS,
  HotkeyAction,
  bindingFromEvent,
  formatBinding,
  useHotkeyStore,
} from "../../stores/hotkeyStore";

/** Which binding is waiting for a key press: a slot of an action, or a new one. */
type Capture = { action: HotkeyAction; index: number | "new" };

/// Keyboard shortcut settings: click a shortcut and press the new keys, + adds another key
/// for the same action, × removes one. A key taken from another action moves here.
export const HotkeysDialog: React.FC<{ onClose: () => void }> = ({ onClose }) => {
  const bindings = useHotkeyStore((s) => s.bindings);
  const setBindings = useHotkeyStore((s) => s.setBindings);
  const reset = useHotkeyStore((s) => s.reset);
  const [capture, setCapture] = useState<Capture | null>(null);
  const [notice, setNotice] = useState<string>();

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
      const owner = HOTKEY_ACTIONS.find(
        ({ action }) => action !== capture.action && bindings[action].includes(binding),
      );
      const list = [...bindings[capture.action]];
      if (capture.index === "new") list.push(binding);
      else list[capture.index] = binding;
      setBindings(capture.action, list);
      setNotice(owner ? `${formatBinding(binding)} was moved here from “${owner.label}”.` : undefined);
      setCapture(null);
    };
    // Capture phase, so the timeline's own listener never sees these keys.
    window.addEventListener("keydown", onKeyDown, true);
    return () => window.removeEventListener("keydown", onKeyDown, true);
  }, [capture, bindings, setBindings, onClose]);

  const groups = [...new Set(HOTKEY_ACTIONS.map((a) => a.group))];
  const chip = (active: boolean) =>
    `h-6 px-2 rounded border font-mono text-meta transition-colors ${
      active
        ? "border-accent-hover bg-accent/30 text-white animate-pulse"
        : "border-studio-700 bg-studio-850 text-studio-200 hover:border-studio-500"
    }`;

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
        className="w-full max-w-xl max-h-[85vh] flex flex-col rounded-panel border border-studio-700 bg-studio-900 shadow-dialog"
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
            : "Click a shortcut to change it, + to add another key for the same action."}
          {notice && <span className="block mt-1 text-suggest-fg">{notice}</span>}
        </p>

        <div className="flex-1 min-h-0 overflow-y-auto px-4 py-3 space-y-4">
          {groups.map((group) => (
            <section key={group}>
              <h3 className="mb-1.5 text-label font-semibold text-studio-300">{group}</h3>
              <div className="divide-y divide-studio-800/60">
                {HOTKEY_ACTIONS.filter((a) => a.group === group).map(({ action, label }) => {
                  const list = bindings[action];
                  return (
                    <div key={action} className="flex items-center justify-between gap-3 py-1.5">
                      <span className="text-xs text-studio-300">{label}</span>
                      <div className="flex flex-wrap items-center justify-end gap-1">
                        {list.length === 0 && capture?.action !== action && (
                          <span className="text-meta text-studio-600">None</span>
                        )}
                        {list.map((binding, index) => {
                          const active = capture?.action === action && capture.index === index;
                          return (
                            <span key={binding} className="group relative">
                              <button
                                type="button"
                                onClick={() => setCapture({ action, index })}
                                className={chip(active)}
                                title="Click, then press the new keys"
                              >
                                {active ? "Press keys…" : formatBinding(binding)}
                              </button>
                              {!active && (
                                <button
                                  type="button"
                                  aria-label={`Remove ${formatBinding(binding)} from ${label}`}
                                  onClick={() => setBindings(action, list.filter((b) => b !== binding))}
                                  className="absolute -top-1.5 -right-1.5 hidden group-hover:flex w-3.5 h-3.5 items-center justify-center rounded-full bg-studio-700 text-studio-200 hover:bg-danger"
                                >
                                  <X className="w-2.5 h-2.5" />
                                </button>
                              )}
                            </span>
                          );
                        })}
                        {capture?.action === action && capture.index === "new" ? (
                          <span className={chip(true)}>Press keys…</span>
                        ) : (
                          <button
                            type="button"
                            aria-label={`Add a shortcut for ${label}`}
                            onClick={() => setCapture({ action, index: "new" })}
                            className="h-6 w-6 flex items-center justify-center rounded border border-dashed border-studio-700 text-studio-500 hover:text-white hover:border-studio-500"
                          >
                            <Plus className="w-3 h-3" />
                          </button>
                        )}
                        <button
                          type="button"
                          aria-label={`Reset ${label}`}
                          title="Back to the default"
                          onClick={() => reset(action)}
                          className="h-6 w-6 flex items-center justify-center rounded text-studio-600 hover:text-white hover:bg-studio-800"
                        >
                          <RotateCcw className="w-3 h-3" />
                        </button>
                      </div>
                    </div>
                  );
                })}
              </div>
            </section>
          ))}
        </div>
      </div>
    </div>,
    document.body,
  );
};
