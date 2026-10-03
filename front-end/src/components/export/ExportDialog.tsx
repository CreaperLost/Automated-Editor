import React, { useEffect, useState } from "react";
import { Download, FolderOpen, X, XCircle } from "lucide-react";
import { api } from "../../lib/ipc";
import type { ExportQuality, ExportSettings, ExportStatus } from "../../lib/types";

/** Base 16:9 sizes; the backend fits them to the canvas aspect ratio. */
const RESOLUTIONS = [
  { label: "720p", width: 1280, height: 720 },
  { label: "1080p", width: 1920, height: 1080 },
  { label: "1440p", width: 2560, height: 1440 },
  { label: "4K", width: 3840, height: 2160 },
] as const;
const FRAME_RATES = [24, 25, 30, 60] as const;
const QUALITIES: { key: ExportQuality | "custom"; label: string; hint: string }[] = [
  { key: "standard", label: "Standard", hint: "Smaller files for drafts and quick shares." },
  { key: "high", label: "High", hint: "Sharp screen text at a sensible size. Recommended." },
  { key: "max", label: "Max", hint: "Near lossless. Large files." },
  { key: "custom", label: "Bitrate", hint: "Set the average video bitrate yourself." },
];
const MIN_MBPS = 1;
const MAX_MBPS = 200;
const STORAGE_KEY = "aeroedits.exportPreferences";

interface ExportPreferences {
  resolution: string;
  fps: number;
  quality: ExportQuality | "custom";
  mbps: number;
}

const DEFAULT_PREFERENCES: ExportPreferences = { resolution: "1080p", fps: 30, quality: "high", mbps: 16 };

function loadPreferences(): ExportPreferences {
  try {
    const stored = JSON.parse(window.localStorage.getItem(STORAGE_KEY) ?? "null");
    if (stored && typeof stored === "object") return { ...DEFAULT_PREFERENCES, ...stored };
  } catch {
    // Storage can be unavailable; the defaults still work.
  }
  return DEFAULT_PREFERENCES;
}

function savePreferences(preferences: ExportPreferences) {
  try {
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify(preferences));
  } catch {
    // Not remembering the choice is harmless.
  }
}

/** Mirrors `fit_export_size` in src-tauri/src/project/layout.rs for the size label. */
function fittedSize(width: number, height: number, aspect: string): [number, number] {
  const even = (value: number) => value & ~1;
  switch (aspect) {
    case "9:16":
      return [even(height), even(Math.floor((height * 16) / 9))];
    case "4:3":
      return [even(Math.floor((height * 4) / 3)), even(height)];
    case "1:1":
      return [even(height), even(height)];
    default:
      return [width, height];
  }
}

function formatBytes(bytes: number): string {
  if (bytes >= 1e9) return `${(bytes / 1e9).toFixed(1)} GB`;
  return `${Math.max(1, Math.round(bytes / 1e6))} MB`;
}

interface ExportDialogProps {
  aspectRatio: string;
  editedDurationUs: number;
  destination: string;
  exportJob?: ExportStatus;
  onChooseDestination: () => void;
  onStart: (settings: ExportSettings) => void;
  onCancel: () => void;
  onClose: () => void;
}

export const ExportDialog: React.FC<ExportDialogProps> = ({
  aspectRatio,
  editedDurationUs,
  destination,
  exportJob,
  onChooseDestination,
  onStart,
  onCancel,
  onClose,
}) => {
  const [preferences, setPreferences] = useState(loadPreferences);
  const update = (patch: Partial<ExportPreferences>) =>
    setPreferences((current) => {
      const next = { ...current, ...patch };
      savePreferences(next);
      return next;
    });

  const exporting = exportJob?.state === "queued" || exportJob?.state === "running";
  const resolution = RESOLUTIONS.find((r) => r.label === preferences.resolution) ?? RESOLUTIONS[1];
  const custom = preferences.quality === "custom";
  const mbpsValid = Number.isFinite(preferences.mbps) && preferences.mbps >= MIN_MBPS && preferences.mbps <= MAX_MBPS;
  const progress =
    exportJob && exportJob.progressDenominator > 0
      ? Math.min(1, exportJob.progressNumerator / exportJob.progressDenominator)
      : 0;

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [onClose]);

  const start = () => {
    if (custom && !mbpsValid) return;
    onStart({
      videoCodec: "h264",
      audioCodec: "aac",
      width: resolution.width,
      height: resolution.height,
      fps: preferences.fps,
      quality: custom ? "high" : (preferences.quality as ExportQuality),
      bitrateKbps: custom ? Math.round(preferences.mbps * 1000) : null,
    });
  };

  const option = (active: boolean) =>
    `py-1.5 rounded-md text-xs transition-colors disabled:opacity-40 ${
      active ? "bg-accent text-white font-semibold" : "text-studio-300 hover:text-white hover:bg-studio-800"
    }`;
  const seconds = editedDurationUs / 1e6;

  return (
    <div
      className="fixed inset-0 z-50 bg-black/70 backdrop-blur-sm flex items-center justify-center p-4 select-none"
      onPointerDown={(event) => {
        if (event.target === event.currentTarget) onClose();
      }}
    >
      <div
        role="dialog"
        aria-modal="true"
        aria-labelledby="export-dialog-title"
        className="w-full max-w-lg bg-studio-900 border border-studio-700 rounded-2xl shadow-2xl overflow-hidden flex flex-col max-h-[85vh] text-xs"
      >
        <div className="px-6 py-4 border-b border-studio-800 flex items-center justify-between bg-studio-850">
          <div>
            <h3 id="export-dialog-title" className="text-sm font-semibold text-white">Export video</h3>
            <p className="text-studio-400">MP4, H.264 video and AAC audio · {seconds.toFixed(1)}s</p>
          </div>
          <button
            type="button"
            aria-label="Close"
            onClick={onClose}
            className="p-1.5 rounded-lg text-studio-400 hover:text-white hover:bg-studio-700"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        <div className="p-6 space-y-5 overflow-y-auto">
          <fieldset disabled={exporting} className="space-y-5">
            <div className="space-y-2">
              <div className="flex items-baseline justify-between">
                <span className="font-semibold text-studio-300">Resolution</span>
                <span className="font-mono text-studio-500">
                  {fittedSize(resolution.width, resolution.height, aspectRatio).join(" × ")} · {aspectRatio}
                </span>
              </div>
              <div className="grid grid-cols-4 gap-1 bg-studio-950/60 border border-studio-800 rounded-lg p-1">
                {RESOLUTIONS.map((r) => (
                  <button
                    key={r.label}
                    type="button"
                    className={option(r.label === resolution.label)}
                    onClick={() => update({ resolution: r.label })}
                  >
                    {r.label}
                  </button>
                ))}
              </div>
            </div>

            <div className="space-y-2">
              <span className="font-semibold text-studio-300">Frame rate</span>
              <div className="grid grid-cols-4 gap-1 bg-studio-950/60 border border-studio-800 rounded-lg p-1">
                {FRAME_RATES.map((fps) => (
                  <button
                    key={fps}
                    type="button"
                    className={option(fps === preferences.fps)}
                    onClick={() => update({ fps })}
                  >
                    {fps} fps
                  </button>
                ))}
              </div>
            </div>

            <div className="space-y-2">
              <span className="font-semibold text-studio-300">Quality</span>
              <div className="grid grid-cols-4 gap-1 bg-studio-950/60 border border-studio-800 rounded-lg p-1">
                {QUALITIES.map((q) => (
                  <button
                    key={q.key}
                    type="button"
                    className={option(q.key === preferences.quality)}
                    onClick={() => update({ quality: q.key })}
                  >
                    {q.label}
                  </button>
                ))}
              </div>
              <p className="text-studio-500">{QUALITIES.find((q) => q.key === preferences.quality)?.hint}</p>
              {custom && (
                <label className="flex items-center gap-2">
                  <input
                    type="number"
                    min={MIN_MBPS}
                    max={MAX_MBPS}
                    step={1}
                    value={Number.isFinite(preferences.mbps) ? preferences.mbps : ""}
                    onChange={(event) => update({ mbps: event.target.valueAsNumber })}
                    aria-label="Video bitrate in megabits per second"
                    className={`w-24 bg-studio-800 border rounded px-2 py-1 text-studio-100 font-mono ${
                      mbpsValid ? "border-studio-700" : "border-danger"
                    }`}
                  />
                  <span className="text-studio-400">Mbps</span>
                  {mbpsValid ? (
                    <span className="text-studio-500 ml-auto">
                      ≈ {formatBytes(((preferences.mbps * 1e6 + 192_000) * seconds) / 8)}
                    </span>
                  ) : (
                    <span className="text-danger-fg ml-auto">
                      {MIN_MBPS} to {MAX_MBPS} Mbps
                    </span>
                  )}
                </label>
              )}
            </div>

            <div className="space-y-2">
              <span className="font-semibold text-studio-300">Save to</span>
              <div className="flex items-center gap-2">
                <span
                  className="flex-1 min-w-0 truncate font-mono text-studio-300 bg-studio-950/60 border border-studio-800 rounded px-2 py-1.5 select-text"
                  title={destination}
                >
                  {destination || "Next to the project folder"}
                </span>
                <button
                  type="button"
                  onClick={onChooseDestination}
                  className="px-2.5 py-1.5 rounded-lg bg-studio-850 hover:bg-studio-800 border border-studio-700 text-studio-200"
                >
                  Change…
                </button>
              </div>
            </div>
          </fieldset>

          {exportJob && exportJob.state !== "idle" && (
            <div className="space-y-2 rounded-lg border border-studio-800 bg-studio-950/60 p-3">
              <div className="flex items-center justify-between">
                <span className="font-semibold capitalize text-studio-200">{exportJob.state}</span>
                <span className="font-mono text-studio-400">{Math.round(progress * 100)}%</span>
              </div>
              <div className="h-1.5 rounded-full bg-studio-800 overflow-hidden">
                <div
                  className={`h-full transition-[width] ${exportJob.state === "failed" ? "bg-danger" : "bg-accent-hover"}`}
                  style={{ width: `${progress * 100}%` }}
                />
              </div>
              {exportJob.failure && (
                <p role="alert" className="text-danger-fg">
                  {exportJob.failure.message}
                </p>
              )}
              {exportJob.state === "completed" && exportJob.outputPath && (
                <div className="flex items-center gap-2">
                  <span className="flex-1 min-w-0 truncate font-mono text-audio-fg" title={exportJob.outputPath}>
                    {exportJob.outputPath}
                  </span>
                  <button
                    type="button"
                    onClick={() => void api.showInFinder(exportJob.outputPath!)}
                    className="flex items-center gap-1 px-2 py-1 rounded border border-studio-700 text-studio-200 hover:bg-studio-800"
                  >
                    <FolderOpen className="w-3.5 h-3.5" />
                    Show
                  </button>
                </div>
              )}
            </div>
          )}
        </div>

        <div className="px-6 py-4 border-t border-studio-800 bg-studio-850 flex items-center justify-end gap-2">
          {exporting ? (
            <button
              type="button"
              onClick={onCancel}
              className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-danger/15 hover:bg-danger/15 border border-danger/30 text-danger-fg font-medium"
            >
              <XCircle className="w-3.5 h-3.5" />
              Cancel export
            </button>
          ) : (
            <>
              <button
                type="button"
                onClick={onClose}
                className="px-3 py-1.5 rounded-lg text-studio-300 hover:text-white hover:bg-studio-800"
              >
                Close
              </button>
              <button
                type="button"
                disabled={custom && !mbpsValid}
                onClick={start}
                className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-accent hover:bg-accent-hover text-white font-semibold shadow-md shadow-accent/20 disabled:opacity-40"
              >
                <Download className="w-3.5 h-3.5" />
                Export
              </button>
            </>
          )}
        </div>
      </div>
    </div>
  );
};
