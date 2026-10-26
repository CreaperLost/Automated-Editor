import React, { useEffect, useRef, useState } from "react";
import { Search } from "lucide-react";
import { useProjectStore } from "../../stores/projectStore";
import { api } from "../../lib/ipc";
import { seekPlayback } from "../../lib/playbackControl";
import { SearchHit, SearchResults } from "../../lib/types";
import { Badge, cn } from "../ui";

function formatTime(us: number): string {
  const total = us / 1_000_000;
  const minutes = Math.floor(total / 60);
  const seconds = total - minutes * 60;
  return `${minutes}:${seconds.toFixed(1).padStart(4, "0")}`;
}

/** The excerpt with the hit (between `[` and `]`) in bold. */
function Excerpt({ text }: { text: string }) {
  const match = /^(.*?)\[(.*)\](.*)$/s.exec(text);
  if (!match) return <>{text}</>;
  return (
    <>
      {match[1]}
      <strong className="text-studio-100 font-semibold">{match[2]}</strong>
      {match[3]}
    </>
  );
}

function where(hit: SearchHit): React.ReactNode {
  if (hit.status === "plays") return <Badge tone="success">plays</Badge>;
  if (hit.status === "cut") return <Badge tone="danger">cut</Badge>;
  return <Badge>{hit.short ? "short" : "not placed"}</Badge>;
}

/// Search: a word or phrase in every transcript (the recording, imported takes and videos),
/// including what the edit cut out. Click a hit to go to it (a cut one: to the join over it).
export const SearchPanel: React.FC = () => {
  const handle = useProjectStore((s) => s.openedProject?.projectHandle);
  const revision = useProjectStore((s) => s.openedProject?.revision);
  const captionsVersion = useProjectStore((s) => s.captionsVersion);
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<SearchResults | null>(null);
  const [error, setError] = useState<string | null>(null);
  const asked = useRef(0);

  // Searches as you type (after a short pause), and again when the edit or a transcript changes.
  useEffect(() => {
    if (!handle || query.trim().length < 2) {
      setResults(null);
      return;
    }
    const id = ++asked.current;
    const timer = window.setTimeout(() => {
      void api
        .sourceSearch(handle, query)
        .then((next) => {
          if (id !== asked.current) return;
          setResults(next);
          setError(null);
        })
        .catch((err) => id === asked.current && setError(String(err)));
    }, 200);
    return () => window.clearTimeout(timer);
  }, [handle, query, revision, captionsVersion]);

  if (!handle) return <p className="p-3 text-label text-studio-500">Open a project to search what was said.</p>;

  const go = (hit: SearchHit) => {
    if (hit.editedUs === undefined) return;
    seekPlayback(hit.editedUs);
    useProjectStore.getState().revealPlayhead();
  };

  return (
    <div className="h-full flex flex-col min-h-0">
      <div className="shrink-0 p-2 border-b border-studio-800">
        <label className="flex items-center gap-2 h-8 px-2 rounded-control bg-studio-850 border border-studio-700 focus-within:border-accent/60">
          <Search className="w-4 h-4 text-studio-400 shrink-0" aria-hidden />
          <input
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Search what was said, cut parts too"
            aria-label="Search every transcript"
            className="flex-1 min-w-0 bg-transparent text-label text-studio-100 placeholder:text-studio-500 outline-none"
          />
        </label>
        {results && (
          <p className="mt-1 text-meta text-studio-500">
            {results.total} hit{results.total === 1 ? "" : "s"} in {results.streams} transcript{results.streams === 1 ? "" : "s"}
            {results.total > results.hits.length ? ` (first ${results.hits.length})` : ""}
          </p>
        )}
      </div>
      {error && <p role="alert" className="px-3 py-2 text-label text-danger-fg">{error}</p>}
      <ul className="flex-1 min-h-0 overflow-y-auto p-2 space-y-1">
        {results?.hits.map((hit) => (
          <li
            key={`${hit.stream}:${hit.firstWord}`}
            onClick={() => go(hit)}
            title={hit.editedUs === undefined ? `In ${hit.media}, not on the timeline` : undefined}
            className={cn(
              "rounded-control border border-studio-800 px-2 py-1.5 space-y-0.5 transition-colors",
              hit.editedUs !== undefined ? "cursor-pointer hover:bg-studio-850" : "opacity-70",
            )}
          >
            <div className="flex items-center gap-2 text-meta text-studio-500">
              <span className="font-mono tabular-nums text-studio-300">
                {hit.editedUs !== undefined ? formatTime(hit.editedUs) : formatTime(hit.sourceStartUs)}
              </span>
              <span className="flex-1 min-w-0 truncate">{hit.short ?? hit.media}</span>
              {!hit.exact && <span>close</span>}
              {where(hit)}
            </div>
            <p className="text-label text-studio-400">
              <Excerpt text={hit.excerpt} />
            </p>
          </li>
        ))}
        {results && results.hits.length === 0 && <li className="p-2 text-label text-studio-500">Not said in any transcript.</li>}
      </ul>
    </div>
  );
};
