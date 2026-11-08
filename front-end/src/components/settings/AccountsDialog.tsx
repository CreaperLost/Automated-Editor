import React, { useEffect, useState } from "react";
import { createPortal } from "react-dom";
import { CheckCircle2, ExternalLink, Loader2, RefreshCw, Upload, X } from "lucide-react";
import { api } from "../../lib/ipc";
import type { PublishAccounts } from "../../lib/types";
import { Button, DialogBackdrop, IconButton } from "../ui";
import { useEscapeKey } from "../../hooks/useEscapeKey";

/**
 * Accounts (PUB-2): uploads go to YouTube through Zernio, using the channel connected in the
 * Zernio dashboard, so AeroEdits needs no Google sign-in. Paste a Zernio key once; the key is
 * kept in the system's keychain.
 */
export const AccountsDialog: React.FC<{ onClose: () => void }> = ({ onClose }) => {
  useEscapeKey(onClose);
  const [accounts, setAccounts] = useState<PublishAccounts | null>(null);
  const [key, setKey] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string>();

  const [tregSource, setTregSource] = useState<string | null>(null);
  const [tregKey, setTregKey] = useState("");
  useEffect(() => {
    void api.tregKeySource().then(setTregSource).catch(() => undefined);
  }, []);
  const saveTreg = async () => {
    setError(undefined);
    try {
      setTregSource(await api.tregSetKey(tregKey.trim()));
      setTregKey("");
    } catch (err) {
      setError(String(err));
    }
  };

  const load = () => {
    setBusy(true);
    void api
      .publishAccounts()
      .then(setAccounts)
      .catch((err) => setError(String(err)))
      .finally(() => setBusy(false));
  };
  useEffect(load, []);

  const saveKey = async () => {
    setBusy(true);
    setError(undefined);
    try {
      setAccounts(await api.publishSetKey(key.trim()));
      setKey("");
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  };

  const pick = async (channel: string) => {
    try {
      await api.publishSetChannel(channel);
      setAccounts((a) => (a ? { ...a, channel } : a));
    } catch (err) {
      setError(String(err));
    }
  };

  const dashboard = accounts?.dashboard ?? "https://zernio.com/dashboard";
  const youtube = accounts?.channels ?? [];

  return createPortal(
    <DialogBackdrop onClose={onClose} selectable>
      <div className="w-[520px] max-w-full rounded-panel border border-studio-700 bg-studio-900 shadow-xl">
        <div className="flex items-center gap-2 px-5 py-4 border-b border-studio-800">
          <Upload className="w-5 h-5 text-studio-300" />
          <h2 className="flex-1 text-body font-semibold text-studio-100">Accounts</h2>
          <IconButton icon={RefreshCw} size="sm" label="Check again" onClick={load} disabled={busy} />
          <IconButton icon={X} size="sm" label="Close" onClick={onClose} />
        </div>
        <div className="px-5 py-4 space-y-4 text-label">
          <p className="text-studio-400">
            Uploads go to YouTube through <b className="text-studio-200">Zernio</b>, with the channel you connect in
            Zernio's dashboard. AeroEdits never signs in to Google.
          </p>

          {busy && !accounts && (
            <p className="flex items-center gap-2 text-studio-400">
              <Loader2 className="w-4 h-4 animate-spin" /> Checking…
            </p>
          )}

          {accounts && (
            <div className="rounded-control border border-studio-800 p-3 space-y-3">
              <div className="flex items-center gap-2">
                <span className="font-medium text-studio-200">Zernio</span>
                {accounts.connected ? (
                  <span className="inline-flex items-center gap-1 text-success-fg">
                    <CheckCircle2 className="w-4 h-4" /> Connected
                  </span>
                ) : (
                  <span className="text-studio-500">{accounts.keySource ? "Key not accepted" : "Not connected"}</span>
                )}
              </div>
              {accounts.error && <p className="text-meta text-danger-fg">{accounts.error}</p>}

              <ol className="list-decimal pl-5 space-y-1 text-meta text-studio-400">
                <li>
                  Make an account and an API key in Zernio.{" "}
                  <button type="button" className="inline-flex items-center gap-1 text-accent-fg hover:underline" onClick={() => void api.openUrl(dashboard)}>
                    Open Zernio <ExternalLink className="w-3 h-3" />
                  </button>
                </li>
                <li>Connect your YouTube channel there.</li>
                <li>Paste the key here.</li>
              </ol>
              <form
                className="flex gap-2"
                onSubmit={(e) => {
                  e.preventDefault();
                  void saveKey();
                }}
              >
                <input
                  type="password"
                  value={key}
                  onChange={(e) => setKey(e.target.value)}
                  placeholder={accounts.keySource ? "Paste a new key to replace it" : "Zernio API key"}
                  autoComplete="off"
                  className="flex-1 min-w-0 h-8 rounded-control border border-studio-700 bg-studio-950 px-2 text-label text-studio-100 placeholder:text-studio-600 focus:outline-none focus:border-accent"
                />
                <Button type="submit" size="sm" variant="primary" disabled={busy || !key.trim()}>
                  Save key
                </Button>
              </form>

              {accounts.connected && (
                <div className="space-y-1">
                  <p className="text-meta text-studio-400">Upload to</p>
                  {youtube.length === 0 ? (
                    <p className="text-meta text-studio-500">
                      No YouTube channel is connected in Zernio yet. Connect one there, then check again.
                    </p>
                  ) : (
                    youtube.map((c) => (
                      <label key={c.id} className="flex items-center gap-2 text-label text-studio-200">
                        <input
                          type="radio"
                          name="channel"
                          checked={accounts.channel === c.id}
                          onChange={() => void pick(c.id)}
                        />
                        {c.name}
                        {!c.active && <span className="text-meta text-studio-500">(inactive)</span>}
                      </label>
                    ))
                  )}
                </div>
              )}
            </div>
          )}
          <div className="rounded-control border border-studio-800 p-3 space-y-2">
            <div className="flex items-center gap-2">
              <span className="font-medium text-studio-200">Treg</span>
              {tregSource ? (
                <span className="inline-flex items-center gap-1 text-success-fg">
                  <CheckCircle2 className="w-4 h-4" /> Key saved{tregSource === "environment" ? " (from TREG_API_KEY)" : ""}
                </span>
              ) : (
                <span className="text-studio-500">No key</span>
              )}
            </div>
            <p className="text-meta text-studio-400">
              Research for faceless videos: X and Reddit in the story search, web search, and reading PDFs and Google News
              links. Paid per answer from your Treg balance, usually a fraction of a cent.{" "}
              <button
                type="button"
                className="inline-flex items-center gap-1 text-accent-fg hover:underline"
                onClick={() => void api.openUrl("https://treg.to/app#orgs/keys")}
              >
                Make a key <ExternalLink className="w-3 h-3" />
              </button>
            </p>
            <form
              className="flex gap-2"
              onSubmit={(e) => {
                e.preventDefault();
                void saveTreg();
              }}
            >
              <input
                type="password"
                value={tregKey}
                onChange={(e) => setTregKey(e.target.value)}
                placeholder={tregSource ? "Paste a new key to replace it" : "Treg key"}
                autoComplete="off"
                className="flex-1 min-w-0 h-8 rounded-control border border-studio-700 bg-studio-950 px-2 text-label text-studio-100 placeholder:text-studio-600 focus:outline-none focus:border-accent"
              />
              <Button type="submit" size="sm" variant="primary" disabled={!tregKey.trim()}>
                Save key
              </Button>
            </form>
          </div>
          {error && <p className="text-meta text-danger-fg">{error}</p>}
        </div>
      </div>
    </DialogBackdrop>,
    document.body,
  );
};
