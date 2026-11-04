import React, { useCallback, useEffect, useState } from "react";
import { AlertTriangle, CheckCircle2, Info, Loader2, Scissors, Wand2, XCircle } from "lucide-react";
import { api } from "../../lib/ipc";
import { useProjectStore } from "../../stores/projectStore";
import { Button, cn } from "../ui";
import type { CheckFinding, CheckReport, CheckSeverity } from "../../lib/types";

const SEVERITY: Record<CheckSeverity, { icon: typeof Info; className: string; one: string; many: string }> = {
  error: { icon: XCircle, className: "text-danger-fg", one: "error", many: "errors" },
  warning: { icon: AlertTriangle, className: "text-suggest-fg", one: "warning", many: "warnings" },
  info: { icon: Info, className: "text-studio-400", one: "note", many: "notes" },
};

function clock(us: number): string {
  const seconds = Math.floor(us / 1e6);
  return `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, "0")}`;
}

interface CheckPanelProps {
  /** Shows a moment on the timeline (the dialog closes so it can be seen). Without it, times are only read. */
  onShow?: (timeUs: number) => void;
  disabled?: boolean;
  /** Checks this short as its export plays it; its findings are fixed on its own timeline, so no fixes here. */
  shortId?: string;
}

const SEVERITY_ORDER: CheckSeverity[] = ["error", "warning", "info"];

/** Errors first, then warnings, then notes; each in timeline order, as the report sorts them. */
function sorted(findings: CheckFinding[]): CheckFinding[] {
  const rank = (f: CheckFinding) => SEVERITY_ORDER.indexOf(f.severity);
  const placed = (f: CheckFinding) => (f.startUs === undefined ? 0 : 1);
  return [...findings].sort((a, b) => rank(a) - rank(b) || placed(a) - placed(b) || (a.startUs ?? 0) - (b.startUs ?? 0));
}

/**
 * The check before export (`check.rs`): problems a viewer would notice, each with where it is
 * and, when one is safe, a fix. Agents read the same findings from the `check` action. What the
 * graphics' frames show comes after, as drawing them takes longer (`graphic_checks.rs`).
 */
export const CheckPanel: React.FC<CheckPanelProps> = ({ onShow, disabled, shortId }) => {
  const handle = useProjectStore((s) => s.openedProject?.projectHandle);
  const revision = useProjectStore((s) => s.openedProject?.revision);
  const [report, setReport] = useState<CheckReport>();
  const [frameFindings, setFrameFindings] = useState<CheckFinding[]>();
  const [error, setError] = useState<string>();
  const [fixing, setFixing] = useState<string>();

  // Checks again whenever the edit changes, from here or anywhere else.
  useEffect(() => {
    if (!handle) return;
    let current = true;
    setError(undefined);
    api
      .projectCheck(handle, shortId)
      .then((next) => current && setReport(next))
      .catch((err) => current && setError(String(err)));
    setFrameFindings(undefined);
    api
      .projectCheckFrames(handle, shortId)
      .then((next) => current && setFrameFindings(next))
      .catch((err) => {
        if (current) {
          setFrameFindings([
            {
              id: "graphic_frames_not_checked-failed",
              kind: "graphic_frames_not_checked",
              severity: "info",
              title: "Graphics' frames not checked",
              detail: String(err),
            },
          ]);
        }
      });
    return () => {
      current = false;
    };
  }, [handle, revision, shortId]);

  const fix = useCallback(
    async (ids: string[], key: string) => {
      if (!handle || !report) return;
      setFixing(key);
      setError(undefined);
      try {
        const project = await api.projectCheckFix(handle, report.revision, ids);
        useProjectStore.getState().applyOpenedProject(project);
      } catch (err) {
        setError(String(err));
      } finally {
        setFixing(undefined);
      }
    },
    [handle, report],
  );

  const findings = sorted([...(report?.findings ?? []), ...(frameFindings ?? [])]);
  const checkingFrames = frameFindings === undefined;
  const fixable = shortId ? [] : findings.filter((f) => f.fixLabel);
  const blips = fixable.filter((f) => f.kind === "fragment");
  const counts = (["error", "warning", "info"] as const)
    .map((severity) => {
      const n = findings.filter((f) => f.severity === severity).length;
      return n > 0 ? `${n} ${n === 1 ? SEVERITY[severity].one : SEVERITY[severity].many}` : null;
    })
    .filter(Boolean)
    .join(" · ");

  return (
    <section aria-labelledby="export-check-title" className="space-y-2">
      <div className="flex items-center gap-2">
        <h3 id="export-check-title" className="text-label font-medium text-studio-200">
          Check
        </h3>
        <span className="text-label text-studio-400 flex items-center gap-1.5">
          {!report && !error && <Loader2 className="w-3.5 h-3.5 animate-spin" aria-hidden />}
          {report && !checkingFrames && findings.length === 0 && (
            <>
              <CheckCircle2 className="w-4 h-4 text-success" aria-hidden />
              No problems found
            </>
          )}
          {counts}
          {report && checkingFrames && (
            <>
              <Loader2 className="w-3.5 h-3.5 animate-spin" aria-hidden />
              Checking the graphics&apos; frames…
            </>
          )}
        </span>
        <span className="flex-1" />
        {blips.length > 0 && (
          <Button
            size="sm"
            variant="secondary"
            icon={Scissors}
            disabled={disabled || fixing !== undefined}
            onClick={() => void fix(["blips"], "blips")}
            title="Cuts every blip between two cuts, as one undo step"
          >
            {fixing === "blips" ? "Removing…" : `Remove all blips (${blips.length})`}
          </Button>
        )}
        {fixable.length > 1 && (
          <Button
            size="sm"
            variant="secondary"
            icon={Wand2}
            disabled={disabled || fixing !== undefined}
            onClick={() => void fix(["all"], "all")}
            title="Each fix is its own undo step"
          >
            {fixing === "all" ? "Fixing…" : `Fix all (${fixable.length})`}
          </Button>
        )}
      </div>
      {error && (
        <p role="alert" className="text-label text-danger-fg">
          {error}
        </p>
      )}
      {findings.length > 0 && (
        <ul className="max-h-60 overflow-y-auto rounded-control border border-studio-800 divide-y divide-studio-800">
          {findings.map((finding) => (
            <FindingRow
              key={finding.id}
              finding={finding}
              busy={fixing === finding.id}
              disabled={disabled || fixing !== undefined}
              onShow={onShow}
              canFix={!shortId}
              onFix={() => void fix([finding.id], finding.id)}
            />
          ))}
        </ul>
      )}
    </section>
  );
};

const FindingRow: React.FC<{
  finding: CheckFinding;
  busy: boolean;
  disabled?: boolean;
  canFix: boolean;
  onShow?: (timeUs: number) => void;
  onFix: () => void;
}> = ({ finding, busy, disabled, canFix, onShow, onFix }) => {
  const { icon: Icon, className } = SEVERITY[finding.severity];
  return (
    <li className="flex items-start gap-2.5 px-3 py-2 bg-studio-850">
      <Icon className={cn("w-4 h-4 mt-0.5 shrink-0", className)} aria-label={finding.severity} />
      <div className="flex-1 min-w-0">
        <p className="text-label font-medium text-studio-100">{finding.title}</p>
        <p className="text-meta text-studio-400 select-text">{finding.detail}</p>
        {canFix && finding.fixLabel && <p className="text-meta text-accent-fg">Fix: {finding.fixLabel}</p>}
      </div>
      <div className="flex items-center gap-1.5 shrink-0">
        {finding.startUs !== undefined &&
          (onShow ? (
            <Button size="sm" variant="ghost" className="font-mono tabular-nums" onClick={() => onShow(finding.startUs!)} title="Show it on the timeline">
              {clock(finding.startUs)}
            </Button>
          ) : (
            <span className="text-meta font-mono tabular-nums text-studio-400">{clock(finding.startUs)}</span>
          ))}
        {canFix && finding.fixLabel && (
          <Button size="sm" variant="secondary" disabled={disabled} onClick={onFix} title="One undo step">
            {busy ? "Fixing…" : "Fix"}
          </Button>
        )}
      </div>
    </li>
  );
};
