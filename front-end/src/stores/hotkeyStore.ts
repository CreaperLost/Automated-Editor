import { create } from "zustand";

/**
 * Editor keyboard shortcuts. A binding is "Mod+Shift+Alt+<KeyboardEvent.code>" (modifiers in
 * that order, each optional; Mod is Ctrl, or Cmd on a Mac). Codes name physical keys, so a
 * shortcut stays on the same key whatever the keyboard layout (S is "KeyS" on a Greek layout too).
 */
export type HotkeyAction =
  | "playPause"
  | "split"
  | "rippleTrimPrevious"
  | "rippleTrimNext"
  | "deleteSelection"
  | "rippleDelete"
  | "deselect"
  | "toggleLink"
  | "markIn"
  | "markOut"
  | "selectAll"
  | "deselectAll"
  | "undo"
  | "redo"
  | "stepBack"
  | "stepForward"
  | "stepBackLong"
  | "stepForwardLong"
  | "previousEdit"
  | "nextEdit"
  | "zoomIn"
  | "zoomOut"
  | "goToStart"
  | "goToEnd";

export const HOTKEY_ACTIONS: { action: HotkeyAction; label: string; group: string; defaults: string[] }[] = [
  { action: "playPause", label: "Play / pause", group: "Playback", defaults: ["Space"] },
  { action: "stepBack", label: "Back one frame", group: "Playback", defaults: ["ArrowLeft"] },
  { action: "stepForward", label: "Forward one frame", group: "Playback", defaults: ["ArrowRight"] },
  { action: "stepBackLong", label: "Back one second", group: "Playback", defaults: ["Shift+ArrowLeft"] },
  { action: "stepForwardLong", label: "Forward one second", group: "Playback", defaults: ["Shift+ArrowRight"] },
  { action: "previousEdit", label: "Previous edit point", group: "Playback", defaults: ["ArrowUp"] },
  { action: "nextEdit", label: "Next edit point", group: "Playback", defaults: ["ArrowDown"] },
  { action: "goToStart", label: "Go to start", group: "Playback", defaults: ["Home"] },
  { action: "goToEnd", label: "Go to end", group: "Playback", defaults: ["End"] },
  { action: "split", label: "Split at playhead", group: "Editing", defaults: ["KeyS"] },
  { action: "rippleTrimPrevious", label: "Ripple trim to previous edit", group: "Editing", defaults: ["KeyQ"] },
  { action: "rippleTrimNext", label: "Ripple trim to next edit", group: "Editing", defaults: ["KeyE"] },
  { action: "deleteSelection", label: "Delete selection (closes up when Magnetic)", group: "Editing", defaults: ["Delete", "Backspace"] },
  { action: "rippleDelete", label: "Ripple delete selection", group: "Editing", defaults: ["Shift+Delete", "Shift+Backspace"] },
  { action: "toggleLink", label: "Link / unlink clips", group: "Editing", defaults: ["KeyU"] },
  { action: "undo", label: "Undo", group: "Editing", defaults: ["Mod+KeyZ"] },
  { action: "redo", label: "Redo", group: "Editing", defaults: ["Mod+Shift+KeyZ", "Mod+KeyY"] },
  { action: "markIn", label: "Mark in (range start)", group: "Selection", defaults: ["KeyI"] },
  { action: "markOut", label: "Mark out (range end)", group: "Selection", defaults: ["KeyO"] },
  { action: "selectAll", label: "Select all clips", group: "Selection", defaults: ["Mod+KeyA"] },
  { action: "deselectAll", label: "Deselect all", group: "Selection", defaults: ["Mod+KeyD"] },
  { action: "deselect", label: "Clear range and selection", group: "Selection", defaults: ["Escape"] },
  { action: "zoomIn", label: "Zoom timeline in", group: "View", defaults: ["Equal", "Shift+Equal", "NumpadAdd"] },
  { action: "zoomOut", label: "Zoom timeline out", group: "View", defaults: ["Minus", "NumpadSubtract"] },
];

const STORAGE_KEY = "aeroedits.hotkeys.v1";
const MODIFIER_CODES = new Set([
  "ShiftLeft",
  "ShiftRight",
  "ControlLeft",
  "ControlRight",
  "AltLeft",
  "AltRight",
  "MetaLeft",
  "MetaRight",
]);

type Bindings = Record<HotkeyAction, string[]>;

function defaultBindings(): Bindings {
  return Object.fromEntries(HOTKEY_ACTIONS.map(({ action, defaults }) => [action, [...defaults]])) as Bindings;
}

/** Saved bindings over the defaults; actions added in later versions keep their defaults. */
function loadBindings(): Bindings {
  const bindings = defaultBindings();
  try {
    const saved = JSON.parse(window.localStorage.getItem(STORAGE_KEY) ?? "{}") as Partial<Bindings>;
    for (const { action } of HOTKEY_ACTIONS) {
      const value = saved[action];
      if (Array.isArray(value) && value.every((b) => typeof b === "string")) bindings[action] = value;
    }
  } catch {
    // Unreadable or unavailable storage: the defaults apply.
  }
  return bindings;
}

function saveBindings(bindings: Bindings) {
  try {
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify(bindings));
  } catch {
    // Not remembered on this machine; the change still applies for this session.
  }
}

/** The binding a key press makes, or null for a lone modifier. */
export function bindingFromEvent(
  event: Pick<KeyboardEvent, "code" | "ctrlKey" | "metaKey" | "shiftKey" | "altKey">,
): string | null {
  if (!event.code || MODIFIER_CODES.has(event.code)) return null;
  const parts: string[] = [];
  if (event.ctrlKey || event.metaKey) parts.push("Mod");
  if (event.shiftKey) parts.push("Shift");
  if (event.altKey) parts.push("Alt");
  parts.push(event.code);
  return parts.join("+");
}

const isMac = typeof navigator !== "undefined" && /Mac|iPhone|iPad/.test(navigator.platform);
const CODE_LABELS: Record<string, string> = {
  Space: "Space",
  ArrowLeft: "←",
  ArrowRight: "→",
  ArrowUp: "↑",
  ArrowDown: "↓",
  Equal: "=",
  Minus: "-",
  NumpadAdd: "Num +",
  NumpadSubtract: "Num -",
  Backspace: "Backspace",
  Delete: "Delete",
  Escape: "Esc",
  BracketLeft: "[",
  BracketRight: "]",
  Comma: ",",
  Period: ".",
  Slash: "/",
  Semicolon: ";",
  Quote: "'",
  Backquote: "`",
  Backslash: "\\",
};

/** "Ctrl+Shift+Z" for "Mod+Shift+KeyZ". */
export function formatBinding(binding: string): string {
  return binding
    .split("+")
    .map((part) => {
      if (part === "Mod") return isMac ? "⌘" : "Ctrl";
      if (part === "Shift" || part === "Alt") return isMac && part === "Alt" ? "⌥" : part;
      if (part.startsWith("Key")) return part.slice(3);
      if (part.startsWith("Digit")) return part.slice(5);
      if (part.startsWith("Numpad") && part.length === 7) return `Num ${part.slice(6)}`;
      return CODE_LABELS[part] ?? part;
    })
    .join("+");
}

interface HotkeyStore {
  bindings: Bindings;
  /** Sets one action's bindings; any of them taken by another action is removed from it. */
  setBindings: (action: HotkeyAction, bindings: string[]) => void;
  reset: (action?: HotkeyAction) => void;
  /** The action a key press triggers, if any. */
  actionFor: (event: KeyboardEvent) => HotkeyAction | null;
}

export const useHotkeyStore = create<HotkeyStore>((set, get) => ({
  bindings: loadBindings(),
  setBindings: (action, list) =>
    set((state) => {
      const unique = [...new Set(list)];
      const next = { ...state.bindings };
      for (const other of Object.keys(next) as HotkeyAction[]) {
        if (other !== action) next[other] = next[other].filter((b) => !unique.includes(b));
      }
      next[action] = unique;
      saveBindings(next);
      return { bindings: next };
    }),
  reset: (action) =>
    set((state) => {
      const defaults = defaultBindings();
      let next: Bindings;
      if (action) {
        next = { ...state.bindings };
        for (const other of Object.keys(next) as HotkeyAction[]) {
          if (other !== action) next[other] = next[other].filter((b) => !defaults[action].includes(b));
        }
        next[action] = defaults[action];
      } else {
        next = defaults;
      }
      saveBindings(next);
      return { bindings: next };
    }),
  actionFor: (event) => {
    const binding = bindingFromEvent(event);
    if (!binding) return null;
    const { bindings } = get();
    return (Object.keys(bindings) as HotkeyAction[]).find((action) => bindings[action].includes(binding)) ?? null;
  },
}));

/** The first binding of `action`, formatted for a tooltip ("" when it has none). */
export function hotkeyHint(bindings: Bindings, action: HotkeyAction): string {
  const first = bindings[action][0];
  return first ? ` (${formatBinding(first)})` : "";
}
