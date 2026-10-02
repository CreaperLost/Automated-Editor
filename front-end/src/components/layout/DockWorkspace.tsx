import React, { useEffect } from "react";
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
import { restoreLayout, saveLayout, setActiveDockApi, type DockPanelId } from "./dockLayout";

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
  useEffect(() => () => setActiveDockApi(null), []);

  const onReady = (event: DockviewReadyEvent) => {
    restoreLayout(event.api);
    setActiveDockApi(event.api);
    let timer = 0;
    event.api.onDidLayoutChange(() => {
      window.clearTimeout(timer);
      timer = window.setTimeout(() => saveLayout(event.api), 250);
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
