import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { AudioLines, Captions, Check, Eye, EyeOff, Loader2, Pencil, Play, RotateCcw, Scissors, Settings2, Sparkles } from "lucide-react";
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
import { soundSources } from "../../lib/sequence";
import { Badge, Button, IconButton, Notice, Segmented, cn } from "../ui";

/** Every sound in the project, speech first, as `{ id, label, placed }`. */
function audioTracks(project: OpenedProject | null) {
  return soundSources(project).map((sound) => ({ ...sound, id: sound.key }));
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

/** What a transcript line does for clicks and typing; read through a ref, so a line does not
 * re-render because the panel made new handlers. */
interface LineActions {
  pick: (index: number, extend: boolean) => void;
  seekTo: (us: number) => void;
  setEditing: (editing: { index: number; text: string } | null) => void;
  commitEdit: () => void;
}

/**
 * One line of the transcript. Memoized: the playing word moving on, or a selection changing,
 * redraws only the lines it touches, not every word of a long transcript.
 */
const TranscriptLineRow = React.memo(function TranscriptLineRow({
  line,
  lineIndex,
  playing,
  active,
  selFrom,
  selTo,
  editing,
  suggestionKind,
  activeRef,
  actions,
}: {
  line: TranscriptLine;
  lineIndex: number;
  playing: boolean;
  /** The playing word, if it is in this line; else -1. */
  active: number;
  /** The selected words of this line (indices into the transcript); -1 when none are. */
  selFrom: number;
  selTo: number;
  editing: { index: number; text: string } | null;
  suggestionKind: Map<string, string>;
  activeRef: React.MutableRefObject<HTMLSpanElement | null>;
  actions: React.MutableRefObject<LineActions>;
}) {
  return (
    <div
      className={`flex gap-4 rounded-control px-2 py-1.5 ${playing ? "bg-accent/10 shadow-[inset_2px_0_0_rgb(var(--accent-hover))]" : lineIndex % 2 ? "bg-studio-850/40" : ""}`}
    >
      <button
        type="button"
        disabled={line.startUs === null}
        onClick={() => line.startUs !== null && actions.current.seekTo(line.startUs)}
        className={`shrink-0 w-14 pt-1 text-left font-mono text-meta tabular-nums disabled:cursor-default ${
          playing ? "text-accent-fg" : line.startUs === null ? "text-studio-600 line-through" : "text-studio-500 hover:text-accent-fg"
        }`}
        title={line.startUs === null ? "This line is cut" : "Jump here"}
      >
        {formatTime(line.startUs ?? line.sourceStartUs)}
      </button>
      <p className="flex-1 min-w-0">
        {line.words.map(({ w, i }, position) => {
          const cut = w.editedStartUs === null;
          const selected = selFrom >= 0 && i >= selFrom && i <= selTo;
          const kind = suggestionKind.get(w.id);
          if (editing?.index === i) {
            return (
              <React.Fragment key={w.id}>
                <input
                  autoFocus
                  aria-label="Word text"
                  value={editing.text}
                  size={Math.max(4, editing.text.length + 1)}
                  onChange={(e) => actions.current.setEditing({ index: i, text: e.target.value })}
                  onKeyDown={(e) => {
                    e.stopPropagation();
                    if (e.key === "Enter") actions.current.commitEdit();
                    else if (e.key === "Escape") actions.current.setEditing(null);
                  }}
                  onBlur={() => actions.current.commitEdit()}
                  className="bg-studio-950 text-white rounded px-1 border border-accent-hover"
                />{" "}
              </React.Fragment>
            );
          }
          const classes = [
            "rounded px-0.5 cursor-pointer",
            cut ? "line-through decoration-danger/70 text-studio-500" : "hover:bg-studio-800",
            selected && !cut ? "bg-accent/30 text-white shadow-[0_0_0_1px_rgb(var(--accent-hover)/0.6)]" : "",
            i === active && !selected ? "bg-studio-700 text-white" : "",
            kind === "filler" && !cut ? "underline decoration-suggest decoration-2" : "",
            kind === "retake" && !cut ? "underline decoration-studio-400 decoration-2" : "",
            w.kind === "audioEvent" ? "italic text-studio-400" : "",
            // Heard but left out of the captions.
            w.captionHidden && !cut ? "opacity-50 decoration-dotted underline decoration-studio-500" : "",
          ].join(" ");
          return (
            <React.Fragment key={w.id}>
              {w.captionBreak && !cut && (
                <span
                  className="inline-block w-0.5 h-3 mx-0.5 align-middle bg-suggest/80 rounded"
                  title="A new caption starts here"
                  aria-label="Caption break"
                />
              )}
              <span
                ref={i === active ? activeRef : undefined}
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
                  actions.current.pick(i, e.shiftKey);
                }}
                onDoubleClick={() => {
                  if (w.editedStartUs !== null) actions.current.seekTo(w.editedStartUs);
                }}
              >
                {w.text}
              </span>
              {/* No space before punctuation that comes as a word of its own. */}
              {/^[^\p{L}\p{N}]+$/u.test(line.words[position + 1]?.w.text ?? "") ? "" : " "}
            </React.Fragment>
          );
        })}
      </p>
    </div>
  );
});

export const TranscriptPanel: React.FC = () => {
  const openedProject = useProjectStore((s) => s.openedProject);
  const applyOpenedProject = useProjectStore((s) => s.applyOpenedProject);
  const applyPlaybackStatus = useProjectStore((s) => s.applyPlaybackStatus);
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

  // Where words land changes with the sequence (cuts, moves, undo), and their text and caption
  // marks with the transcript. Other edits (layout, captions' look, zooms) keep the same
  // sequence object, so they do not reload and redraw a long transcript.
  const sequence = openedProject?.sequence;
  useEffect(() => {
    void refresh();
  }, [refresh, sequence, captionsVersion]);

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

  // The word and line under the playhead: the panel re-renders when they change, not on
  // every tick of the playhead.
  const activeIndex = useProjectStore((s) =>
    words.findIndex(
      (w) => w.editedStartUs !== null && w.editedEndUs !== null && s.currentTimeUs >= w.editedStartUs && s.currentTimeUs < w.editedEndUs,
    ),
  );
  const playingLine = useProjectStore((s) =>
    lines.findIndex((line) => line.startUs !== null && line.endUs !== null && s.currentTimeUs >= line.startUs && s.currentTimeUs < line.endUs),
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

  const selectionRange: [number, number] | null = selection
    ? [Math.min(selection.anchor, selection.focus), Math.max(selection.anchor, selection.focus)]
    : null;
  const lineActions = useRef<LineActions>(null as unknown as LineActions);
  lineActions.current = {
    pick: (index, extend) => setSelection(extend && selection ? { anchor: selection.anchor, focus: index } : { anchor: index, focus: index }),
    seekTo,
    setEditing,
    commitEdit: () => void commitEdit(),
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

  const chosen = selectedIds.map((id) => words[wordIndex.get(id) ?? -1]).filter(Boolean);
  const chosenHidden = chosen.length > 0 && chosen.every((w) => w.captionHidden);
  const firstChosen = chosen[0];

  return (
    <div className="h-full flex flex-col min-h-0 bg-studio-900 text-label">
      <div className="shrink-0 flex items-center gap-2 h-11 px-3 border-b border-studio-800">
        <AudioLines className="w-4 h-4 text-studio-400 shrink-0" aria-hidden />
        <span className="font-semibold text-studio-100">Transcript</span>
        {tracks.length > 1 && (
          <select
            aria-label="Transcript track"
            value={trackId}
            onChange={(e) => {
              setTrackId(e.target.value);
              setSelection(null);
            }}
            className="ui-field !h-control-sm !text-meta !px-2 min-w-0 max-w-[14rem] truncate"
          >
            {tracks.map((t) => (
              <option key={t.id} value={t.id}>
                {t.label}
              </option>
            ))}
          </select>
        )}
        {view && (
          <span className="hidden lg:inline text-meta text-studio-500 truncate" title={view.model}>
            {view.words.filter((w) => w.editedStartUs !== null).length} of {view.words.length} words kept
          </span>
        )}
        {view && view.words.length > 0 && !tracks.find((t) => t.id === trackId)?.placed && (
          <Badge tone="suggest" title="Its words show once a clip of it is on the timeline, on any track">
            Not on the timeline yet
          </Badge>
        )}
        <div className="ml-auto flex items-center gap-1 shrink-0">
          {view && (
            <Button
              size="sm"
              variant="ghost"
              icon={Sparkles}
              aria-pressed={reviewOpen}
              onClick={() => setReviewOpen(!reviewOpen)}
              title="Review filler sounds and restarted sentences one by one, or find more with AI"
              className={cn("text-suggest-fg", reviewOpen && "!bg-suggest/15 !border-suggest/50")}
            >
              Review {pending.length}
            </Button>
          )}
          {view && (
            <Button
              size="sm"
              variant="ghost"
              disabled={running}
              onClick={() => void stripPunctuation()}
              title="Remove punctuation (. , ! ? : ; quotes, brackets) from every word, and so from the captions. Apostrophes, hyphens and numbers stay."
              className="font-mono"
            >
              .,?<span className="sr-only"> Remove punctuation</span>
            </Button>
          )}
          {view && (
            <IconButton
              size="sm"
              icon={showCut ? EyeOff : Eye}
              label={showCut ? "Hide cut words" : "Show cut words"}
              active={showCut}
              onClick={() => setShowCut(!showCut)}
            />
          )}
          {running ? (
            <Button size="sm" variant="secondary" icon={Loader2} className="[&>svg]:animate-spin" onClick={() => void api.transcriptCancel()}>
              Cancel
            </Button>
          ) : (
            <Button size="sm" variant={view ? "secondary" : "primary"} disabled={!trackId} onClick={() => void runTranscription()}>
              {view ? "Re-transcribe" : "Transcribe"}
            </Button>
          )}
          <IconButton size="sm" icon={Settings2} label="Transcription settings" onClick={() => setSettingsOpen(true)} />
        </div>
      </div>

      {selectedIds.length > 0 && (
        <div className="shrink-0 flex items-center gap-1.5 h-10 px-3 border-b border-studio-800 bg-accent/5" role="toolbar" aria-label="Selected words">
          <span className="text-meta text-accent-fg mr-1">
            {selectedIds.length} word{selectedIds.length === 1 ? "" : "s"} selected
          </span>
          {selectedIds.length === 1 && !editing && (
            <Button size="sm" variant="ghost" icon={Pencil} onClick={startEditing} title="Fix this word's text (Enter). Captions show the corrected word.">
              Fix word
            </Button>
          )}
          <Button
            size="sm"
            variant="ghost"
            icon={Captions}
            title={chosenHidden ? "Show these words in the captions again" : "Keep the sound but leave these words out of the captions"}
            onClick={() =>
              void editCaptions(
                { kind: "hide", wordIds: selectedIds, hidden: !chosenHidden },
                chosenHidden ? "Shown in the captions again." : "Hidden from the captions; the sound stays.",
              )
            }
          >
            {chosenHidden ? "Show in captions" : "Hide in captions"}
          </Button>
          {firstChosen && (
            <Button
              size="sm"
              variant="ghost"
              title={firstChosen.captionBreak ? "Let this caption join the one before" : "Start a new caption at this word"}
              onClick={() =>
                void editCaptions(
                  firstChosen.captionBreak ? { kind: "merge", wordId: firstChosen.id } : { kind: "split", wordId: firstChosen.id },
                  firstChosen.captionBreak ? "Captions merged." : "A new caption starts here.",
                )
              }
            >
              {firstChosen.captionBreak ? "Merge caption" : "New caption here"}
            </Button>
          )}
          <span className="flex-1" />
          <Button
            size="sm"
            variant="danger"
            icon={Scissors}
            onClick={() => void cutWords(selectedIds, `Cut ${selectedIds.length} words`)}
            title="Cut these words from the video (Delete)"
          >
            Cut {selectedIds.length}
          </Button>
        </div>
      )}

      {running && (
        <div className="shrink-0 px-3 py-2 border-b border-studio-800 flex items-center gap-3 text-meta text-accent-fg">
          <div className="flex-1 h-1.5 bg-studio-800 rounded-full overflow-hidden">
            <div className="h-full bg-accent-hover transition-all" style={{ width: `${Math.round((progress?.fraction ?? 0) * 100)}%` }} />
          </div>
          <span className="shrink-0">{progress?.message ?? "Starting"}</span>
        </div>
      )}
      {error && (
        <Notice tone="danger" onDismiss={() => setError(null)}>
          {error}
        </Notice>
      )}
      {notice && !error && (
        <Notice tone="neutral" onDismiss={() => setNotice(null)}>
          {notice}
        </Notice>
      )}

      <div className="flex-1 min-h-0 flex">
      <div
        tabIndex={0}
        onKeyDown={onKeyDown}
        className="flex-1 min-w-0 overflow-y-auto px-3 py-3 space-y-1 text-reading text-studio-200 outline-none select-none"
      >
        {!view && !running && (
          <p className="text-body text-studio-500 max-w-prose">
            Transcribe {tracks.find((t) => t.id === trackId)?.label ?? "a sound track"} to edit
            the video by deleting words. Click a word to select it, shift-click to extend, then press Delete. Double-click a
            word to jump to it, or press Enter to fix a misheard word. Review lists every filler sound and restarted sentence so
            you can keep or cut each one.
          </p>
        )}
        {lines.map((line, lineIndex) => {
          // Each line gets only what concerns it, so a change elsewhere leaves it alone.
          const first = line.words[0].i;
          const last = line.words[line.words.length - 1].i;
          const lo = selectionRange ? Math.max(selectionRange[0], first) : -1;
          const hi = selectionRange ? Math.min(selectionRange[1], last) : -1;
          const inLine = lo <= hi && lo >= 0;
          return (
            <TranscriptLineRow
              key={line.words[0].w.id}
              line={line}
              lineIndex={lineIndex}
              playing={lineIndex === playingLine}
              active={activeIndex >= first && activeIndex <= last ? activeIndex : -1}
              selFrom={inLine ? lo : -1}
              selTo={inLine ? hi : -1}
              editing={editing && editing.index >= first && editing.index <= last ? editing : null}
              suggestionKind={suggestionKind}
              activeRef={activeRef}
              actions={lineActions}
            />
          );
        })}
      </div>
      {reviewOpen && view && (
        <aside className="w-80 shrink-0 border-l border-studio-800 flex flex-col min-h-0 bg-studio-900" aria-label="Suggestion review">
          <div className="shrink-0 p-3 space-y-2 border-b border-studio-800">
            <div className="flex items-center gap-2">
              <Segmented<"all" | "filler" | "retake">
                label="Show"
                size="sm"
                className="flex-1"
                value={reviewFilter}
                onChange={setReviewFilter}
                options={[
                  { value: "all", label: "All" },
                  { value: "filler", label: "Fillers" },
                  { value: "retake", label: "Retakes" },
                ]}
              />
              <Button
                size="sm"
                variant="ghost"
                onClick={() => setShowRejected(!showRejected)}
                disabled={rejectedCount === 0 && !showRejected}
                title="Rejected suggestions stay out of Cut all"
              >
                {showRejected ? "Back" : `Rejected ${rejectedCount}`}
              </Button>
            </div>
            <Button
              size="sm"
              variant="secondary"
              icon={Sparkles}
              className="w-full"
              disabled={running}
              onClick={() => void runAiReview()}
              title="Ask the AI provider from Transcription and AI settings to find filler words and retakes in context. Sends the transcript text."
            >
              Find with AI
            </Button>
          </div>
          <ul className="flex-1 min-h-0 overflow-y-auto p-2 space-y-1.5">
            {reviewed.length === 0 && (
              <li className="text-studio-500 px-1">{showRejected ? "Nothing rejected." : "Nothing left to review."}</li>
            )}
            {reviewed.map((s) => (
              <li
                key={s.id}
                className="rounded-control border border-studio-800 bg-studio-850 hover:border-studio-600 px-2.5 py-2 cursor-pointer"
                onClick={() => focusSuggestion(s)}
              >
                <div className="flex items-center gap-1.5">
                  <Badge tone={s.kind === "filler" ? "suggest" : "neutral"}>{s.kind === "filler" ? "Filler" : "Retake"}</Badge>
                  {s.source === "ai" && <Badge title="Found by the AI review">AI</Badge>}
                  <span className="font-mono text-meta tabular-nums text-studio-500">{formatTime(s.editedStartUs)}</span>
                  <div className="ml-auto flex items-center gap-0.5" onClick={(e) => e.stopPropagation()}>
                    <button
                      type="button"
                      title="Play from just before it"
                      onClick={() => focusSuggestion(s)}
                      className="h-7 w-7 inline-flex items-center justify-center rounded-control text-studio-400 hover:text-studio-100 hover:bg-studio-700"
                    >
                      <Play className="w-3.5 h-3.5" />
                    </button>
                    {s.dismissed ? (
                      <button
                        type="button"
                        title="Put it back in the review list"
                        onClick={() => void dismiss([s.id], false)}
                        className="h-7 w-7 inline-flex items-center justify-center rounded-control text-studio-400 hover:text-studio-100 hover:bg-studio-700"
                      >
                        <RotateCcw className="w-3.5 h-3.5" />
                      </button>
                    ) : (
                      <>
                        <button
                          type="button"
                          title="Cut these words"
                          onClick={() =>
                            void cutWords(s.wordIds, `Cut ${s.kind === "filler" ? "filler" : "retake"} "${s.text}"`)
                          }
                          className="h-7 w-7 inline-flex items-center justify-center rounded-control text-danger-fg hover:bg-danger/15"
                        >
                          <Scissors className="w-3.5 h-3.5" />
                        </button>
                        <button
                          type="button"
                          title="Keep these words"
                          onClick={() => void dismiss([s.id], true)}
                          className="h-7 w-7 inline-flex items-center justify-center rounded-control text-studio-400 hover:text-studio-100 hover:bg-studio-700"
                        >
                          <Check className="w-3.5 h-3.5" />
                        </button>
                      </>
                    )}
                  </div>
                </div>
                <p className={`mt-1.5 text-body ${s.dismissed ? "text-studio-500" : "text-studio-100"}`}>&ldquo;{s.text}&rdquo;</p>
                {s.reason && <p className="mt-0.5 text-meta text-studio-500 italic">{s.reason}</p>}
              </li>
            ))}
          </ul>
          {!showRejected && reviewed.length > 1 && (
            <div className="shrink-0 p-3 border-t border-studio-800 space-y-1.5">
              <Button
                variant="danger"
                icon={Scissors}
                className="w-full"
                onClick={() =>
                  void cutWords(
                    reviewed.flatMap((s) => s.wordIds),
                    `Removed ${reviewed.length} suggestion${reviewed.length === 1 ? "" : "s"}`,
                  )
                }
              >
                Cut all {reviewed.length}
              </Button>
              <p className="text-meta text-studio-500 text-center">Every track loses the same time. Undo brings it back.</p>
            </div>
          )}
        </aside>
      )}
      </div>

      {settingsOpen && <TranscriptSettingsModal onClose={() => setSettingsOpen(false)} />}
    </div>
  );
};
