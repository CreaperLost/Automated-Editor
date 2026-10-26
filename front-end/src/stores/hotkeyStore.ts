import { create } from "zustand";

/**
 * Editor keyboard shortcuts. A binding is "Mod+Shift+Alt+<KeyboardEvent.code>" (modifiers in
 * that order, each optional; Mod is Ctrl, or Cmd on a Mac). Codes name physical keys, so a
 * shortcut stays on the same key whatever the keyboard layout (S is "KeyS" on a Greek layout too).
 */
export type HotkeyAction =
  | "playPause"
  | "shuttleForward"
  | "pause"
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
  { action: "shuttleForward", label: "Play; press again for 2x", group: "Playback", defaults: ["KeyL"] },
  { action: "pause", label: "Stop", group: "Playback", defaults: ["KeyK"] },
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

/**
 * Keys that switch layouts (LAY-2), by layout: `builtin:<name>` or `fav:<favourite id>`. None
 * by default: people bind the layouts they use (George, 21 Oct).
 */
export type LayoutKeys = Record<string, string[]>;
const LAYOUT_KEYS_STORAGE_KEY = "aeroedits.layoutKeys.v1";

function loadLayoutKeys(): LayoutKeys {
  try {
    const saved = JSON.parse(window.localStorage.getItem(LAYOUT_KEYS_STORAGE_KEY) ?? "{}") as unknown;
    if (!saved || typeof saved !== "object") return {};
    return Object.fromEntries(
      Object.entries(saved as Record<string, unknown>).filter(
        (entry): entry is [string, string[]] => Array.isArray(entry[1]) && entry[1].every((b) => typeof b === "string"),
      ),
    );
  } catch {
    return {};
  }
}

function saveLayoutKeys(keys: LayoutKeys) {
  try {
    window.localStorage.setItem(LAYOUT_KEYS_STORAGE_KEY, JSON.stringify(keys));
  } catch {
    // Not remembered on this machine; the change still applies for this session.
  }
}

/** `keys` without `taken`, and without layouts left with no key. */
function withoutKeys(keys: LayoutKeys, taken: string[], except?: string): LayoutKeys {
  const next: LayoutKeys = {};
  for (const [id, list] of Object.entries(keys)) {
    const kept = id === except ? list : list.filter((b) => !taken.includes(b));
    if (kept.length > 0) next[id] = kept;
  }
  return next;
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
  layoutKeys: LayoutKeys;
  /** Sets one action's bindings; any of them taken by another action or layout is removed from it. */
  setBindings: (action: HotkeyAction, bindings: string[]) => void;
  /** Sets the keys of one layout, taking them from any action or other layout. */
  setLayoutKeys: (layout: string, bindings: string[]) => void;
  /** One action back to its defaults, or every shortcut (layouts then have no keys). */
  reset: (action?: HotkeyAction) => void;
  /** The action a key press triggers, if any. */
  actionFor: (event: KeyboardEvent) => HotkeyAction | null;
  /** The layout a key press switches to, if any. */
  layoutFor: (event: KeyboardEvent) => string | null;
}

/** `bindings` without `taken`, except on `except`. */
function withoutBindings(bindings: Bindings, taken: string[], except?: HotkeyAction): Bindings {
  const next = { ...bindings };
  for (const other of Object.keys(next) as HotkeyAction[]) {
    if (other !== except) next[other] = next[other].filter((b) => !taken.includes(b));
  }
  return next;
}

export const useHotkeyStore = create<HotkeyStore>((set, get) => ({
  bindings: loadBindings(),
  layoutKeys: loadLayoutKeys(),
  setBindings: (action, list) =>
    set((state) => {
      const unique = [...new Set(list)];
      const next = { ...withoutBindings(state.bindings, unique, action), [action]: unique };
      const layoutKeys = withoutKeys(state.layoutKeys, unique);
      saveBindings(next);
      saveLayoutKeys(layoutKeys);
      return { bindings: next, layoutKeys };
    }),
  setLayoutKeys: (layout, list) =>
    set((state) => {
      const unique = [...new Set(list)];
      const bindings = withoutBindings(state.bindings, unique);
      const layoutKeys = withoutKeys({ ...state.layoutKeys, [layout]: unique }, unique, layout);
      if (unique.length === 0) delete layoutKeys[layout];
      saveBindings(bindings);
      saveLayoutKeys(layoutKeys);
      return { bindings, layoutKeys };
    }),
  reset: (action) =>
    set((state) => {
      const defaults = defaultBindings();
      let next: Bindings;
      let layoutKeys: LayoutKeys = {};
      if (action) {
        next = { ...withoutBindings(state.bindings, defaults[action], action), [action]: defaults[action] };
        layoutKeys = withoutKeys(state.layoutKeys, defaults[action]);
      } else {
        next = defaults;
      }
      saveBindings(next);
      saveLayoutKeys(layoutKeys);
      return { bindings: next, layoutKeys };
    }),
  actionFor: (event) => {
    const binding = bindingFromEvent(event);
    if (!binding) return null;
    const { bindings } = get();
    return (Object.keys(bindings) as HotkeyAction[]).find((action) => bindings[action].includes(binding)) ?? null;
  },
  layoutFor: (event) => {
    const binding = bindingFromEvent(event);
    if (!binding) return null;
    const { layoutKeys } = get();
    return Object.keys(layoutKeys).find((layout) => layoutKeys[layout].includes(binding)) ?? null;
  },
}));

/** The first binding of `action`, formatted for a tooltip ("" when it has none). */
export function hotkeyHint(bindings: Bindings, action: HotkeyAction): string {
  const first = bindings[action][0];
  return first ? ` (${formatBinding(first)})` : "";
}
