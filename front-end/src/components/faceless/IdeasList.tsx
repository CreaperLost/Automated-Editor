import React, { useCallback, useEffect, useState } from "react";
import {
  ArrowDown,
  ArrowUp,
  ChevronDown,
  ChevronRight,
  EyeOff,
  Globe,
  Loader2,
  Pin,
  PinOff,
  Plus,
  Search,
  Scissors,
  TrendingUp,
} from "lucide-react";
import { Badge, Button, IconButton, Segmented, cn } from "../ui";
import { api } from "../../lib/ipc";
import type { FacelessDemand, FacelessDiscovered, FacelessFormat, FacelessIdea, FacelessIdeas } from "../../lib/types";

type LaneId = FacelessIdea["lane"];
const LANES: { id: LaneId; label: string }[] = [
  { id: "stories", label: "Stories" },
  { id: "repositories", label: "Viral repositories" },
  { id: "papers", label: "Papers" },
];
const TOP = 10;

function ago(iso?: string) {
  if (!iso) return "never";
  const minutes = Math.round((Date.now() - new Date(iso).getTime()) / 60000);
  if (minutes < 1) return "just now";
  if (minutes < 60) return `${minutes} min ago`;
  const hours = Math.round(minutes / 60);
  return hours < 48 ? `${hours} h ago` : `${Math.round(hours / 24)} days ago`;
}

function host(url: string) {
  try {
    return new URL(url).hostname.replace(/^www\./, "");
  } catch {
    return url;
  }
}

/** Where a story moved since the previous search. */
const Movement: React.FC<{ idea: FacelessIdea }> = ({ idea }) => {
  if (idea.new) return <Badge tone="accent">new</Badge>;
  if (!idea.previousRank) return null;
  const by = idea.previousRank - idea.rank;
  if (by === 0) return <span className="text-meta text-studio-600">=</span>;
  return (
    <span className={cn("inline-flex items-center text-meta", by > 0 ? "text-success-fg" : "text-danger-fg")}>
      {by > 0 ? <ArrowUp className="w-3 h-3" /> : <ArrowDown className="w-3 h-3" />}
      {Math.abs(by)}
    </span>
  );
};

/** One part of the score as a bar, 0 to 1. */
const Bar: React.FC<{ label: string; value: number; hint: string }> = ({ label, value, hint }) => (
  <div className="grid grid-cols-[6rem_1fr_2.5rem] items-center gap-2" title={hint}>
    <span className="text-meta text-studio-400">{label}</span>
    <span className="h-1.5 rounded-full bg-studio-800 overflow-hidden">
      <span className="block h-full bg-accent" style={{ width: `${Math.round(Math.max(0, Math.min(1, value)) * 100)}%` }} />
    </span>
    <span className="text-meta text-studio-300 text-right tabular-nums">{value.toFixed(2)}</span>
  </div>
);

function count(n: number) {
  return n >= 1e6 ? `${(n / 1e6).toFixed(1)}M` : n >= 1e3 ? `${Math.round(n / 1e3)}k` : String(Math.round(n));
}

function age(hours?: number) {
  if (hours === undefined) return "";
  return hours < 48 ? `${Math.round(hours)} h ago` : `${Math.round(hours / 24)} days ago`;
}

/** What the demand check found: videos already made, the watched channels, searches, reels. */
const DemandDetails: React.FC<{ demand: FacelessDemand }> = ({ demand }) => (
  <div className="rounded-control border border-studio-800 p-2 space-y-1.5">
    <p className="text-studio-300">
      Demand {demand.score.toFixed(2)} · {demand.recentVideos} videos on it in 3 days · best {count(demand.viewsPerHour)} views an
      hour
      {demand.subject &&
        ` · "${demand.subject.keyword}" ${count(demand.subject.volume)} searches a month${demand.subject.trend ? ` (${demand.subject.trend >= 1 ? "+" : ""}${Math.round((demand.subject.trend - 1) * 100)}%)` : ""}`}
      <span className="text-studio-600"> · checked {ago(demand.checkedAt)}</span>
    </p>
    {(demand.watched ?? []).map((w) => (
      <p key={w.url} className="text-success-fg truncate" title={w.title}>
        {w.channel}: {count(w.views)} views, {w.ratio.toFixed(1)}× its usual · {w.title}
      </p>
    ))}
    {(demand.videos ?? []).slice(0, 3).map((v) => (
      <p key={v.id} className="truncate">
        <span className="text-studio-500">YouTube</span> {v.channel} · {count(v.views)} views {age(v.hours)} · {v.title}
      </p>
    ))}
    {(demand.reels ?? []).slice(0, 2).map((r) => (
      <p key={r.url} className="truncate">
        <span className="text-studio-500">{r.url.includes("tiktok") ? "TikTok" : "Reel"}</span> @{r.author} · {count(r.views)} views ·{" "}
        {r.caption}
      </p>
    ))}
    {(demand.videos ?? []).length === 0 && <p className="text-studio-500">No videos on this story yet: it is open.</p>}
  </div>
);

const STAR_FIELDS = [
  { key: "importance", label: "Importance" },
  { key: "longFit", label: "Long fit" },
  { key: "shortFit", label: "Short fit" },
] as const;

/** Your read of a story: it counts for 40% of its score. */
const RateForm: React.FC<{ idea: FacelessIdea; onSave: (read: Parameters<typeof api.facelessTriage>[0]) => void }> = ({
  idea,
  onSave,
}) => {
  const e = idea.editorial;
  const [read, setRead] = useState({
    importance: e?.importance ?? 3,
    longFit: e?.longFit ?? 3,
    shortFit: e?.shortFit ?? 3,
    angle: e?.angle ?? "",
    reason: e?.reason ?? "",
  });
  return (
    <div className="rounded-control border border-studio-800 p-2 space-y-2">
      <div className="flex flex-wrap gap-3">
        {STAR_FIELDS.map((f) => (
          <label key={f.key} className="flex items-center gap-1 text-meta text-studio-400">
            {f.label}
            <select
              value={read[f.key]}
              onChange={(ev) => setRead({ ...read, [f.key]: Number(ev.target.value) })}
              className="h-6 rounded border border-studio-700 bg-studio-950 px-1 text-meta text-studio-100"
            >
              {[1, 2, 3, 4, 5].map((n) => (
                <option key={n} value={n}>
                  {n}
                </option>
              ))}
            </select>
          </label>
        ))}
      </div>
      <input
        value={read.angle}
        onChange={(ev) => setRead({ ...read, angle: ev.target.value })}
        placeholder="The angle that would make it worth watching"
        className="w-full h-7 rounded border border-studio-700 bg-studio-950 px-2 text-meta text-studio-100 placeholder:text-studio-600"
      />
      <Button size="sm" variant="secondary" onClick={() => onSave({ id: idea.id, verdict: "keep", ...read })}>
        Save my read
      </Button>
    </div>
  );
};

/**
 * The ranking, for reviewing it: stories found on the web, grouped by event and ranked for the
 * format, with how each moved since the last search and why it scores what it does. Search
 * runs only on the button (or your agent's call).
 */
export const IdeasList: React.FC<{
  format: FacelessFormat;
  busy: boolean;
  onMake: (idea: FacelessIdea) => void;
  onError: (message: string) => void;
}> = ({ format, busy, onMake, onError }) => {
  const [view, setView] = useState<FacelessIdeas | null>(null);
  const [lane, setLane] = useState<LaneId>("stories");
  const [searching, setSearching] = useState(false);
  const [report, setReport] = useState<FacelessDiscovered | null>(null);
  const [showReport, setShowReport] = useState(false);
  const [open, setOpen] = useState<number | null>(null);
  const [all, setAll] = useState(false);
  // The shortlist (what the agent sees) by default; everything shows what the filters left out.
  const [everything, setEverything] = useState(false);
  const [checking, setChecking] = useState(false);
  const [demandNote, setDemandNote] = useState<string | null>(null);

  const checkDemand = async () => {
    setChecking(true);
    try {
      const r = await api.facelessDemand(format, 10);
      setDemandNote(
        `Checked ${r.checked} stories · Treg $${(r.costMicro / 1e6).toFixed(4)}${r.problems.length ? ` · ${r.problems.join("; ")}` : ""}`,
      );
      await load();
    } catch (err) {
      onError(String(err));
    } finally {
      setChecking(false);
    }
  };

  const load = useCallback(
    () =>
      api
        .facelessIdeas(format, 3, 30)
        .then(setView)
        .catch((err) => onError(String(err))),
    [format, onError],
  );
  useEffect(() => void load(), [load]);

  const run = async (work: Promise<unknown>) => {
    try {
      await work;
      await load();
    } catch (err) {
      onError(String(err));
    }
  };

  const search = async () => {
    setSearching(true);
    try {
      setReport(await api.facelessDiscover());
      await load();
    } catch (err) {
      onError(String(err));
    } finally {
      setSearching(false);
    }
  };

  const laneAll = view?.ideas.filter((i) => i.lane === lane) ?? [];
  const inLane = everything ? laneAll : laneAll.filter((i) => i.shortlisted);
  const filteredOut = laneAll.length - laneAll.filter((i) => i.shortlisted).length;
  const shown = all ? inLane : inLane.slice(0, TOP);
  const broken = report?.funnels.filter((f) => f.issues.length > 0).length ?? 0;
  const others = inLane.slice(0, 20);

  return (
    <section className="mt-10">
      <div className="flex items-center gap-3 border-b border-studio-800 pb-2">
        <h2 className="text-label font-semibold text-studio-300 whitespace-nowrap">
          Ranking · {format === "long" ? "Long" : "Short"}
        </h2>
        <span className="text-meta text-studio-500 truncate">
          {view ? `${view.profile} · last search ${ago(view.lastDiscoverAt)} · ${view.stories} stories this week` : "…"}
        </span>
        <Button
          className="ml-auto"
          size="sm"
          variant="ghost"
          icon={checking ? Loader2 : TrendingUp}
          disabled={checking || searching || busy || !view?.ideas.length}
          onClick={() => void checkDemand()}
          title="For the top 10: YouTube videos already on each story and their views, your watched channels, Google searches for the subject; TikTok and reels for shorts. A few cents through Treg."
        >
          {checking ? "Checking…" : "Check demand"}
        </Button>
        <Button
          size="sm"
          variant="secondary"
          icon={searching ? Loader2 : Search}
          disabled={searching || busy}
          onClick={() => void search()}
          title="Search news sites, Hacker News, GitHub, Hugging Face, lab sites, and X and Reddit through Treg"
        >
          {searching ? "Searching…" : "Search the web"}
        </Button>
      </div>

      {report && (
        <div className="mt-2 text-meta text-studio-400">
          <button type="button" className="inline-flex items-center gap-1 hover:text-studio-200" onClick={() => setShowReport((v) => !v)}>
            {showReport ? <ChevronDown className="w-3 h-3" /> : <ChevronRight className="w-3 h-3" />}
            Found {report.newStories} new stories from {report.funnels.length} sources
            {report.costMicro > 0 && ` · Treg $${(report.costMicro / 1e6).toFixed(4)}`}
            {broken > 0 && <span className="text-suggest-fg"> · {broken} with problems</span>}
          </button>
          {showReport && (
            <ul className="mt-1 space-y-0.5 pl-4">
              {report.funnels.map((f) => (
                <li key={f.name} className={cn(f.issues.length > 0 && "text-suggest-fg")}>
                  {f.skipped
                    ? `${f.name}: skipped, ${f.skipped}`
                    : `${f.name}: kept ${f.accepted} of ${f.found}, ${f.newStories} new${f.costMicro ? `, $${(f.costMicro / 1e6).toFixed(4)}` : ""}`}
                  {f.issues.length > 0 && ` · ${f.issues.slice(0, 3).join("; ")}`}
                </li>
              ))}
            </ul>
          )}
        </div>
      )}

      <div className="mt-3 flex items-center gap-3">
        <Segmented
          size="sm"
          label="Kind of idea"
          value={lane}
          onChange={setLane}
          options={LANES.map((l) => ({
            value: l.id,
            label: `${l.label} ${view ? view.ideas.filter((i) => i.lane === l.id).length : ""}`,
          }))}
        />
        <Segmented
          size="sm"
          label="Filtered"
          value={everything ? "all" : "short"}
          onChange={(v) => setEverything(v === "all")}
          options={[
            { value: "short", label: "Shortlist", title: "What passes the filters: what your agent sees" },
            { value: "all", label: `Everything${filteredOut ? ` (+${filteredOut})` : ""}`, title: "Also what the filters left out, with why" },
          ]}
        />
      </div>
      {demandNote && <p className="mt-1 text-meta text-studio-400">{demandNote}</p>}

      {view && view.ideas.length === 0 ? (
        <p className="py-6 text-label text-studio-500">No stories yet. Search the web to find some.</p>
      ) : (
        <ol className="mt-2 divide-y divide-studio-800/70">
          {shown.map((idea) => (
            <li key={idea.id} className="py-2">
              <div className="flex items-start gap-2">
                <span className="w-7 shrink-0 pt-0.5 text-right text-label font-semibold tabular-nums text-studio-300">
                  {idea.rank}
                </span>
                <span className="w-8 shrink-0 pt-1">
                  <Movement idea={idea} />
                </span>
                <button
                  type="button"
                  onClick={() => setOpen(open === idea.id ? null : idea.id)}
                  className="mt-0.5 text-studio-500 hover:text-studio-200"
                  aria-label={open === idea.id ? "Hide details" : "Show details"}
                >
                  {open === idea.id ? <ChevronDown className="w-4 h-4" /> : <ChevronRight className="w-4 h-4" />}
                </button>
                <div className="min-w-0 flex-1">
                  <p className={cn("text-body text-studio-100", idea.made && "text-studio-500 line-through")}>
                    {idea.pinned && <Pin className="inline w-3.5 h-3.5 mr-1 text-accent-fg" />}
                    {idea.title}
                  </p>
                  <p className="mt-0.5 flex flex-wrap items-center gap-x-2 gap-y-1 text-meta text-studio-500">
                    <span className="text-studio-300 tabular-nums" title="Score in this ranking">
                      {idea.score.toFixed(2)}
                    </span>
                    <span>
                      {idea.outlets.length} {idea.outlets.length === 1 ? "source" : "sources"}
                    </span>
                    {idea.attention > 0 && (
                      <span>
                        {Math.round(idea.attention)}
                        {idea.growth > 0 && ` (+${Math.round(idea.growth)})`}
                      </span>
                    )}
                    <span>{ago(idea.eventAt)}</span>
                    {idea.hooks.map((h) => (
                      <Badge key={h} tone={h === "mainstream" ? "neutral" : undefined}>
                        {h}
                      </Badge>
                    ))}
                    {idea.editorial && (
                      <Badge tone="success" title={idea.editorial.reason}>
                        {idea.editorial.by === "you" ? "your read" : `read by ${idea.editorial.by}`}
                      </Badge>
                    )}
                    {idea.made && <Badge tone="danger">already made</Badge>}
                    {idea.demand && (
                      <Badge tone="accent" title={`${idea.demand.recentVideos} recent videos · ${Math.round(idea.demand.viewsPerHour)} views an hour`}>
                        demand {idea.demand.score.toFixed(2)}
                      </Badge>
                    )}
                    {!idea.shortlisted && idea.leftOut && <span className="text-studio-600">· left out: {idea.leftOut}</span>}
                  </p>
                </div>
                <IconButton
                  icon={idea.pinned ? PinOff : Pin}
                  size="sm"
                  label={idea.pinned ? "Unpin" : "Pin it first"}
                  onClick={() => void run(api.facelessPin(idea.id, !idea.pinned))}
                />
                <IconButton
                  icon={EyeOff}
                  size="sm"
                  label="Set aside: not for this channel"
                  onClick={() => void run(api.facelessSetAside(idea.ids))}
                />
                <Button
                  size="sm"
                  variant="secondary"
                  icon={Plus}
                  disabled={busy || idea.made}
                  onClick={() => onMake(idea)}
                  title={`Make a ${format} video of this story`}
                >
                  Make {format}
                </Button>
              </div>
              {open === idea.id && (
                <div className="mt-2 ml-[4.75rem] grid gap-4 md:grid-cols-[1fr_15rem]">
                  <div className="space-y-2 text-meta text-studio-400 min-w-0">
                    {idea.editorial?.angle && (
                      <p className="text-studio-200">
                        <span className="text-studio-500">Angle: </span>
                        {idea.editorial.angle}
                      </p>
                    )}
                    {idea.summary && <p className="text-studio-300">{idea.summary.slice(0, 400)}</p>}
                    <ul className="space-y-1">
                      {idea.urls.map((url, n) => (
                        <li key={url} className="flex items-center gap-1 min-w-0">
                          <Globe className="w-3 h-3 shrink-0" />
                          <span className="text-studio-500 shrink-0">{host(url)}</span>
                          <span className="truncate">{idea.titles[n] ?? url}</span>
                          {idea.ids.length > 1 && idea.ids[n] !== undefined && (
                            <IconButton
                              icon={Scissors}
                              size="sm"
                              label="Not the same event: take this report out of the group"
                              onClick={() => void run(api.facelessSeparate(idea.ids[n], true))}
                            />
                          )}
                        </li>
                      ))}
                    </ul>
                    {idea.demand && <DemandDetails demand={idea.demand} />}
                    {lane === "stories" && (
                      <label className="flex items-center gap-2">
                        <span>Same event as</span>
                        <select
                          className="h-6 min-w-0 flex-1 rounded border border-studio-700 bg-studio-950 px-1 text-meta text-studio-100"
                          value=""
                          onChange={(ev) => {
                            const to = Number(ev.target.value);
                            if (to) void run(api.facelessJoin(idea.ids, to));
                          }}
                        >
                          <option value="">another story… (joins them)</option>
                          {others
                            .filter((o) => o.id !== idea.id)
                            .map((o) => (
                              <option key={o.id} value={o.id}>
                                {o.rank}. {o.title.slice(0, 90)}
                              </option>
                            ))}
                        </select>
                      </label>
                    )}
                  </div>
                  <div className="space-y-1.5">
                    <Bar label="Coverage" value={idea.parts.coverage} hint={`${idea.outlets.join(", ")}`} />
                    <Bar label="Attention" value={idea.parts.attention} hint="Points, likes, stars or comments, and their growth" />
                    <Bar label="Fit" value={idea.parts.fit} hint={`The channel's hooks: ${idea.hooks.join(", ") || "none"}`} />
                    <Bar label="Freshness" value={idea.parts.freshness} hint="Halves every two days (a day and a half for shorts)" />
                    {idea.parts.editorial !== undefined && (
                      <Bar label="Editor" value={idea.parts.editorial} hint="Importance × format fit, 40% of the score" />
                    )}
                    {idea.parts.demand !== undefined && (
                      <Bar label="Demand" value={idea.parts.demand} hint="Who watches videos on it and searches for its subject" />
                    )}
                    {idea.parts.saturation !== undefined && idea.parts.saturation > 0 && (
                      <Bar label="Saturation" value={idea.parts.saturation} hint="Videos on it from the last three days (takes a little off)" />
                    )}
                    <p className="text-meta text-studio-500">
                      Code {idea.parts.code.toFixed(2)}
                      {idea.parts.primary && " · maker's own post"}
                      {idea.parts.lowValue && " · low-value format"} · long {idea.longScore.toFixed(2)} · short{" "}
                      {idea.shortScore.toFixed(2)}
                    </p>
                    {lane === "stories" && (
                      <RateForm key={idea.id} idea={idea} onSave={(read) => void run(api.facelessTriage(read))} />
                    )}
                  </div>
                </div>
              )}
            </li>
          ))}
        </ol>
      )}
      {inLane.length > TOP && (
        <Button size="sm" variant="ghost" className="mt-2" onClick={() => setAll((v) => !v)}>
          {all ? `Show the top ${TOP}` : `Show all ${inLane.length}`}
        </Button>
      )}
    </section>
  );
};
