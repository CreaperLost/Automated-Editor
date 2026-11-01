import React, { useCallback, useEffect, useState } from "react";
import { CheckCircle2, Clock, FolderOpen, Loader2, RotateCw, Trash2, XCircle } from "lucide-react";
import { api } from "../../lib/ipc";
import { FILE_MANAGER } from "../../lib/projectActions";
import type { DeliveryView } from "../../lib/types";
import { IconButton } from "../ui";

/// The project's exports (DLV-2): what waits its turn, what runs, and what went out, each with
/// the revision it was made from. An output older than the edit, or one that failed, can be made
/// again from the edit as it is now (beside the old file; it waits if another export runs).
export const DeliveryList: React.FC<{ projectHandle: string; revision: number; refreshKey?: string }> = ({
  projectHandle,
  revision,
  refreshKey,
}) => {
  const [rows, setRows] = useState<DeliveryView[]>([]);
  const [error, setError] = useState<string>();

  const load = useCallback(() => {
    void api
      .exportDeliveries(projectHandle)
      .then(setRows)
      .catch((err) => setError(String(err)));
  }, [projectHandle]);
  useEffect(load, [load, revision, refreshKey]);
  const active = rows.some((r) => r.state === "queued" || r.state === "running");
  useEffect(() => {
    if (!active) return;
    const timer = window.setInterval(load, 500);
    return () => window.clearInterval(timer);
  }, [active, load]);

  if (rows.length === 0) return null;
  const run = (work: Promise<unknown>) =>
    void work.then(load).catch((err) => setError(String(err)));
  const name = (path?: string) => path?.split(/[\\/]/).pop();

  return (
    <div className="mx-6 mb-6 space-y-1">
      <div className="text-label font-medium text-studio-200">Exports</div>
      {error && <p className="text-meta text-danger-fg">{error}</p>}
      <ul className="divide-y divide-studio-800 rounded-control border border-studio-800">
        {rows.map((row) => {
          const waiting = row.state === "queued";
          const running = row.state === "running";
          const Icon = running ? Loader2 : waiting ? Clock : row.state === "completed" ? CheckCircle2 : XCircle;
          const tone =
            row.state === "completed" ? (row.older ? "text-zoom-fg" : "text-success") : running || waiting ? "text-studio-300" : "text-danger-fg";
          const note = waiting
            ? "Waiting its turn"
            : running
              ? `${Math.round(row.progress * 100)} %`
              : row.state === "completed"
                ? row.missing
                  ? "File moved or deleted"
                  : row.older
                    ? `Older than the edit (revision ${row.revision}, now ${revision})`
                    : `Up to date${row.finishedAt ? ` · ${row.finishedAt}` : ""}`
                : (row.failure ?? row.state);
          return (
            <li key={row.jobId} className="flex items-center gap-2 px-3 py-2 text-label">
              <Icon className={`w-4 h-4 shrink-0 ${tone} ${running ? "animate-spin" : ""}`} />
              <div className="min-w-0 flex-1">
                <div className="truncate text-studio-100" title={row.outputPath}>
                  {row.label}
                  <span className="text-studio-500">
                    {row.width > 0 ? ` · ${row.width}×${row.height}` : ""}
                    {row.outputPath ? ` · ${name(row.outputPath)}` : ""}
                  </span>
                </div>
                <div className={`truncate text-meta ${row.older && row.state === "completed" ? "text-zoom-fg" : "text-studio-500"}`} title={note}>
                  {note}
                </div>
              </div>
              {(waiting || running) && (
                <IconButton
                  icon={XCircle}
                  size="sm"
                  label={waiting ? "Take it out of the queue" : "Stop this export"}
                  onClick={() => run(api.exportCancel(row.jobId))}
                />
              )}
              {row.state === "completed" && row.outputPath && !row.missing && (
                <IconButton
                  icon={FolderOpen}
                  size="sm"
                  label={`Show in ${FILE_MANAGER}`}
                  onClick={() => void api.showInFinder(row.outputPath!)}
                />
              )}
              {!waiting && !running && (
                <>
                  <IconButton
                    icon={RotateCw}
                    size="sm"
                    label={
                      row.request.file
                        ? "Write it again from the edit as it is now (over the old file)"
                        : "Export it again from the edit as it is now (beside the old file; waits if another export runs)"
                    }
                    onClick={() => run(api.exportAgain(projectHandle, row.jobId))}
                  />
                  <IconButton
                    icon={Trash2}
                    size="sm"
                    label="Take it off the list (the file stays)"
                    onClick={() => run(api.exportForget(projectHandle, row.jobId))}
                  />
                </>
              )}
            </li>
          );
        })}
      </ul>
    </div>
  );
};
