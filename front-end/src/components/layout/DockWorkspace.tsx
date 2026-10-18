import React, { useEffect, useRef, useState } from "react";
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
import { AgentPanel } from "../agent/AgentPanel";
import { useAgentStore } from "../../stores/agentStore";
import { TimelineStudio } from "../timeline/TimelineStudio";
import { TranscriptPanel } from "../transcript/TranscriptPanel";
import { StagePanel } from "./StagePanel";
import { JumpCutsPanel } from "../cleanup/JumpCutsPanel";
import { NoisesPanel } from "../cleanup/NoisesPanel";
import { ShortsListPanel } from "../shorts/ShortsListPanel";
import { ShortLookPanel } from "../shorts/ShortLookPanel";
import { useShortsWorkspace } from "../shorts/useShortsWorkspace";
import { useProjectStore } from "../../stores/projectStore";
import {
  restoreLayout,
  saveLayout,
  setActiveDockApi,
  setActiveWorkspace,
  takePendingPanel,
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
  transcript: () => fill(<OnTake what="transcript"><TranscriptPanel /></OnTake>),
  inspector: () => fill(<InspectorPanel />),
  media: () => fill(<MediaPanel />),
  zoom: () => fill(<ZoomPanel />),
  chapters: () => fill(<ChaptersPanel />),
  agent: () => fill(<AgentPanel />),
  jumpCuts: () => fill(<OnTake what="jump cuts"><JumpCutsPanel /></OnTake>),
  noises: () => fill(<OnTake what="mouth noises"><NoisesPanel /></OnTake>),
  timeline: () => fill(<Timeline />),
  shorts: () => fill(<ShortsListPanel />),
  shortLook: () => fill(<ShortLookPanel />),
};

/// A panel that works on a short's own timeline only for a short made from a take: a short cut
/// from the video follows the video, so the video is what gets cleaned.
const OnTake: React.FC<{ what: string; children: React.ReactNode }> = ({ what, children }) => {
  const cutFromVideo = useProjectStore((s) => {
    const view = s.openedProject?.shortView;
    return !!view && !s.openedProject?.shorts?.find((short) => short.id === view)?.take;
  });
  if (!cutFromVideo) return <>{children}</>;
  return (
    <p className="p-4 text-label text-studio-500 leading-relaxed">
      This short is cut from the video and follows it: work on its {what} in Edit or Cleanup, and the short keeps
      what the video keeps. Shorts made from a take have their own.
    </p>
  );
};

/// The timeline, saying so when it is a short's own.
function Timeline() {
  const showsShort = useProjectStore((s) => !!s.openedProject?.shortView);
  if (!showsShort) return <TimelineStudio />;
  return (
    <section className="h-full flex flex-col min-h-0" aria-label="Short timeline">
      <div className="shrink-0 px-3 py-1.5 text-meta text-studio-400 border-b border-studio-800 bg-accent/5">
        This short's timeline. Cuts, splits, moves and clips here change only the short (the video stays as it is);
        caption text is shared with the video's transcript.
      </div>
      <div className="flex-1 min-h-0">
        <TimelineStudio />
      </div>
    </section>
  );
}

/// The panel `requestPanel` asked for while switching workspace, in front now.
function showPending(api: DockviewApi) {
  const id = takePendingPanel();
  if (id) api.getPanel(id)?.api.setActive();
}

/// Panels stay open: there is no close button, so none can be lost.
const Tab: React.FC<IDockviewPanelHeaderProps> = (props) => <DockviewDefaultTab {...props} hideClose />;

/// The editor workspace. Drag a panel's tab onto another panel's edge to dock it there, or
/// onto its middle to share a tab group; drag the gaps between panels to resize them.
export const DockWorkspace: React.FC = () => {
  const workspace = useWorkspaceStore((s) => s.workspace);
  useShortsWorkspace(workspace === "shorts");
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
    showPending(api);
  }, [workspace]);

  // The Agent tab says how many batches wait, so a new one is noticed without taking focus.
  const waiting = useAgentStore((s) => s.proposals?.batches.filter((b) => b.status === "pending").length ?? 0);
  const [ready, setReady] = useState(false);
  useEffect(() => {
    apiRef.current?.getPanel("agent")?.api.setTitle(waiting > 0 ? `Agent (${waiting})` : "Agent");
  }, [waiting, ready, workspace]);

  const onReady = (event: DockviewReadyEvent) => {
    apiRef.current = event.api;
    setActiveWorkspace(shown.current);
    restoreLayout(event.api, shown.current);
    showPending(event.api);
    setActiveDockApi(event.api);
    setReady(true);
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
