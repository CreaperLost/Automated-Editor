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
  return bytes >= 1_000_000_000 ? `${(bytes / 1_000_000_000).toFixed(1)} GB` : `${Math.round(bytes / 1_000_000)} MB`;
}

type DownloadKind = "parakeet" | "aligner" | "gpu" | "llm";

/** Where the local models run: the GPU once the NVIDIA pack is in, else the build's own. */
function acceleratorLabel(view: TranscriptSettingsView): string {
  if (!view.gpuPackWanted) return view.parakeetAccelerator;
  if (!view.nvidiaDriver) return "CPU (no NVIDIA GPU found)";
  return view.gpuPackPresent ? "CUDA (NVIDIA GPU)" : "CPU until the GPU pack is downloaded";
}

export const TranscriptSettingsModal: React.FC<{ onClose: () => void }> = ({ onClose }) => {
  const [view, setView] = useState<TranscriptSettingsView | null>(null);
  const [draft, setDraft] = useState<TranscriptSettings | null>(null);
  const [keyterms, setKeyterms] = useState("");
  const [apiKey, setApiKey] = useState("");
  const [download, setDownload] = useState<ModelDownloadProgress | null>(null);
  /** Which model is downloading, if any. */
  const [downloading, setDownloading] = useState<DownloadKind | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);
  const [aiView, setAiView] = useState<AiSettingsView | null>(null);
  const [aiDraft, setAiDraft] = useState<AiSettings | null>(null);
  const [aiKey, setAiKey] = useState("");
  /** The OpenRouter key typed in the transcription section. */
  const [routerKey, setRouterKey] = useState("");

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
      if (aiDraft) {
        let ai = await api.aiSettingsSet(aiDraft);
        if (aiKey.trim()) {
          ai = await api.aiSetApiKey(aiDraft.provider, aiKey.trim());
          setAiKey("");
        }
        if (routerKey.trim()) {
          ai = await api.aiSetApiKey("openRouter", routerKey.trim());
          setRouterKey("");
        }
        loadAi(ai);
        // The OpenRouter key is shared, so the transcription view reads it again.
        next = await api.transcriptSettingsGet();
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

  const removeRouterKey = async () => {
    setError(null);
    try {
      loadAi(await api.aiSetApiKey("openRouter", ""));
      load(await api.transcriptSettingsGet());
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

  const downloadModel = async (model: DownloadKind = "parakeet") => {
    setDownloading(model);
    setError(null);
    setDownload(null);
    try {
      if (draft) await api.transcriptSettingsSet(draft);
      load(await api.transcriptDownloadModel(model));
      // The local model shows in the AI section.
      if (model === "llm") loadAi(await api.aiSettingsGet());
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setDownloading(null);
    }
  };

  const downloadLabel = () =>
    download ? `${formatMb(download.bytesDone)}${download.bytesTotal ? ` of ${formatMb(download.bytesTotal)}` : ""}` : "Starting";

  /** A download's state: installed, or what it is and a Download button. */
  const downloadRow = (model: DownloadKind, present: boolean, presentLabel: string, missingLabel: string) => (
    <div className="flex items-center gap-2">
      {present ? (
        <span className="flex items-center gap-1 text-accent-fg">
          <Check className="w-3 h-3" /> {presentLabel}
        </span>
      ) : (
        <span className="text-studio-400">{missingLabel}</span>
      )}
      {!present && (
        <button
          type="button"
          disabled={downloading !== null}
          onClick={() => void downloadModel(model)}
          className="ml-auto shrink-0 flex items-center gap-1 px-2 py-1 rounded bg-accent hover:bg-accent-hover disabled:opacity-50 text-white"
        >
          <Download className="w-3 h-3" />
          {downloading === model ? downloadLabel() : "Download"}
        </button>
      )}
      {downloading === model && (
        <button type="button" onClick={() => void api.transcriptCancel()} className="text-studio-400 hover:text-white">
          Cancel
        </button>
      )}
    </div>
  );

  const input = "w-full bg-studio-800 border border-studio-700 rounded px-2 py-1 text-studio-100";

  // Rendered into <body>: inside a dock panel the panel resize bars would sit on top of it.
  return createPortal(
    <div className="fixed inset-0 z-50 bg-black/70 flex items-center justify-center p-4 select-none">
      <div className="w-full max-w-lg bg-studio-900 border border-studio-700 rounded-panel shadow-dialog overflow-hidden flex flex-col max-h-[85vh] text-label">
        <div className="px-6 py-4 border-b border-studio-800 flex items-center justify-between bg-studio-850">
          <div>
            <h3 className="text-heading text-studio-100">Transcription and AI settings</h3>
            <p className="text-studio-400">Used for every project on this computer.</p>
          </div>
          <button type="button" onClick={onClose} className="p-1.5 rounded-control text-studio-400 hover:text-white hover:bg-studio-700">
            <X className="w-4 h-4" />
          </button>
        </div>

        {draft && view && (
          <div className="p-6 space-y-5 overflow-y-auto">
            <div className="space-y-2">
              <span className="font-semibold text-studio-300">Language spoken</span>
              <div className="grid grid-cols-2 gap-1 bg-studio-950/60 border border-studio-800 rounded-control p-1">
                {(
                  [
                    ["en", "English"],
                    ["el", "Ελληνικά (Greek)"],
                  ] as const
                ).map(([value, label]) => (
                  <button
                    key={value}
                    type="button"
                    aria-pressed={draft.language === value}
                    onClick={() =>
                      setDraft({
                        ...draft,
                        language: value,
                        // Greek has no good local model: move off Parakeet to an online provider.
                        provider:
                          value === "el" && draft.provider === "parakeet"
                            ? view.elevenlabsKeySource && !view.openrouterKeySource
                              ? "elevenlabs"
                              : "openrouter"
                            : draft.provider,
                      })
                    }
                    className={`py-1.5 rounded-md ${
                      draft.language === value ? "bg-accent text-white font-semibold" : "text-studio-300 hover:bg-studio-800"
                    }`}
                  >
                    {label}
                  </button>
                ))}
              </div>
              <p className="text-studio-500">
                {draft.language === "el"
                  ? "Greek is transcribed online with Scribe. Filler words, retakes and the AI review follow the language; word alignment is English only."
                  : "Filler words, retakes and the AI review follow the language."}
              </p>
            </div>

            <div className="space-y-2">
              <span className="font-semibold text-studio-300">Provider</span>
              {(
                [
                  [
                    "parakeet",
                    "Parakeet (local)",
                    draft.language === "el"
                      ? "English only: no local model transcribes Greek well enough."
                      : "NVIDIA Parakeet 0.6B on this computer. Free and private.",
                  ],
                  ["elevenlabs", "ElevenLabs Scribe (cloud)", "Uploads the audio to ElevenLabs. Needs an API key."],
                  [
                    "openrouter",
                    "Scribe via OpenRouter (cloud)",
                    "The same Scribe v2, uploaded through OpenRouter and billed to your OpenRouter key (about $0.11 an hour).",
                  ],
                ] as const
              ).map(([value, label, hint]) => (
                <label
                  key={value}
                  className={`flex items-start gap-2 p-2 rounded-control border border-studio-800 bg-studio-850/60 ${
                    value === "parakeet" && draft.language === "el" ? "opacity-50 cursor-not-allowed" : "cursor-pointer"
                  }`}
                >
                  <input
                    type="radio"
                    name="provider"
                    checked={draft.provider === value}
                    disabled={value === "parakeet" && draft.language === "el"}
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

            {draft.provider === "parakeet" ? (
              <div className="space-y-2 p-3 rounded-control border border-studio-800 bg-studio-850/60">
                <div className="flex justify-between">
                  <span className="text-studio-400">Accelerator</span>
                  <span className={view.parakeetAvailable ? "text-accent-fg" : "text-suggest-fg"}>{acceleratorLabel(view)}</span>
                </div>
                {view.gpuPackWanted &&
                  view.nvidiaDriver &&
                  downloadRow(
                    "gpu",
                    view.gpuPackPresent,
                    "NVIDIA GPU pack installed (CUDA 13, cuDNN 9)",
                    `Runs Parakeet and word alignment on your NVIDIA GPU, several times faster. NVIDIA GPU pack not downloaded (about ${formatMb(view.gpuPackBytes)}).`,
                  )}
                {view.gpuProblem && view.gpuPackPresent && (
                  <p className="text-suggest-fg">The GPU is not in use: {view.gpuProblem}.</p>
                )}
                {!view.parakeetAvailable && (
                  <p className="text-suggest-fg">
                    This build has no local transcription. Rebuild with <code>--features tauri-app,parakeet-cuda</code> (or
                    <code> parakeet-directml</code>), or use ElevenLabs.
                  </p>
                )}
                <div className="space-y-1">
                  <span className="text-studio-400">Model</span>
                  <div className="grid grid-cols-2 gap-1 bg-studio-950/60 border border-studio-800 rounded-control p-1">
                    {(
                      [
                        ["unified", "Accurate words"],
                        ["verbatim", "Verbatim"],
                      ] as const
                    ).map(([value, label]) => (
                      <button
                        key={value}
                        type="button"
                        aria-pressed={draft.parakeetModel === value}
                        onClick={() => setDraft({ ...draft, parakeetModel: value })}
                        className={`py-1.5 rounded-md ${
                          draft.parakeetModel === value ? "bg-accent text-white font-semibold" : "text-studio-300 hover:bg-studio-800"
                        }`}
                      >
                        {label}
                      </button>
                    ))}
                  </div>
                  <p className="text-studio-500">
                    {draft.parakeetModel === "verbatim"
                      ? "Parakeet TDT v3 writes down every um, stutter and repeated word, so filler and retake removal can cut them. A little less accurate on names and numbers."
                      : "Parakeet Unified gets names and numbers right more often, but tidies speech: it can drop stutters, repeats and some ums before they can be cut."}
                  </p>
                </div>
                <label className="block space-y-1">
                  <span className="text-studio-400">Model folder (leave empty for the default)</span>
                  <input
                    value={draft.parakeetModelDir}
                    placeholder={view.parakeetModelDir}
                    onChange={(e) => setDraft({ ...draft, parakeetModelDir: e.target.value })}
                    className={input}
                  />
                </label>
                {(() => {
                  const status = view.parakeetModels.find((m) => m.model === draft.parakeetModel);
                  return downloadRow(
                    "parakeet",
                    status?.present ?? false,
                    "Model installed",
                    `Model not downloaded (about ${formatMb(status?.bytes ?? view.parakeetModelBytes)}).`,
                  );
                })()}
              </div>
            ) : draft.provider === "openrouter" ? (
              <div className="space-y-3 p-3 rounded-control border border-studio-800 bg-studio-850/60">
                <label className="block space-y-1">
                  <span className="flex items-center gap-1 text-studio-400">
                    <KeyRound className="w-3 h-3" /> OpenRouter API key
                    {view.openrouterKeySource && (
                      <span className="ml-auto text-accent-fg">
                        saved ({view.openrouterKeySource === "environment" ? "from OPENROUTER_API_KEY" : view.openrouterKeySource})
                      </span>
                    )}
                  </span>
                  <input
                    type="password"
                    autoComplete="off"
                    value={routerKey}
                    placeholder={view.openrouterKeySource ? "Enter a new key to replace it" : "Paste your OpenRouter API key"}
                    onChange={(e) => setRouterKey(e.target.value)}
                    className={input}
                  />
                  <span className="text-studio-500">The same key OpenRouter review uses below.</span>
                </label>
                {view.openrouterKeySource && view.openrouterKeySource !== "environment" && (
                  <button type="button" onClick={() => void removeRouterKey()} className="text-danger-fg hover:text-danger-fg">
                    Remove saved key
                  </button>
                )}
                <label className="block space-y-1">
                  <span className="text-studio-400">Key terms, comma separated (names it should spell right)</span>
                  <input value={keyterms} onChange={(e) => setKeyterms(e.target.value)} className={input} />
                </label>
              </div>
            ) : (
              <div className="space-y-3 p-3 rounded-control border border-studio-800 bg-studio-850/60">
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

            <div className="space-y-2 p-3 rounded-control border border-studio-800 bg-studio-850/60">
              <label className="flex items-start gap-2 cursor-pointer">
                <input
                  type="checkbox"
                  checked={draft.alignWords}
                  disabled={!view.alignerAvailable}
                  onChange={(e) => setDraft({ ...draft, alignWords: e.target.checked })}
                  className="mt-0.5"
                />
                <span>
                  <span className="block text-studio-100">Align words to the audio</span>
                  <span className="text-studio-500">
                    After transcribing, times every word against the audio (to 20 ms) with a small local model, so cuts at
                    word edges keep whole words and drop the pauses beside them. Works with either provider; English only.
                  </span>
                </span>
              </label>
              {!view.alignerAvailable ? (
                <p className="text-suggest-fg">This build has no local models; word alignment needs a Parakeet build.</p>
              ) : (
                <div className="flex items-center gap-2">
                  {view.alignerModelPresent ? (
                    <span className="flex items-center gap-1 text-accent-fg">
                      <Check className="w-3 h-3" /> Alignment model installed
                    </span>
                  ) : (
                    <span className="text-studio-400">
                      Alignment model not downloaded (about {formatMb(view.alignerModelBytes)}).
                    </span>
                  )}
                  {!view.alignerModelPresent && (
                    <button
                      type="button"
                      disabled={downloading !== null}
                      onClick={() => void downloadModel("aligner")}
                      className="ml-auto flex items-center gap-1 px-2 py-1 rounded bg-accent hover:bg-accent-hover disabled:opacity-50 text-white"
                    >
                      <Download className="w-3 h-3" />
                      {downloading === "aligner" ? downloadLabel() : "Download"}
                    </button>
                  )}
                  {downloading === "aligner" && (
                    <button type="button" onClick={() => void api.transcriptCancel()} className="text-studio-400 hover:text-white">
                      Cancel
                    </button>
                  )}
                </div>
              )}
            </div>

            {aiDraft && aiView && (
              <div className="space-y-3 pt-4 border-t border-studio-800">
                <div>
                  <span className="font-semibold text-studio-300">Filler and retake review</span>
                  <p className="text-studio-500">
                    Who checks the words the rules flag, in English or Greek, when you press Find with AI.
                  </p>
                </div>
                <div className="grid grid-cols-2 gap-1 bg-studio-950/60 border border-studio-800 rounded-control p-1">
                  {(
                    [
                      ["cloud", "OpenRouter / OpenAI"],
                      ["local", "Local model"],
                    ] as const
                  ).map(([value, label]) => (
                    <button
                      key={value}
                      type="button"
                      aria-pressed={aiDraft.fillerEngine === value}
                      onClick={() => setAiDraft({ ...aiDraft, fillerEngine: value })}
                      className={`py-1.5 rounded-md ${
                        aiDraft.fillerEngine === value ? "bg-accent text-white font-semibold" : "text-studio-300 hover:bg-studio-800"
                      }`}
                    >
                      {label}
                    </button>
                  ))}
                </div>
                {aiDraft.fillerEngine === "cloud" && (
                  <p className="text-studio-500">
                    Recommended: it misses fewer fillers and retakes and makes fewer wrong cuts. Until a key is set
                    below, the local model reviews them when it is downloaded.
                  </p>
                )}
                {aiDraft.fillerEngine === "local" && (
                  <div className="space-y-2 p-3 rounded-control border border-studio-800 bg-studio-850/60">
                    <p className="text-studio-500">
                      Qwen3.5 4B runs inside the app on your GPU: free, private, no key. It reads only the windows around
                      what the rules flagged.
                    </p>
                    {aiView.localAvailable ? (
                      downloadRow(
                        "llm",
                        aiView.localPresent,
                        "Local model installed",
                        `Local model not downloaded (about ${formatMb(aiView.localModelBytes)}).`,
                      )
                    ) : (
                      <p className="text-suggest-fg">This build has no local model. Rebuild with the local-llm feature.</p>
                    )}
                  </div>
                )}
              </div>
            )}

            {aiDraft && aiView && (
              <div className="space-y-3 pt-4 border-t border-studio-800">
                <div>
                  <span className="font-semibold text-studio-300">OpenRouter review (filler words and retakes, chapters, shorts)</span>
                  <p className="text-studio-500">
                    Optional. Sends the transcript text, never audio or video, to the provider when you press Find with
                    OpenRouter. A connected agent reviews fillers and retakes in-house instead, with no key.
                  </p>
                </div>
                <div className="grid grid-cols-2 gap-1 bg-studio-950/60 border border-studio-800 rounded-control p-1">
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
                    <div className="space-y-3 p-3 rounded-control border border-studio-800 bg-studio-850/60">
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
              <div className="p-3 rounded-control bg-danger/10 border border-danger/40 flex items-start gap-2 text-danger-fg">
                <AlertCircle className="w-4 h-4 text-danger shrink-0" />
                <span>{error}</span>
              </div>
            )}
          </div>
        )}

        <div className="px-6 py-4 border-t border-studio-800 bg-studio-850 flex items-center justify-end gap-3">
          {saved && <span className="text-accent-fg">Saved</span>}
          <button type="button" onClick={onClose} className="px-4 py-2 rounded-control text-studio-400 hover:text-white">
            Close
          </button>
          <button
            type="button"
            disabled={!draft}
            onClick={() => void save()}
            className="px-5 py-2 rounded-control bg-accent hover:bg-accent-hover disabled:opacity-40 text-white font-semibold"
          >
            Save
          </button>
        </div>
      </div>
    </div>,
    document.body,
  );
};
