import React, { useCallback, useEffect, useRef, useState } from "react";
import { Check, ChevronLeft, ChevronRight, Ear, Play, Search, Trash2, Wrench } from "lucide-react";
import { useProjectStore } from "../../stores/projectStore";
import { api, isTauriEnvironment } from "../../lib/ipc";
import { pausePlayback, seekPlayback, togglePlayback } from "../../lib/playbackControl";
import { JoinMark, JoinsView, JoinView } from "../../lib/types";
import { Badge, Button, IconButton, cn } from "../ui";

/** How much is heard either side of a join. */
const AROUND_US = 2_000_000;

function formatTime(us: number): string {
  const total = us / 1_000_000;
  const minutes = Math.floor(total / 60);
  const seconds = total - minutes * 60;
  return `${minutes}:${seconds.toFixed(1).padStart(4, "0")}`;
}

/** Stops the join playing now, if one is. */
let stopPlaying: (() => void) | null = null;

/** Plays two seconds either side of `atUs`, then stops. */
function playAround(atUs: number) {
  stopPlaying?.();
  const { durationUs, isPlaying } = useProjectStore.getState();
  const to = Math.min(durationUs, atUs + AROUND_US);
  seekPlayback(Math.max(0, atUs - AROUND_US));
  if (!isPlaying) window.setTimeout(() => togglePlayback(), 60);
  let started = isPlaying;
  const unsubscribe = useProjectStore.subscribe((state) => {
    if (state.isPlaying) started = true;
    if (state.isPlaying && state.currentTimeUs >= to) {
      pausePlayback();
      stop();
    } else if (started && !state.isPlaying) {
      stop();
    }
  });
  const stop = () => {
    unsubscribe();
    if (stopPlaying === stop) stopPlaying = null;
  };
  stopPlaying = stop;
}

/** The first join not heard yet after `index` (wrapping), else none. */
function nextUnheard(items: JoinView[], index: number): number {
  for (let step = 1; step <= items.length; step++) {
    const i = (index + step) % items.length;
    if (items[i].atUs !== undefined && !items[i].mark) return i;
  }
  return -1;
}

/// Joins: the cuts an agent (or the app) thinks may sound wrong. Step through them, hear two
/// seconds either side of each, and mark it keep or fix; the agent reads the marks back. Nothing
/// here changes the edit.
export const JoinsPanel: React.FC = () => {
  const handle = useProjectStore((s) => s.openedProject?.projectHandle);
  const revision = useProjectStore((s) => s.openedProject?.revision);
  const [view, setView] = useState<JoinsView | null>(null);
  const [current, setCurrent] = useState(0);
  const [note, setNote] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const listRef = useRef<HTMLUListElement>(null);

  const load = useCallback(
    (request?: Promise<JoinsView>) => {
      if (!handle) return;
      void (request ?? api.joinsList(handle))
        .then((next) => {
          setView(next);
          setError(null);
        })
        .catch((err) => setError(String(err)));
    },
    [handle],
  );

  // Joins move with the edit: read them again at each revision, and when an agent changes them.
  useEffect(() => load(), [load, revision]);
  useEffect(() => {
    if (!isTauriEnvironment()) return;
    let unlisten: (() => void) | undefined;
    let disposed = false;
    void import("@tauri-apps/api/event").then(({ listen }) =>
      listen("aeroedits-joins-updated", () => load()).then((fn) => {
        if (disposed) fn();
        else unlisten = fn;
      }),
    );
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [load]);
  useEffect(() => () => stopPlaying?.(), []);

  const items = view?.items ?? [];
  const index = Math.min(current, Math.max(0, items.length - 1));
  const join = items[index] as JoinView | undefined;

  useEffect(() => {
    setNote(join?.note ?? "");
    listRef.current?.querySelector(`[data-join="${join?.id}"]`)?.scrollIntoView({ block: "nearest" });
  }, [join?.id, join?.note]);

  if (!handle) return <p className="p-3 text-label text-studio-500">Open a project to listen to its cuts.</p>;

  const go = (i: number, play = true) => {
    setCurrent(i);
    const at = items[i]?.atUs;
    if (play && at !== undefined) playAround(at);
  };

  const mark = (value: JoinMark | null) => {
    if (!join) return;
    const after = nextUnheard(items, index);
    load(api.joinsMark(handle, join.id, value, note));
    // On to the next one not heard yet, playing.
    if (value && after >= 0 && after !== index) go(after);
  };

  const run = (request: Promise<JoinsView>) => {
    setBusy(true);
    load(request);
    void request.finally(() => setBusy(false));
  };

  const counts = view && items.length > 0
    ? `${view.unheard} to hear · ${view.keep} keep · ${view.fix} fix${view.changed ? ` · ${view.changed} changed` : ""}`
    : null;

  return (
    <div className="h-full flex flex-col min-h-0">
      <div className="shrink-0 px-3 py-2 border-b border-studio-800 flex items-center gap-2">
        <Ear className="w-4 h-4 text-studio-400 shrink-0" aria-hidden />
        <span className="flex-1 min-w-0 text-label text-studio-300 truncate">{counts ?? "No joins to listen to yet"}</span>
        <Button size="sm" variant="ghost" icon={Search} disabled={busy} onClick={() => run(api.joinsFind(handle))}>
          Find risky cuts
        </Button>
        <IconButton icon={Trash2} size="sm" label="Clear the list" disabled={busy || items.length === 0} onClick={() => run(api.joinsClear(handle))} />
      </div>

      {error && <p role="alert" className="px-3 py-2 text-label text-danger-fg">{error}</p>}

      {join && (
        <div className="shrink-0 p-3 border-b border-studio-800 space-y-2 bg-studio-900/60">
          <div className="flex items-center gap-2">
            <IconButton icon={ChevronLeft} size="sm" label="Previous join" disabled={index === 0} onClick={() => go(index - 1)} />
            <Button
              size="sm"
              variant="primary"
              icon={Play}
              disabled={join.atUs === undefined}
              onClick={() => join.atUs !== undefined && playAround(join.atUs)}
              className="flex-1"
            >
              {join.atUs === undefined ? "Changed by an edit" : `Play ${formatTime(join.atUs)} ± 2 s`}
            </Button>
            <IconButton icon={ChevronRight} size="sm" label="Next join" disabled={index >= items.length - 1} onClick={() => go(index + 1)} />
          </div>
          <p className="text-label text-studio-200">{join.reason || "No reason given"}</p>
          <input
            value={note}
            onChange={(e) => setNote(e.target.value)}
            placeholder="Note for the agent (optional): what is wrong"
            className="w-full h-8 px-2 rounded-control bg-studio-850 border border-studio-700 text-label text-studio-100 placeholder:text-studio-500"
          />
          <div className="flex items-center gap-2">
            <Button size="sm" icon={Check} className="flex-1" variant={join.mark === "keep" ? "primary" : "secondary"} onClick={() => mark(join.mark === "keep" ? null : "keep")}>
              Keep
            </Button>
            <Button size="sm" icon={Wrench} className="flex-1" variant={join.mark === "fix" ? "danger" : "secondary"} onClick={() => mark(join.mark === "fix" ? null : "fix")}>
              Fix
            </Button>
          </div>
        </div>
      )}

      <ul ref={listRef} className="flex-1 min-h-0 overflow-y-auto p-2 space-y-1">
        {items.map((item, i) => (
          <li
            key={item.id}
            data-join={item.id}
            onClick={() => go(i)}
            className={cn(
              "flex items-start gap-2 rounded-control border px-2 py-1.5 cursor-pointer transition-colors",
              i === index ? "border-accent/60 bg-accent/10" : "border-studio-800 hover:bg-studio-850",
              item.atUs === undefined && "opacity-50",
            )}
          >
            <span className="w-12 shrink-0 font-mono text-label tabular-nums text-studio-200">
              {item.atUs === undefined ? "—" : formatTime(item.atUs)}
            </span>
            <span className="flex-1 min-w-0 text-meta text-studio-400 line-clamp-2">{item.reason}</span>
            {item.mark === "keep" && <Badge tone="success">keep</Badge>}
            {item.mark === "fix" && <Badge tone="danger">fix</Badge>}
            {item.atUs === undefined && <Badge>changed</Badge>}
          </li>
        ))}
        {items.length === 0 && (
          <li className="p-2 text-label text-studio-500">
            Agents add the cuts they can't judge by ear here. Find risky cuts adds the ones most likely to sound wrong: retakes,
            words cut into, blips.
          </li>
        )}
      </ul>
    </div>
  );
};
