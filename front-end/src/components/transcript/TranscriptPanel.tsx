import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { AudioLines, Captions, Check, Eye, EyeOff, Loader2, Pencil, Play, RotateCcw, Scissors, Settings2, Sparkles, X } from "lucide-react";
import { useProjectStore } from "../../stores/projectStore";
import { api, isTauriEnvironment } from "../../lib/ipc";
import {
  OpenedProject,
  TranscriptCutSuggestion,
  TranscriptViewWord,
  TranscriptionProgress,
  TranscriptView,
} from "../../lib/types";
import { TranscriptSettingsModal } from "./TranscriptSettingsModal";
import { transcribableSounds } from "../../lib/trackUtils";

/** The recording's audio tracks and imported sound, speech first. */
function audioTracks(project: OpenedProject | null) {
  return transcribableSounds(project);
}

function formatTime(us: number): string {
  const total = us / 1_000_000;
  const minutes = Math.floor(total / 60);
  const seconds = total - minutes * 60;
  return `${minutes}:${seconds.toFixed(1).padStart(4, "0")}`;
}

type ReviewFilter = "all" | "filler" | "retake";

/** Lines break at pauses, sentence ends, speaker changes and jumps in time. */
const LINE_PAUSE_US = 1_000_000;
const LINE_MAX_US = 12_000_000;
const LINE_SENTENCE_MIN_WORDS = 6;

type LineWord = { w: TranscriptViewWord; i: number };
type TranscriptLine = { words: LineWord[]; startUs: number | null; endUs: number | null; sourceStartUs: number };

/// Groups words (already in playback order) into time-stamped lines.
function transcriptLines(words: LineWord[]): TranscriptLine[] {
  const lines: TranscriptLine[] = [];
  let current: LineWord[] = [];
  const flush = () => {
    if (current.length === 0) return;
    const kept = current.filter(({ w }) => w.editedStartUs !== null);
    lines.push({
      words: current,
      startUs: kept.length > 0 ? kept[0].w.editedStartUs : null,
      endUs: kept.length > 0 ? kept[kept.length - 1].w.editedEndUs : null,
      sourceStartUs: current[0].w.sourceStartUs,
    });
    current = [];
  };
  for (const item of words) {
    const prev = current[current.length - 1]?.w;
    if (prev) {
      const w = item.w;
      const gap = w.sourceStartUs - prev.sourceEndUs;
      const jumped = gap < 0 || (prev.editedEndUs !== null && w.editedStartUs !== null && w.editedStartUs - prev.editedEndUs > LINE_PAUSE_US);
      const span = w.sourceEndUs - current[0].w.sourceStartUs;
      const sentenceEnd = /[.!?…]["')\]]?$/.test(prev.text) && current.length >= LINE_SENTENCE_MIN_WORDS;
      if (jumped || gap > LINE_PAUSE_US || sentenceEnd || span > LINE_MAX_US || (w.speaker ?? "") !== (prev.speaker ?? "")) {
        flush();
      }
    }
    current.push(item);
  }
  flush();
  return lines;
}

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
  const captionsVersion = useProjectStore((s) => s.captionsVersion);
  const bumpCaptions = useProjectStore((s) => s.bumpCaptions);

  useEffect(() => {
    setTrackId(audioTracks(openedProject)[0]?.id ?? "");
    setSelection(null);
  }, [handle]);
  // Sound added after opening (media imported into an empty project), or the chosen sound
  // removed: pick the first one there is.
  const trackIds = tracks.map((t) => t.id).join("|");
  useEffect(() => {
    if (!tracks.some((t) => t.id === trackId)) setTrackId(tracks[0]?.id ?? "");
  }, [trackIds]);

  // Only the latest refresh may apply: a slow older reply must not replace a newer one.
  const refreshGeneration = useRef(0);
  const refresh = useCallback(async () => {
    const generation = ++refreshGeneration.current;
    if (!handle || !trackId) {
      setView(null);
      setSuggestions([]);
      return;
    }
    try {
      const next = await api.transcriptGet(handle, trackId);
      const nextSuggestions = next ? await api.transcriptSuggestions(handle, trackId) : [];
      if (generation !== refreshGeneration.current) return;
      setView(next);
      setSuggestions(nextSuggestions);
    } catch (err) {
      if (generation === refreshGeneration.current) setError(errorMessage(err));
    }
  }, [handle, trackId]);

  /** A caption change from here: saved in the transcript, then the caption track reloads. */
  const editCaptions = async (change: Parameters<typeof api.transcriptCaptionEdit>[2], label: string) => {
    if (!handle || !trackId) return;
    setError(null);
    try {
      await api.transcriptCaptionEdit(handle, trackId, change);
      bumpCaptions();
      setNotice(label);
    } catch (err) {
      setError(errorMessage(err));
    }
  };

  // Edited positions change with every cut and undo, so reload on each revision.
  useEffect(() => {
    void refresh();
  }, [refresh, revision, captionsVersion]);

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
  const lines = useMemo(() => transcriptLines(visible), [visible]);
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
      bumpCaptions();
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

  const runAiReview = async () => {
    if (!handle || !trackId) return;
    // Without a key the request cannot start: say so and open the settings instead.
    try {
      const ai = await api.aiSettingsGet();
      const source = ai.settings.provider === "openAi" ? ai.openaiKeySource : ai.openrouterKeySource;
      if (!source) {
        setError(null);
        setNotice(
          `Add your ${ai.settings.provider === "openAi" ? "OpenAI" : "OpenRouter"} API key under AI review, then press Find with AI again.`,
        );
        setSettingsOpen(true);
        return;
      }
    } catch (err) {
      setError(errorMessage(err));
      return;
    }
    setRunning(true);
    setError(null);
    setNotice(null);
    setProgress(null);
    try {
      const before = new Set(suggestions.map((s) => s.id));
      const next = await api.transcriptAiSuggest(handle, trackId);
      setSuggestions(next);
      const added = next.filter((s) => s.source === "ai" && !before.has(s.id)).length;
      setReviewOpen(true);
      setNotice(
        added > 0
          ? `AI review found ${added} more suggestion${added === 1 ? "" : "s"}.`
          : "AI review found nothing the rules had not already found.",
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

  /** Takes the punctuation off every word; captions follow. */
  const stripPunctuation = async () => {
    if (!handle || !trackId) return;
    setError(null);
    try {
      setView(await api.transcriptStripPunctuation(handle, trackId));
      bumpCaptions();
      setNotice("Removed the punctuation. Fix a word by hand (Enter) to put any back.");
    } catch (err) {
      setError(errorMessage(err));
    }
  };

  const commitEdit = async () => {
    if (!editing || !handle || !trackId) return;
    const word = words[editing.index];
    setEditing(null);
    if (!word || editing.text.trim() === word.text) return;
    setError(null);
    try {
      setView(await api.transcriptSetWordText(handle, trackId, word.id, editing.text));
      bumpCaptions();
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
            className="min-w-0 max-w-[15rem] truncate bg-studio-800 text-studio-100 text-xs rounded px-1.5 py-0.5 border border-studio-700 focus:outline-none focus:border-teal-500"
          >
            {tracks.map((t) => (
              <option key={t.id} value={t.id}>
                {t.label}
              </option>
            ))}
          </select>
        )}
        {view && (
          <span className="text-studio-500 truncate">
            {view.model} · {view.words.filter((w) => w.editedStartUs !== null).length}/{view.words.length} words kept
          </span>
        )}
        {view && view.words.length > 0 && view.words.every((w) => w.editedStartUs === null) && trackId.startsWith("msound-") && (
          <span className="text-amber-300 truncate" title="Its words show once a clip of it is on the timeline, on any track">
            Not on the timeline yet: place it to edit and caption it
          </span>
        )}
        <div className="ml-auto flex items-center gap-1.5">
          {view && (
            <button
              type="button"
              title="Review filler sounds and restarted sentences one by one, or find more with AI"
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
          {selectedIds.length > 0 && (() => {
            const chosen = selectedIds.map((id) => words[wordIndex.get(id) ?? -1]).filter(Boolean);
            const hidden = chosen.length > 0 && chosen.every((w) => w.captionHidden);
            const first = chosen[0];
            return (
              <>
                <button
                  type="button"
                  title={hidden ? "Show these words in the captions again" : "Keep the sound but leave these words out of the captions"}
                  onClick={() =>
                    void editCaptions(
                      { kind: "hide", wordIds: selectedIds, hidden: !hidden },
                      hidden ? "Shown in the captions again." : "Hidden from the captions; the sound stays.",
                    )
                  }
                  className="flex items-center gap-1 px-2 py-1 rounded bg-studio-800 text-studio-200 hover:bg-studio-700"
                >
                  <Captions className="w-3 h-3" /> {hidden ? "Show in captions" : "Hide in captions"}
                </button>
                {first && (
                  <button
                    type="button"
                    title={first.captionBreak ? "Let this caption join the one before" : "Start a new caption at this word"}
                    onClick={() =>
                      void editCaptions(
                        first.captionBreak ? { kind: "merge", wordId: first.id } : { kind: "split", wordId: first.id },
                        first.captionBreak ? "Captions merged." : "A new caption starts here.",
                      )
                    }
                    className="px-2 py-1 rounded bg-studio-800 text-studio-200 hover:bg-studio-700"
                  >
                    {first.captionBreak ? "Merge caption" : "New caption here"}
                  </button>
                )}
              </>
            );
          })()}
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
              disabled={running}
              title="Remove punctuation (. , ! ? : ; quotes, brackets) from every word, and so from the captions. Apostrophes, hyphens and numbers stay."
              onClick={() => void stripPunctuation()}
              className="px-1.5 py-0.5 rounded font-mono text-studio-400 hover:text-white hover:bg-studio-800 disabled:opacity-40"
            >
              .,?<span className="sr-only"> Remove punctuation</span>
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
        className="flex-1 min-w-0 overflow-y-auto px-2 py-2 space-y-0.5 leading-6 text-[13px] text-studio-200 outline-none select-none"
      >
        {!view && !running && (
          <p className="text-studio-500">
            Transcribe {tracks.find((t) => t.id === trackId)?.label ?? "a sound track"} to edit
            the video by deleting words. Click a word to select it, shift-click to extend, then press Delete. Double-click a
            word to jump to it, or press Enter to fix a misheard word. Review lists every filler sound and restarted sentence so
            you can keep or cut each one.
          </p>
        )}
        {lines.map((line, lineIndex) => {
          const playing =
            line.startUs !== null && line.endUs !== null && currentTimeUs >= line.startUs && currentTimeUs < line.endUs;
          return (
            <div
              key={line.words[0].w.id}
              className={`flex gap-3 rounded-md px-1.5 py-1 ${playing ? "bg-teal-950/40" : lineIndex % 2 ? "bg-studio-900/30" : ""}`}
            >
              <button
                type="button"
                disabled={line.startUs === null}
                onClick={() => line.startUs !== null && seekTo(line.startUs)}
                className={`shrink-0 w-14 text-left font-mono text-[11px] leading-6 tabular-nums disabled:cursor-default ${
                  playing ? "text-teal-300" : line.startUs === null ? "text-studio-600 line-through" : "text-studio-500 hover:text-teal-300"
                }`}
                title={line.startUs === null ? "This line is cut" : "Jump here"}
              >
                {formatTime(line.startUs ?? line.sourceStartUs)}
              </button>
              <p className="flex-1 min-w-0">
                {line.words.map(({ w, i }) => {
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
                    // Heard but left out of the captions.
                    w.captionHidden && !cut ? "opacity-50 decoration-dotted underline decoration-studio-500" : "",
                  ].join(" ");
                  return (
                    <React.Fragment key={w.id}>
                      {w.captionBreak && !cut && (
                        <span
                          className="inline-block w-0.5 h-3 mx-0.5 align-middle bg-amber-400/80 rounded"
                          title="A new caption starts here"
                          aria-label="Caption break"
                        />
                      )}
                      <span
                        ref={i === activeIndex ? activeRef : undefined}
                        className={classes}
                        title={
                          kind === "filler"
                            ? "Filler sound"
                            : kind === "retake"
                              ? "Abandoned take"
                              : w.captionHidden
                                ? "Hidden from the captions"
                                : undefined
                        }
                        onClick={(e) => {
                          if (cut) return;
                          setSelection(e.shiftKey && selection ? { anchor: selection.anchor, focus: i } : { anchor: i, focus: i });
                        }}
                        onDoubleClick={() => {
                          if (w.editedStartUs !== null) seekTo(w.editedStartUs);
                        }}
                      >
                        {w.text}
                      </span>
                      {/* No space before punctuation that comes as a word of its own. */}
                      {/^[^\p{L}\p{N}]+$/u.test(line.words[line.words.findIndex((x) => x.i === i) + 1]?.w.text ?? "") ? "" : " "}
                    </React.Fragment>
                  );
                        })}
              </p>
            </div>
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
          <div className="px-2 pt-2">
            <button
              type="button"
              disabled={running}
              onClick={() => void runAiReview()}
              className="w-full flex items-center justify-center gap-1 px-2 py-1 rounded bg-violet-900/40 border border-violet-600/40 text-violet-100 hover:bg-violet-800/50 disabled:opacity-40"
              title="Ask the AI provider from Transcription and AI settings to find filler words and retakes in context. Sends the transcript text."
            >
              <Sparkles className="w-3 h-3" /> Find with AI
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
                  {s.source === "ai" && (
                    <span className="px-1 rounded text-[10px] font-semibold bg-sky-900/60 text-sky-200" title="Found by the AI review">
                      AI
                    </span>
                  )}
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
                {s.reason && <p className="mt-0.5 text-[11px] text-studio-500 italic">{s.reason}</p>}
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
