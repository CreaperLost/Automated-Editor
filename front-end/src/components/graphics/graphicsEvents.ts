import { useCallback, useEffect, useRef, useState } from "react";
import { api, isTauriEnvironment } from "../../lib/ipc";
import type { BrandsView, GraphicLibraryItem } from "../../lib/types";

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

const BRANDS_CHANGED = "aeroedits-brands-changed";

/** Tells every `useBrands` the kits changed (saved, renamed, removed). */
export const brandsChanged = () => window.dispatchEvent(new Event(BRANDS_CHANGED));

/** The brand kits and the project's default kit, kept current. */
export function useBrands(projectHandle: string | undefined, revision: number | undefined) {
  const [brands, setBrands] = useState<BrandsView>();
  const load = useCallback(() => {
    if (!isTauriEnvironment()) return;
    void api
      .graphicsBrands(projectHandle)
      .then(setBrands)
      .catch(() => undefined);
  }, [projectHandle]);
  // The project's default is in the edit, so it follows undo too.
  useEffect(load, [load, revision]);
  useEffect(() => {
    window.addEventListener(BRANDS_CHANGED, load);
    return () => window.removeEventListener(BRANDS_CHANGED, load);
  }, [load]);
  return brands;
}
