import React, { useState } from "react";
import { Check, Download, FolderOpen, Loader2, Pencil, Plus, Sparkles, Video } from "lucide-react";
import { api } from "../../lib/ipc";
import { focusShortPlayback } from "../../lib/playbackControl";
import { switchStage, switchStarted } from "../../lib/switchTiming";
import { soundSources } from "../../lib/sequence";
import { useProjectStore } from "../../stores/projectStore";
import {
  exportShorts,
  findShorts,
  newShort,
  renameShort,
  selectShort,
  setShortsError,
  useSelectedShort,
  useShortsStore,
} from "../../stores/shortsStore";
import { Button, Notice } from "../ui";
import { TakeForm } from "./TakeForm";

function formatTime(us: number): string {
  const total = Math.max(0, us) / 1_000_000;
  const minutes = Math.floor(total / 60);
  const seconds = total - minutes * 60;
  return `${minutes}:${seconds.toFixed(1).padStart(4, "0")}`;
}

/// The shorts of the video: pick one to show and edit it, rename, tick for export; new ones
/// by hand or from the AI provider.
export const ShortsListPanel: React.FC = () => {
  const project = useProjectStore((s) => s.openedProject);
  const selected = useSelectedShort();
  const { busy, error, notice, checked, exports, renaming } = useShortsStore();
  const hasMain = useShortsStore((s) => !!s.main);
  const [takeOpen, setTakeOpen] = useState(false);
  if (!project) return null;

  const shorts = project.shorts ?? [];
  const hasSpeech = soundSources(project).length > 0;
  const toExport = checked.size > 0 ? shorts.filter((s) => checked.has(s.id)) : shorts;
  const exporting = Object.values(exports).some((e) => e.state === "running" || e.state === "waiting");
  const setRenaming = (id: string | null) => useShortsStore.setState({ renaming: id });

  return (
    <div className="h-full flex flex-col min-h-0 text-label">
      <div className="shrink-0 flex flex-wrap gap-1.5 p-2 border-b border-studio-800">
        <Button size="sm" variant="secondary" icon={Plus} disabled={!!busy || !hasMain} onClick={newShort} className="flex-1">
          New short
        </Button>
        <Button
          size="sm"
          variant="secondary"
          icon={busy === "Finding shorts" ? Loader2 : Sparkles}
          className={`flex-1 ${busy === "Finding shorts" ? "[&>svg]:animate-spin" : ""}`}
          disabled={!!busy || !hasSpeech}
          onClick={findShorts}
          title="Ask the AI provider from Transcription and AI settings for moments that work on their own. Replaces the list (undoable)."
        >
          Find with AI
        </Button>
        <Button
          size="sm"
          variant="secondary"
          icon={Video}
          className="w-full"
          disabled={!!busy}
          aria-expanded={takeOpen}
          onClick={() => setTakeOpen((open) => !open)}
          title="A short you recorded again: your camera and voice, with footage from this video under it"
        >
          From a take…
        </Button>
        {takeOpen && (
          <div className="w-full">
            <TakeForm onDone={() => setTakeOpen(false)} />
          </div>
        )}
        <Button
          size="sm"
          variant="primary"
          icon={Download}
          className="w-full"
          disabled={toExport.length === 0 || exporting}
          onClick={() => void exportShorts(toExport.map((s) => s.id))}
        >
          Export {checked.size > 0 ? `${checked.size} selected` : `all ${shorts.length}`}
        </Button>
      </div>

      {error && (
        <Notice tone="danger" onDismiss={() => setShortsError(undefined)}>
          {error}
        </Notice>
      )}
      {notice && (
        <Notice tone="accent" onDismiss={() => useShortsStore.setState({ notice: undefined })}>
          {notice}
        </Notice>
      )}

      <div className="flex-1 min-h-0 overflow-y-auto p-2 space-y-1.5" aria-label="Shorts" role="list">
        {shorts.length === 0 && (
          <p className="p-2 text-studio-500 leading-relaxed">
            No shorts yet. Transcribe the video, then press Find with AI, or start one with New short and shape it on
            its timeline.
          </p>
        )}
        {shorts.map((s) => {
          const ok = s.editedStartUs !== undefined && s.editedEndUs !== undefined;
          const row = exports[s.id];
          return (
            <div
              key={s.id}
              role="listitem"
              onClick={() => {
                if (s.id !== selected?.id) {
                  switchStarted(s.id);
                  // The engine starts on the short now, while the panels redraw.
                  void focusShortPlayback(project.projectHandle, s.id, 0).then(() => switchStage("engine focused", s.id));
                }
                selectShort(s.id);
              }}
              className={`group rounded-control border px-3 py-2.5 cursor-pointer transition-colors ${
                s.id === selected?.id
                  ? "border-accent-hover/70 bg-accent/10 shadow-[inset_2px_0_0_rgb(var(--accent-hover))]"
                  : "border-studio-800 bg-studio-850 hover:border-studio-600"
              }`}
            >
              <div className="flex items-center gap-2 min-h-7">
                <input
                  type="checkbox"
                  aria-label={`Select ${s.title} for export`}
                  checked={checked.has(s.id)}
                  onClick={(e) => e.stopPropagation()}
                  onChange={(e) => {
                    const next = new Set(checked);
                    if (e.target.checked) next.add(s.id);
                    else next.delete(s.id);
                    useShortsStore.setState({ checked: next });
                  }}
                  className="w-4 h-4"
                />
                {renaming === s.id ? (
                  <input
                    autoFocus
                    aria-label="Short title (also the file name)"
                    defaultValue={s.title}
                    maxLength={100}
                    onClick={(e) => e.stopPropagation()}
                    onBlur={(e) => {
                      setRenaming(null);
                      const title = e.target.value.trim();
                      if (title && title !== s.title) renameShort(s.id, title);
                    }}
                    onKeyDown={(e) => {
                      if (e.key === "Enter") e.currentTarget.blur();
                      if (e.key === "Escape") setRenaming(null);
                    }}
                    className="ui-field !h-7 flex-1 min-w-0"
                  />
                ) : (
                  <>
                    <span
                      className="flex-1 truncate font-medium text-studio-100"
                      title={s.reason ? `${s.title}\nAI: ${s.reason}` : s.title}
                      onDoubleClick={(e) => {
                        e.stopPropagation();
                        setRenaming(s.id);
                      }}
                    >
                      {s.title}
                    </span>
                    <button
                      type="button"
                      aria-label={`Rename ${s.title}`}
                      title="Rename (the title is also the file name)"
                      onClick={(e) => {
                        e.stopPropagation();
                        setRenaming(s.id);
                      }}
                      className="h-7 w-7 shrink-0 inline-flex items-center justify-center rounded-control text-studio-500 hover:text-studio-100 hover:bg-studio-700 opacity-0 group-hover:opacity-100 focus:opacity-100"
                    >
                      <Pencil className="w-3.5 h-3.5" />
                    </button>
                  </>
                )}
              </div>
              <div className="mt-1 pl-6 text-meta tabular-nums text-studio-500">
                {s.take
                  ? `From a take · ${((s.lengthUs ?? 0) / 1e6).toFixed(1)}s`
                  : ok
                    ? `${formatTime(s.editedStartUs!)} · ${((s.editedEndUs! - s.editedStartUs!) / 1e6).toFixed(1)}s`
                    : "Part of it was cut; adjust it on its timeline"}
              </div>
              {row && (
                <div className="mt-1 pl-6 flex items-center gap-1 text-meta">
                  {row.state === "completed" ? (
                    <>
                      <Check className="w-3 h-3 text-audio-fg" />
                      <span className="text-audio-fg">Exported</span>
                      {row.path && (
                        <button
                          type="button"
                          onClick={(e) => {
                            e.stopPropagation();
                            void api.showInFinder(row.path!);
                          }}
                          className="ml-auto flex items-center gap-1 text-studio-300 hover:text-white"
                        >
                          <FolderOpen className="w-3 h-3" /> Show
                        </button>
                      )}
                    </>
                  ) : row.state === "failed" ? (
                    <span className="text-danger-fg truncate" title={row.message}>
                      Failed: {row.message}
                    </span>
                  ) : (
                    <span className="text-accent-fg">
                      {row.state === "waiting" ? "Waiting" : `Exporting ${Math.round(row.progress * 100)}%`}
                    </span>
                  )}
                </div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
};
