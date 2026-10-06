import React, { useEffect, useState } from "react";
import { Check, ClipboardCopy, Loader2, Plus, Sparkles, Trash2 } from "lucide-react";
import { useProjectStore } from "../../stores/projectStore";
import { api } from "../../lib/ipc";
import { clipAt, soundSources } from "../../lib/sequence";
import type { Chapter, OpenedProject } from "../../lib/types";

/** YouTube's rules for description chapters. */
const YOUTUBE_MIN_CHAPTERS = 3;
const YOUTUBE_MIN_LENGTH_US = 10_000_000;

export function formatChapterTime(us: number): string {
  const total = Math.floor(us / 1_000_000);
  const h = Math.floor(total / 3600);
  const m = Math.floor(total / 60) % 60;
  const s = total % 60;
  return h > 0 ? `${h}:${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}` : `${m}:${String(s).padStart(2, "0")}`;
}

/** Kept chapters in playback order, the first moved to 0:00 like the exported file. */
export function placedChapters(chapters: Chapter[]): { chapter: Chapter; atUs: number }[] {
  const placed = chapters
    .filter((c): c is Chapter & { editedUs: number } => c.editedUs !== null && c.editedUs !== undefined)
    .sort((a, b) => a.editedUs - b.editedUs)
    .filter((c, i, all) => i === 0 || c.editedUs !== all[i - 1].editedUs)
    .map((chapter) => ({ chapter, atUs: chapter.editedUs }));
  if (placed.length > 0) placed[0].atUs = 0;
  return placed;
}

function youtubeProblems(placed: { atUs: number }[], durationUs: number): string[] {
  const problems: string[] = [];
  if (placed.length < YOUTUBE_MIN_CHAPTERS) problems.push(`YouTube needs at least ${YOUTUBE_MIN_CHAPTERS} chapters.`);
  const short = placed.filter((c, i) => (placed[i + 1]?.atUs ?? durationUs) - c.atUs < YOUTUBE_MIN_LENGTH_US).length;
  if (short > 0) problems.push(`${short} chapter${short === 1 ? " is" : "s are"} shorter than 10 seconds.`);
  return problems;
}

function errorMessage(err: unknown): string {
  if (err instanceof Error && err.message) return err.message;
  if (typeof err === "string" && err.trim()) return err;
  return "Something went wrong.";
}

/// Chapter markers: find them with AI from the transcript, add, rename, delete, and copy a
/// YouTube description list. Export writes them into the MP4.
export const ChaptersPanel: React.FC = () => {
  const openedProject = useProjectStore((s) => s.openedProject);
  const applyOpenedProject = useProjectStore((s) => s.applyOpenedProject);
  const applyPlaybackStatus = useProjectStore((s) => s.applyPlaybackStatus);
  const durationUs = useProjectStore((s) => s.durationUs);
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string>();
  const [copied, setCopied] = useState(false);
  const [titles, setTitles] = useState<Record<string, string>>({});

  const chapters = openedProject?.chapters ?? [];
  useEffect(() => setTitles({}), [openedProject?.revision]);

  // The speech on the timeline the AI reads.
  const speechKey = soundSources(openedProject).find((sound) => sound.placed)?.key;

  const run = async (label: string, work: (project: OpenedProject) => Promise<OpenedProject>) => {
    const project = useProjectStore.getState().openedProject;
    if (!project || busy) return;
    setBusy(label);
    setError(undefined);
    try {
      applyOpenedProject(await work(project));
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setBusy(null);
    }
  };

  const save = (next: Chapter[]) =>
    run("Saving", (project) => api.projectChaptersSet(project.projectHandle, project.revision, next));

  const seek = (us: number) => {
    if (!openedProject) return;
    void api.playbackSeek(openedProject.projectHandle, us).then(applyPlaybackStatus).catch(() => undefined);
  };

  const addAtPlayhead = () => {
    if (!openedProject) return;
    const currentTimeUs = useProjectStore.getState().currentTimeUs;
    // Anchored to what plays there: the top picture, else any clip.
    const tracks = [...openedProject.sequence.tracks.filter((t) => t.kind === "video")].reverse().concat(
      openedProject.sequence.tracks.filter((t) => t.kind === "audio"),
    );
    const clip = tracks.map((t) => clipAt(t, currentTimeUs)).find((c) => c !== undefined);
    if (!clip) {
      setError("Move the playhead onto a clip to add a chapter there.");
      return;
    }
    const sourceUs = Math.round(clip.inUs + (currentTimeUs - clip.startUs));
    const id = `ch-${sourceUs}-${Date.now().toString(36)}`;
    void save([...chapters, { id, sourceUs, title: "New chapter", media: clip.asset }]);
  };

  const commitTitle = (chapter: Chapter) => {
    const title = (titles[chapter.id] ?? chapter.title).trim();
    if (!title || title === chapter.title) {
      setTitles(({ [chapter.id]: _, ...rest }) => rest);
      return;
    }
    void save(chapters.map((c) => (c.id === chapter.id ? { ...c, title } : c)));
  };

  const placed = placedChapters(chapters);
  const cut = chapters.filter((c) => c.editedUs === null || c.editedUs === undefined);
  const list = placed.map((p) => p.chapter.id);
  const youtube = placed.map((p) => `${formatChapterTime(p.atUs)} ${p.chapter.title}`).join("\n");
  const problems = youtubeProblems(placed, durationUs);

  const copy = async () => {
    try {
      await navigator.clipboard.writeText(youtube);
      setCopied(true);
      window.setTimeout(() => setCopied(false), 1500);
    } catch {
      setError("Could not copy; select the text below and copy it.");
    }
  };

  if (!openedProject) return null;

  const button =
    "flex items-center justify-center gap-1 px-2 py-1.5 rounded-md border disabled:opacity-40";

  return (
    <div className="h-full overflow-y-auto p-3 space-y-3 bg-studio-900/95 text-label select-none">
      <div className="grid grid-cols-2 gap-2">
        <button
          type="button"
          disabled={!!busy || !speechKey}
          onClick={() =>
            speechKey &&
            void run("Finding chapters", (project) =>
              api.projectChaptersGenerate(project.projectHandle, speechKey),
            )
          }
          className={`${button} bg-studio-800/40 border-studio-600/40 text-studio-100 hover:bg-studio-700/50`}
          title="Ask the AI provider from Transcription and AI settings to split the transcript into chapters. Replaces the current chapters (undoable)."
        >
          {busy === "Finding chapters" ? <Loader2 className="w-3.5 h-3.5 animate-spin" /> : <Sparkles className="w-3.5 h-3.5" />}
          Find with AI
        </button>
        <button
          type="button"
          disabled={!!busy}
          onClick={addAtPlayhead}
          className={`${button} bg-studio-850 border-studio-700 text-studio-200 hover:bg-studio-800`}
        >
          <Plus className="w-3.5 h-3.5" /> Add at playhead
        </button>
      </div>
      {!speechKey && <p className="text-meta text-studio-500">Finding chapters needs speech on the timeline, transcribed.</p>}
      {error && (
        <p role="alert" className="text-meta text-danger-fg bg-danger/10 border border-danger/30 rounded p-2">
          {error}
        </p>
      )}

      {chapters.length === 0 ? (
        <p className="text-meta text-studio-500 leading-relaxed">
          No chapters yet. Transcribe the video, then press Find with AI, or add them at the playhead. Chapters are
          written into exported MP4 files, and you can copy them as a YouTube description list.
        </p>
      ) : (
        <ul className="space-y-1">
          {[...list.map((id) => placed.find((p) => p.chapter.id === id)!), ...cut.map((chapter) => ({ chapter, atUs: null }))].map(
            ({ chapter, atUs }) => (
              <li key={chapter.id} className="flex items-center gap-1.5 rounded-md border border-studio-800 bg-studio-850 px-2 py-1">
                <button
                  type="button"
                  disabled={atUs === null}
                  onClick={() => chapter.editedUs != null && seek(chapter.editedUs)}
                  className={`w-12 shrink-0 text-left font-mono ${atUs === null ? "text-studio-600 line-through" : "text-studio-400 hover:text-accent-fg"}`}
                  title={atUs === null ? "This moment was cut; the chapter is left out" : "Jump here"}
                >
                  {atUs === null ? "cut" : formatChapterTime(atUs)}
                </button>
                <input
                  aria-label="Chapter title"
                  value={titles[chapter.id] ?? chapter.title}
                  maxLength={100}
                  disabled={!!busy}
                  onChange={(e) => setTitles({ ...titles, [chapter.id]: e.target.value })}
                  onBlur={() => commitTitle(chapter)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") e.currentTarget.blur();
                    if (e.key === "Escape") setTitles(({ [chapter.id]: _, ...rest }) => rest);
                  }}
                  className="flex-1 min-w-0 bg-transparent border border-transparent hover:border-studio-700 focus:border-accent-hover rounded px-1 py-0.5 text-studio-100 outline-none"
                />
                <button
                  type="button"
                  aria-label={`Delete chapter ${chapter.title}`}
                  disabled={!!busy}
                  onClick={() => void save(chapters.filter((c) => c.id !== chapter.id))}
                  className="p-1 rounded text-studio-500 hover:text-danger-fg hover:bg-studio-700 disabled:opacity-40"
                >
                  <Trash2 className="w-3.5 h-3.5" />
                </button>
              </li>
            ),
          )}
        </ul>
      )}

      {placed.length > 0 && (
        <div className="space-y-1.5 pt-2 border-t border-studio-800">
          <div className="flex items-center justify-between">
            <span className="font-semibold text-studio-300">YouTube description</span>
            <button
              type="button"
              onClick={() => void copy()}
              className="flex items-center gap-1 px-2 py-1 rounded-md bg-accent hover:bg-accent text-white"
            >
              {copied ? <Check className="w-3.5 h-3.5" /> : <ClipboardCopy className="w-3.5 h-3.5" />}
              {copied ? "Copied" : "Copy"}
            </button>
          </div>
          <textarea
            readOnly
            aria-label="YouTube chapter list"
            value={youtube}
            rows={Math.min(10, placed.length + 1)}
            className="w-full bg-studio-950 border border-studio-800 rounded p-2 font-mono text-meta text-studio-200 select-text"
          />
          {problems.map((p) => (
            <p key={p} className="text-meta text-suggest-fg">
              {p}
            </p>
          ))}
        </div>
      )}
    </div>
  );
};
