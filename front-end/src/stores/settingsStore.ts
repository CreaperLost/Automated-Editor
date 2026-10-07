import { create } from "zustand";
import {
  CameraBubbleSettings,
  CanvasSettings,
  EditLayout,
  canvasFromLayout,
  cameraFromLayout,
} from "../lib/types";

interface SettingsStore {
  cameraBubble: CameraBubbleSettings;
  canvas: CanvasSettings;
  layoutOwnedByProject: boolean;

  updateCameraBubble: (settings: Partial<CameraBubbleSettings>) => void;
  updateCanvas: (settings: Partial<CanvasSettings>) => void;
  hydrateLayout: (layout: EditLayout) => void;
}

export const useSettingsStore = create<SettingsStore>((set) => ({
  // Until a project loads its own layout; the same as a new project's (`EditLayout::default`).
  cameraBubble: {
    enabled: true,
    shape: "squircle",
    size: "md",
    sizePct: 60,
    roundnessPct: 0,
    position: "bottom-left",
    customX: 80,
    customY: 80,
    borderColor: "#6366f1",
    borderWidth: 8,
    mirror: true,
    shadow: true,
  },

  canvas: {
    backgroundType: "preset",
    backgroundPreset: "forest",
    colorStart: "#312e81",
    colorEnd: "#0f172a",
    screenCrop: { left: 0, top: 0, right: 0, bottom: 5.5 },
    screenScalePct: 84,
    cornerRadiusPx: 17,
    shadowBlurPx: 0,
    shadowOpacity: 0.5,
    aspectRatio: "16:9",
    cursorVisible: true,
    cursorSizePct: 150,
  },

  layoutOwnedByProject: false,

  updateCameraBubble: (settings) =>
    set((state) => ({ cameraBubble: { ...state.cameraBubble, ...settings } })),

  updateCanvas: (settings) =>
    set((state) => ({ canvas: { ...state.canvas, ...settings } })),

  hydrateLayout: (layout) =>
    set({
      canvas: canvasFromLayout(layout),
      cameraBubble: cameraFromLayout(layout),
      layoutOwnedByProject: true,
    }),
}));
