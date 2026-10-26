import React, { Suspense, useEffect, useRef, useState } from "react";
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
import { useAgentStore } from "../../stores/agentStore";
import { TimelineStudio } from "../timeline/TimelineStudio";
import { TranscriptPanel } from "../transcript/TranscriptPanel";
import { StagePanel } from "./StagePanel";
import { useShortsWorkspace } from "../shorts/useShortsWorkspace";
import { useProjectStore } from "../../stores/projectStore";
import {
  isShortsWorkspace,
  requestPanel,
  restoreLayout,
  saveLayout,
  setActiveDockApi,
  setActiveWorkspace,
  takePendingPanel,
  type DockPanelId,
} from "./dockLayout";
import { useWorkspaceStore } from "../../stores/workspaceStore";
import { isTauriEnvironment } from "../../lib/ipc";
import type { DockviewApi } from "dockview-react";

const THEME: DockviewTheme = {
  name: "aeroedits",
  className: "dockview-theme-aero",
  colorScheme: "dark",
  gap: 6,
};

const fill = (node: React.ReactNode) => (
  <div className="h-full w-full min-h-0 min-w-0 overflow-hidden">
    <Suspense fallback={null}>{node}</Suspense>
  </div>
);

// The Edit layout's own panels (preview, transcript, timeline, inspector) come with the editor;
// the rest load the first time they are shown.
const MediaPanel = React.lazy(() => import("../media/MediaPanel").then((m) => ({ default: m.MediaPanel })));
const ZoomPanel = React.lazy(() => import("../zoom/ZoomPanel").then((m) => ({ default: m.ZoomPanel })));
const ChaptersPanel = React.lazy(() => import("../chapters/ChaptersPanel").then((m) => ({ default: m.ChaptersPanel })));
const AgentPanel = React.lazy(() => import("../agent/AgentPanel").then((m) => ({ default: m.AgentPanel })));
const JumpCutsPanel = React.lazy(() => import("../cleanup/JumpCutsPanel").then((m) => ({ default: m.JumpCutsPanel })));
const NoisesPanel = React.lazy(() => import("../cleanup/NoisesPanel").then((m) => ({ default: m.NoisesPanel })));
const ShortsListPanel = React.lazy(() => import("../shorts/ShortsListPanel").then((m) => ({ default: m.ShortsListPanel })));
const ShortLookPanel = React.lazy(() => import("../shorts/ShortLookPanel").then((m) => ({ default: m.ShortLookPanel })));
const SoundsPage = React.lazy(() => import("../media/SoundsPage").then((m) => ({ default: m.SoundsPage })));
const MusicPage = React.lazy(() => import("../media/MusicPage").then((m) => ({ default: m.MusicPage })));
const StylePanel = React.lazy(() => import("../style/StylePanel").then((m) => ({ default: m.StylePanel })));
const JoinsPanel = React.lazy(() => import("../review/JoinsPanel").then((m) => ({ default: m.JoinsPanel })));
const SearchPanel = React.lazy(() => import("../review/SearchPanel").then((m) => ({ default: m.SearchPanel })));

const COMPONENTS: Record<DockPanelId, React.FunctionComponent<IDockviewPanelProps>> = {
  preview: () => fill(<StagePanel />),
  transcript: () => fill(<TranscriptPanel />),
  inspector: () => fill(<InspectorPanel />),
  media: () => fill(<MediaPanel />),
  sounds: () => fill(<Library><SoundsPage /></Library>),
  music: () => fill(<Library><MusicPage /></Library>),
  zoom: () => fill(<ZoomPanel />),
  chapters: () => fill(<ChaptersPanel />),
  agent: () => fill(<AgentPanel />),
  jumpCuts: () => fill(<JumpCutsPanel />),
  noises: () => fill(<NoisesPanel />),
  joins: () => fill(<JoinsPanel />),
  search: () => fill(<SearchPanel />),
  timeline: () => fill(<Timeline />),
  shorts: () => fill(<ShortsListPanel />),
  shortLook: () => fill(<ShortLookPanel />),
  shortStyle: () => fill(<div className="h-full overflow-y-auto"><StylePanel /></div>),
};

/// A library page as a panel of its own.
function Library({ children }: { children: React.ReactNode }) {
  return <div className="h-full flex flex-col min-h-0 bg-studio-900 text-label select-none">{children}</div>;
}

/// The timeline, saying so when it is a short's own.
function Timeline() {
  const showsShort = useProjectStore((s) => !!s.openedProject?.shortView);
  if (!showsShort) return <TimelineStudio />;
  return (
    <section className="h-full flex flex-col min-h-0" aria-label="Short timeline">
      <div className="shrink-0 px-3 py-1.5 text-meta text-studio-400 border-b border-studio-800 bg-accent/5">
        This short's own timeline. Cuts, deleted words, muted noises, splits and clips here change only the short
        (the video stays as it is); fixing a word's spelling fixes it in the video too.
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
  useShortsWorkspace(isShortsWorkspace(workspace) ? workspace : null);
  const apiRef = useRef<DockviewApi | null>(null);
  // The workspace whose layout is showing, and a pending save of it.
  const shown = useRef(workspace);
  const saveTimer = useRef(0);

  useEffect(() => () => setActiveDockApi(null), []);

  // An agent shows the user a moment (back to the video, the playhead in view), or hands them
  // joins to listen to (the Joins panel comes forward).
  useEffect(() => {
    if (!isTauriEnvironment()) return;
    const unlisteners: (() => void)[] = [];
    let disposed = false;
    void import("@tauri-apps/api/event").then(({ listen }) =>
      Promise.all([
        listen<{ atUs: number }>("aeroedits-agent-show", (event) => {
          if (isShortsWorkspace(useWorkspaceStore.getState().workspace)) useWorkspaceStore.getState().setWorkspace("edit");
          const store = useProjectStore.getState();
          store.setCurrentTimeUs(event.payload.atUs);
          store.revealPlayhead();
        }),
        listen("aeroedits-joins-updated", () => requestPanel("joins")),
      ]).then((fns) => {
        if (disposed) fns.forEach((fn) => fn());
        else unlisteners.push(...fns);
      }),
    );
    return () => {
      disposed = true;
      unlisteners.forEach((fn) => fn());
    };
  }, []);

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
    // One list panel, named after the tab it is in.
    apiRef.current?.getPanel("shorts")?.api.setTitle(workspace === "clips" ? "Clips" : "Shorts");
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
