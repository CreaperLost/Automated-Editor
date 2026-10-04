import type { DockviewApi } from "dockview-react";

/// The editor's dockable panels. Every one is always open somewhere in the layout.
export const DOCK_PANELS = {
  preview: { title: "Preview", minimumWidth: 360, minimumHeight: 220 },
  transcript: { title: "Transcript", minimumWidth: 260, minimumHeight: 110 },
  inspector: { title: "Inspector", minimumWidth: 240, minimumHeight: 200 },
  media: { title: "Media", minimumWidth: 220, minimumHeight: 110 },
  zoom: { title: "Zoom", minimumWidth: 220, minimumHeight: 160 },
  chapters: { title: "Chapters", minimumWidth: 220, minimumHeight: 110 },
  timeline: { title: "Timeline", minimumWidth: 360, minimumHeight: 140 },
} as const;

export type DockPanelId = keyof typeof DOCK_PANELS;

export const LAYOUT_PRESETS = {
  editing: { label: "Editing", hint: "Preview and transcript, inspector on the right, timeline below" },
  transcript: { label: "Transcript", hint: "Tall transcript on the left for editing by text" },
  review: { label: "Review", hint: "Large preview; inspector and transcript share the right column" },
} as const;

export type LayoutPreset = keyof typeof LAYOUT_PRESETS;

/**
 * Workspaces in the editor window. Each keeps its own panel arrangement; switching swaps
 * them. (Shorts is its own window.)
 */
export const WORKSPACES = {
  edit: { label: "Edit", preset: "editing" as LayoutPreset },
  cleanup: { label: "Cleanup", preset: "transcript" as LayoutPreset },
} as const;

export type Workspace = keyof typeof WORKSPACES;

/** Edit keeps the key layouts were always saved under. */
function storageKey(workspace: Workspace) {
  return workspace === "edit" ? "aeroedits.dockLayout.v1" : `aeroedits.dockLayout.${workspace}.v1`;
}

function panel(id: DockPanelId) {
  const { title, minimumWidth, minimumHeight } = DOCK_PANELS[id];
  return { id, component: id, title, minimumWidth, minimumHeight };
}

/// The media bin and the zoom panel share the inspector's tab group, behind it.
function addMedia(api: DockviewApi) {
  api.addPanel({ ...panel("media"), position: { referencePanel: "inspector", direction: "within" }, inactive: true });
  addZoom(api);
}

/// Chapters sit behind the transcript they are made from.
function addChapters(api: DockviewApi) {
  api.addPanel({ ...panel("chapters"), position: { referencePanel: "transcript", direction: "within" }, inactive: true });
}

function addZoom(api: DockviewApi) {
  api.addPanel({ ...panel("zoom"), position: { referencePanel: "inspector", direction: "within" }, inactive: true });
}

/// Replaces the current layout with `preset`.
export function applyPreset(api: DockviewApi, preset: LayoutPreset) {
  api.clear();
  switch (preset) {
    case "transcript": {
      api.addPanel(panel("preview"));
      api.addPanel({ ...panel("transcript"), position: { referencePanel: "preview", direction: "left" }, initialWidth: 420 });
      addChapters(api);
      api.addPanel({ ...panel("inspector"), position: { referencePanel: "preview", direction: "right" }, initialWidth: 320 });
      addMedia(api);
      api.addPanel({ ...panel("timeline"), position: { direction: "below" }, initialHeight: 260 });
      break;
    }
    case "review": {
      api.addPanel(panel("preview"));
      api.addPanel({ ...panel("inspector"), position: { referencePanel: "preview", direction: "right" }, initialWidth: 340 });
      api.addPanel({ ...panel("transcript"), position: { referencePanel: "inspector", direction: "within" }, inactive: true });
      addChapters(api);
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
      addChapters(api);
    }
  }
  api.getPanel("preview")?.api.setActive();
}

/// Restores the workspace's saved layout. Falls back to its preset when nothing usable is
/// stored, including a stored layout that lost one of the panels.
export function restoreLayout(api: DockviewApi, workspace: Workspace = "edit") {
  try {
    const stored = window.localStorage.getItem(storageKey(workspace));
    if (stored) {
      api.fromJSON(JSON.parse(stored));
      const ids = new Set(api.panels.map((p) => p.id));
      // Layouts saved before the media bin existed keep their arrangement and gain it.
      if (!ids.has("media") && ids.has("inspector")) {
        addMedia(api);
        ids.add("media");
        ids.add("zoom");
      }
      if (!ids.has("zoom") && ids.has("inspector")) {
        addZoom(api);
        ids.add("zoom");
      }
      if (!ids.has("chapters") && ids.has("transcript")) {
        addChapters(api);
        ids.add("chapters");
      }
      if ((Object.keys(DOCK_PANELS) as DockPanelId[]).every((id) => ids.has(id))) return;
    }
  } catch {
    // Unreadable storage or an old layout format: start from the default.
  }
  applyPreset(api, WORKSPACES[workspace].preset);
}

export function saveLayout(api: DockviewApi, workspace: Workspace = activeWorkspace) {
  try {
    window.localStorage.setItem(storageKey(workspace), JSON.stringify(api.toJSON()));
  } catch {
    // Not remembering the layout is harmless.
  }
}

/// The mounted workspace, so the layout menu can apply presets.
let activeApi: DockviewApi | null = null;
let activeWorkspace: Workspace = "edit";

/// Which workspace's layout is showing (saves go under its name).
export function setActiveWorkspace(workspace: Workspace) {
  activeWorkspace = workspace;
}

export function setActiveDockApi(api: DockviewApi | null) {
  activeApi = api;
  // Development only: lets the browser preview arrange panels from the console.
  if (import.meta.env.DEV) (window as unknown as { __aeroDock?: DockviewApi | null }).__aeroDock = api;
}

export function applyPresetToWorkspace(preset: LayoutPreset) {
  if (!activeApi) return;
  applyPreset(activeApi, preset);
  saveLayout(activeApi);
}

/// Puts the current workspace back to its own default arrangement.
export function resetWorkspaceLayout() {
  applyPresetToWorkspace(WORKSPACES[activeWorkspace].preset);
}
