import React, { useEffect, useState } from "react";
import { AlertTriangle, CheckCircle2, Quote } from "lucide-react";
import { Badge } from "../ui";
import { api } from "../../lib/ipc";
import type { FacelessFinding, FacelessScriptView } from "../../lib/types";

const seconds = (s: number) => (s >= 90 ? `${Math.floor(s / 60)} min ${Math.round(s % 60)} s` : `${Math.round(s)} s`);

const REVIEW_NAMES: Record<string, string> = {
  attention: "Attention",
  coldReader: "Cold reader",
  fairness: "Fairness",
};

/** The open production's script, read from the app every time the panel refreshes. */
export const ScriptView: React.FC<{ refreshKey: unknown }> = ({ refreshKey }) => {
  const [view, setView] = useState<FacelessScriptView | null>(null);
  useEffect(() => {
    let active = true;
    void api
      .facelessScriptGet()
      .then((v) => active && setView(v))
      .catch(() => active && setView(null));
    return () => {
      active = false;
    };
  }, [refreshKey]);
  if (!view) return <p className="mt-2 text-meta text-studio-500">No script yet: your agent writes it from the facts.</p>;
  return <ScriptDetails view={view} />;
};

/** A finding as the review gave it. */
const FindingLine: React.FC<{ finding: FacelessFinding }> = ({ finding }) => {
  const open = !finding.resolved;
  return (
    <li className="space-y-0.5">
      <div className="flex flex-wrap items-center gap-1.5">
        <span className="text-studio-300">{REVIEW_NAMES[finding.review] ?? finding.review}</span>
        {finding.severity === "blocker" && open && <Badge tone="danger">Blocker</Badge>}
        {finding.severity !== "blocker" && open && <Badge tone="suggest">Minor</Badge>}
        {!open && <Badge tone="success">Fixed</Badge>}
        {finding.problem && <span className="text-studio-500">{finding.problem}</span>}
      </div>
      <p className={open ? "text-studio-200" : "text-studio-500 line-through"}>“{finding.claim}”</p>
      {finding.why && <p className="text-studio-400">{finding.why}</p>}
      {finding.change && open && <p className="text-studio-400">Instead: “{finding.change}”</p>}
    </li>
  );
};

/** The script: length against the format, what the check says, each scene with the facts it
 * cites (the exact page words), and the review with the revision it accepted. */
export const ScriptDetails: React.FC<{ view: FacelessScriptView }> = ({ view }) => {
  const [openScene, setOpenScene] = useState<number | null>(null);
  const { script, check, review } = view;
  const facts = new Map(view.facts.map((f) => [f.id, f]));
  const [shortest, longest] = view.targetSeconds;
  const fits = check.estimatedSeconds >= shortest && check.estimatedSeconds <= longest;
  const findings = view.reviewCurrent ? (review?.findings ?? []) : [];
  const blockers = findings.filter((f) => f.severity === "blocker" && !f.resolved).length;
  return (
    <div className="mt-2 space-y-2 text-meta">
      <p className="text-label font-semibold text-studio-100 break-words">{script.title}</p>
      <p className="text-studio-400">
        {script.scenes.length} scenes · {check.words} words · about {seconds(check.estimatedSeconds)}{" "}
        <span className={fits ? "text-success-fg" : "text-suggest-fg"}>
          ({fits ? "fits" : "outside"} {seconds(shortest)} to {seconds(longest)})
        </span>
      </p>
      {view.acceptedAt ? (
        <p className="flex items-center gap-1 text-success-fg">
          <CheckCircle2 className="w-3 h-3 shrink-0" aria-hidden /> This revision was accepted{" "}
          {new Date(view.acceptedAt).toLocaleString()}
        </p>
      ) : (
        <p className="text-studio-500">
          {!review
            ? "Not reviewed yet."
            : !view.reviewCurrent
              ? "Changed since the review: it needs the three reviews again."
              : `Not accepted: ${blockers} blocker${blockers === 1 ? "" : "s"} open.`}
        </p>
      )}
      {[...check.errors, ...view.missingFacts.map((id) => `${id} is cited but no longer in the research.`)].map((e) => (
        <p key={e} className="flex gap-1 text-danger-fg">
          <AlertTriangle className="w-3 h-3 mt-0.5 shrink-0" aria-hidden /> {e}
        </p>
      ))}
      {check.notes.map((n) => (
        <p key={n} className="flex gap-1 text-suggest-fg">
          <AlertTriangle className="w-3 h-3 mt-0.5 shrink-0" aria-hidden /> {n}
        </p>
      ))}
      <ol className="space-y-1.5" aria-label="Scenes">
        {script.scenes.map((scene, i) => {
          const open = openScene === i;
          return (
            <li key={i} className="rounded border border-studio-800 px-2 py-1.5">
              <button
                type="button"
                className="w-full text-left"
                aria-expanded={open}
                onClick={() => setOpenScene(open ? null : i)}
              >
                <span className="flex flex-wrap items-center gap-1.5">
                  <span className="text-studio-500 tabular-nums">{i + 1}</span>
                  {scene.analysis && <Badge>Analysis</Badge>}
                  {scene.factIds.map((id) => (
                    <Badge key={id} tone={facts.has(id) ? "accent" : "danger"}>
                      {id}
                    </Badge>
                  ))}
                  {scene.source && <span className="text-studio-500">{scene.source}</span>}
                </span>
                <span className="mt-0.5 block text-studio-200 break-words">{scene.narration}</span>
              </button>
              {open && (
                <div className="mt-1.5 space-y-1 border-t border-studio-800 pt-1.5">
                  {scene.onScreen && <p className="text-studio-400">On screen: {scene.onScreen}</p>}
                  {scene.factIds.map((id) => {
                    const fact = facts.get(id);
                    if (!fact) return null;
                    return (
                      <div key={id} className="space-y-0.5">
                        <p className="text-studio-300">
                          {id}: {fact.statement}
                        </p>
                        <p className="flex gap-1 text-studio-500 break-words">
                          <Quote className="w-3 h-3 mt-0.5 shrink-0" aria-hidden />“{fact.quote}”
                        </p>
                        <p className="text-studio-600 break-all">
                          {fact.primary ? "Primary source · " : ""}
                          {fact.url}
                        </p>
                      </div>
                    );
                  })}
                </div>
              )}
            </li>
          );
        })}
      </ol>
      {findings.length > 0 && (
        <section className="space-y-1.5" aria-label="Review findings">
          <p className="text-label font-semibold text-studio-100">Review findings</p>
          <ul className="space-y-2">
            {findings.map((f, i) => (
              <FindingLine key={i} finding={f} />
            ))}
          </ul>
        </section>
      )}
    </div>
  );
};
