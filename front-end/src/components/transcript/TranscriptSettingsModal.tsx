import React, { useEffect, useState } from "react";
import { createPortal } from "react-dom";
import { AlertCircle, Check, Download, KeyRound, X } from "lucide-react";
import { api, isTauriEnvironment } from "../../lib/ipc";
import {
  AiSettings,
  AiSettingsView,
  ModelDownloadProgress,
  TranscriptSettings,
  TranscriptSettingsView,
} from "../../lib/types";

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
  const [aiView, setAiView] = useState<AiSettingsView | null>(null);
  const [aiDraft, setAiDraft] = useState<AiSettings | null>(null);
  const [aiKey, setAiKey] = useState("");

  const loadAi = (next: AiSettingsView) => {
    setAiView(next);
    setAiDraft(next.settings);
  };

  const load = (next: TranscriptSettingsView) => {
    setView(next);
    setDraft(next.settings);
    setKeyterms(next.settings.keyterms.join(", "));
  };

  useEffect(() => {
    void api.transcriptSettingsGet().then(load).catch((err) => setError(errorMessage(err)));
    void api.aiSettingsGet().then(loadAi).catch((err) => setError(errorMessage(err)));
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
      if (aiDraft) {
        let ai = await api.aiSettingsSet(aiDraft);
        if (aiKey.trim()) {
          ai = await api.aiSetApiKey(aiDraft.provider, aiKey.trim());
          setAiKey("");
        }
        loadAi(ai);
      }
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

  const removeAiKey = async () => {
    if (!aiDraft) return;
    setError(null);
    try {
      loadAi(await api.aiSetApiKey(aiDraft.provider, ""));
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

  // Rendered into <body>: inside a dock panel the panel resize bars would sit on top of it.
  return createPortal(
    <div className="fixed inset-0 z-50 bg-black/70 backdrop-blur-sm flex items-center justify-center p-4 select-none">
      <div className="w-full max-w-lg bg-studio-900 border border-studio-700 rounded-2xl shadow-2xl overflow-hidden flex flex-col max-h-[85vh] text-xs">
        <div className="px-6 py-4 border-b border-studio-800 flex items-center justify-between bg-studio-850">
          <div>
            <h3 className="text-sm font-semibold text-white">Transcription and AI settings</h3>
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
                    className="mt-0.5"
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
                  <span className={view.parakeetAvailable ? "text-accent-fg" : "text-suggest-fg"}>{view.parakeetAccelerator}</span>
                </div>
                {!view.parakeetAvailable && (
                  <p className="text-suggest-fg">
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
                    <span className="flex items-center gap-1 text-accent-fg">
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
                      className="ml-auto flex items-center gap-1 px-2 py-1 rounded bg-accent hover:bg-accent disabled:opacity-50 text-white"
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
                      <span className="ml-auto text-accent-fg">
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
                  <button type="button" onClick={() => void removeKey()} className="text-danger-fg hover:text-danger-fg">
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

            {aiDraft && aiView && (
              <div className="space-y-3 pt-4 border-t border-studio-800">
                <div>
                  <span className="font-semibold text-studio-300">AI review (filler words and retakes)</span>
                  <p className="text-studio-500">
                    Sends the transcript text, never audio or video, to the provider when you press Find with AI.
                  </p>
                </div>
                <div className="grid grid-cols-2 gap-1 bg-studio-950/60 border border-studio-800 rounded-lg p-1">
                  {(
                    [
                      ["openAi", "OpenAI"],
                      ["openRouter", "OpenRouter"],
                    ] as const
                  ).map(([value, label]) => (
                    <button
                      key={value}
                      type="button"
                      aria-pressed={aiDraft.provider === value}
                      onClick={() => setAiDraft({ ...aiDraft, provider: value })}
                      className={`py-1.5 rounded-md ${
                        aiDraft.provider === value ? "bg-accent text-white font-semibold" : "text-studio-300 hover:bg-studio-800"
                      }`}
                    >
                      {label}
                    </button>
                  ))}
                </div>
                {(() => {
                  const openAi = aiDraft.provider === "openAi";
                  const source = openAi ? aiView.openaiKeySource : aiView.openrouterKeySource;
                  const envName = openAi ? "OPENAI_API_KEY" : "OPENROUTER_API_KEY";
                  return (
                    <div className="space-y-3 p-3 rounded-lg border border-studio-800 bg-studio-850/60">
                      <label className="block space-y-1">
                        <span className="flex items-center gap-1 text-studio-400">
                          <KeyRound className="w-3 h-3" /> {openAi ? "OpenAI" : "OpenRouter"} API key
                          {source && (
                            <span className="ml-auto text-accent-fg">
                              saved ({source === "environment" ? `from ${envName}` : source})
                            </span>
                          )}
                        </span>
                        <input
                          type="password"
                          autoComplete="off"
                          value={aiKey}
                          placeholder={source ? "Enter a new key to replace it" : `Paste your ${openAi ? "OpenAI" : "OpenRouter"} API key`}
                          onChange={(e) => setAiKey(e.target.value)}
                          className={input}
                        />
                      </label>
                      {source && source !== "environment" && (
                        <button type="button" onClick={() => void removeAiKey()} className="text-danger-fg hover:text-danger-fg">
                          Remove saved key
                        </button>
                      )}
                      <label className="block space-y-1">
                        <span className="text-studio-400">
                          Model (leave empty for {openAi ? aiView.openaiDefaultModel : aiView.openrouterDefaultModel})
                        </span>
                        <input
                          value={openAi ? aiDraft.openaiModel : aiDraft.openrouterModel}
                          placeholder={openAi ? aiView.openaiDefaultModel : aiView.openrouterDefaultModel}
                          onChange={(e) =>
                            setAiDraft(
                              openAi
                                ? { ...aiDraft, openaiModel: e.target.value.trim() }
                                : { ...aiDraft, openrouterModel: e.target.value.trim() },
                            )
                          }
                          className={input}
                        />
                      </label>
                      {!openAi && (
                        <p className="text-studio-500">
                          Any OpenRouter model id works, such as <code>anthropic/claude-sonnet-4.5</code> or{" "}
                          <code>google/gemini-2.5-flash</code>, if it supports JSON output.
                        </p>
                      )}
                    </div>
                  );
                })()}
              </div>
            )}

            {error && (
              <div className="p-3 rounded-lg bg-danger/10 border border-danger/40 flex items-start gap-2 text-danger-fg">
                <AlertCircle className="w-4 h-4 text-danger shrink-0" />
                <span>{error}</span>
              </div>
            )}
          </div>
        )}

        <div className="px-6 py-4 border-t border-studio-800 bg-studio-850 flex items-center justify-end gap-3">
          {saved && <span className="text-accent-fg">Saved</span>}
          <button type="button" onClick={onClose} className="px-4 py-2 rounded-lg text-studio-400 hover:text-white">
            Close
          </button>
          <button
            type="button"
            disabled={!draft}
            onClick={() => void save()}
            className="px-5 py-2 rounded-lg bg-accent hover:bg-accent-hover disabled:opacity-40 text-white font-semibold"
          >
            Save
          </button>
        </div>
      </div>
    </div>,
    document.body,
  );
};
