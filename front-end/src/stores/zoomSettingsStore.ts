import { create } from "zustand";
import type { ZoomConfig } from "../lib/types";

/** Mirrors `ZoomConfig::default()` in src-tauri/src/zoom/mod.rs. */
export const DEFAULT_ZOOM_CONFIG: ZoomConfig = {
  generationVersion: 1,
  dwellRadiusNorm: 0.04,
  minDwellUs: 700_000,
  clusterRadiusNorm: 0.12,
  clusterGapUs: 800_000,
  rapidClickWindowUs: 350_000,
  minHoldUs: 1_200_000,
  transitionUs: 400_000,
  maxScale: 2,
  clickScale: 2,
  dwellScale: 1.5,
  geometryUncertaintyUs: 100_000,
  primaryButtons: [0],
  viewportMargin: 0,
};

/** The auto-zoom options people can change; the rest of the config keeps its defaults. */
export interface AutoZoomOptions {
  clickScale: number;
  /** Zoom where the mouse rests; 1 switches hover zooms off. */
  dwellScale: number;
  transitionMs: number;
  minHoldMs: number;
}

export const DEFAULT_AUTO_ZOOM: AutoZoomOptions = {
  clickScale: 2,
  dwellScale: 1.5,
  transitionMs: 400,
  minHoldMs: 1200,
};

const STORAGE_KEY = "aeroedits.autoZoom.v1";

function load(): AutoZoomOptions {
  try {
    const stored = JSON.parse(window.localStorage.getItem(STORAGE_KEY) ?? "null");
    if (stored && typeof stored === "object") return { ...DEFAULT_AUTO_ZOOM, ...stored };
  } catch {
    // Storage can be unavailable; the defaults still work.
  }
  return DEFAULT_AUTO_ZOOM;
}

/** The generator config for these options. Hover zooms off means a dwell no one can reach. */
export function zoomConfigFor(options: AutoZoomOptions): ZoomConfig {
  const dwellOff = options.dwellScale <= 1;
  return {
    ...DEFAULT_ZOOM_CONFIG,
    clickScale: options.clickScale,
    dwellScale: dwellOff ? 1 : options.dwellScale,
    maxScale: Math.max(options.clickScale, options.dwellScale, 1),
    transitionUs: Math.round(options.transitionMs * 1000),
    minHoldUs: Math.round(options.minHoldMs * 1000),
    minDwellUs: dwellOff ? 3_600_000_000 : DEFAULT_ZOOM_CONFIG.minDwellUs,
  };
}

interface ZoomSettingsStore {
  options: AutoZoomOptions;
  setOptions: (patch: Partial<AutoZoomOptions>) => void;
  reset: () => void;
}

export const useZoomSettingsStore = create<ZoomSettingsStore>((set) => ({
  options: load(),
  setOptions: (patch) =>
    set((state) => {
      const options = { ...state.options, ...patch };
      try {
        window.localStorage.setItem(STORAGE_KEY, JSON.stringify(options));
      } catch {
        // Not remembering the options is harmless.
      }
      return { options };
    }),
  reset: () => {
    try {
      window.localStorage.removeItem(STORAGE_KEY);
    } catch {
      // Nothing stored.
    }
    set({ options: DEFAULT_AUTO_ZOOM });
  },
}));
