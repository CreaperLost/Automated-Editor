import React, { useEffect, useState } from "react";
import { AlertCircle, Check, Download, KeyRound, X } from "lucide-react";
import { api, isTauriEnvironment } from "../../lib/ipc";
import { ModelDownloadProgress, TranscriptSettings, TranscriptSettingsView } from "../../lib/types";

function errorMessage(err: unknown): string {
  if (err instanceof Error && err.message) return err.message;
  if (typeof err === "string" && err.trim()) return err;
  return "Something went wrong.";
}

function formatMb(bytes: number): string {
  return `${Math.round(bytes / 1_000_000)} MB`;
}

export const TranscriptSettingsModal: React.FC<{ onClose: () => void }> = ({ onClose }) => {
  const [view, setView] = useState<TranscriptSettingsView | null>(null);
  const [draft, setDraft] = useState<TranscriptSettings | null>(null);
  const [keyterms, setKeyterms] = useState("");
  const [apiKey, setApiKey] = useState("");
  const [download, setDownload] = useState<ModelDownloadProgress | null>(null);
  const [downloading, setDownloading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);

  const load = (next: TranscriptSettingsView) => {
    setView(next);
    setDraft(next.settings);
    setKeyterms(next.settings.keyterms.join(", "));
  };

  useEffect(() => {
    void api.transcriptSettingsGet().then(load).catch((err) => setError(errorMessage(err)));
  }, []);

  useEffect(() => {
    if (!isTauriEnvironment()) return;
    let unlisten: (() => void) | undefined;
    let disposed = false;
    void import("@tauri-apps/api/event").then(({ listen }) =>
      listen<ModelDownloadProgress>("transcript-model-progress", (event) => setDownload(event.payload)).then((fn) => {
        if (disposed) fn();
        else unlisten = fn;
      }),
    );
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  const save = async () => {
    if (!draft) return;
    setError(null);
    try {
      const terms = keyterms
        .split(",")
        .map((t) => t.trim())
        .filter(Boolean);
      let next = await api.transcriptSettingsSet({ ...draft, keyterms: terms });
      if (apiKey.trim()) {
        next = await api.transcriptSetApiKey(apiKey.trim());
        setApiKey("");
      }
      load(next);
      setSaved(true);
      window.setTimeout(() => setSaved(false), 1500);
    } catch (err) {
      setError(errorMessage(err));
    }
  };

  const removeKey = async () => {
    setError(null);
    try {
      load(await api.transcriptSetApiKey(""));
    } catch (err) {
      setError(errorMessage(err));
    }
  };

  const downloadModel = async () => {
    setDownloading(true);
    setError(null);
    setDownload(null);
    try {
      if (draft) await api.transcriptSettingsSet(draft);
      load(await api.transcriptDownloadModel());
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setDownloading(false);
    }
  };

  const input = "w-full bg-studio-800 border border-studio-700 rounded px-2 py-1 text-studio-100";

  return (
    <div className="fixed inset-0 z-50 bg-black/70 backdrop-blur-sm flex items-center justify-center p-4 select-none">
      <div className="w-full max-w-lg bg-studio-900 border border-studio-700 rounded-2xl shadow-2xl overflow-hidden flex flex-col max-h-[85vh] text-xs">
        <div className="px-6 py-4 border-b border-studio-800 flex items-center justify-between bg-studio-850">
          <div>
            <h3 className="text-sm font-semibold text-white">Transcription settings</h3>
            <p className="text-studio-400">Used for every project on this computer.</p>
          </div>
          <button type="button" onClick={onClose} className="p-1.5 rounded-lg text-studio-400 hover:text-white hover:bg-studio-700">
            <X className="w-4 h-4" />
          </button>
        </div>

        {draft && view && (
          <div className="p-6 space-y-5 overflow-y-auto">
            <div className="space-y-2">
              <span className="font-semibold text-studio-300">Provider</span>
              {(
                [
                  ["parakeet", "Parakeet (local)", "NVIDIA Parakeet TDT 0.6B on this computer. Free and private."],
                  ["elevenlabs", "ElevenLabs Scribe (cloud)", "Uploads the audio to ElevenLabs. Needs an API key."],
                ] as const
              ).map(([value, label, hint]) => (
                <label key={value} className="flex items-start gap-2 p-2 rounded-lg border border-studio-800 bg-studio-850/60 cursor-pointer">
                  <input
                    type="radio"
                    name="provider"
                    checked={draft.provider === value}
                    onChange={() => setDraft({ ...draft, provider: value })}
                    className="mt-0.5 accent-teal-500"
                  />
                  <span>
                    <span className="block text-studio-100">{label}</span>
                    <span className="text-studio-500">{hint}</span>
                  </span>
                </label>
              ))}
            </div>

            <label className="block space-y-1">
              <span className="text-studio-400">Language code (leave empty to detect)</span>
              <input value={draft.language} onChange={(e) => setDraft({ ...draft, language: e.target.value })} className={input} />
            </label>

            {draft.provider === "parakeet" ? (
              <div className="space-y-2 p-3 rounded-lg border border-studio-800 bg-studio-850/60">
                <div className="flex justify-between">
                  <span className="text-studio-400">Accelerator</span>
                  <span className={view.parakeetAvailable ? "text-teal-300" : "text-amber-300"}>{view.parakeetAccelerator}</span>
                </div>
                {!view.parakeetAvailable && (
                  <p className="text-amber-300">
                    This build has no local transcription. Rebuild with <code>--features tauri-app,parakeet-cuda</code> (or
                    <code> parakeet-directml</code>), or use ElevenLabs.
                  </p>
                )}
                <label className="block space-y-1">
                  <span className="text-studio-400">Model folder (leave empty for the default)</span>
                  <input
                    value={draft.parakeetModelDir}
                    placeholder={view.parakeetModelDir}
                    onChange={(e) => setDraft({ ...draft, parakeetModelDir: e.target.value })}
                    className={input}
                  />
                </label>
                <div className="flex items-center gap-2">
                  {view.parakeetModelPresent ? (
                    <span className="flex items-center gap-1 text-teal-300">
                      <Check className="w-3 h-3" /> Model installed
                    </span>
                  ) : (
                    <span className="text-studio-400">Model not downloaded (about 670 MB).</span>
                  )}
                  {!view.parakeetModelPresent && (
                    <button
                      type="button"
                      disabled={downloading}
                      onClick={() => void downloadModel()}
                      className="ml-auto flex items-center gap-1 px-2 py-1 rounded bg-teal-700 hover:bg-teal-600 disabled:opacity-50 text-white"
                    >
                      <Download className="w-3 h-3" />
                      {downloading
                        ? download
                          ? `${formatMb(download.bytesDone)}${download.bytesTotal ? ` of ${formatMb(download.bytesTotal)}` : ""}`
                          : "Starting"
                        : "Download"}
                    </button>
                  )}
                  {downloading && (
                    <button type="button" onClick={() => void api.transcriptCancel()} className="text-studio-400 hover:text-white">
                      Cancel
                    </button>
                  )}
                </div>
              </div>
            ) : (
              <div className="space-y-3 p-3 rounded-lg border border-studio-800 bg-studio-850/60">
                <label className="block space-y-1">
                  <span className="flex items-center gap-1 text-studio-400">
                    <KeyRound className="w-3 h-3" /> API key
                    {view.elevenlabsKeySource && (
                      <span className="ml-auto text-teal-300">
                        saved ({view.elevenlabsKeySource === "environment" ? "from ELEVENLABS_API_KEY" : view.elevenlabsKeySource})
                      </span>
                    )}
                  </span>
                  <input
                    type="password"
                    autoComplete="off"
                    value={apiKey}
                    placeholder={view.elevenlabsKeySource ? "Enter a new key to replace it" : "Paste your ElevenLabs API key"}
                    onChange={(e) => setApiKey(e.target.value)}
                    className={input}
                  />
                </label>
                {view.elevenlabsKeySource && view.elevenlabsKeySource !== "environment" && (
                  <button type="button" onClick={() => void removeKey()} className="text-rose-300 hover:text-rose-200">
                    Remove saved key
                  </button>
                )}
                <label className="block space-y-1">
                  <span className="text-studio-400">Model (leave empty for scribe_v2)</span>
                  <input value={draft.scribeModel} onChange={(e) => setDraft({ ...draft, scribeModel: e.target.value })} className={input} />
                </label>
                <label className="block space-y-1">
                  <span className="text-studio-400">Key terms, comma separated (names it should spell right)</span>
                  <input value={keyterms} onChange={(e) => setKeyterms(e.target.value)} className={input} />
                </label>
              </div>
            )}

            {error && (
              <div className="p-3 rounded-lg bg-rose-950/40 border border-rose-600/40 flex items-start gap-2 text-rose-200">
                <AlertCircle className="w-4 h-4 text-rose-400 shrink-0" />
                <span>{error}</span>
              </div>
            )}
          </div>
        )}

        <div className="px-6 py-4 border-t border-studio-800 bg-studio-850 flex items-center justify-end gap-3">
          {saved && <span className="text-teal-300">Saved</span>}
          <button type="button" onClick={onClose} className="px-4 py-2 rounded-lg text-studio-400 hover:text-white">
            Close
          </button>
          <button
            type="button"
            disabled={!draft}
            onClick={() => void save()}
            className="px-5 py-2 rounded-lg bg-teal-600 hover:bg-teal-500 disabled:opacity-40 text-white font-semibold"
          >
            Save
          </button>
        </div>
      </div>
    </div>
  );
};
