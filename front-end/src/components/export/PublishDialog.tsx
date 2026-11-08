import React, { useEffect, useState } from "react";
import { createPortal } from "react-dom";
import { ExternalLink, Loader2, RefreshCw, Upload, X } from "lucide-react";
import { api } from "../../lib/ipc";
import { packageSaver } from "../../lib/packageSaver";
import type { PublishAccounts, PublishPackageView, PublishRelease } from "../../lib/types";
import { PackageEditor } from "./PackageEditor";
import { Button, DialogBackdrop, IconButton, Segmented, cn } from "../ui";
import { useEscapeKey } from "../../hooks/useEscapeKey";

const STATUS_TEXT: Record<string, string> = {
  submitting: "Sent, no answer yet: upload again to check (it won't post twice)",
  submitted: "With Zernio",
  scheduled: "Scheduled",
  published: "On YouTube",
  draft: "Draft in Zernio",
  failed: "Failed",
  partial: "Failed",
};

/** The time an hour from now, for the date-time field (local time, minutes). */
function inAnHour() {
  const d = new Date(Date.now() + 3600_000);
  d.setSeconds(0, 0);
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

/**
 * Publish one finished export (PUB-1, PUB-2): its title, description, tags and thumbnail, then
 * an upload to YouTube through Zernio: private, scheduled or a draft. Never public without the
 * user's click. Every call is for `projectHandle`, the project whose export this is; the back end
 * refuses them once another project is open.
 */
export const PublishDialog: React.FC<{
  projectHandle: string;
  target: string;
  label: string;
  file: string;
  durationUs?: number;
  onClose: () => void;
  onOpenAccounts: () => void;
}> = ({ projectHandle, target, label, file, durationUs, onClose, onOpenAccounts }) => {
  useEscapeKey(onClose);
  const [view, setView] = useState<PublishPackageView | null>(null);
  const [accounts, setAccounts] = useState<PublishAccounts | null>(null);
  const [release, setRelease] = useState<PublishRelease["kind"]>("private");
  const [at, setAt] = useState(inAnHour);
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string>();
  const [done, setDone] = useState<string>();

  useEffect(() => {
    void api.publishAccounts().then(setAccounts).catch(() => undefined);
  }, [target]);

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

  const upload = () =>
    run("upload", async () => {
      // The fields are locked from here on. Send the last change and wait for every save to
      // land; a failed save or a package that isn't ready stops the upload (GAP-016).
      const saved = await packageSaver.flush(projectHandle, target);
      setView(saved);
      if (saved.problem) throw new Error(saved.problem);
      const r: PublishRelease =
        release === "scheduled" ? { kind: "scheduled", at: new Date(at).toISOString() } : { kind: release };
      const result = await api.publishUpload(projectHandle, target, file, r);
      setDone(
        result.youtubeId
          ? `Uploaded (${STATUS_TEXT[result.status] ?? result.status}${result.visibility ? `, YouTube says ${result.visibility}` : ""}).`
          : `Sent to Zernio: ${STATUS_TEXT[result.status] ?? result.status}.`,
      );
      setView(await api.publishPackage(projectHandle, target));
    });

  const channel = accounts?.channels.find((c) => c.id === accounts.channel);
  const field = "w-full rounded-control border border-studio-700 bg-studio-950 px-2 text-label text-studio-100 placeholder:text-studio-600 focus:outline-none focus:border-accent";

  return createPortal(
    <DialogBackdrop onClose={onClose} selectable>
      <div className="w-[760px] max-w-full max-h-[92vh] overflow-y-auto rounded-panel border border-studio-700 bg-studio-900 shadow-xl">
        <div className="flex items-center gap-2 px-5 py-4 border-b border-studio-800">
          <Upload className="w-5 h-5 text-studio-300" />
          <h2 className="flex-1 min-w-0 truncate text-body font-semibold text-studio-100">Publish · {label}</h2>
          <IconButton icon={X} size="sm" label="Close" onClick={onClose} />
        </div>

        <div className="px-5 py-4">
          <PackageEditor
            projectHandle={projectHandle}
            target={target}
            file={file}
            durationUs={durationUs}
            onChange={setView}
            refreshKey={done}
            locked={busy === "upload"}
          />
        </div>

        <div className="px-5 py-4 border-t border-studio-800 space-y-3">
          <div className="flex items-center gap-3 text-label">
            <span className="text-studio-400">Upload to</span>
            {accounts?.connected && channel ? (
              <span className="text-studio-100">{channel.name}</span>
            ) : (
              <span className="text-studio-500">{accounts ? "no channel connected" : "checking…"}</span>
            )}
            <Button size="sm" variant="ghost" onClick={onOpenAccounts}>
              Accounts…
            </Button>
          </div>
          <div className="flex flex-wrap items-center gap-3">
            <Segmented
              label="Release"
              value={release}
              onChange={setRelease}
              options={[
                { value: "private", label: "Private", title: "On YouTube now, visible only to you" },
                { value: "scheduled", label: "Scheduled", title: "Public at the time you set" },
                { value: "draft", label: "Draft", title: "Kept in Zernio; nothing reaches YouTube" },
              ]}
            />
            {release === "scheduled" && (
              <input type="datetime-local" value={at} onChange={(e) => setAt(e.target.value)} className={cn(field, "h-8 w-56")} />
            )}
            <Button
              className="ml-auto"
              variant="primary"
              icon={busy === "upload" ? Loader2 : Upload}
              disabled={!!busy || !accounts?.connected || !channel || !view || !!view.problem}
              onClick={() => void upload()}
            >
              {busy === "upload" ? "Uploading…" : release === "scheduled" ? "Schedule" : "Upload"}
            </Button>
          </div>
          {view?.problem && <p className="text-meta text-studio-500">{view.problem}.</p>}
          {error && <p className="text-meta text-danger-fg">{error}</p>}
          {done && <p className="text-meta text-success-fg">{done}</p>}
          {view && view.uploads.length > 0 && (
            <ul className="space-y-1">
              {view.uploads.map((u) => (
                <li key={u.id} className="flex items-center gap-2 text-meta text-studio-400">
                  <span className="text-studio-300">{new Date(u.attemptedAt).toLocaleString()}</span>
                  <span>{u.release.kind}</span>
                  <span className={cn(u.status === "failed" && "text-danger-fg")}>{STATUS_TEXT[u.status] ?? u.status}</span>
                  {u.visibility && <span>· YouTube says {u.visibility}</span>}
                  {u.postId && (
                    <IconButton
                      icon={RefreshCw}
                      size="sm"
                      label="Read the status again"
                      onClick={() =>
                        void run("refresh", async () => {
                          await api.publishRefresh(projectHandle, u.id);
                          setView(await api.publishPackage(projectHandle, target));
                        })
                      }
                    />
                  )}
                  {u.youtubeId && (
                    <button
                      type="button"
                      className="inline-flex items-center gap-1 text-accent-fg hover:underline"
                      onClick={() => void api.openUrl(`https://studio.youtube.com/video/${u.youtubeId}/edit`)}
                    >
                      Studio <ExternalLink className="w-3 h-3" />
                    </button>
                  )}
                </li>
              ))}
            </ul>
          )}
          <p className="text-meta text-studio-500">
            Ads can't be turned on through an upload: check Monetization in YouTube Studio after it arrives.
          </p>
        </div>
      </div>
    </DialogBackdrop>,
    document.body,
  );
};
