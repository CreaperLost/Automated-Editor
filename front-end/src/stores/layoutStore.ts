import { create } from "zustand";
import { persist } from "zustand/middleware";

/// Panels of the editor shell whose size the user can drag.
export type PanelId = "inspector" | "timeline" | "transcript";

export interface PanelLimits {
  /// Smallest size a drag can set. Dragging well below it collapses the panel.
  min: number;
  defaultSize: number;
  /// Whether dragging past the minimum hides the panel instead of stopping.
  collapsible: boolean;
}

export const PANEL_LIMITS: Record<PanelId, PanelLimits> = {
  inspector: { min: 240, defaultSize: 320, collapsible: true },
  timeline: { min: 140, defaultSize: 260, collapsible: false },
  transcript: { min: 110, defaultSize: 180, collapsible: true },
};

interface PanelState {
  /// The size the user picked, in CSS px. The shell may show less when the window is too small.
  size: number;
  collapsed: boolean;
}

interface LayoutStore {
  panels: Record<PanelId, PanelState>;
  setPanelSize: (id: PanelId, size: number) => void;
  setPanelCollapsed: (id: PanelId, collapsed: boolean) => void;
  resetPanel: (id: PanelId) => void;
}

function defaultPanels(): Record<PanelId, PanelState> {
  return {
    inspector: { size: PANEL_LIMITS.inspector.defaultSize, collapsed: false },
    timeline: { size: PANEL_LIMITS.timeline.defaultSize, collapsed: false },
    transcript: { size: PANEL_LIMITS.transcript.defaultSize, collapsed: false },
  };
}

/// Panel sizes for the editor shell, remembered between sessions.
export const useLayoutStore = create<LayoutStore>()(
  persist(
    (set) => ({
      panels: defaultPanels(),
      setPanelSize: (id, size) =>
        set((state) => ({
          panels: { ...state.panels, [id]: { size: Math.round(size), collapsed: false } },
        })),
      setPanelCollapsed: (id, collapsed) =>
        set((state) => ({ panels: { ...state.panels, [id]: { ...state.panels[id], collapsed } } })),
      resetPanel: (id) =>
        set((state) => ({ panels: { ...state.panels, [id]: defaultPanels()[id] } })),
    }),
    {
      name: "aeroedits.layout",
      version: 1,
      partialize: (state) => ({ panels: state.panels }),
      // Keep defaults for any panel a stored layout is missing or has garbage for.
      merge: (persisted, current) => {
        const stored = (persisted as Partial<LayoutStore> | undefined)?.panels ?? {};
        const panels = defaultPanels();
        for (const id of Object.keys(panels) as PanelId[]) {
          const entry = (stored as Partial<Record<PanelId, PanelState>>)[id];
          if (entry && Number.isFinite(entry.size) && entry.size > 0) {
            panels[id] = { size: entry.size, collapsed: entry.collapsed === true };
          }
        }
        return { ...current, panels };
      },
    },
  ),
);

/// The size a panel is actually drawn at: the user's choice, shrunk to fit `max` when
/// the window is too small. A collapsible panel that no longer fits at its minimum is
/// hidden rather than squashed. The stored choice is untouched, so it comes back when the
/// window grows again.
export function fittedPanelSize(id: PanelId, panel: PanelState, max: number): number {
  if (panel.collapsed) return 0;
  const { min, collapsible } = PANEL_LIMITS[id];
  const ceiling = Math.max(0, max);
  if (collapsible && ceiling < min) return 0;
  return Math.min(Math.max(panel.size, Math.min(min, ceiling)), ceiling);
}
