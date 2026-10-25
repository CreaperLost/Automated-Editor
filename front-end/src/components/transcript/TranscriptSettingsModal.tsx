import React, { useEffect, useState } from "react";
import { createPortal } from "react-dom";
import { AlertCircle, Check, ChevronDown, ChevronRight, Download, KeyRound, X } from "lucide-react";
import { api, isTauriEnvironment } from "../../lib/ipc";
import { AiSettings, AiSettingsView, ModelDownloadProgress, TranscriptSettings, TranscriptSettingsView } from "../../lib/types";
import { Segmented } from "../ui";
import { useUiMode } from "../../stores/uiModeStore";

function errorMessage(err: unknown): string {
  if (err instanceof Error && err.message) return err.message;
  if (typeof err === "string" && err.trim()) return err;
  return "Something went wrong.";
}

function formatMb(bytes: number): string {
  return bytes >= 1_000_000_000 ? `${(bytes / 1_000_000_000).toFixed(1)} GB` : `${Math.round(bytes / 1_000_000)} MB`;
}

type DownloadKind = "parakeet" | "gpu" | "llm";
type KeyName = "elevenlabs" | "openRouter" | "openAi";

const LANGUAGE_NAMES: Record<string, string> = { en: "English", el: "Greek" };

/// Transcription and AI settings, in two columns with nothing to scroll: on the left what changes
/// from video to video (language, transcriber, review), on the right what is set up once (keys,
/// model downloads). Every change saves as it is made.
export const TranscriptSettingsModal: React.FC<{
  /** The language the open transcript was heard in, shown while the language is on auto. */
  detected?: string | null;
  onClose: () => void;
}> = ({ detected, onClose }) => {
  const [view, setView] = useState<TranscriptSettingsView | null>(null);
  const [aiView, setAiView] = useState<AiSettingsView | null>(null);
  const [keyterms, setKeyterms] = useState("");
  const [download, setDownload] = useState<ModelDownloadProgress | null>(null);
  const [downloading, setDownloading] = useState<DownloadKind | null>(null);
  /** The key being typed, and for which service. */
  const [keyEntry, setKeyEntry] = useState<{ name: KeyName; value: string } | null>(null);
  const [advanced, setAdvanced] = useState(() => useUiMode.getState().advanced);
  const [error, setError] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);

  const load = (next: TranscriptSettingsView) => {
    setView(next);
    setKeyterms(next.settings.keyterms.join(", "));
  };

  useEffect(() => {
    void api.transcriptSettingsGet().then(load).catch((err) => setError(errorMessage(err)));
    void api.aiSettingsGet().then(setAiView).catch((err) => setError(errorMessage(err)));
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

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape" && !keyEntry) onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [keyEntry, onClose]);

  const flashSaved = () => {
    setSaved(true);
    window.setTimeout(() => setSaved(false), 1200);
  };

  /** Saves at once; a refused change leaves the settings as they were. */
  const run = async (work: () => Promise<void>) => {
    setError(null);
    try {
      await work();
      flashSaved();
    } catch (err) {
      setError(errorMessage(err));
    }
  };

  const change = (patch: Partial<TranscriptSettings>) =>
    view && run(async () => load(await api.transcriptSettingsSet({ ...view.settings, ...patch })));

  const changeAi = (patch: Partial<AiSettings>) =>
    aiView && run(async () => setAiView(await api.aiSettingsSet({ ...aiView.settings, ...patch })));

  const saveKey = (name: KeyName, key: string) =>
    run(async () => {
      if (name === "elevenlabs") load(await api.transcriptSetApiKey(key));
      else {
        setAiView(await api.aiSetApiKey(name, key));
        // The OpenRouter key is shared with transcription.
        load(await api.transcriptSettingsGet());
      }
      setKeyEntry(null);
    });

  const downloadModel = async (model: DownloadKind) => {
    setDownloading(model);
    setError(null);
    setDownload(null);
    try {
      load(await api.transcriptDownloadModel(model));
      if (model === "llm") setAiView(await api.aiSettingsGet());
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setDownloading(null);
    }
  };

  if (!view || !aiView) {
    return createPortal(
      <div className="fixed inset-0 z-50 bg-black/70 flex items-center justify-center p-4">
        {error && <p className="text-danger-fg">{error}</p>}
      </div>,
      document.body,
    );
  }

  const settings = view.settings;
  const ai = aiView.settings;
  const local = settings.provider === "parakeet";
  const english = settings.language === "en";
  const parakeetStatus = view.parakeetModels.find((m) => m.model === settings.parakeetModel);
  const heard = detected ? (LANGUAGE_NAMES[detected] ?? detected) : null;
  const input = "w-full bg-studio-800 border border-studio-700 rounded px-2 py-1 text-studio-100";

  /** A one-line row on the right: what it is, its state, and the one button it needs. */
  const row = (label: string, state: React.ReactNode, action?: React.ReactNode) => (
    <div className="h-9 flex items-center gap-2 border-b border-studio-800/70 last:border-b-0">
      <span className="w-28 shrink-0 text-studio-300">{label}</span>
      <span className="flex-1 min-w-0 truncate">{state}</span>
      {action}
    </div>
  );
  const ok = (text: string) => (
    <span className="inline-flex items-center gap-1 text-accent-fg">
      <Check className="w-3 h-3" /> {text}
    </span>
  );
  const missing = (text: string) => <span className="text-studio-500">{text}</span>;
  const smallButton = "shrink-0 flex items-center gap-1 px-2 py-1 rounded text-meta disabled:opacity-50";

  const downloadButton = (model: DownloadKind, bytes: number) => (
    <span className="flex items-center gap-1.5">
      <button
        type="button"
        disabled={downloading !== null}
        onClick={() => void downloadModel(model)}
        className={`${smallButton} bg-accent hover:bg-accent-hover text-white`}
      >
        <Download className="w-3 h-3" />
        {downloading === model
          ? download
            ? `${formatMb(download.bytesDone)}${download.bytesTotal ? ` / ${formatMb(download.bytesTotal)}` : ""}`
            : "Starting"
          : formatMb(bytes)}
      </button>
      {downloading === model && (
        <button type="button" onClick={() => void api.transcriptCancel()} className="text-meta text-studio-400 hover:text-white">
          Cancel
        </button>
      )}
    </span>
  );

  const keyRow = (name: KeyName, label: string, source: string | null) => {
    if (keyEntry?.name === name) {
      return (
        <form
          key={name}
          className="h-9 flex items-center gap-2 border-b border-studio-800/70"
          onSubmit={(e) => {
            e.preventDefault();
            if (keyEntry.value.trim()) void saveKey(name, keyEntry.value.trim());
          }}
        >
          <span className="w-28 shrink-0 text-studio-300">{label}</span>
          <input
            autoFocus
            type="password"
            autoComplete="off"
            aria-label={`${label} API key`}
            placeholder="Paste the key"
            value={keyEntry.value}
            onChange={(e) => setKeyEntry({ name, value: e.target.value })}
            onKeyDown={(e) => e.key === "Escape" && setKeyEntry(null)}
            className={`${input} flex-1 min-w-0`}
          />
          <button type="submit" className={`${smallButton} bg-accent hover:bg-accent-hover text-white`}>
            Save
          </button>
        </form>
      );
    }
    return (
      <React.Fragment key={name}>
        {row(
          label,
          source ? ok(source === "environment" ? "from the environment" : "saved") : missing("not set"),
          <span className="flex items-center gap-1">
            <button
              type="button"
              onClick={() => setKeyEntry({ name, value: "" })}
              className={`${smallButton} border border-studio-700 text-studio-200 hover:bg-studio-800`}
            >
              <KeyRound className="w-3 h-3" />
              {source ? "Replace" : "Set"}
            </button>
            {source && source !== "environment" && (
              <button
                type="button"
                aria-label={`Remove the ${label} key`}
                onClick={() => void saveKey(name, "")}
                className="p-1 rounded text-studio-500 hover:text-danger-fg hover:bg-studio-800"
              >
                <X className="w-3 h-3" />
              </button>
            )}
          </span>,
        )}
      </React.Fragment>
    );
  };

  /** One choice on the left: a label, its buttons, and a line of what it means. */
  const choice = (label: string, control: React.ReactNode, hint?: React.ReactNode) => (
    <div className="grid grid-cols-[6rem_1fr] items-start gap-x-3 gap-y-1">
      <span className="pt-1.5 text-studio-300">{label}</span>
      <div className="space-y-1 min-w-0">
        {control}
        {hint && <p className="text-meta text-studio-500 leading-snug">{hint}</p>}
      </div>
    </div>
  );

  // Rendered into <body>: inside a dock panel the panel resize bars would sit on top of it.
  return createPortal(
    <div
      className="fixed inset-0 z-50 bg-black/70 flex items-center justify-center p-4 select-none"
      onPointerDown={(event) => {
        if (event.target === event.currentTarget) onClose();
      }}
    >
      <div
        role="dialog"
        aria-modal="true"
        aria-label="Transcription and AI"
        className="w-full max-w-4xl bg-studio-900 border border-studio-700 rounded-panel shadow-dialog flex flex-col text-label"
      >
        <div className="px-5 py-3 border-b border-studio-800 flex items-center justify-between bg-studio-850 rounded-t-panel">
          <h3 className="text-heading text-studio-100">Transcription and AI</h3>
          <div className="flex items-center gap-3">
            {saved && <span className="text-meta text-accent-fg">Saved</span>}
            <button type="button" aria-label="Close" onClick={onClose} className="p-1.5 rounded-control text-studio-400 hover:text-white hover:bg-studio-700">
              <X className="w-4 h-4" />
            </button>
          </div>
        </div>

        <div className="grid grid-cols-2 divide-x divide-studio-800">
          {/* Every video */}
          <section className="p-5 space-y-4">
            <h4 className="text-meta font-semibold uppercase tracking-wide text-studio-500">Every video</h4>
            {choice(
              "Language",
              <Segmented
                label="Language spoken"
                className="w-full"
                value={settings.language}
                onChange={(language) =>
                  // Only English runs on this computer: anything else goes online.
                  void change({ language, provider: language !== "en" && local ? "elevenlabs" : settings.provider })
                }
                options={[
                  { value: "auto", label: "Auto", title: "Scribe hears the language (online)" },
                  { value: "en", label: "English" },
                  { value: "el", label: "Ελληνικά" },
                ]}
              />,
              settings.language === "auto"
                ? heard
                  ? `Detected: ${heard}. Fillers and the review follow it.`
                  : "Detected online when transcribing; fillers and the review follow it."
                : "Fillers, retakes and the review follow the language.",
            )}
            {choice(
              "Transcribe",
              english ? (
                <Segmented
                  label="Where to transcribe"
                  className="w-full"
                  value={local ? "local" : "online"}
                  onChange={(where) => void change({ provider: where === "local" ? "parakeet" : "elevenlabs" })}
                  options={[
                    {
                      value: "local",
                      label: "On this computer",
                      title: "NVIDIA Parakeet: free and private, English only",
                      disabled: !view.parakeetAvailable,
                    },
                    { value: "online", label: "Online", title: "ElevenLabs Scribe, or Scribe through OpenRouter" },
                  ]}
                />
              ) : (
                <p className="pt-1.5 text-studio-100">Online</p>
              ),
              local
                ? "Parakeet, free and private."
                : `Scribe: ElevenLabs first, OpenRouter when ElevenLabs has no key or credits.${english ? "" : " Only English runs on this computer."}`,
            )}
            {local &&
              choice(
                "Words",
                <Segmented
                  label="Parakeet model"
                  className="w-full"
                  value={settings.parakeetModel}
                  onChange={(parakeetModel) => void change({ parakeetModel })}
                  options={[
                    { value: "unified", label: "Accurate" },
                    { value: "verbatim", label: "Verbatim" },
                  ]}
                />,
                settings.parakeetModel === "verbatim"
                  ? "Keeps every um, stutter and repeat, so they can be cut."
                  : "Gets names and numbers right more often; may drop stutters.",
              )}
            {choice(
              "Key terms",
              <input
                aria-label="Key terms"
                value={keyterms}
                placeholder="Claude Code, Codex, ..."
                onChange={(e) => setKeyterms(e.target.value)}
                onBlur={() => {
                  const terms = keyterms.split(",").map((t) => t.trim()).filter(Boolean);
                  if (terms.join(",") !== settings.keyterms.join(",")) void change({ keyterms: terms });
                }}
                onKeyDown={(e) => e.key === "Enter" && e.currentTarget.blur()}
                className={input}
              />,
              local
                ? "Names spelled your way, comma separated. Fixes spacing, case and sound-alikes like “cloud code”."
                : "Names spelled your way, comma separated. Scribe listens for them (ElevenLabs adds a small fee per hour).",
            )}
            {choice(
              "Review by",
              <Segmented
                label="Filler and retake review"
                className="w-full"
                value={ai.fillerEngine}
                onChange={(fillerEngine) => void changeAi({ fillerEngine })}
                options={[
                  { value: "cloud", label: ai.provider === "openAi" ? "OpenAI" : "OpenRouter" },
                  { value: "local", label: "Local model", disabled: !aiView.localAvailable },
                ]}
              />,
              ai.fillerEngine === "cloud"
                ? "Recommended: fewer misses and fewer wrong cuts. Sends the words, never the audio."
                : "Qwen3.5 4B on your GPU: free and private, makes more wrong cuts.",
            )}
          </section>

          {/* Set up once */}
          <section className="p-5 space-y-3">
            <h4 className="text-meta font-semibold uppercase tracking-wide text-studio-500">Set up once</h4>
            <div>
              {view.parakeetAvailable &&
                row(
                  "Parakeet",
                  parakeetStatus?.present ? ok("installed") : missing("not downloaded"),
                  !parakeetStatus?.present && downloadButton("parakeet", parakeetStatus?.bytes ?? view.parakeetModelBytes),
                )}
              {view.gpuPackWanted &&
                view.nvidiaDriver &&
                row(
                  "GPU pack",
                  view.gpuPackPresent
                    ? view.gpuProblem
                      ? <span className="text-suggest-fg" title={view.gpuProblem}>not in use: {view.gpuProblem}</span>
                      : ok("CUDA")
                    : missing("CPU until downloaded"),
                  !view.gpuPackPresent && downloadButton("gpu", view.gpuPackBytes),
                )}
              {aiView.localAvailable &&
                row(
                  "Local review",
                  aiView.localPresent ? ok("installed") : missing("not downloaded"),
                  !aiView.localPresent && downloadButton("llm", aiView.localModelBytes),
                )}
              {keyRow("elevenlabs", "ElevenLabs key", view.elevenlabsKeySource)}
              {keyRow("openRouter", "OpenRouter key", aiView.openrouterKeySource)}
              {keyRow("openAi", "OpenAI key", aiView.openaiKeySource)}
              {row(
                "AI agents",
                settings.agentsOnline ? (
                  <span title="Agents read this choice and its date before they transcribe">
                    {ok(`may transcribe online, since ${new Date(settings.agentsOnline).toLocaleDateString()}`)}
                  </span>
                ) : (
                  missing("ask before sending audio online")
                ),
                <button
                  type="button"
                  onClick={() => void change({ agentsOnline: settings.agentsOnline ? null : "on" })}
                  className={`${smallButton} bg-studio-700 hover:bg-studio-600 text-white`}
                  title="Lets connected AI agents send the speech to ElevenLabs (or OpenRouter) to transcribe it, without asking you each time"
                >
                  {settings.agentsOnline ? "Turn off" : "Allow"}
                </button>,
              )}
            </div>

            <button
              type="button"
              onClick={() => setAdvanced(!advanced)}
              aria-expanded={advanced}
              className="flex items-center gap-1 text-meta text-studio-400 hover:text-white"
            >
              {advanced ? <ChevronDown className="w-3 h-3" /> : <ChevronRight className="w-3 h-3" />}
              Model ids and folder
            </button>
            {advanced && (
              <div className="grid grid-cols-[7rem_1fr] items-center gap-x-2 gap-y-1.5">
                <span className="text-studio-400">Cloud review</span>
                <Segmented
                  label="Cloud AI provider"
                  size="sm"
                  value={ai.provider}
                  onChange={(provider) => void changeAi({ provider })}
                  options={[
                    { value: "openRouter", label: "OpenRouter" },
                    { value: "openAi", label: "OpenAI" },
                  ]}
                />
                <span className="text-studio-400">Review model</span>
                <input
                  key={`${ai.provider}-model`}
                  aria-label="Review model"
                  defaultValue={ai.provider === "openAi" ? ai.openaiModel : ai.openrouterModel}
                  placeholder={ai.provider === "openAi" ? aiView.openaiDefaultModel : aiView.openrouterDefaultModel}
                  onBlur={(e) =>
                    void changeAi(
                      ai.provider === "openAi" ? { openaiModel: e.target.value.trim() } : { openrouterModel: e.target.value.trim() },
                    )
                  }
                  className={input}
                />
                <span className="text-studio-400">Scribe model</span>
                <input
                  aria-label="Scribe model"
                  defaultValue={settings.scribeModel}
                  placeholder="scribe_v2"
                  onBlur={(e) => e.target.value.trim() !== settings.scribeModel && void change({ scribeModel: e.target.value.trim() })}
                  className={input}
                />
                <span className="text-studio-400">Parakeet folder</span>
                <input
                  aria-label="Parakeet model folder"
                  defaultValue={settings.parakeetModelDir}
                  placeholder={view.parakeetModelDir}
                  onBlur={(e) =>
                    e.target.value.trim() !== settings.parakeetModelDir && void change({ parakeetModelDir: e.target.value.trim() })
                  }
                  className={input}
                />
              </div>
            )}
          </section>
        </div>

        {error && (
          <div className="mx-5 mb-4 p-2.5 rounded-control bg-danger/10 border border-danger/40 flex items-start gap-2 text-danger-fg">
            <AlertCircle className="w-4 h-4 text-danger shrink-0" />
            <span>{error}</span>
          </div>
        )}
      </div>
    </div>,
    document.body,
  );
};
