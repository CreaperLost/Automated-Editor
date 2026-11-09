import React, { useCallback, useEffect, useRef, useState } from "react";
import { ChevronDown, ChevronRight, Loader2, Plus, Sparkles, Trash2 } from "lucide-react";
import { Button, IconButton, Segmented, cn } from "../ui";
import { api } from "../../lib/ipc";
import type { FacelessCandidate, FacelessWatched } from "../../lib/types";

type Platform = FacelessWatched["platform"];
const PLATFORMS: { id: Platform; label: string; hint: string }[] = [
  { id: "x", label: "X", hint: "Tracked accounts, searched in batches with each group's likes floor" },
  { id: "youtube", label: "YouTube", hint: "Channels watched in demand checks: what they cover, against their usual views" },
  { id: "tiktok", label: "TikTok", hint: "Creators watched for shorts' demand" },
  { id: "instagram", label: "Instagram", hint: "Creators watched for shorts' demand" },
];
const GROUPS = ["labs", "leaders", "research", "news", "security", "creators"];

function count(n: number) {
  return n >= 1e6 ? `${(n / 1e6).toFixed(1)}M` : n >= 1e3 ? `${Math.round(n / 1e3)}k` : String(Math.round(n));
}

/**
 * The research watchlist: who the story search follows on X, which YouTube channels the demand
 * check watches, and TikTok and Instagram creators. Suggestions are proposals you add yourself.
 */
export const SourcesPanel: React.FC<{ onError: (message: string) => void }> = ({ onError }) => {
  const [open, setOpen] = useState(false);
  const [platform, setPlatform] = useState<Platform>("x");
  const [list, setList] = useState<FacelessWatched[]>([]);
  const [handle, setHandle] = useState("");
  const [group, setGroup] = useState("creators");
  // Suggestions belong to the platform they were asked for (GAP-020).
  const [found, setFound] = useState<{ platform: Platform; candidates: FacelessCandidate[] } | null>(null);
  const candidates = found?.platform === platform ? found.candidates : null;
  const setCandidates = (update: (cs: FacelessCandidate[] | null) => FacelessCandidate[] | null) =>
    setFound((f) => (f ? { ...f, candidates: update(f.candidates) ?? [] } : f));
  const [finding, setFinding] = useState(false);
  const asked = useRef(0);

  const load = useCallback(
    () =>
      api
        .facelessWatchlist()
        .then(setList)
        .catch((err) => onError(String(err))),
    [onError],
  );
  useEffect(() => {
    if (open) void load();
  }, [open, load]);
  useEffect(() => {
    asked.current++;
    setFound(null);
    setFinding(false);
  }, [platform]);

  const run = async (work: Promise<unknown>) => {
    try {
      await work;
      await load();
    } catch (err) {
      onError(String(err));
    }
  };

  const suggest = async () => {
    const mine = ++asked.current;
    const askedFor = platform;
    setFinding(true);
    try {
      const result =
        askedFor === "x" ? await api.facelessSuggestX() : await api.facelessFindCreators(askedFor as "tiktok" | "instagram");
      // A platform switched since: this answer is for a list no longer shown.
      if (mine === asked.current) setFound({ platform: askedFor, candidates: result });
    } catch (err) {
      if (mine === asked.current) onError(String(err));
    } finally {
      if (mine === asked.current) setFinding(false);
    }
  };

  const shown = list.filter((w) => w.platform === platform);
  const byGroup = platform === "x" ? GROUPS.filter((g) => shown.some((w) => w.group === g)) : [""];

  return (
    <section className="mt-10">
      <button
        type="button"
        onClick={() => setOpen((v) => !v)}
        className="w-full flex items-center gap-2 border-b border-studio-800 pb-2 text-left"
      >
        {open ? <ChevronDown className="w-4 h-4 text-studio-500" /> : <ChevronRight className="w-4 h-4 text-studio-500" />}
        <h2 className="text-label font-semibold text-studio-300">Sources</h2>
        <span className="text-meta text-studio-500">who the research follows on X, YouTube, TikTok and Instagram</span>
      </button>
      {open && (
        <div className="mt-3 space-y-3">
          <div className="flex items-center gap-3">
            <Segmented
              size="sm"
              label="Platform"
              value={platform}
              onChange={setPlatform}
              options={PLATFORMS.map((p) => ({
                value: p.id,
                label: `${p.label} ${list.filter((w) => w.platform === p.id).length}`,
                title: p.hint,
              }))}
            />
            {platform !== "youtube" && (
              <Button
                size="sm"
                variant="ghost"
                icon={finding ? Loader2 : Sparkles}
                disabled={finding}
                onClick={() => void suggest()}
                title={
                  platform === "x"
                    ? "Accounts several tracked ones follow (about half a cent through Treg)"
                    : "Who posts the most-watched videos on AI topics (about a cent through Treg)"
                }
              >
                {platform === "x" ? "Suggest accounts" : "Find creators"}
              </Button>
            )}
          </div>

          <form
            className="flex items-center gap-2"
            onSubmit={(e) => {
              e.preventDefault();
              if (handle.trim())
                void run(api.facelessWatchAdd(platform, handle.trim(), platform === "x" ? group : undefined)).then(() =>
                  setHandle(""),
                );
            }}
          >
            <input
              value={handle}
              onChange={(e) => setHandle(e.target.value)}
              placeholder="@handle"
              className="h-7 w-48 rounded border border-studio-700 bg-studio-950 px-2 text-meta text-studio-100 placeholder:text-studio-600"
            />
            {platform === "x" && (
              <select
                value={group}
                onChange={(e) => setGroup(e.target.value)}
                className="h-7 rounded border border-studio-700 bg-studio-950 px-1 text-meta text-studio-100"
              >
                {GROUPS.map((g) => (
                  <option key={g} value={g}>
                    {g}
                  </option>
                ))}
              </select>
            )}
            <Button type="submit" size="sm" variant="secondary" icon={Plus} disabled={!handle.trim()}>
              Add
            </Button>
          </form>

          {candidates && (
            <div className="rounded-control border border-accent/30 bg-accent/5 p-2">
              <p className="text-meta text-studio-300 mb-1">
                {candidates.length ? "Suggestions: add the ones that fit the channel." : "No new suggestions."}
              </p>
              <ul className="space-y-1">
                {candidates.map((c) => (
                  <li key={c.handle} className="flex items-center gap-2 text-meta text-studio-400">
                    <span className="text-studio-100">@{c.handle}</span>
                    {c.name && <span>{c.name}</span>}
                    <span>{count(c.followers)} followers</span>
                    <span className="truncate flex-1 min-w-0">
                      {c.followedBy?.length
                        ? `followed by ${c.followedBy.length}: ${c.followedBy.join(", ")}`
                        : `${c.posts} posts, ${count(c.views)} views`}
                      {c.bio && ` · ${c.bio}`}
                    </span>
                    <IconButton
                      icon={Plus}
                      size="sm"
                      label={`Watch @${c.handle}`}
                      onClick={() =>
                        void run(api.facelessWatchAdd(
                            c.platform as Platform,
                            c.handle,
                            c.platform === "x" ? group : undefined,
                            c.name,
                            c.followers,
                          )).then(
                          () => setCandidates((cs) => cs?.filter((x) => x.handle !== c.handle) ?? null),
                        )
                      }
                    />
                  </li>
                ))}
              </ul>
            </div>
          )}

          {byGroup.map((g) => (
            <div key={g || platform}>
              {g && <p className="text-meta font-semibold text-studio-400 uppercase tracking-wide">{g}</p>}
              <ul className="mt-1 grid gap-x-4 sm:grid-cols-2">
                {shown
                  .filter((w) => !g || w.group === g)
                  .map((w) => (
                    <li key={w.handle} className="group flex items-center gap-2 py-0.5 text-meta text-studio-400">
                      <span className="text-studio-100">@{w.handle.replace(/^@/, "")}</span>
                      {w.name && <span className="truncate">{w.name}</span>}
                      {w.followers > 0 && <span>{count(w.followers)}</span>}
                      <span className={cn("ml-auto", w.top10 > 0 && "text-success-fg")} title="Posts that passed the filters · times in a top 10">
                        {w.kept} kept · {w.top10} top 10
                        {w.usual !== undefined && ` · usually ${count(w.usual)}`}
                      </span>
                      <IconButton
                        icon={Trash2}
                        size="sm"
                        label={`Stop watching ${w.handle}`}
                        className="opacity-0 group-hover:opacity-100"
                        onClick={() => void run(api.facelessWatchRemove(platform, w.handle))}
                      />
                    </li>
                  ))}
              </ul>
            </div>
          ))}
        </div>
      )}
    </section>
  );
};
