import { useCallback, useEffect, useState } from "react";
import { api, isTauriEnvironment } from "../../lib/ipc";
import type { PresetsView } from "../../lib/types";

const PRESETS_CHANGED = "aeroedits-presets-changed";

/** Tells every `usePresets` the presets changed (saved, removed). */
export const presetsChanged = () => window.dispatchEvent(new Event(PRESETS_CHANGED));

/** The presets, the project's, and what it changed from it, kept current (it follows undo). */
export function usePresets(projectHandle: string | undefined, revision: number | undefined) {
  const [presets, setPresets] = useState<PresetsView>();
  const load = useCallback(() => {
    if (!isTauriEnvironment()) return;
    void api
      .presets(projectHandle)
      .then(setPresets)
      .catch(() => undefined);
  }, [projectHandle]);
  useEffect(load, [load, revision]);
  useEffect(() => {
    window.addEventListener(PRESETS_CHANGED, load);
    return () => window.removeEventListener(PRESETS_CHANGED, load);
  }, [load]);
  return presets;
}
