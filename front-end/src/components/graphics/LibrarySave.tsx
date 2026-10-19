import React, { useEffect, useState } from "react";
import { Check, AlertCircle, Loader2, X } from "lucide-react";
import { Badge, Button } from "../ui";
import { api, isTauriEnvironment } from "../../lib/ipc";
import type { GateReport, GraphicCloseMatch, GraphicLibrarySave } from "../../lib/types";
import { libraryChanged } from "./graphicsEvents";

const fieldClass =
  "w-full min-w-0 bg-studio-950 text-studio-100 text-label rounded px-2 py-1 border border-studio-700 focus:outline-none focus:border-accent-hover";

/// Save to library, through the gate: the app renders the graphic at 16:9 and 9:16, at two
/// lengths, with long text and with each text changed. If it passes it is saved as a template;
/// if not, it says why and offers to save it as is (marked not general). A library item or
/// template much like it is named first, to update instead.
export const LibrarySave: React.FC<{
  projectHandle: string;
  assetId: string;
  name: string;
  onClose: () => void;
  onSaved: (note: string) => void;
}> = ({ projectHandle, assetId, name, onClose, onSaved }) => {
  const [label, setLabel] = useState(name);
  const [useWhen, setUseWhen] = useState("");
  const [busy, setBusy] = useState<string>();
  const [error, setError] = useState<string>();
  const [check, setCheck] = useState<GateReport>();
  const [close, setClose] = useState<GraphicCloseMatch>();
  // What the user picked so far (a close item to update, save as new, save as is), kept for
  // the next try.
  const [choice, setChoice] = useState<Pick<GraphicLibrarySave, "update" | "asNew" | "asIs">>({});
  const running = busy !== undefined;

  useEffect(() => {
    if (!running || !isTauriEnvironment()) return;
    let unlisten: (() => void) | undefined;
    let disposed = false;
    void import("@tauri-apps/api/event").then(({ listen }) =>
      listen<{ fraction: number; message: string }>("graphics-check-progress", (event) =>
        setBusy(event.payload.message),
      ).then((fn) => {
        if (disposed) fn();
        else unlisten = fn;
      }),
    );
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [running]);

  const save = async (extra: Pick<GraphicLibrarySave, "update" | "asNew" | "asIs"> = {}) => {
    const picked = { ...choice, ...extra };
    setChoice(picked);
    setBusy("Checking the graphic");
    setError(undefined);
    setClose(undefined);
    try {
      const outcome = await api.projectGraphicLibrarySave(projectHandle, {
        assetId,
        label: label.trim() || null,
        useWhen: useWhen.trim() || null,
        ...picked,
      });
      setCheck(outcome.check);
      if (outcome.saved) {
        libraryChanged();
        const item = outcome.saved;
        onSaved(
          `Saved "${item.label}" to your library ${item.general ? "as a template" : "as is (not general)"}` +
            `${item.version > 1 ? `, version ${item.version}` : ""} (Media panel, Graphics).`,
        );
      } else if (outcome.close) {
        setClose(outcome.close);
      }
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(undefined);
    }
  };

  const failed = check && !check.passed;
  return (
    <div className="space-y-2 rounded-control border border-studio-700 bg-studio-900 p-2">
      <div className="flex items-center gap-2 text-label font-medium text-studio-100">
        <span className="flex-1">Save to library</span>
        <button type="button" onClick={onClose} className="text-studio-500 hover:text-studio-200" aria-label="Close">
          <X className="w-3.5 h-3.5" />
        </button>
      </div>
      <label className="block text-label text-studio-300 space-y-1">
        <span>Label</span>
        <input aria-label="Library label" value={label} onChange={(e) => setLabel(e.target.value)} className={fieldClass} />
      </label>
      <label className="block text-label text-studio-300 space-y-1">
        <span>Use when</span>
        <input
          aria-label="When to use it"
          value={useWhen}
          placeholder="A new section starts"
          onChange={(e) => setUseWhen(e.target.value)}
          className={fieldClass}
        />
      </label>
      <p className="text-meta text-studio-500 leading-relaxed">
        The app checks it first: it renders it at 16:9 and 9:16, at 3 s and 8 s, with long text and with each text
        changed. A graphic that passes is saved as a template any project can fill with new text.
      </p>
      {busy ? (
        <div className="flex items-center gap-2 text-meta text-studio-300">
          <Loader2 className="w-3.5 h-3.5 animate-spin" />
          <span className="truncate">{busy}</span>
        </div>
      ) : (
        !close &&
        !failed && (
          <Button variant="primary" size="sm" disabled={!label.trim()} onClick={() => void save()}>
            Check and save
          </Button>
        )
      )}
      {close && !busy && (
        <div className="space-y-1.5 rounded border border-suggest/30 bg-suggest/10 p-2 text-meta text-suggest-fg">
          <p>
            {close.kind === "library"
              ? `Your library already has "${close.label}" (version ${close.version ?? 1}): ${close.why}.`
              : `The bundled template "${close.label}" is much like it: ${close.why}.`}
          </p>
          <div className="flex flex-wrap gap-1.5">
            {close.kind === "library" && (
              <Button size="sm" variant="primary" onClick={() => void save({ update: close.id, asNew: false })}>
                Update it (version {(close.version ?? 1) + 1})
              </Button>
            )}
            <Button size="sm" onClick={() => void save({ asNew: true, update: null })}>
              Save as new
            </Button>
          </div>
        </div>
      )}
      {check && <GateChecks report={check} />}
      {failed && !busy && !close && (
        <div className="flex flex-wrap items-center gap-1.5">
          <Button size="sm" onClick={() => void save({ asIs: true })}>
            Save as is
          </Button>
          <span className="text-meta text-studio-500">Kept, but marked as not general.</span>
        </div>
      )}
      {error && (
        <p role="alert" className="text-meta text-danger-fg">
          {error}
        </p>
      )}
    </div>
  );
};

/// The gate's checks, failures first, and its contact sheet.
export const GateChecks: React.FC<{ report: GateReport }> = ({ report }) => {
  const order = { fail: 0, warn: 1, pass: 2 } as const;
  const checks = [...report.checks].sort((a, b) => order[a.status] - order[b.status]);
  return (
    <div className="space-y-1.5">
      <div className="flex items-center gap-2">
        {report.passed ? <Badge tone="success">Passed</Badge> : <Badge tone="danger">Not general yet</Badge>}
        <span className="text-meta text-studio-500">checked in {Math.round(report.seconds)} s</span>
      </div>
      <ul className="space-y-1">
        {checks.map((c) => (
          <li key={c.id} className="flex gap-1.5 text-meta leading-snug">
            {c.status === "pass" ? (
              <Check className="mt-0.5 w-3.5 h-3.5 shrink-0 text-success-fg" />
            ) : (
              <AlertCircle
                className={`mt-0.5 w-3.5 h-3.5 shrink-0 ${c.status === "fail" ? "text-danger-fg" : "text-suggest-fg"}`}
              />
            )}
            <span className={c.status === "pass" ? "text-studio-400" : "text-studio-200"}>
              {c.label}
              {c.status !== "pass" && <span className="text-studio-400"> · {c.reason}</span>}
            </span>
          </li>
        ))}
      </ul>
      {report.sheetUrl && (
        <img
          src={report.sheetUrl}
          alt="Contact sheet: 16:9 with its text and with long text, then 9:16"
          className="w-full rounded border border-studio-800"
        />
      )}
    </div>
  );
};
