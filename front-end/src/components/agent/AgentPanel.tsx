import React, { useMemo, useState } from "react";
import { Bot, Check, Loader2, Play, Scissors, Settings2, Undo2, X } from "lucide-react";
import { api } from "../../lib/ipc";
import { seekPlayback } from "../../lib/playbackControl";
import type { ProposalBatch, ProposalItem, ProposalOutcome } from "../../lib/types";
import { useAgentStore } from "../../stores/agentStore";
import { useProjectStore } from "../../stores/projectStore";
import { formatTimeUs } from "../../hooks/useTimeline";
import { Badge, Button, IconButton, Notice, PanelHeader, Segmented, Switch, cn, type Tone } from "../ui";

const KIND_LABEL: Record<ProposalItem["kind"], string> = { filler: "Filler", retake: "Retake", cut: "Cut" };
/** Lines of an edit batch's diff shown before "Show all". */
const SUMMARY_LINES = 4;

function errorMessage(err: unknown) {
  return err instanceof Error ? err.message : String(err);
}

function statusBadge(batch: ProposalBatch): { label: string; tone: Tone; title?: string } {
  if (batch.status === "pending") {
    if (batch.stale) return { label: "Out of date", tone: "danger", title: `Made for revision ${batch.baseRevision}` };
    return { label: "Waiting", tone: "suggest" };
  }
  if (batch.status === "accepted") {
    if (batch.kind === "action") return { label: "Done", tone: "success" };
    return batch.auto ? { label: "Autopilot", tone: "accent" } : { label: "Accepted", tone: "success" };
  }
  if (batch.status === "rejected") return { label: "Rejected", tone: "neutral" };
  if (batch.status === "undone") return { label: "Undone", tone: "neutral" };
  return { label: "Withdrawn", tone: "neutral" };
}

/** How an action the agent ran reads in the panel. */
const ACTION_LABEL: Record<string, string> = {
  mouth_noises: "Mouth noises",
  silence: "Silence",
  gaps: "Gaps",
  fillers: "Fillers",
  layouts: "Layouts",
  zooms: "Zooms",
  settings: "Settings",
  chapters: "Chapters",
  shorts: "Shorts",
};

function kindLabel(batch: ProposalBatch) {
  if (batch.kind === "cuts") return "Cuts";
  if (batch.kind === "edits") return "Edits";
  return ACTION_LABEL[batch.action ?? ""] ?? batch.action ?? "Action";
}

function since(iso: string) {
  const seconds = Math.max(0, (Date.now() - Date.parse(iso)) / 1000);
  if (!Number.isFinite(seconds)) return "";
  if (seconds < 60) return "just now";
  if (seconds < 3600) return `${Math.floor(seconds / 60)} min ago`;
  if (seconds < 86_400) return `${Math.floor(seconds / 3600)} h ago`;
  return new Date(iso).toLocaleDateString();
}

const ItemRow: React.FC<{
  item: ProposalItem;
  checked: boolean;
  onToggle: () => void;
  canChoose: boolean;
}> = ({ item, checked, onToggle, canChoose }) => {
  const waiting = item.state === "pending";
  const note = item.alreadyCut ? "Already cut" : item.stale ? "Out of date" : null;
  return (
    <li className={cn("flex items-start gap-2 px-2 py-1.5", !waiting && "opacity-60")}>
      {canChoose && waiting ? (
        <input
          type="checkbox"
          checked={checked}
          onChange={onToggle}
          disabled={item.stale}
          aria-label={`Include "${item.text}"`}
          className="mt-0.5 h-3.5 w-3.5 shrink-0 accent-[var(--color-accent,#14b8a6)]"
        />
      ) : (
        <span className="mt-0.5 w-3.5 shrink-0 text-center text-meta">
          {item.state === "accepted" ? <Check className="w-3.5 h-3.5 text-success-fg" /> : item.state === "rejected" ? "–" : ""}
        </span>
      )}
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-1.5">
          <span className="font-mono text-meta tabular-nums text-studio-500">
            {item.editedStartUs !== null ? formatTimeUs(item.editedStartUs) : "--:--.--"}
          </span>
          {item.kind !== "cut" && <Badge tone={item.kind === "filler" ? "suggest" : "neutral"}>{KIND_LABEL[item.kind]}</Badge>}
          {note && <span className="text-meta text-studio-500">{note}</span>}
        </div>
        <p className={cn("text-body text-studio-100 break-words", item.state === "accepted" && "line-through")}>
          &ldquo;{item.text}&rdquo;
        </p>
        {item.reason && <p className="text-meta text-studio-500 italic">{item.reason}</p>}
      </div>
      {item.editedStartUs !== null && (
        <IconButton
          icon={Play}
          size="sm"
          label="Play from just before it"
          onClick={() => seekPlayback(Math.max(0, (item.editedStartUs ?? 0) - 1_000_000))}
        />
      )}
    </li>
  );
};

const BatchCard: React.FC<{
  batch: ProposalBatch;
  busy: boolean;
  onAccept: (batch: ProposalBatch, itemIds: string[] | null) => void;
  onReject: (batch: ProposalBatch) => void;
  onUndo: (batch: ProposalBatch) => void;
}> = ({ batch, busy, onAccept, onReject, onUndo }) => {
  // Cuts the user unticked; everything waiting is ticked at first.
  const [unticked, setUnticked] = useState<Set<string>>(() => new Set());
  const [allLines, setAllLines] = useState(false);
  const badge = statusBadge(batch);
  const waiting = batch.status === "pending";
  const choosable = batch.items.filter((i) => i.state === "pending" && !i.stale);
  const chosen = choosable.filter((i) => !unticked.has(i.id));
  // An action's reason is its first summary line already.
  const summary =
    batch.kind === "cuts" ? batch.summary.slice(0, 1) : batch.kind === "action" ? batch.summary.slice(1) : batch.summary;
  const folded = summary.length > SUMMARY_LINES + 1 && !allLines;
  const lines = folded ? summary.slice(0, SUMMARY_LINES) : summary;

  return (
    <li className={cn("rounded-control border bg-studio-850", waiting ? "border-studio-700" : "border-studio-800")}>
      <div className="px-3 pt-2.5 pb-2 space-y-1.5">
        <div className="flex items-center gap-1.5 min-w-0">
          <Badge tone="accent" title="The agent that proposed it">
            <Bot className="w-3 h-3" aria-hidden />
            {batch.agent}
          </Badge>
          <Badge title={batch.kind === "action" ? "One of the app's actions, run by the agent" : undefined}>
            {kindLabel(batch)}
          </Badge>
          <Badge tone={badge.tone} title={badge.title}>
            {badge.label}
          </Badge>
          <span className="ml-auto text-meta text-studio-500 shrink-0" title={batch.createdAt}>
            {batch.id} · {since(batch.decidedAt ?? batch.createdAt)}
          </span>
        </div>
        {batch.reason && <p className="text-body text-studio-100">{batch.reason}</p>}
        {lines.length > 0 && (
          <div
            className={cn(
              "text-meta text-studio-400 whitespace-pre-line",
              batch.kind === "edits" && "font-mono whitespace-pre-wrap rounded border border-studio-800 bg-studio-950 px-2 py-1.5",
            )}
          >
            {lines.join("\n")}
            {folded && (
              <button type="button" className="block mt-1 font-sans text-accent-fg hover:underline" onClick={() => setAllLines(true)}>
                Show all {summary.length} changes
              </button>
            )}
          </div>
        )}
        {batch.stale && waiting && (
          <p className="text-meta text-danger-fg">
            The edit changed after revision {batch.baseRevision}, so this can no longer be applied as proposed. Ask the agent to
            propose it again.
          </p>
        )}
      </div>

      {batch.items.length > 0 && (
        <ul className="border-t border-studio-800 divide-y divide-studio-800/60 max-h-72 overflow-y-auto">
          {batch.items.map((item) => (
            <ItemRow
              key={item.id}
              item={item}
              canChoose={waiting}
              checked={!unticked.has(item.id)}
              onToggle={() =>
                setUnticked((current) => {
                  const next = new Set(current);
                  if (next.has(item.id)) next.delete(item.id);
                  else next.add(item.id);
                  return next;
                })
              }
            />
          ))}
        </ul>
      )}

      {(waiting || batch.canUndo) && (
        <div className="flex items-center gap-2 px-3 py-2 border-t border-studio-800">
          {waiting && !batch.stale && (
            <Button
              size="sm"
              variant="primary"
              icon={batch.kind === "cuts" ? Scissors : Check}
              disabled={busy || (batch.kind === "cuts" && chosen.length === 0)}
              onClick={() =>
                onAccept(batch, batch.kind === "cuts" && chosen.length < choosable.length ? chosen.map((i) => i.id) : null)
              }
              title="Applies it as one step; Undo takes it back"
            >
              {batch.kind === "cuts" ? `Apply ${chosen.length} cut${chosen.length === 1 ? "" : "s"}` : "Accept"}
            </Button>
          )}
          {waiting && (
            <Button size="sm" variant="ghost" icon={X} disabled={busy} onClick={() => onReject(batch)}>
              Reject{batch.kind === "cuts" && batch.status === "pending" && chosen.length < choosable.length ? " all" : ""}
            </Button>
          )}
          {batch.canUndo && (
            <Button size="sm" variant="secondary" icon={Undo2} disabled={busy} onClick={() => onUndo(batch)} className="ml-auto">
              Undo
            </Button>
          )}
        </div>
      )}
    </li>
  );
};

/// What connected agents propose: batches of cuts and timeline edits with their reasons, to
/// accept (one undo step each), reject or undo. Not a chat: the agent is talked to in its own app.
export const AgentPanel: React.FC = () => {
  const proposals = useAgentStore((s) => s.proposals);
  const status = useAgentStore((s) => s.status);
  const setProposals = useAgentStore((s) => s.setProposals);
  const setStatus = useAgentStore((s) => s.setStatus);
  const setDialogOpen = useAgentStore((s) => s.setDialogOpen);
  const openedProject = useProjectStore((s) => s.openedProject);
  const applyOpenedProject = useProjectStore((s) => s.applyOpenedProject);
  const [filter, setFilter] = useState<"waiting" | "history">("waiting");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  const batches = proposals?.batches ?? [];
  const waiting = useMemo(() => batches.filter((b) => b.status === "pending"), [batches]);
  const shown = filter === "waiting" ? waiting : batches.filter((b) => b.status !== "pending");

  const run = async (work: () => Promise<ProposalOutcome | void>, done: string) => {
    setBusy(true);
    setError(null);
    setNotice(null);
    const handle = useProjectStore.getState().openedProject?.projectHandle;
    try {
      const outcome = await work();
      // Another project was opened meanwhile: its panel has nothing to say about this one.
      if (useProjectStore.getState().openedProject?.projectHandle !== handle) return;
      if (outcome) {
        if (outcome.project) applyOpenedProject(outcome.project);
        setProposals(outcome.proposals);
      }
      if (outcome?.proposals.warning) setError(outcome.proposals.warning);
      else setNotice(done);
    } catch (err) {
      if (useProjectStore.getState().openedProject?.projectHandle !== handle) return;
      setError(errorMessage(err));
      void useAgentStore.getState().refresh();
    } finally {
      setBusy(false);
    }
  };

  /** The batches shown are the open project's: only then do their ids name its batches. */
  const shownHere = proposals?.projectHandle === openedProject?.projectHandle;

  const accept = (batch: ProposalBatch, itemIds: string[] | null) => {
    if (!openedProject || !shownHere) return;
    void run(
      () => api.proposalAccept(openedProject.projectHandle, openedProject.revision, batch.id, itemIds, true),
      `Applied ${batch.id}. Undo takes it back.`,
    );
  };
  const reject = (batch: ProposalBatch) => {
    if (!openedProject || !shownHere) return;
    void run(async () => {
      setProposals(await api.proposalReject(openedProject.projectHandle, batch.id, null, true));
    }, `Rejected ${batch.id}.`);
  };
  const undo = (batch: ProposalBatch) => {
    if (!openedProject || !shownHere) return;
    void run(
      () => api.proposalUndo(openedProject.projectHandle, openedProject.revision, batch.id),
      batch.kind === "action" ? `Undid ${kindLabel(batch).toLowerCase()} (${batch.id}).` : `Undid ${batch.id}; it waits again.`,
    );
  };

  const connected = status?.connected ?? [];
  const detail = !status?.enabled
    ? "Agents off"
    : connected.length === 0
      ? "No agent connected"
      : connected.join(", ");

  return (
    <div className="h-full flex flex-col min-h-0 bg-studio-900">
      <PanelHeader
        title="Agent"
        icon={Bot}
        detail={detail}
        actions={
          <>
            <Switch
              checked={!!status?.autopilot}
              disabled={!status}
              onChange={(autopilot) =>
                void api
                  .mcpSetAutopilot(autopilot)
                  .then(setStatus)
                  .catch((err) => setError(errorMessage(err)))
              }
              label={<span className="text-meta">Autopilot</span>}
              title="Apply agents' proposals as they arrive, without waiting for you. Each stays here to undo."
            />
            <IconButton icon={Settings2} size="sm" label="AI agents…" onClick={() => setDialogOpen(true)} />
          </>
        }
      />
      {error && (
        <Notice tone="danger" onDismiss={() => setError(null)}>
          {error}
        </Notice>
      )}
      {notice && !error && <Notice onDismiss={() => setNotice(null)}>{notice}</Notice>}
      {status?.autopilot && (
        <Notice tone="suggest">Autopilot is on: proposals are applied as they arrive.</Notice>
      )}
      {(status?.working ?? []).length > 0 && (
        <ul className="shrink-0 px-3 pt-2.5 space-y-1.5" aria-label="Running now">
          {status!.working.map((call, i) => (
            <li key={`${call.tool}-${i}`} className="rounded-control border border-accent/30 bg-accent/10 px-2.5 py-1.5">
              <div className="flex items-center gap-1.5 text-meta">
                <Loader2 className="w-3.5 h-3.5 text-accent-fg animate-spin shrink-0" aria-hidden />
                <span className="text-studio-300">{call.client ?? "agent"}</span>
                <span className="font-mono text-studio-100">{call.tool}</span>
                {call.fraction !== null && (
                  <span className="ml-auto font-mono tabular-nums text-studio-400">{Math.round(call.fraction * 100)}%</span>
                )}
              </div>
              {call.message && <p className="mt-0.5 text-meta text-studio-400 truncate">{call.message}</p>}
              {call.fraction !== null && (
                <div className="mt-1 h-1 rounded-full bg-studio-800 overflow-hidden">
                  <div className="h-full bg-accent" style={{ width: `${Math.round(call.fraction * 100)}%` }} />
                </div>
              )}
            </li>
          ))}
        </ul>
      )}

      <div className="shrink-0 px-3 pt-2.5">
        <Segmented<"waiting" | "history">
          label="Show"
          size="sm"
          className="w-full"
          value={filter}
          onChange={setFilter}
          options={[
            { value: "waiting", label: `Waiting ${waiting.length}` },
            { value: "history", label: "History" },
          ]}
        />
      </div>

      <ul className="flex-1 min-h-0 overflow-y-auto p-3 space-y-2.5">
        {!openedProject ? (
          <li className="text-studio-500 text-body">Open a project to see what agents propose.</li>
        ) : shown.length === 0 ? (
          <li className="text-studio-500 text-body space-y-2">
            {filter === "history" ? (
              <p>Nothing decided yet.</p>
            ) : !status?.enabled ? (
              <>
                <p>Let an AI agent you already use (Claude, Codex, …) read this project and propose cuts and edits here.</p>
                <Button size="sm" variant="secondary" icon={Bot} onClick={() => setDialogOpen(true)}>
                  Set up agents
                </Button>
              </>
            ) : (
              <p>
                Nothing waiting. Ask your agent, for example: &ldquo;Clean up the open AeroEdits project: mouth noises, silences,
                fillers and zooms.&rdquo; What it runs shows under History, each with Undo.
              </p>
            )}
          </li>
        ) : (
          shown.map((batch) => (
            <BatchCard key={batch.id} batch={batch} busy={busy} onAccept={accept} onReject={reject} onUndo={undo} />
          ))
        )}
      </ul>
    </div>
  );
};
