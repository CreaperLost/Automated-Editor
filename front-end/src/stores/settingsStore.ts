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
  cameraBubble: {
    enabled: true,
    shape: "rect",
    size: "md",
    position: "bottom-right",
    customX: 80,
    customY: 80,
    borderColor: "#6366f1",
    borderWidth: 3,
    mirror: true,
    shadow: false,
  },

  canvas: {
    backgroundType: "gradient",
    colorStart: "#312e81",
    colorEnd: "#0f172a",
    paddingPx: 32,
    cornerRadiusPx: 16,
    shadowBlurPx: 24,
    shadowOpacity: 0.5,
    aspectRatio: "16:9",
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
