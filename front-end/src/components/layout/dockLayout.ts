import type { DockviewApi } from "dockview-react";

/// The editor's dockable panels. Every one a workspace uses is always open somewhere in it.
export const DOCK_PANELS = {
  preview: { title: "Preview", minimumWidth: 360, minimumHeight: 220 },
  transcript: { title: "Transcript", minimumWidth: 260, minimumHeight: 110 },
  inspector: { title: "Inspector", minimumWidth: 240, minimumHeight: 200 },
  media: { title: "Media", minimumWidth: 220, minimumHeight: 110 },
  sounds: { title: "Sounds", minimumWidth: 220, minimumHeight: 160 },
  music: { title: "Music", minimumWidth: 220, minimumHeight: 160 },
  zoom: { title: "Zoom", minimumWidth: 220, minimumHeight: 160 },
  chapters: { title: "Chapters", minimumWidth: 220, minimumHeight: 110 },
  agent: { title: "Agent", minimumWidth: 260, minimumHeight: 160 },
  jumpCuts: { title: "Jump cuts", minimumWidth: 260, minimumHeight: 200 },
  noises: { title: "Mouth noises", minimumWidth: 260, minimumHeight: 200 },
  joins: { title: "Joins", minimumWidth: 260, minimumHeight: 200 },
  search: { title: "Search", minimumWidth: 220, minimumHeight: 110 },
  shorts: { title: "Shorts", minimumWidth: 220, minimumHeight: 120 },
  shortLook: { title: "Look", minimumWidth: 240, minimumHeight: 160 },
  shortStyle: { title: "Style", minimumWidth: 240, minimumHeight: 160 },
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
 * them. In Shorts the shared panels (preview, timeline, inspector) show the selected short.
 */
export const WORKSPACES = {
  edit: { label: "Edit", preset: "editing" as LayoutPreset },
  cleanup: { label: "Cleanup", preset: "transcript" as LayoutPreset },
  shorts: { label: "Shorts", preset: null },
  clips: { label: "Clips", preset: null },
} as const;

export type Workspace = keyof typeof WORKSPACES;

/** Shorts (made from a take) and Clips (cut from the video) share their panels and arrangement. */
export function isShortsWorkspace(workspace: Workspace): workspace is "shorts" | "clips" {
  return workspace === "shorts" || workspace === "clips";
}

/** The panels only Shorts has, and the editor's panels Shorts has too. */
const SHORTS_ONLY: readonly DockPanelId[] = ["shorts", "shortLook", "shortStyle"];
const SHORTS_PANELS: readonly DockPanelId[] = [
  ...SHORTS_ONLY,
  "preview",
  "timeline",
  "transcript",
  "jumpCuts",
  "noises",
  "media",
  "sounds",
  "music",
  "agent",
];

/** The panels `workspace` shows. */
export function workspacePanels(workspace: Workspace): DockPanelId[] {
  const all = Object.keys(DOCK_PANELS) as DockPanelId[];
  return isShortsWorkspace(workspace) ? [...SHORTS_PANELS] : all.filter((id) => !SHORTS_ONLY.includes(id));
}

/** Edit keeps the key layouts were always saved under. */
function storageKey(workspace: Workspace) {
  return workspace === "edit" ? "aeroedits.dockLayout.v1" : `aeroedits.dockLayout.${workspace}.v1`;
}

function panel(id: DockPanelId) {
  const { title, minimumWidth, minimumHeight } = DOCK_PANELS[id];
  return { id, component: id, title, minimumWidth, minimumHeight };
}

/// The media bin, the sound and music libraries, the zoom panel and the agent's proposals share
/// the inspector's tab group, behind it.
function addMedia(api: DockviewApi) {
  api.addPanel({ ...panel("media"), position: { referencePanel: "inspector", direction: "within" }, inactive: true });
  addLibraries(api, "inspector");
  addZoom(api);
  addAgent(api);
}

/// The sound effects and music libraries, behind `beside`.
function addLibraries(api: DockviewApi, beside: DockPanelId) {
  for (const id of ["sounds", "music"] as const) {
    api.addPanel({ ...panel(id), position: { referencePanel: beside, direction: "within" }, inactive: true });
  }
}

function addAgent(api: DockviewApi) {
  api.addPanel({ ...panel("agent"), position: { referencePanel: "inspector", direction: "within" }, inactive: true });
  addJoins(api);
}

/// The joins the agent hands over to listen to sit next to its proposals.
function addJoins(api: DockviewApi) {
  api.addPanel({ ...panel("joins"), position: { referencePanel: "inspector", direction: "within" }, inactive: true });
}

/// The cleanup passes share the inspector's tab group too. In Cleanup they are its first tabs
/// (never hidden in the tab overflow) and Jump cuts is in front.
function addCleanup(api: DockviewApi, inFront = false) {
  const at = (index: number) => (inFront ? { index } : {});
  api.addPanel({ ...panel("jumpCuts"), position: { referencePanel: "inspector", direction: "within", ...at(0) }, inactive: !inFront });
  api.addPanel({ ...panel("noises"), position: { referencePanel: "inspector", direction: "within", ...at(1) }, inactive: true });
}

/// Chapters sit behind the transcript they are made from.
function addChapters(api: DockviewApi) {
  api.addPanel({ ...panel("chapters"), position: { referencePanel: "transcript", direction: "within" }, inactive: true });
  addSearch(api);
}

/// Searching every transcript sits behind the transcript too.
function addSearch(api: DockviewApi) {
  api.addPanel({ ...panel("search"), position: { referencePanel: "transcript", direction: "within" }, inactive: true });
}

function addZoom(api: DockviewApi) {
  api.addPanel({ ...panel("zoom"), position: { referencePanel: "inspector", direction: "within" }, inactive: true });
}

/// Shorts: the transcript first, on the left under a short list of the shorts, the preview,
/// the look on the right (the style, the short's cleanup passes, media and the agent behind
/// it), the short's timeline below. Only settings of shorts: the video's own (the Inspector)
/// stay in Edit, but for the style, which shorts wear too.
function applyShortsLayout(api: DockviewApi) {
  api.clear();
  api.addPanel(panel("preview"));
  api.addPanel({ ...panel("shorts"), position: { referencePanel: "preview", direction: "left" }, initialWidth: 340 });
  api.addPanel({ ...panel("transcript"), position: { referencePanel: "shorts", direction: "below" }, initialHeight: 420 });
  api.addPanel({ ...panel("shortLook"), position: { referencePanel: "preview", direction: "right" }, initialWidth: 320 });
  for (const id of ["shortStyle", "jumpCuts", "noises", "media", "sounds", "music", "agent"] as const) {
    api.addPanel({ ...panel(id), position: { referencePanel: "shortLook", direction: "within" }, inactive: true });
  }
  api.addPanel({ ...panel("timeline"), position: { direction: "below" }, initialHeight: 260 });
  api.getPanel("transcript")?.api.setActive();
  api.getPanel("shortLook")?.api.setActive();
  api.getPanel("preview")?.api.setActive();
}

/// Puts `workspace` in its own default arrangement.
function applyWorkspaceDefault(api: DockviewApi, workspace: Workspace) {
  const { preset } = WORKSPACES[workspace];
  if (preset) applyPreset(api, preset);
  else applyShortsLayout(api);
}

/// Replaces the current layout with `preset`.
export function applyPreset(api: DockviewApi, preset: LayoutPreset) {
  api.clear();
  switch (preset) {
    case "transcript": {
      api.addPanel(panel("preview"));
      api.addPanel({ ...panel("transcript"), position: { referencePanel: "preview", direction: "left" }, initialWidth: 420 });
      addChapters(api);
      api.addPanel({ ...panel("inspector"), position: { referencePanel: "preview", direction: "right" }, initialWidth: 340 });
      addMedia(api);
      addCleanup(api, true);
      api.addPanel({ ...panel("timeline"), position: { direction: "below" }, initialHeight: 260 });
      break;
    }
    case "review": {
      api.addPanel(panel("preview"));
      api.addPanel({ ...panel("inspector"), position: { referencePanel: "preview", direction: "right" }, initialWidth: 340 });
      api.addPanel({ ...panel("transcript"), position: { referencePanel: "inspector", direction: "within" }, inactive: true });
      addChapters(api);
      addMedia(api);
      addCleanup(api);
      api.addPanel({ ...panel("timeline"), position: { direction: "below" }, initialHeight: 220 });
      break;
    }
    default: {
      api.addPanel(panel("preview"));
      api.addPanel({ ...panel("inspector"), position: { referencePanel: "preview", direction: "right" }, initialWidth: 320 });
      addMedia(api);
      addCleanup(api);
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
      const wanted = workspacePanels(workspace);
      if (isShortsWorkspace(workspace)) {
        if (wanted.every((id) => ids.has(id)) && ids.size === wanted.length) return;
        throw new Error("Shorts layout without its panels");
      }
      // Layouts saved before the media bin existed keep their arrangement and gain it.
      if (!ids.has("media") && ids.has("inspector")) {
        addMedia(api);
        ids.add("media");
        ids.add("zoom");
      }
      // Layouts from before the sound and music libraries were panels.
      if (!ids.has("sounds") && !ids.has("music") && ids.has("inspector")) {
        addLibraries(api, "inspector");
        ids.add("sounds");
        ids.add("music");
      }
      if (!ids.has("zoom") && ids.has("inspector")) {
        addZoom(api);
        ids.add("zoom");
      }
      if (!ids.has("agent") && ids.has("inspector")) {
        addAgent(api);
        ids.add("agent");
        ids.add("joins");
      }
      if (!ids.has("joins") && ids.has("inspector")) {
        addJoins(api);
        ids.add("joins");
      }
      if (!ids.has("chapters") && ids.has("transcript")) {
        addChapters(api);
        ids.add("chapters");
        ids.add("search");
      }
      if (!ids.has("search") && ids.has("transcript")) {
        addSearch(api);
        ids.add("search");
      }
      // Layouts from before Jump cuts and Mouth noises were panels (they were a dialog).
      if (!ids.has("jumpCuts") && !ids.has("noises") && ids.has("inspector")) {
        addCleanup(api, workspace === "cleanup");
        ids.add("jumpCuts");
        ids.add("noises");
      }
      if (wanted.every((id) => ids.has(id)) && !SHORTS_ONLY.some((id) => ids.has(id))) return;
    }
  } catch {
    // Unreadable storage or an old layout format: start from the default.
  }
  applyWorkspaceDefault(api, workspace);
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
  if (!activeApi || isShortsWorkspace(activeWorkspace)) return;
  applyPreset(activeApi, preset);
  saveLayout(activeApi);
}

/// A panel to bring to the front once the workspace switching to shows (see `requestPanel`).
let pendingPanel: DockPanelId | null = null;

/// Brings panel `id` to the front where it is; a panel this workspace lacks in the one made
/// for it (cleanup in Cleanup, shorts in Shorts, the rest in Edit).
export function requestPanel(id: DockPanelId) {
  // Where it is, else the workspace made for it.
  const target: Workspace = workspacePanels(activeWorkspace).includes(id)
    ? activeWorkspace
    : id === "jumpCuts" || id === "noises"
      ? "cleanup"
      : SHORTS_ONLY.includes(id)
        ? "shorts"
        : "edit";
  if (target !== activeWorkspace) {
    pendingPanel = id;
    switchWorkspace?.(target);
    return;
  }
  activeApi?.getPanel(id)?.api.setActive();
}

/// The panel asked for while the workspace switched, once (`DockWorkspace` shows it).
export function takePendingPanel(): DockPanelId | null {
  const id = pendingPanel;
  pendingPanel = null;
  return id;
}

/// How `requestPanel` switches workspace (the workspace store registers it, so this file needs
/// no store).
let switchWorkspace: ((workspace: Workspace) => void) | null = null;
export function setWorkspaceSwitcher(switcher: (workspace: Workspace) => void) {
  switchWorkspace = switcher;
}

/// Puts the current workspace back to its own default arrangement.
export function resetWorkspaceLayout() {
  if (!activeApi) return;
  applyWorkspaceDefault(activeApi, activeWorkspace);
  saveLayout(activeApi);
}
