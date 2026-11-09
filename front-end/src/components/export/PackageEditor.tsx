import React, { useEffect, useRef, useState } from "react";
import { Film, ImagePlus, Loader2, Search } from "lucide-react";
import { api } from "../../lib/ipc";
import { packageSaver } from "../../lib/packageSaver";
import type { PublishPackage, PublishPackageView, TregKeyword } from "../../lib/types";
import { useProjectStore } from "../../stores/projectStore";
import { Button, Switch, cn } from "../ui";

/** What a package still lacks for YouTube, in a few words; empty when it has it all. */
export function packageGaps(view: PublishPackageView | null): string[] {
  if (!view) return [];
  const p = view.package;
  const gaps: string[] = [];
  if (!p.title.trim()) gaps.push("a title");
  if (!p.description.trim()) gaps.push("a description");
  if (p.tags.length === 0) gaps.push("tags");
  if (!view.thumbnailData) gaps.push("a thumbnail");
  return gaps;
}

/**
 * A video's YouTube package (PUB-1), the same for every kind of video: title, description,
 * tags, the AI label and the thumbnail, saved in the project as you type. Your agent writes the
 * same fields with `publish`. `file`: a finished export to take a thumbnail frame from; without
 * it the frame comes from the edit at the playhead. Saves belong to `projectHandle` and
 * `target`, not to the editor: closing it sends the last change (see `packageSaver`). Key the
 * editor by `projectHandle` so another project starts a fresh one.
 */
export const PackageEditor: React.FC<{
  projectHandle: string;
  target: string;
  file?: string;
  durationUs?: number;
  onChange?: (view: PublishPackageView) => void;
  /** Read again when this changes (an agent may have written the package). */
  refreshKey?: unknown;
  /** Fields can't change: an upload is using the package as saved. */
  locked?: boolean;
}> = ({ projectHandle, target, file, durationUs, onChange, refreshKey, locked = false }) => {
  const [view, setView] = useState<PublishPackageView | null>(null);
  const [draft, setDraft] = useState<PublishPackage | null>(null);
  const [tags, setTags] = useState("");
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string>();
  const [seed, setSeed] = useState("");
  const [keywords, setKeywords] = useState<TregKeyword[] | null>(null);
  const [frameAt, setFrameAt] = useState(1);
  const editing = useRef(false);

  const show = (v: PublishPackageView, fields = true) => {
    setView(v);
    if (fields) {
      setDraft(v.package);
      setTags(v.package.tags.join(", "));
    }
    onChange?.(v);
  };

  useEffect(() => {
    // An agent's change shows, unless the user is typing.
    if (editing.current || packageSaver.pending(projectHandle, target)) return;
    void api
      .publishPackage(projectHandle, target)
      .then((v) => {
        // A draft whose save failed comes back rather than being replaced by the old package.
        const unsaved = packageSaver.unsaved(projectHandle, target);
        show(v, !unsaved);
        if (unsaved) {
          editing.current = true;
          setDraft(unsaved.draft);
          setTags(unsaved.draft.tags.join(", "));
          setError(unsaved.error);
        }
      })
      .catch((err) => setError(String(err)));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [projectHandle, target, refreshKey]);

  useEffect(() => {
    const off = packageSaver.onSaved(projectHandle, target, (result) => {
      if ("error" in result) {
        setError(result.error);
        return;
      }
      if (!packageSaver.pending(projectHandle, target)) editing.current = false;
      setError(undefined);
      show(result.view, false);
    });
    // Closing sends what is still waiting, to the project it was typed in.
    return () => {
      off();
      packageSaver.sendNow(projectHandle, target);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [projectHandle, target]);

  const tagList = (text: string) =>
    text
      .split(",")
      .map((t) => t.trim())
      .filter(Boolean);

  /** Keeps `next` a moment after the last keystroke. */
  const change = (next: PublishPackage, nextTags = tags) => {
    if (locked) return;
    setDraft(next);
    setTags(nextTags);
    editing.current = true;
    packageSaver.schedule(projectHandle, target, { ...next, tags: tagList(nextTags) });
  };

  const run = async (what: string, work: () => Promise<void>) => {
    setBusy(what);
    setError(undefined);
    try {
      await work();
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(null);
    }
  };

  if (!draft || !view) {
    return <div className="p-3 text-meta text-studio-500">{error ?? <Loader2 className="w-4 h-4 animate-spin" />}</div>;
  }

  const field =
    "w-full rounded-control border border-studio-700 bg-studio-950 px-2 text-label text-studio-100 placeholder:text-studio-600 focus:outline-none focus:border-accent";
  const titleLength = draft.title.length;

  return (
    <div className="grid gap-5 md:grid-cols-[minmax(0,1fr)_240px]">
      <div className="space-y-3 min-w-0">
        <label className="block">
          <span className="flex justify-between text-meta text-studio-400">
            Title <span className={cn(titleLength > 100 && "text-danger-fg")}>{titleLength}/100</span>
          </span>
          <input
            className={cn(field, "h-8 mt-1")}
            value={draft.title}
            disabled={locked}
            onChange={(e) => change({ ...draft, title: e.target.value })}
            placeholder="Name the subject early, with one concrete stake"
          />
        </label>
        <label className="block">
          <span className="flex justify-between text-meta text-studio-400">
            Description <span>{draft.description.length}/5000</span>
          </span>
          <textarea
            className={cn(field, "mt-1 h-36 py-1.5 resize-y")}
            value={draft.description}
            disabled={locked}
            onChange={(e) => change({ ...draft, description: e.target.value })}
            placeholder="What it covers, chapters, sources"
          />
        </label>
        {view.chapters && !draft.description.includes(view.chapters.split("\n")[0]) && (
          <Button
            size="sm"
            variant="ghost"
            onClick={() =>
              change({
                ...draft,
                description: `${draft.description.trimEnd()}${draft.description.trim() ? "\n\n" : ""}${view.chapters}`,
              })
            }
          >
            Add the chapters
          </Button>
        )}
        <label className="block">
          <span className="text-meta text-studio-400">Tags, separated by commas</span>
          <input className={cn(field, "h-8 mt-1")} value={tags} disabled={locked} onChange={(e) => change(draft, e.target.value)} />
        </label>
        <div className="space-y-1">
          <div className="flex items-center gap-2">
            <input
              className={cn(field, "h-7 w-56")}
              value={seed}
              onChange={(e) => setSeed(e.target.value)}
              placeholder="Subject, e.g. claude code"
              aria-label="Keyword to start from"
            />
            <Button
              size="sm"
              variant="ghost"
              icon={busy === "keywords" ? Loader2 : Search}
              disabled={!!busy || !seed.trim()}
              onClick={() => void run("keywords", async () => setKeywords(await api.keywordSuggestions(seed.trim())))}
              title="What people search around it, with monthly Google searches and trend (Treg, under a cent)"
            >
              Suggest keywords
            </Button>
          </div>
          {keywords && (
            <div className="flex flex-wrap gap-1">
              {keywords.length === 0 && <span className="text-meta text-studio-500">No search data for that.</span>}
              {keywords.map((k) => (
                <button
                  key={k.keyword}
                  type="button"
                  className="rounded border border-studio-700 px-1.5 py-0.5 text-meta text-studio-300 hover:border-accent hover:text-studio-100"
                  title={`${Math.round(k.volume)} searches a month${k.trend ? `, ${k.trend >= 1 ? "+" : ""}${Math.round((k.trend - 1) * 100)}% over 3 months` : ""}${k.difficulty !== undefined ? `, difficulty ${Math.round(k.difficulty)}` : ""}. Click to add as a tag.`}
                  onClick={() =>
                    tagList(tags)
                      .map((x) => x.toLowerCase())
                      .includes(k.keyword)
                      ? undefined
                      : change(draft, `${tags.trim()}${tags.trim() ? ", " : ""}${k.keyword}`)
                  }
                >
                  {k.keyword}{" "}
                  <span className="text-studio-500">{k.volume >= 1000 ? `${Math.round(k.volume / 1000)}k` : Math.round(k.volume)}</span>
                  {k.trend !== undefined && k.trend >= 1.2 && <span className="text-success-fg"> ↑</span>}
                </button>
              ))}
            </div>
          )}
        </div>
        <div className="flex items-center gap-3">
          <Switch
            checked={draft.aiDisclosure}
            disabled={locked}
            onChange={(on) => change({ ...draft, aiDisclosure: on })}
            label="Altered or synthetic content"
          />
          <span className="text-meta text-studio-400">YouTube's AI label: needed for realistic AI voice or images.</span>
        </div>
        {error && <p className="text-meta text-danger-fg">{error}</p>}
      </div>

      <div className="space-y-2">
        <p className="text-meta text-studio-400">Thumbnail</p>
        <div className="aspect-video w-full overflow-hidden rounded-control border border-studio-700 bg-studio-950 flex items-center justify-center">
          {view.thumbnailData ? (
            <img src={view.thumbnailData} alt="Thumbnail" className="w-full h-full object-cover" />
          ) : (
            <span className="text-meta text-studio-600">None: YouTube picks a frame</span>
          )}
        </div>
        {view.thumbnailData && (
          <div className="flex items-end gap-2">
            <img src={view.thumbnailData} alt="" className="w-[160px] rounded border border-studio-800" />
            <span className="text-meta text-studio-500">at 160 px, as in a feed</span>
          </div>
        )}
        <Button
          size="sm"
          variant="secondary"
          icon={ImagePlus}
          disabled={!!busy || locked}
          onClick={() =>
            void run("image", async () => {
              const path = await api.pickWallpaperSource();
              if (path) show(await api.publishThumbnailFile(projectHandle, target, path), false);
            })
          }
        >
          Choose an image…
        </Button>
        {file ? (
          <div className="flex items-center gap-2">
            <input
              type="number"
              min={0}
              step={0.5}
              max={durationUs ? durationUs / 1e6 : undefined}
              value={frameAt}
              onChange={(e) => setFrameAt(Number(e.target.value))}
              className={cn(field, "h-7 w-20")}
              aria-label="Frame time in seconds"
            />
            <Button
              size="sm"
              variant="ghost"
              icon={busy === "frame" ? Loader2 : Film}
              disabled={!!busy || locked}
              onClick={() =>
                void run("frame", async () =>
                  show(await api.publishThumbnailFrame(projectHandle, target, file, Math.round(frameAt * 1e6)), false),
                )
              }
            >
              Use this second's frame
            </Button>
          </div>
        ) : (
          target === "video" && (
            <Button
              size="sm"
              variant="ghost"
              icon={busy === "frame" ? Loader2 : Film}
              disabled={!!busy || locked}
              title="The edit's frame where the playhead is, graphics drawn"
              onClick={() =>
                void run("frame", async () =>
                  show(await api.publishThumbnailEdit(projectHandle, target, Math.round(useProjectStore.getState().currentTimeUs)), false),
                )
              }
            >
              Use the frame at the playhead
            </Button>
          )
        )}
        {view.thumbnailSize && (
          <p className="text-meta text-studio-500">
            {view.thumbnailSize[0]}×{view.thumbnailSize[1]}
          </p>
        )}
      </div>
    </div>
  );
};
