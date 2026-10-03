import { create } from "zustand";
import { api } from "../lib/ipc";
import type { PreviewQuality } from "../lib/types";

/** Shorter side of the preview canvas; 0 draws it at the project's full size. */
export const PREVIEW_RESOLUTIONS: { value: number; label: string }[] = [
  { value: 0, label: "Full" },
  { value: 1080, label: "1080p" },
  { value: 720, label: "720p" },
  { value: 540, label: "540p" },
  { value: 360, label: "360p" },
];

/** Frames drawn per second while playing; 0 follows the source. */
export const PREVIEW_FPS: { value: number; label: string }[] = [
  { value: 0, label: "Source fps" },
  { value: 60, label: "60 fps" },
  { value: 30, label: "30 fps" },
  { value: 24, label: "24 fps" },
];

const STORAGE_KEY = "aeroedits.previewQuality.v1";

function load(): PreviewQuality | null {
  try {
    const stored = JSON.parse(window.localStorage.getItem(STORAGE_KEY) ?? "null");
    if (stored && typeof stored.resolution === "number" && typeof stored.fps === "number") {
      return { resolution: stored.resolution, fps: stored.fps };
    }
  } catch {
    // Storage can be unavailable; the backend default still works.
  }
  return null;
}

function save(quality: PreviewQuality) {
  try {
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify(quality));
  } catch {
    // Not remembered for next time, but still applied now.
  }
}

interface PreviewQualityStore {
  /** The quality in use, once the backend has answered. */
  quality: PreviewQuality | null;
  /** Webview preview frames drawn in the last second, while playing. */
  measuredFps: number | null;
  error: string | null;
  /** Applies the remembered choice, or reads the backend's default for this surface. */
  init: () => Promise<void>;
  setQuality: (change: Partial<PreviewQuality>) => void;
  setMeasuredFps: (fps: number | null) => void;
}

export const usePreviewQualityStore = create<PreviewQualityStore>((set, get) => ({
  quality: null,
  measuredFps: null,
  error: null,

  init: async () => {
    const saved = load();
    try {
      const quality = saved ? await api.previewQualitySet(saved) : await api.previewQuality();
      set({ quality, error: null });
    } catch (err) {
      try {
        set({ quality: await api.previewQuality(), error: saved ? String(err) : null });
      } catch {
        // Outside the desktop app there is no preview to configure.
      }
    }
  },

  setQuality: (change) => {
    const current = get().quality;
    if (!current) return;
    const next = { ...current, ...change };
    set({ quality: next, error: null });
    save(next);
    void api.previewQualitySet(next).catch((err) => set({ error: String(err) }));
  },

  setMeasuredFps: (measuredFps) => {
    if (get().measuredFps !== measuredFps) set({ measuredFps });
  },
}));
