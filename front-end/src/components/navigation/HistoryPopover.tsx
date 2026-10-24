import React, { useEffect, useRef, useState } from "react";
import { Bookmark, Bot, History, Pencil, RotateCcw, Trash2 } from "lucide-react";
import { api } from "../../lib/ipc";
import type { HistoryView, HistoryCheckpoint } from "../../lib/types";
import { useAgentStore } from "../../stores/agentStore";
import { useProjectStore } from "../../stores/projectStore";
import { Button, IconButton, cn } from "../ui";

const fieldClass =
  "w-full min-w-0 bg-studio-950 text-studio-100 text-label rounded px-2 py-1 border border-studio-700 focus:outline-none focus:border-accent-hover";

function when(iso: string) {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return "";
  const today = new Date().toDateString() === date.toDateString();
  return today
    ? date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })
    : date.toLocaleString([], { day: "numeric", month: "short", hour: "2-digit", minute: "2-digit" });
}

function defaultName() {
  return `Checkpoint ${new Date().toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })}`;
}

/// The top bar's History: checkpoints to save and go back to, and the recent steps with who
/// made them. Going back is one undo step; what was there is saved first as a checkpoint.
export const HistoryPopover: React.FC<{ onError: (message: string) => void }> = ({ onError }) => {
  const openedProject = useProjectStore((s) => s.openedProject);
  const applyOpenedProject = useProjectStore((s) => s.applyOpenedProject);
  const [open, setOpen] = useState(false);
  const [view, setView] = useState<HistoryView | null>(null);
  const [name, setName] = useState("");
  const [renaming, setRenaming] = useState<{ id: string; name: string } | null>(null);
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);
  const rootRef = useRef<HTMLDivElement>(null);
  const handle = openedProject?.projectHandle;
  const revision = openedProject?.revision;
  // The parent may pass a new function each render; reading it through a ref keeps the
  // effect below from refetching on every one.
  const onErrorRef = useRef(onError);
  onErrorRef.current = onError;

  // Read again whenever it opens or the edit moves on while it is open.
  useEffect(() => {
    if (!open || !handle) return;
    let current = true;
    api
      .projectHistory(handle)
      .then((v) => current && setView(v))
      .catch((err) => onErrorRef.current(String(err)));
    return () => {
      current = false;
    };
  }, [open, handle, revision]);

  useEffect(() => {
    if (!open) return;
    const onPointerDown = (event: PointerEvent) => {
      if (!rootRef.current?.contains(event.target as Node)) setOpen(false);
    };
    window.addEventListener("pointerdown", onPointerDown);
    return () => window.removeEventListener("pointerdown", onPointerDown);
  }, [open]);

  const act = async (work: () => Promise<void>) => {
    setBusy(true);
    setNotice(null);
    try {
      await work();
    } catch (err) {
      onError(String(err));
    } finally {
      setBusy(false);
    }
  };

  const save = () =>
    handle &&
    void act(async () => {
      setView(await api.checkpointSave(handle, name.trim() || defaultName()));
      setName("");
    });
  const rename = () =>
    handle &&
    renaming &&
    void act(async () => {
      setView(await api.checkpointRename(handle, renaming.id, renaming.name));
      setRenaming(null);
    });
  const remove = (checkpoint: HistoryCheckpoint) =>
    handle && void act(async () => setView(await api.checkpointDelete(handle, checkpoint.id)));
  const restore = (checkpoint: HistoryCheckpoint) =>
    handle &&
    revision !== undefined &&
    void act(async () => {
      const outcome = await api.checkpointRestore(handle, revision, checkpoint.id);
      applyOpenedProject(outcome.project);
      useAgentStore.getState().setProposals(outcome.proposals);
      setView(outcome.history);
      setNotice(outcome.message);
    });

  if (!openedProject) return null;
  const shown = view?.projectHandle === handle ? view : null;

  return (
    <div ref={rootRef} className="relative">
      <IconButton
        icon={History}
        label="History and checkpoints"
        aria-expanded={open}
        onClick={() => setOpen((v) => !v)}
        className={cn(open && "bg-studio-800 text-studio-100")}
      />
      {open && (
        <div
          role="dialog"
          aria-label="History"
          className="absolute top-full right-0 mt-1.5 z-[1000] w-80 max-h-[70vh] flex flex-col rounded-panel border border-studio-700 bg-studio-850 shadow-popover"
          onKeyDown={(e) => e.key === "Escape" && setOpen(false)}
        >
          <form
            className="flex items-center gap-1.5 p-2 border-b border-studio-800"
            onSubmit={(e) => {
              e.preventDefault();
              save();
            }}
          >
            <input
              aria-label="Checkpoint name"
              placeholder={defaultName()}
              value={name}
              onChange={(e) => setName(e.target.value)}
              className={fieldClass}
            />
            <Button size="sm" variant="secondary" icon={Bookmark} type="submit" disabled={busy} className="shrink-0">
              Save
            </Button>
          </form>
          {notice && <p className="px-3 py-2 text-meta text-studio-300 border-b border-studio-800">{notice}</p>}
          <div className="flex-1 min-h-0 overflow-y-auto p-1">
            <p className="px-2.5 pt-1.5 pb-1 text-meta font-semibold uppercase tracking-wide text-studio-500">Checkpoints</p>
            {!shown ? (
              <p className="px-2.5 py-1 text-meta text-studio-500">Loading…</p>
            ) : shown.checkpoints.length === 0 ? (
              <p className="px-2.5 py-1 text-meta text-studio-500">
                None yet. Save one before a big change; the app saves one before each agent run.
              </p>
            ) : (
              <ul>
                {shown.checkpoints.map((c) => (
                  <li key={c.id} className="group flex items-center gap-1 rounded-control px-2.5 py-1 hover:bg-studio-800">
                    <div className="flex-1 min-w-0">
                      {renaming?.id === c.id ? (
                        <form
                          onSubmit={(e) => {
                            e.preventDefault();
                            rename();
                          }}
                        >
                          <input
                            autoFocus
                            aria-label="New name"
                            value={renaming.name}
                            onChange={(e) => setRenaming({ id: c.id, name: e.target.value })}
                            onBlur={() => setRenaming(null)}
                            className={fieldClass}
                          />
                        </form>
                      ) : (
                        <p className="text-label text-studio-100 truncate" title={c.name}>
                          {c.name}
                        </p>
                      )}
                      <p className="text-meta text-studio-500 truncate">
                        {when(c.at)}
                        {c.auto ? " · saved by the app" : c.by === "agent" ? ` · ${c.agent ?? "agent"}` : ""}
                      </p>
                    </div>
                    <IconButton
                      icon={RotateCcw}
                      size="sm"
                      label={`Go back to "${c.name}"`}
                      disabled={busy}
                      onClick={() => restore(c)}
                    />
                    <IconButton
                      icon={Pencil}
                      size="sm"
                      label="Rename"
                      disabled={busy}
                      onClick={() => setRenaming({ id: c.id, name: c.name })}
                    />
                    <IconButton icon={Trash2} size="sm" label="Delete" disabled={busy} onClick={() => remove(c)} />
                  </li>
                ))}
              </ul>
            )}
            <p className="px-2.5 pt-3 pb-1 text-meta font-semibold uppercase tracking-wide text-studio-500">Steps</p>
            {shown && shown.steps.length === 0 && <p className="px-2.5 py-1 text-meta text-studio-500">No changes yet.</p>}
            <ul>
              {shown?.steps.map((s, i) => (
                <li
                  key={`${s.at}-${i}`}
                  className={cn("flex items-center gap-2 px-2.5 py-0.5 text-meta", s.undone ? "text-studio-500" : "text-studio-300")}
                  title={s.undone ? "Undone: Redo brings it back" : undefined}
                >
                  {s.by === "agent" && <Bot className="w-3.5 h-3.5 shrink-0 text-studio-400" aria-label={s.agent ?? "agent"} />}
                  <span className={cn("flex-1 min-w-0 truncate", s.undone && "line-through")}>{s.label}</span>
                  <span className="shrink-0 tabular-nums text-studio-500">{when(s.at)}</span>
                </li>
              ))}
            </ul>
          </div>
        </div>
      )}
    </div>
  );
};
