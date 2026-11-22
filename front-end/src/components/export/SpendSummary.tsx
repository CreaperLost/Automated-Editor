import React, { useEffect, useState } from "react";
import { api } from "../../lib/ipc";
import type { SpendTotal, SpendView } from "../../lib/types";

const dollars = (total: SpendTotal) => `$${total.knownUsd.toFixed(3)}`;
const unknown = (total: SpendTotal) => (total.unknown > 0 ? `, ${total.unknown} of unknown cost` : "");
const calls = (n: number) => `${n} call${n === 1 ? "" : "s"}`;
const when = (at: string) =>
  new Date(at).toLocaleString(undefined, { month: "short", day: "numeric", hour: "2-digit", minute: "2-digit" });

/// What the app has paid for on this video (narration, web searches, drawn pictures,
/// thumbnails, online transcription), from the project's spend record. Folded by default.
export const SpendSummary: React.FC<{ projectHandle: string; refreshKey?: string }> = ({ projectHandle, refreshKey }) => {
  const [view, setView] = useState<SpendView | null>(null);
  useEffect(() => {
    let live = true;
    api
      .projectSpend(projectHandle)
      .then((found) => {
        if (live) setView(found);
      })
      .catch(() => {
        if (live) setView(null);
      });
    return () => {
      live = false;
    };
  }, [projectHandle, refreshKey]);
  if (!view) return null;
  const { report } = view;
  if (report.total.calls === 0) {
    return <p className="text-meta text-studio-500">Nothing paid for on this video yet.</p>;
  }
  return (
    <details className="rounded-control border border-studio-800 bg-studio-850 px-4 py-2.5 text-label">
      <summary className="cursor-pointer text-studio-200">
        Spent on this video: <span className="font-mono tabular-nums">{dollars(report.total)}</span>{" "}
        <span className="text-studio-500">
          over {calls(report.total.calls)}
          {unknown(report.total)}
        </span>
      </summary>
      <div className="mt-2 space-y-2">
        <table className="w-full text-meta">
          <tbody>
            {Object.entries(report.byProvider).map(([provider, total]) => (
              <tr key={provider}>
                <td className="text-studio-300">{provider}</td>
                <td className="text-right font-mono text-studio-200">{dollars(total)}</td>
                <td className="text-right text-studio-500">
                  {calls(total.calls)}
                  {unknown(total)}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
        <ul className="space-y-0.5 text-meta text-studio-400" aria-label="Latest paid calls">
          {view.latest.map((entry, i) => (
            <li key={`${entry.at}-${i}`} className="flex gap-2">
              <span className="w-28 shrink-0 font-mono text-studio-500">{when(entry.at)}</span>
              <span className="flex-1 min-w-0 truncate">
                {entry.step} · {entry.provider}
                {entry.note ? ` · ${entry.note}` : ""}
              </span>
              <span className="font-mono">{entry.costUsd == null ? "?" : `$${entry.costUsd.toFixed(3)}`}</span>
            </li>
          ))}
        </ul>
        <p className="text-meta text-studio-500">{view.notCounted}</p>
      </div>
    </details>
  );
};
