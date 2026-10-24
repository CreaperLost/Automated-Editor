import { useCallback, useEffect, useRef, useState } from "react";
import { api, isTauriEnvironment } from "../../lib/ipc";
import type { GraphicLibraryItem, StylesView } from "../../lib/types";

/** Calls `onUpdate` (throttled) whenever a graphic render moves along, finishes or fails. */
export function useGraphicsUpdates(onUpdate: () => void) {
  const latest = useRef(onUpdate);
  latest.current = onUpdate;
  useEffect(() => {
    if (!isTauriEnvironment()) return;
    let unlisten: (() => void) | undefined;
    let disposed = false;
    let timer: ReturnType<typeof setTimeout> | undefined;
    void import("@tauri-apps/api/event").then(({ listen }) =>
      listen("aeroedits-graphics-updated", () => {
        // Progress comes many times a second; a few refreshes a second are enough.
        if (timer) return;
        timer = setTimeout(() => {
          timer = undefined;
          latest.current();
        }, 300);
      }).then((fn) => {
        if (disposed) fn();
        else unlisten = fn;
      }),
    );
    return () => {
      disposed = true;
      if (timer) clearTimeout(timer);
      unlisten?.();
    };
  }, []);
}

export const formatSeconds = (us: number) => `${(us / 1e6).toFixed(1)}s`;

const LIBRARY_CHANGED = "aeroedits-graphics-library-changed";

/** Tells every `useGraphicsLibrary` a graphic was saved to or removed from the library. */
export const libraryChanged = () => window.dispatchEvent(new Event(LIBRARY_CHANGED));

/** The app's graphics library, kept current. */
export function useGraphicsLibrary() {
  const [items, setItems] = useState<GraphicLibraryItem[]>([]);
  useEffect(() => {
    if (!isTauriEnvironment()) return;
    const load = () =>
      void api
        .graphicsLibrary()
        .then(setItems)
        .catch(() => undefined);
    load();
    window.addEventListener(LIBRARY_CHANGED, load);
    return () => window.removeEventListener(LIBRARY_CHANGED, load);
  }, []);
  return items;
}

const STYLES_CHANGED = "aeroedits-styles-changed";

/** Tells every `useStyles` the styles changed (saved, renamed, removed). */
export const stylesChanged = () => window.dispatchEvent(new Event(STYLES_CHANGED));

/** The styles and the project's style, kept current. */
export function useStyles(projectHandle: string | undefined, revision: number | undefined) {
  const [styles, setStyles] = useState<StylesView>();
  const load = useCallback(() => {
    if (!isTauriEnvironment()) return;
    void api
      .styles(projectHandle)
      .then(setStyles)
      .catch(() => undefined);
  }, [projectHandle]);
  // The project's style is in the edit, so it follows undo too.
  useEffect(load, [load, revision]);
  useEffect(() => {
    window.addEventListener(STYLES_CHANGED, load);
    return () => window.removeEventListener(STYLES_CHANGED, load);
  }, [load]);
  return styles;
}
