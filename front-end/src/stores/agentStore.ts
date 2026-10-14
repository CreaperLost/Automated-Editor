import { create } from "zustand";
import { api, isTauriEnvironment } from "../lib/ipc";
import type { McpStatus, ProposalsView } from "../lib/types";
import { useProjectStore } from "./projectStore";

const EVENT = "aeroedits-proposals-updated";
/** An agent's call started, progressed or ended. */
const ACTIVITY_EVENT = "aeroedits-agent-activity";

interface AgentStore {
  /** The open project's proposal batches, as they would land in the edit now. */
  proposals: ProposalsView | null;
  /** The agent server: on or off, autopilot, who is connected. */
  status: McpStatus | null;
  /** Bumped whenever the proposals change, so the transcript's review list reloads. */
  version: number;
  /** File > AI agents… */
  dialogOpen: boolean;
  setDialogOpen: (open: boolean) => void;
  setProposals: (proposals: ProposalsView) => void;
  setStatus: (status: McpStatus) => void;
  refresh: () => Promise<void>;
}

let refreshGeneration = 0;

export const useAgentStore = create<AgentStore>((set, get) => ({
  proposals: null,
  status: null,
  version: 0,
  dialogOpen: false,
  setDialogOpen: (dialogOpen) => set({ dialogOpen }),
  setProposals: (proposals) => {
    // A reply about a project switched away from while the call ran is dropped: its batch ids
    // would name the open project's batches.
    if (proposals.projectHandle !== useProjectStore.getState().openedProject?.projectHandle) return;
    set({ proposals, version: get().version + 1 });
  },
  setStatus: (status) => set({ status }),
  refresh: async () => {
    const generation = ++refreshGeneration;
    const handle = useProjectStore.getState().openedProject?.projectHandle;
    try {
      const [proposals, status] = await Promise.all([
        handle ? api.proposalsList(handle) : Promise.resolve(null),
        api.mcpStatus(),
      ]);
      // A newer refresh (another project, a later change) wins.
      if (generation !== refreshGeneration) return;
      if (useProjectStore.getState().openedProject?.projectHandle !== handle) return;
      set({ proposals, status, version: get().version + 1 });
    } catch {
      // Proposals are an extra; the editor works without them.
    }
  },
}));

/**
 * Keeps the proposals current: on the agents' changes (and the other window's), and whenever
 * the project's revision moves (an item's place and whether it is stale depend on it).
 * Returns the unsubscribe.
 */
export function startAgentSync(): () => void {
  const refresh = () => void useAgentStore.getState().refresh();
  let revision: string | undefined;
  const unsubscribe = useProjectStore.subscribe((state) => {
    const key = state.openedProject ? `${state.openedProject.projectHandle}:${state.openedProject.revision}` : undefined;
    if (key !== revision) {
      revision = key;
      // Another project's batches leave at once rather than wait for the refresh.
      const shown = useAgentStore.getState().proposals;
      if (shown && shown.projectHandle !== state.openedProject?.projectHandle) {
        useAgentStore.setState({ proposals: null, version: useAgentStore.getState().version + 1 });
      }
      refresh();
    }
  });
  refresh();
  if (!isTauriEnvironment()) return unsubscribe;
  // What agents are running changes often (progress): only the status is read again.
  let statusTimer = 0;
  const refreshStatus = () => {
    window.clearTimeout(statusTimer);
    statusTimer = window.setTimeout(() => {
      void api
        .mcpStatus()
        .then((status) => useAgentStore.getState().setStatus(status))
        .catch(() => undefined);
    }, 150);
  };
  const unlisteners: (() => void)[] = [];
  let disposed = false;
  void import("@tauri-apps/api/event").then(({ listen }) =>
    Promise.all([listen(EVENT, refresh), listen(ACTIVITY_EVENT, refreshStatus)]).then((fns) => {
      if (disposed) fns.forEach((fn) => fn());
      else unlisteners.push(...fns);
    }),
  );
  return () => {
    disposed = true;
    window.clearTimeout(statusTimer);
    unsubscribe();
    unlisteners.forEach((fn) => fn());
  };
}
