import test from "node:test";
import assert from "node:assert/strict";
import { startAgentSync, useAgentStore } from "../src/stores/agentStore";
import { useProjectStore } from "../src/stores/projectStore";
import { api } from "../src/lib/ipc";
import type { OpenedProject, ProposalBatch, ProposalsView } from "../src/lib/types";

const project = (projectHandle: string): OpenedProject => ({
  projectHandle, revision: 1, name: projectHandle, durationUs: 2_000_000, assets: [],
  sequence: { tracks: [], magnetic: true }, fps: 30, diagnostics: [], previewAvailable: false, undoAvailable: false, redoAvailable: false,
});
// Batch ids start at b-1 in every project, so A's b-1 and B's b-1 are different batches.
const view = (projectHandle: string, reason: string): ProposalsView => ({
  projectHandle, revision: 1,
  batches: [{
    id: "b-1", agent: "agent", reason, createdAt: "2026-10-14T00:00:00Z", baseRevision: 1, kind: "edits",
    status: "pending", auto: false, appliedRevision: null, summary: [reason], items: [], stale: false, canUndo: false,
  } as ProposalBatch],
});
const tick = () => new Promise((resolve) => setImmediate(resolve));

test("a proposals reply about a project switched away from never reaches the open one", async () => {
  const original = { list: api.proposalsList, status: api.mcpStatus };
  api.proposalsList = async (handle) => view(handle, `${handle}: its own batch`);
  api.mcpStatus = async () => null as never;
  useProjectStore.setState({ openedProject: project("A") });
  useAgentStore.setState(useAgentStore.getInitialState(), true);
  const stop = startAgentSync();
  try {
    await tick();
    assert.equal(useAgentStore.getState().proposals?.batches[0].reason, "A: its own batch");

    // Switching clears A's batches at once, before B's list arrives.
    useProjectStore.setState({ openedProject: project("B") });
    assert.equal(useAgentStore.getState().proposals, null);
    await tick();
    assert.equal(useAgentStore.getState().proposals?.batches[0].reason, "B: its own batch");

    // A decision made in A answers late: it must not replace B's list.
    useAgentStore.getState().setProposals(view("A", "A: remove filler"));
    const shown = useAgentStore.getState().proposals;
    assert.equal(shown?.projectHandle, "B");
    assert.equal(shown?.batches[0].reason, "B: its own batch");
  } finally {
    stop();
    api.proposalsList = original.list;
    api.mcpStatus = original.status;
  }
});
