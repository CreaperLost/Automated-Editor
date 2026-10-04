import React, { useEffect, useRef } from "react";
import {
  DockviewDefaultTab,
  DockviewReact,
  type DockviewReadyEvent,
  type DockviewTheme,
  type IDockviewPanelHeaderProps,
  type IDockviewPanelProps,
} from "dockview-react";
import "dockview-react/dist/styles/dockview.css";
import "./dockTheme.css";
import { InspectorPanel } from "../inspector/InspectorPanel";
import { MediaPanel } from "../media/MediaPanel";
import { ZoomPanel } from "../zoom/ZoomPanel";
import { ChaptersPanel } from "../chapters/ChaptersPanel";
import { TimelineStudio } from "../timeline/TimelineStudio";
import { TranscriptPanel } from "../transcript/TranscriptPanel";
import { StagePanel } from "./StagePanel";
import {
  restoreLayout,
  saveLayout,
  setActiveDockApi,
  setActiveWorkspace,
  type DockPanelId,
} from "./dockLayout";
import { useWorkspaceStore } from "../../stores/workspaceStore";
import type { DockviewApi } from "dockview-react";

const THEME: DockviewTheme = {
  name: "aeroedits",
  className: "dockview-theme-aero",
  colorScheme: "dark",
  gap: 6,
};

const fill = (node: React.ReactNode) => <div className="h-full w-full min-h-0 min-w-0 overflow-hidden">{node}</div>;

const COMPONENTS: Record<DockPanelId, React.FunctionComponent<IDockviewPanelProps>> = {
  preview: () => fill(<StagePanel />),
  transcript: () => fill(<TranscriptPanel />),
  inspector: () => fill(<InspectorPanel />),
  media: () => fill(<MediaPanel />),
  zoom: () => fill(<ZoomPanel />),
  chapters: () => fill(<ChaptersPanel />),
  timeline: () => fill(<TimelineStudio />),
};

/// Panels stay open: there is no close button, so none can be lost.
const Tab: React.FC<IDockviewPanelHeaderProps> = (props) => <DockviewDefaultTab {...props} hideClose />;

/// The editor workspace. Drag a panel's tab onto another panel's edge to dock it there, or
/// onto its middle to share a tab group; drag the gaps between panels to resize them.
export const DockWorkspace: React.FC = () => {
  const workspace = useWorkspaceStore((s) => s.workspace);
  const apiRef = useRef<DockviewApi | null>(null);
  // The workspace whose layout is showing, and a pending save of it.
  const shown = useRef(workspace);
  const saveTimer = useRef(0);

  useEffect(() => () => setActiveDockApi(null), []);

  // Switching workspace: keep this one's arrangement, then show the other's.
  useEffect(() => {
    const api = apiRef.current;
    if (!api || shown.current === workspace) return;
    window.clearTimeout(saveTimer.current);
    saveLayout(api, shown.current);
    shown.current = workspace;
    setActiveWorkspace(workspace);
    restoreLayout(api, workspace);
  }, [workspace]);

  const onReady = (event: DockviewReadyEvent) => {
    apiRef.current = event.api;
    setActiveWorkspace(shown.current);
    restoreLayout(event.api, shown.current);
    setActiveDockApi(event.api);
    event.api.onDidLayoutChange(() => {
      window.clearTimeout(saveTimer.current);
      const target = shown.current;
      saveTimer.current = window.setTimeout(() => saveLayout(event.api, target), 250);
    });
  };

  return (
    <DockviewReact
      className="h-full w-full"
      theme={THEME}
      components={COMPONENTS}
      defaultTabComponent={Tab}
      disableFloatingGroups
      onReady={onReady}
    />
  );
};
