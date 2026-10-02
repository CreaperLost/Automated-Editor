import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { AudioLines, Check, Eye, EyeOff, Loader2, Pencil, Play, RotateCcw, Scissors, Settings2, Sparkles, X } from "lucide-react";
import { useProjectStore } from "../../stores/projectStore";
import { api, isTauriEnvironment } from "../../lib/ipc";
import {
  OpenedProject,
  TranscriptCutSuggestion,
  TranscriptionProgress,
  TranscriptView,
} from "../../lib/types";
import { TranscriptSettingsModal } from "./TranscriptSettingsModal";

function audioTracks(project: OpenedProject | null) {
  if (!project) return [];
  const tracks = project.tracks.filter(
    (t) => t.descriptor.trackType === "mic_audio" || t.descriptor.trackType === "system_audio",
  );
  // Microphone first: that is where the speech is.
  return tracks.sort((a, b) =>
    a.descriptor.trackType === b.descriptor.trackType ? 0 : a.descriptor.trackType === "mic_audio" ? -1 : 1,
  );
}

function formatTime(us: number): string {
  const total = us / 1_000_000;
  const minutes = Math.floor(total / 60);
  const seconds = total - minutes * 60;
  return `${minutes}:${seconds.toFixed(1).padStart(4, "0")}`;
}

type ReviewFilter = "all" | "filler" | "retake";

function errorMessage(err: unknown): string {
  if (err instanceof Error && err.message) return err.message;
  if (typeof err === "string" && err.trim()) return err;
  return "Something went wrong.";
}

export const TranscriptPanel: React.FC = () => {
  const { openedProject, applyOpenedProject, applyPlaybackStatus, currentTimeUs } = useProjectStore();
  const tracks = audioTracks(openedProject);
  const [trackId, setTrackId] = useState<string>("");
  const [view, setView] = useState<TranscriptView | null>(null);
  const [suggestions, setSuggestions] = useState<TranscriptCutSuggestion[]>([]);
  const [selection, setSelection] = useState<{ anchor: number; focus: number } | null>(null);
  const [showCut, setShowCut] = useState(false);
  const [progress, setProgress] = useState<TranscriptionProgress | null>(null);
  const [running, setRunning] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [reviewOpen, setReviewOpen] = useState(false);
  const [reviewFilter, setReviewFilter] = useState<ReviewFilter>("all");
  const [showRejected, setShowRejected] = useState(false);
  const [editing, setEditing] = useState<{ index: number; text: string } | null>(null);
  const activeRef = useRef<HTMLSpanElement | null>(null);

  const handle = openedProject?.projectHandle;
  const revision = openedProject?.revision;

  useEffect(() => {
    setTrackId(audioTracks(openedProject)[0]?.descriptor.id ?? "");
    setSelection(null);
  }, [handle]);

  const refresh = useCallback(async () => {
    if (!handle || !trackId) {
      setView(null);
      setSuggestions([]);
      return;
    }
    try {
      const next = await api.transcriptGet(handle, trackId);
      setView(next);
      setSuggestions(next ? await api.transcriptSuggestions(handle, trackId) : []);
    } catch (err) {
      setError(errorMessage(err));
    }
  }, [handle, trackId]);

  // Edited positions change with every cut and undo, so reload on each revision.
  useEffect(() => {
    void refresh();
  }, [refresh, revision]);

  useEffect(() => {
    if (!isTauriEnvironment()) return;
    let unlisten: (() => void) | undefined;
    let disposed = false;
    void import("@tauri-apps/api/event").then(({ listen }) =>
      listen<TranscriptionProgress>("transcript-progress", (event) => setProgress(event.payload)).then((fn) => {
        if (disposed) fn();
        else unlisten = fn;
      }),
    );
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  const words = view?.words ?? [];
  const visible = useMemo(
    () => words.map((w, i) => ({ w, i })).filter(({ w }) => showCut || w.editedStartUs !== null),
    [words, showCut],
  );
  const pending = useMemo(() => suggestions.filter((s) => !s.dismissed), [suggestions]);
  const suggestionKind = useMemo(() => {
    const map = new Map<string, TranscriptCutSuggestion["kind"]>();
    for (const s of pending) for (const id of s.wordIds) map.set(id, s.kind);
    return map;
  }, [pending]);
  const wordIndex = useMemo(() => new Map(words.map((w, i) => [w.id, i])), [words]);
  const reviewed = suggestions.filter(
    (s) => (reviewFilter === "all" || s.kind === reviewFilter) && (showRejected ? s.dismissed : !s.dismissed),
  );
  const rejectedCount = suggestions.length - pending.length;

  const activeIndex = useMemo(
    () =>
      words.findIndex(
        (w) => w.editedStartUs !== null && w.editedEndUs !== null && currentTimeUs >= w.editedStartUs && currentTimeUs < w.editedEndUs,
      ),
    [words, currentTimeUs],
  );

  useEffect(() => {
    activeRef.current?.scrollIntoView({ block: "nearest" });
  }, [activeIndex]);

  const selectedIds = useMemo(() => {
    if (!selection) return [];
    const [a, b] = [Math.min(selection.anchor, selection.focus), Math.max(selection.anchor, selection.focus)];
    return words.slice(a, b + 1).filter((w) => w.editedStartUs !== null).map((w) => w.id);
  }, [selection, words]);

  const cutWords = async (ids: string[], label: string) => {
    if (!openedProject || !trackId || ids.length === 0) return;
    setError(null);
    try {
      const next = await api.transcriptCutWords(openedProject.projectHandle, openedProject.revision, trackId, ids);
      applyOpenedProject(next);
      setSelection(null);
      setNotice(`${label}. Undo restores it.`);
    } catch (err) {
      setError(errorMessage(err));
    }
  };

  const runTranscription = async () => {
    if (!handle || !trackId) return;
    setRunning(true);
    setError(null);
    setNotice(null);
    setProgress(null);
    try {
      const result = await api.transcriptRun(handle, trackId);
      setView(result.view);
      setSuggestions(await api.transcriptSuggestions(handle, trackId));
      const count = result.view.words.length;
      setNotice(
        `Transcribed ${count} words.` + (result.diagnostics.length ? ` ${result.diagnostics.join(" · ")}` : ""),
      );
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setRunning(false);
      setProgress(null);
    }
  };

  const dismiss = async (ids: string[], dismissed: boolean) => {
    if (!handle || !trackId || ids.length === 0) return;
    setError(null);
    try {
      setSuggestions(await api.transcriptDismissSuggestions(handle, trackId, ids, dismissed));
    } catch (err) {
      setError(errorMessage(err));
    }
  };

  const focusSuggestion = (s: TranscriptCutSuggestion) => {
    const first = wordIndex.get(s.wordIds[0]);
    const last = wordIndex.get(s.wordIds[s.wordIds.length - 1]);
    if (first !== undefined && last !== undefined) setSelection({ anchor: first, focus: last });
    // Start a moment early so the cut is heard in context.
    seekTo(Math.max(0, s.editedStartUs - 1_000_000));
  };

  const startEditing = () => {
    const index = selectedIds.length === 1 ? wordIndex.get(selectedIds[0]) : undefined;
    if (index === undefined) return;
    setEditing({ index, text: words[index].text });
  };

  const commitEdit = async () => {
    if (!editing || !handle || !trackId) return;
    const word = words[editing.index];
    setEditing(null);
    if (!word || editing.text.trim() === word.text) return;
    setError(null);
    try {
      setView(await api.transcriptSetWordText(handle, trackId, word.id, editing.text));
      setNotice(`Changed "${word.text}" to "${editing.text.trim()}".`);
    } catch (err) {
      setError(errorMessage(err));
    }
  };

  const seekTo = (us: number) => {
    if (!handle) return;
    void api
      .playbackSeek(handle, us)
      .then(applyPlaybackStatus)
      .catch(() => undefined);
  };

  const onKeyDown = (e: React.KeyboardEvent) => {
    if (editing) return;
    if ((e.key === "Enter" || e.key === "F2") && selectedIds.length === 1) {
      e.preventDefault();
      startEditing();
    } else if ((e.key === "Delete" || e.key === "Backspace") && selectedIds.length > 0) {
      e.preventDefault();
      void cutWords(selectedIds, `Cut ${selectedIds.length} word${selectedIds.length === 1 ? "" : "s"}`);
    } else if (e.key === "Escape") {
      setSelection(null);
    }
  };

  if (!openedProject) return null;

  return (
    <div className="h-full flex flex-col min-h-0 bg-studio-900/40 border border-studio-800 rounded-xl text-xs">
      <div className="flex items-center gap-2 px-3 py-2 border-b border-studio-800">
        <AudioLines className="w-3.5 h-3.5 text-teal-400" />
        <span className="font-semibold text-studio-200">Transcript</span>
        {tracks.length > 1 && (
          <select
            aria-label="Transcript track"
            value={trackId}
            onChange={(e) => {
              setTrackId(e.target.value);
              setSelection(null);
            }}
            className="bg-studio-800 text-studio-100 rounded px-1.5 py-0.5"
          >
            {tracks.map((t) => (
              <option key={t.descriptor.id} value={t.descriptor.id}>
                {t.descriptor.trackType === "mic_audio" ? "Microphone" : "System audio"} ({t.descriptor.id})
              </option>
            ))}
          </select>
        )}
        {view && (
          <span className="text-studio-500 truncate">
            {view.model} · {view.words.filter((w) => w.editedStartUs !== null).length}/{view.words.length} words kept
          </span>
        )}
        <div className="ml-auto flex items-center gap-1.5">
          {view && suggestions.length > 0 && (
            <button
              type="button"
              title="Review filler sounds and restarted sentences one by one"
              onClick={() => setReviewOpen(!reviewOpen)}
              aria-pressed={reviewOpen}
              className={`flex items-center gap-1 px-2 py-1 rounded border ${
                reviewOpen
                  ? "bg-amber-800/60 border-amber-600 text-amber-100"
                  : "bg-amber-900/40 border-amber-700/50 text-amber-200 hover:bg-amber-800/50"
              }`}
            >
              <Sparkles className="w-3 h-3" /> Review {pending.length}
            </button>
          )}
          {selectedIds.length === 1 && !editing && (
            <button
              type="button"
              title="Fix this word's text (Enter). Captions show the corrected word."
              onClick={startEditing}
              className="flex items-center gap-1 px-2 py-1 rounded bg-studio-800 text-studio-200 hover:bg-studio-700"
            >
              <Pencil className="w-3 h-3" /> Fix word
            </button>
          )}
          {selectedIds.length > 0 && (
            <button
              type="button"
              onClick={() => void cutWords(selectedIds, `Cut ${selectedIds.length} words`)}
              className="flex items-center gap-1 px-2 py-1 rounded bg-rose-900/50 border border-rose-700/50 text-rose-200 hover:bg-rose-800/60"
            >
              <Scissors className="w-3 h-3" /> Cut {selectedIds.length} selected
            </button>
          )}
          {view && (
            <button
              type="button"
              title={showCut ? "Hide cut words" : "Show cut words"}
              onClick={() => setShowCut(!showCut)}
              className="p-1 rounded text-studio-400 hover:text-white hover:bg-studio-800"
            >
              {showCut ? <EyeOff className="w-3.5 h-3.5" /> : <Eye className="w-3.5 h-3.5" />}
            </button>
          )}
          {running ? (
            <button
              type="button"
              onClick={() => void api.transcriptCancel()}
              className="flex items-center gap-1 px-2 py-1 rounded bg-studio-800 text-studio-200 hover:bg-studio-700"
            >
              <Loader2 className="w-3 h-3 animate-spin" /> Cancel
            </button>
          ) : (
            <button
              type="button"
              disabled={!trackId}
              onClick={() => void runTranscription()}
              className="px-2 py-1 rounded bg-teal-700 hover:bg-teal-600 disabled:opacity-40 text-white font-semibold"
            >
              {view ? "Re-transcribe" : "Transcribe"}
            </button>
          )}
          <button
            type="button"
            title="Transcription settings"
            onClick={() => setSettingsOpen(true)}
            className="p-1 rounded text-studio-400 hover:text-white hover:bg-studio-800"
          >
            <Settings2 className="w-3.5 h-3.5" />
          </button>
        </div>
      </div>

      {running && (
        <div className="px-3 py-1.5 border-b border-studio-800 flex items-center gap-2 text-teal-300">
          <div className="flex-1 h-1 bg-studio-800 rounded overflow-hidden">
            <div className="h-full bg-teal-500 transition-all" style={{ width: `${Math.round((progress?.fraction ?? 0) * 100)}%` }} />
          </div>
          <span>{progress?.message ?? "Starting"}</span>
        </div>
      )}
      {error && (
        <div role="alert" className="px-3 py-1.5 border-b border-rose-900/60 bg-rose-950/40 text-rose-200 flex items-center gap-2">
          <span className="flex-1">{error}</span>
          <button type="button" onClick={() => setError(null)} className="text-rose-400 hover:text-rose-100">
            <X className="w-3 h-3" />
          </button>
        </div>
      )}
      {notice && !error && (
        <div className="px-3 py-1 border-b border-studio-800 text-studio-400 flex items-center gap-2">
          <span className="flex-1 truncate">{notice}</span>
          <button type="button" onClick={() => setNotice(null)} className="text-studio-500 hover:text-studio-200">
            <X className="w-3 h-3" />
          </button>
        </div>
      )}

      <div className="flex-1 min-h-0 flex">
      <div
        tabIndex={0}
        onKeyDown={onKeyDown}
        className="flex-1 min-w-0 overflow-y-auto px-3 py-2 leading-6 text-[13px] text-studio-200 outline-none select-none"
      >
        {!view && !running && (
          <p className="text-studio-500">
            Transcribe the {tracks[0]?.descriptor.trackType === "system_audio" ? "system audio" : "microphone"} track to edit
            the video by deleting words. Click a word to select it, shift-click to extend, then press Delete. Double-click a
            word to jump to it, or press Enter to fix a misheard word. Review lists every filler sound and restarted sentence so
            you can keep or cut each one.
          </p>
        )}
        {visible.map(({ w, i }) => {
          const cut = w.editedStartUs === null;
          const selected =
            selection !== null &&
            i >= Math.min(selection.anchor, selection.focus) &&
            i <= Math.max(selection.anchor, selection.focus);
          const kind = suggestionKind.get(w.id);
          if (editing?.index === i) {
            return (
              <React.Fragment key={w.id}>
                <input
                  autoFocus
                  aria-label="Word text"
                  value={editing.text}
                  size={Math.max(4, editing.text.length + 1)}
                  onChange={(e) => setEditing({ index: i, text: e.target.value })}
                  onKeyDown={(e) => {
                    e.stopPropagation();
                    if (e.key === "Enter") void commitEdit();
                    else if (e.key === "Escape") setEditing(null);
                  }}
                  onBlur={() => void commitEdit()}
                  className="bg-studio-800 text-white rounded px-1 outline outline-1 outline-teal-500"
                />{" "}
              </React.Fragment>
            );
          }
          const classes = [
            "rounded px-0.5 cursor-pointer",
            cut ? "line-through text-studio-600" : "hover:bg-studio-800",
            selected && !cut ? "bg-rose-800/60 text-white" : "",
            i === activeIndex ? "bg-teal-800/70 text-white" : "",
            kind === "filler" && !cut ? "underline decoration-amber-400 decoration-2" : "",
            kind === "retake" && !cut ? "underline decoration-violet-400 decoration-2" : "",
            w.kind === "audioEvent" ? "italic text-studio-400" : "",
          ].join(" ");
          return (
            <React.Fragment key={w.id}>
              <span
                ref={i === activeIndex ? activeRef : undefined}
                className={classes}
                title={kind === "filler" ? "Filler sound" : kind === "retake" ? "Abandoned take" : undefined}
                onClick={(e) => {
                  if (cut) return;
                  setSelection(e.shiftKey && selection ? { anchor: selection.anchor, focus: i } : { anchor: i, focus: i });
                }}
                onDoubleClick={() => {
                  if (w.editedStartUs !== null) seekTo(w.editedStartUs);
                }}
              >
                {w.text}
              </span>{" "}
            </React.Fragment>
          );
        })}
      </div>
      {reviewOpen && view && (
        <aside className="w-72 shrink-0 border-l border-studio-800 flex flex-col min-h-0" aria-label="Suggestion review">
          <div className="px-2 py-1.5 border-b border-studio-800 flex items-center gap-1">
            {(["all", "filler", "retake"] as const).map((f) => (
              <button
                key={f}
                type="button"
                onClick={() => setReviewFilter(f)}
                className={`px-1.5 py-0.5 rounded ${
                  reviewFilter === f ? "bg-studio-700 text-white" : "text-studio-400 hover:text-white"
                }`}
              >
                {f === "all" ? "All" : f === "filler" ? "Fillers" : "Retakes"}
              </button>
            ))}
            <button
              type="button"
              onClick={() => setShowRejected(!showRejected)}
              disabled={rejectedCount === 0 && !showRejected}
              className="ml-auto text-studio-400 hover:text-white disabled:opacity-40"
              title="Rejected suggestions stay out of Accept all"
            >
              {showRejected ? "Back" : `Rejected ${rejectedCount}`}
            </button>
          </div>
          {!showRejected && reviewed.length > 1 && (
            <button
              type="button"
              onClick={() =>
                void cutWords(
                  reviewed.flatMap((s) => s.wordIds),
                  `Removed ${reviewed.length} suggestion${reviewed.length === 1 ? "" : "s"}`,
                )
              }
              className="mx-2 mt-2 flex items-center justify-center gap-1 px-2 py-1 rounded bg-rose-900/50 border border-rose-700/50 text-rose-200 hover:bg-rose-800/60"
            >
              <Scissors className="w-3 h-3" /> Accept all {reviewed.length}
            </button>
          )}
          <ul className="flex-1 min-h-0 overflow-y-auto p-2 space-y-1.5">
            {reviewed.length === 0 && (
              <li className="text-studio-500 px-1">{showRejected ? "Nothing rejected." : "Nothing left to review."}</li>
            )}
            {reviewed.map((s) => (
              <li
                key={s.id}
                className="rounded border border-studio-800 bg-studio-900/60 hover:border-studio-600 p-1.5 cursor-pointer"
                onClick={() => focusSuggestion(s)}
              >
                <div className="flex items-center gap-1.5">
                  <span
                    className={`px-1 rounded text-[10px] uppercase font-semibold ${
                      s.kind === "filler" ? "bg-amber-900/60 text-amber-200" : "bg-violet-900/60 text-violet-200"
                    }`}
                  >
                    {s.kind === "filler" ? "Filler" : "Retake"}
                  </span>
                  <span className="font-mono text-studio-500">{formatTime(s.editedStartUs)}</span>
                  <div className="ml-auto flex items-center gap-0.5" onClick={(e) => e.stopPropagation()}>
                    <button
                      type="button"
                      title="Play from just before it"
                      onClick={() => focusSuggestion(s)}
                      className="p-1 rounded text-studio-400 hover:text-white hover:bg-studio-800"
                    >
                      <Play className="w-3 h-3" />
                    </button>
                    {s.dismissed ? (
                      <button
                        type="button"
                        title="Put it back in the review list"
                        onClick={() => void dismiss([s.id], false)}
                        className="p-1 rounded text-studio-400 hover:text-white hover:bg-studio-800"
                      >
                        <RotateCcw className="w-3 h-3" />
                      </button>
                    ) : (
                      <>
                        <button
                          type="button"
                          title="Accept: cut these words"
                          onClick={() =>
                            void cutWords(s.wordIds, `Cut ${s.kind === "filler" ? "filler" : "retake"} "${s.text}"`)
                          }
                          className="p-1 rounded text-emerald-400 hover:text-white hover:bg-emerald-800/60"
                        >
                          <Check className="w-3 h-3" />
                        </button>
                        <button
                          type="button"
                          title="Reject: keep these words"
                          onClick={() => void dismiss([s.id], true)}
                          className="p-1 rounded text-rose-400 hover:text-white hover:bg-rose-900/60"
                        >
                          <X className="w-3 h-3" />
                        </button>
                      </>
                    )}
                  </div>
                </div>
                <p className={`mt-1 text-studio-200 ${s.dismissed ? "text-studio-500" : ""}`}>&ldquo;{s.text}&rdquo;</p>
              </li>
            ))}
          </ul>
        </aside>
      )}
      </div>

      {settingsOpen && <TranscriptSettingsModal onClose={() => setSettingsOpen(false)} />}
    </div>
  );
};
