import React, { useState } from "react";
import { Check, Download, FolderOpen, Loader2, Pencil, Plus, Sparkles, Trash2, Video } from "lucide-react";
import { api } from "../../lib/ipc";
import { focusShortPlayback } from "../../lib/playbackControl";
import { switchStage, switchStarted } from "../../lib/switchTiming";
import { soundSources } from "../../lib/sequence";
import { useProjectStore } from "../../stores/projectStore";
import {
  exportShorts,
  findShorts,
  newShort,
  ofKind,
  removeShort,
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

/// The list of one tab: in Shorts the shorts made from a take (your camera and voice, footage
/// from the video under it), in Clips the clips cut from the video. Pick one to show and edit
/// it, rename, tick for export; new ones from a take, or by hand or the AI provider.
export const ShortsListPanel: React.FC = () => {
  const project = useProjectStore((s) => s.openedProject);
  const selected = useSelectedShort();
  const { busy, error, notice, checked, exports, renaming, kind } = useShortsStore();
  const hasMain = useShortsStore((s) => !!s.main);
  const [takeOpen, setTakeOpen] = useState(false);
  if (!project) return null;

  const shorts = (project.shorts ?? []).filter((s) => ofKind(s, kind));
  const takes = kind === "shorts";
  const hasSpeech = soundSources(project).length > 0;
  const toExport = shorts.some((s) => checked.has(s.id)) ? shorts.filter((s) => checked.has(s.id)) : shorts;
  const exporting = Object.values(exports).some((e) => e.state === "running" || e.state === "waiting");
  const setRenaming = (id: string | null) => useShortsStore.setState({ renaming: id });
  // A Shorts tab with nothing in it opens on what it is for.
  const showTake = takes && (takeOpen || shorts.length === 0);

  return (
    <div className="h-full flex flex-col min-h-0 text-label">
      <div className="shrink-0 flex flex-wrap gap-1.5 p-2 border-b border-studio-800">
        {takes ? (
          <>
            <Button
              size="sm"
              variant={showTake ? "ghost" : "secondary"}
              icon={Video}
              className="w-full"
              disabled={!!busy}
              aria-expanded={showTake}
              onClick={() => setTakeOpen((open) => !(open || shorts.length === 0))}
              title="Your camera and voice telling one part of this video; footage and graphics from it go under the take"
            >
              New short from a take
            </Button>
            {showTake && (
              <div className="w-full">
                <TakeForm onDone={() => setTakeOpen(false)} />
              </div>
            )}
          </>
        ) : (
          <>
            <Button size="sm" variant="secondary" icon={Plus} disabled={!!busy || !hasMain} onClick={newShort} className="flex-1">
              New clip
            </Button>
            <Button
              size="sm"
              variant="secondary"
              icon={busy === "Finding shorts" ? Loader2 : Sparkles}
              className={`flex-1 ${busy === "Finding shorts" ? "[&>svg]:animate-spin" : ""}`}
              disabled={!!busy || !hasSpeech}
              onClick={findShorts}
              title="Ask the AI provider from Transcription and AI settings for moments that work on their own. Replaces the clips (undoable)."
            >
              Find with AI
            </Button>
          </>
        )}
        {shorts.length > 0 && (
          <Button
            size="sm"
            variant="primary"
            icon={Download}
            className="w-full"
            disabled={exporting}
            onClick={() => void exportShorts(toExport.map((s) => s.id))}
          >
            Export {shorts.some((s) => checked.has(s.id)) ? `${toExport.length} selected` : `all ${shorts.length}`}
          </Button>
        )}
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

      <div className="flex-1 min-h-0 overflow-y-auto p-2 space-y-1.5" aria-label={takes ? "Shorts" : "Clips"} role="list">
        {shorts.length === 0 && !takes && (
          <p className="p-2 text-studio-500 leading-relaxed">
            No clips yet. Transcribe the video, then press Find with AI, or start one with New clip and shape it on its
            timeline.
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
                    <button
                      type="button"
                      aria-label={`Delete ${s.title}`}
                      title="Delete this short (Undo brings it back)"
                      disabled={!!busy}
                      onClick={(e) => {
                        e.stopPropagation();
                        removeShort(s.id);
                      }}
                      className="h-7 w-7 shrink-0 inline-flex items-center justify-center rounded-control text-studio-500 hover:text-danger-fg hover:bg-danger/15 opacity-0 group-hover:opacity-100 focus:opacity-100 disabled:opacity-40"
                    >
                      <Trash2 className="w-3.5 h-3.5" />
                    </button>
                  </>
                )}
              </div>
              <div className="mt-1 pl-6 text-meta tabular-nums text-studio-500">
                {s.take
                  ? `From a take · ${((s.lengthUs ?? 0) / 1e6).toFixed(1)}s`
                  : s.edit
                    ? `Own timeline · ${((s.lengthUs ?? 0) / 1e6).toFixed(1)}s`
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
