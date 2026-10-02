import type { DockviewApi } from "dockview-react";

/// The editor's dockable panels. Every one is always open somewhere in the layout.
export const DOCK_PANELS = {
  preview: { title: "Preview", minimumWidth: 360, minimumHeight: 220 },
  transcript: { title: "Transcript", minimumWidth: 260, minimumHeight: 110 },
  inspector: { title: "Inspector", minimumWidth: 240, minimumHeight: 200 },
  media: { title: "Media", minimumWidth: 220, minimumHeight: 110 },
  timeline: { title: "Timeline", minimumWidth: 360, minimumHeight: 140 },
} as const;

export type DockPanelId = keyof typeof DOCK_PANELS;

export const LAYOUT_PRESETS = {
  editing: { label: "Editing", hint: "Preview and transcript, inspector on the right, timeline below" },
  transcript: { label: "Transcript", hint: "Tall transcript on the left for editing by text" },
  review: { label: "Review", hint: "Large preview; inspector and transcript share the right column" },
} as const;

export type LayoutPreset = keyof typeof LAYOUT_PRESETS;

const STORAGE_KEY = "aeroedits.dockLayout.v1";

function panel(id: DockPanelId) {
  const { title, minimumWidth, minimumHeight } = DOCK_PANELS[id];
  return { id, component: id, title, minimumWidth, minimumHeight };
}

/// The media bin shares the inspector's tab group, behind it.
function addMedia(api: DockviewApi) {
  api.addPanel({ ...panel("media"), position: { referencePanel: "inspector", direction: "within" }, inactive: true });
}

/// Replaces the current layout with `preset`.
export function applyPreset(api: DockviewApi, preset: LayoutPreset) {
  api.clear();
  switch (preset) {
    case "transcript": {
      api.addPanel(panel("preview"));
      api.addPanel({ ...panel("transcript"), position: { referencePanel: "preview", direction: "left" }, initialWidth: 420 });
      api.addPanel({ ...panel("inspector"), position: { referencePanel: "preview", direction: "right" }, initialWidth: 320 });
      addMedia(api);
      api.addPanel({ ...panel("timeline"), position: { direction: "below" }, initialHeight: 260 });
      break;
    }
    case "review": {
      api.addPanel(panel("preview"));
      api.addPanel({ ...panel("inspector"), position: { referencePanel: "preview", direction: "right" }, initialWidth: 340 });
      api.addPanel({ ...panel("transcript"), position: { referencePanel: "inspector", direction: "within" }, inactive: true });
      addMedia(api);
      api.addPanel({ ...panel("timeline"), position: { direction: "below" }, initialHeight: 220 });
      break;
    }
    default: {
      api.addPanel(panel("preview"));
      api.addPanel({ ...panel("inspector"), position: { referencePanel: "preview", direction: "right" }, initialWidth: 320 });
      addMedia(api);
      api.addPanel({ ...panel("timeline"), position: { direction: "below" }, initialHeight: 260 });
      api.addPanel({ ...panel("transcript"), position: { referencePanel: "preview", direction: "below" }, initialHeight: 180 });
    }
  }
  api.getPanel("preview")?.api.setActive();
}

/// Restores the saved layout. Falls back to the default preset when nothing usable is stored,
/// including a stored layout that lost one of the panels.
export function restoreLayout(api: DockviewApi) {
  try {
    const stored = window.localStorage.getItem(STORAGE_KEY);
    if (stored) {
      api.fromJSON(JSON.parse(stored));
      const ids = new Set(api.panels.map((p) => p.id));
      // Layouts saved before the media bin existed keep their arrangement and gain it.
      if (!ids.has("media") && ids.has("inspector")) {
        addMedia(api);
        ids.add("media");
      }
      if ((Object.keys(DOCK_PANELS) as DockPanelId[]).every((id) => ids.has(id))) return;
    }
  } catch {
    // Unreadable storage or an old layout format: start from the default.
  }
  applyPreset(api, "editing");
}

export function saveLayout(api: DockviewApi) {
  try {
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify(api.toJSON()));
  } catch {
    // Not remembering the layout is harmless.
  }
}

/// The mounted workspace, so the layout menu can apply presets.
let activeApi: DockviewApi | null = null;

export function setActiveDockApi(api: DockviewApi | null) {
  activeApi = api;
}

export function applyPresetToWorkspace(preset: LayoutPreset) {
  if (!activeApi) return;
  applyPreset(activeApi, preset);
  saveLayout(activeApi);
}
