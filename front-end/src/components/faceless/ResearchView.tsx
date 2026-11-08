import React, { useEffect, useState } from "react";
import { AlertTriangle, CheckCircle2, ChevronDown, ChevronRight, Globe } from "lucide-react";
import { Badge, cn } from "../ui";
import { api } from "../../lib/ipc";
import type { FacelessDossier } from "../../lib/types";

const GRADE_TEXT = {
  ready: "Ready: enough facts from independent sources for a video",
  thin: "Thin: needs more facts or more independent sources",
  empty: "No checked facts yet",
} as const;

/** The production's research: grade, checked facts with their quotes, and the sources read. */
export const ResearchView: React.FC<{ refreshKey: unknown }> = ({ refreshKey }) => {
  const [dossier, setDossier] = useState<FacelessDossier | null>(null);
  const [show, setShow] = useState<"facts" | "sources" | "refused" | null>("facts");

  useEffect(() => {
    let active = true;
    void api
      .facelessDossier()
      .then((d) => active && setDossier(d))
      .catch(() => active && setDossier(null));
    return () => {
      active = false;
    };
  }, [refreshKey]);

  if (!dossier) {
    return (
      <p className="mt-1 text-meta text-studio-500">
        Ask your agent to research this story: it reads pages through the app, which keeps only facts whose quote it
        finds on the page.
      </p>
    );
  }

  const section = (id: NonNullable<typeof show>, label: string, count: number) => (
    <button
      type="button"
      onClick={() => setShow(show === id ? null : id)}
      className="inline-flex items-center gap-1 text-meta text-studio-400 hover:text-studio-200"
    >
      {show === id ? <ChevronDown className="w-3 h-3" /> : <ChevronRight className="w-3 h-3" />}
      {label} {count}
    </button>
  );

  return (
    <div className="mt-2 space-y-2">
      <p
        className={cn(
          "flex items-center gap-1.5 text-meta",
          dossier.grade === "ready" ? "text-success-fg" : "text-suggest-fg",
        )}
      >
        {dossier.grade === "ready" ? <CheckCircle2 className="w-3.5 h-3.5" /> : <AlertTriangle className="w-3.5 h-3.5" />}
        {GRADE_TEXT[dossier.grade]} · {dossier.facts.length} facts, {dossier.origins.length} independent{" "}
        {dossier.origins.length === 1 ? "source" : "sources"}
      </p>
      <div className="flex gap-3">
        {section("facts", "Facts", dossier.facts.length)}
        {section("sources", "Sources", dossier.sources.length)}
        {dossier.rejected.length > 0 && section("refused", "Refused", dossier.rejected.length)}
      </div>
      {show === "facts" && (
        <ul className="space-y-2">
          {dossier.facts.map((f) => (
            <li key={f.id} className="text-meta">
              <p className="text-studio-200">
                <span className="text-studio-500">[{f.id}]</span> {f.statement}
              </p>
              <p className="mt-0.5 text-studio-500 italic">“{f.quote}”</p>
              <p className="mt-0.5 flex flex-wrap items-center gap-1 text-studio-500">
                {f.domain}
                {f.primary && <Badge tone="accent">primary</Badge>}
                {(dossier.corroboration[f.id] ?? 0) > 0 && <Badge tone="success">also in {dossier.corroboration[f.id]}</Badge>}
                {f.scope && <span>· {f.scope}</span>}
              </p>
            </li>
          ))}
        </ul>
      )}
      {show === "sources" && (
        <ul className="space-y-1">
          {dossier.sources.map((s) => (
            <li key={s.url} className="flex items-start gap-1 text-meta">
              <Globe className={cn("w-3 h-3 mt-0.5 shrink-0", s.status === "fetched" ? "text-studio-400" : "text-suggest-fg")} />
              <span className="min-w-0">
                <span className="block truncate text-studio-300">{s.url}</span>
                <span className="text-studio-500">
                  {s.status === "fetched" ? `read · ${s.chars.toLocaleString()} characters` : `${s.status}: ${s.reason ?? ""}`}
                  {s.publishedOn && ` · ${s.publishedOn}`}
                  {s.notices && s.notices.length > 0 && " · has a correction or update notice"}
                </span>
              </span>
            </li>
          ))}
        </ul>
      )}
      {show === "refused" && (
        <ul className="space-y-1">
          {dossier.rejected
            .slice(-20)
            .reverse()
            .map((r, n) => (
              <li key={n} className="text-meta text-studio-400">
                {r.statement} <span className="text-danger-fg">· {r.reason}</span>
              </li>
            ))}
        </ul>
      )}
      {dossier.brief.beats.length > 0 && (
        <div className="text-meta text-studio-400">
          <p className="font-semibold text-studio-300">{dossier.brief.viewerQuestion || "Story beats"}</p>
          <ol className="mt-1 list-decimal pl-4 space-y-0.5">
            {dossier.brief.beats.map((b, n) => (
              <li key={n}>{b}</li>
            ))}
          </ol>
        </div>
      )}
    </div>
  );
};
