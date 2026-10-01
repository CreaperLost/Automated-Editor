import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { AudioLines, Eye, EyeOff, Loader2, Scissors, Settings2, Sparkles, X } from "lucide-react";
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
  const suggestionKind = useMemo(() => {
    const map = new Map<string, TranscriptCutSuggestion["kind"]>();
    for (const s of suggestions) for (const id of s.wordIds) map.set(id, s.kind);
    return map;
  }, [suggestions]);
  const fillerSuggestions = suggestions.filter((s) => s.kind === "filler");
  const retakeSuggestions = suggestions.filter((s) => s.kind === "retake");

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

  const seekTo = (us: number) => {
    if (!handle) return;
    void api
      .playbackSeek(handle, us)
      .then(applyPlaybackStatus)
      .catch(() => undefined);
  };

  const onKeyDown = (e: React.KeyboardEvent) => {
    if ((e.key === "Delete" || e.key === "Backspace") && selectedIds.length > 0) {
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
          {view && fillerSuggestions.length > 0 && (
            <button
              type="button"
              title="Cut every um, uh, er and similar filler sound"
              onClick={() =>
                void cutWords(
                  fillerSuggestions.flatMap((s) => s.wordIds),
                  `Removed ${fillerSuggestions.length} filler${fillerSuggestions.length === 1 ? "" : "s"}`,
                )
              }
              className="flex items-center gap-1 px-2 py-1 rounded bg-amber-900/40 border border-amber-700/50 text-amber-200 hover:bg-amber-800/50"
            >
              <Sparkles className="w-3 h-3" /> Remove {fillerSuggestions.length} filler{fillerSuggestions.length === 1 ? "" : "s"}
            </button>
          )}
          {view && retakeSuggestions.length > 0 && (
            <button
              type="button"
              title="Cut earlier attempts of sentences you restarted (highlighted in violet)"
              onClick={() =>
                void cutWords(
                  retakeSuggestions.flatMap((s) => s.wordIds),
                  `Removed ${retakeSuggestions.length} retake${retakeSuggestions.length === 1 ? "" : "s"}`,
                )
              }
              className="flex items-center gap-1 px-2 py-1 rounded bg-violet-900/40 border border-violet-700/50 text-violet-200 hover:bg-violet-800/50"
            >
              <Scissors className="w-3 h-3" /> Remove {retakeSuggestions.length} retake{retakeSuggestions.length === 1 ? "" : "s"}
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

      <div
        tabIndex={0}
        onKeyDown={onKeyDown}
        className="flex-1 min-h-0 overflow-y-auto px-3 py-2 leading-6 text-[13px] text-studio-200 outline-none select-none"
      >
        {!view && !running && (
          <p className="text-studio-500">
            Transcribe the {tracks[0]?.descriptor.trackType === "system_audio" ? "system audio" : "microphone"} track to edit
            the video by deleting words. Click a word to select it, shift-click to extend, then press Delete. Double-click a
            word to jump to it.
          </p>
        )}
        {visible.map(({ w, i }) => {
          const cut = w.editedStartUs === null;
          const selected =
            selection !== null &&
            i >= Math.min(selection.anchor, selection.focus) &&
            i <= Math.max(selection.anchor, selection.focus);
          const kind = suggestionKind.get(w.id);
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

      {settingsOpen && <TranscriptSettingsModal onClose={() => setSettingsOpen(false)} />}
    </div>
  );
};
