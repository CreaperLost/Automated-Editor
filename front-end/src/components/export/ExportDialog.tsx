import React, { useEffect, useState } from "react";
import { CheckCircle2, Download, Film, FolderOpen, X, XCircle } from "lucide-react";
import { api } from "../../lib/ipc";
import { FILE_MANAGER } from "../../lib/projectActions";
import { Button, IconButton, Segmented, cn } from "../ui";
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

  const seconds = editedDurationUs / 1e6;
  const [outWidth, outHeight] = fittedSize(resolution.width, resolution.height, aspectRatio);
  const quality = QUALITIES.find((q) => q.key === preferences.quality);
  const length = `${Math.floor(seconds / 60)}:${String(Math.floor(seconds % 60)).padStart(2, "0")}`;

  // The frame at the playhead, from the playback engine, as a picture of what is exported.
  const [thumbnail, setThumbnail] = useState<string>();
  useEffect(() => {
    let url: string | undefined;
    void api
      .previewFrame(0, 0)
      .then((buffer) => {
        if (buffer.byteLength <= 8) return;
        url = URL.createObjectURL(new Blob([buffer.slice(8)], { type: "image/jpeg" }));
        setThumbnail(url);
      })
      .catch(() => undefined);
    return () => {
      if (url) URL.revokeObjectURL(url);
    };
  }, []);

  const field = (label: string, control: React.ReactNode, hint?: React.ReactNode) => (
    <div className="space-y-1.5">
      <div className="text-label font-medium text-studio-200">{label}</div>
      {control}
      {hint && <div className="text-meta text-studio-500">{hint}</div>}
    </div>
  );

  return (
    <div
      className="fixed inset-0 z-50 bg-black/70 flex items-center justify-center p-4 select-none"
      onPointerDown={(event) => {
        if (event.target === event.currentTarget) onClose();
      }}
    >
      <div
        role="dialog"
        aria-modal="true"
        aria-labelledby="export-dialog-title"
        className="w-full max-w-3xl bg-studio-900 border border-studio-700 rounded-panel shadow-dialog overflow-hidden flex flex-col max-h-[90vh]"
      >
        <div className="px-6 py-4 border-b border-studio-800 flex items-start justify-between gap-4">
          <div>
            <h2 id="export-dialog-title" className="text-heading text-studio-100">
              Export video
            </h2>
            <p className="text-label text-studio-400">An MP4 file of the whole edit</p>
          </div>
          <IconButton icon={X} label="Close (Esc)" onClick={onClose} />
        </div>

        <div className="flex-1 min-h-0 overflow-y-auto">
          <div className="grid gap-6 p-6 md:grid-cols-[minmax(0,2fr)_minmax(0,3fr)]">
            {/* What comes out */}
            <div className="space-y-3">
              <div
                className="w-full rounded-control overflow-hidden border border-studio-700 bg-studio-950 flex items-center justify-center"
                style={{ aspectRatio: `${outWidth} / ${outHeight}`, maxHeight: 320 }}
              >
                {thumbnail ? (
                  <img src={thumbnail} alt="The video at the playhead" className="w-full h-full object-contain" />
                ) : (
                  <Film className="w-8 h-8 text-studio-600" aria-hidden />
                )}
              </div>
              <div className="space-y-0.5">
                <p className="text-body font-medium text-studio-100 tabular-nums">
                  {length} · {outWidth} × {outHeight} · {preferences.fps} fps
                </p>
                <p className="text-label text-studio-400">MP4 · H.264 video · AAC audio · {aspectRatio} canvas</p>
              </div>
            </div>

            {/* The choices */}
            <fieldset disabled={exporting} className="space-y-5 min-w-0 disabled:opacity-60">
              {field(
                "Resolution",
                <Segmented
                  label="Resolution"
                  className="w-full"
                  value={resolution.label}
                  onChange={(label) => update({ resolution: label })}
                  options={RESOLUTIONS.map((r) => ({ value: r.label, label: r.label }))}
                />,
                <>Output size {outWidth} × {outHeight} (the canvas shape, {aspectRatio}). Preview quality does not change this.</>,
              )}
              {field(
                "Frame rate",
                <Segmented
                  label="Frame rate"
                  className="w-full"
                  value={preferences.fps}
                  onChange={(fps) => update({ fps })}
                  options={FRAME_RATES.map((fps) => ({ value: fps, label: `${fps} fps` }))}
                />,
              )}
              {field(
                "Quality",
                <Segmented
                  label="Quality"
                  className="w-full"
                  value={preferences.quality}
                  onChange={(q) => update({ quality: q })}
                  options={QUALITIES.map((q) => ({ value: q.key, label: q.label }))}
                />,
                quality?.hint,
              )}
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
                    className={cn("ui-field w-24 font-mono", !mbpsValid && "!border-danger")}
                  />
                  <span className="text-label text-studio-400">Mbps</span>
                  {mbpsValid ? (
                    <span className="ml-auto text-label text-studio-400">
                      About {formatBytes(((preferences.mbps * 1e6 + 192_000) * seconds) / 8)}
                    </span>
                  ) : (
                    <span className="ml-auto text-label text-danger-fg">
                      {MIN_MBPS} to {MAX_MBPS} Mbps
                    </span>
                  )}
                </label>
              )}
              {field(
                "Save to",
                <div className="flex items-center gap-2">
                  <span
                    className="ui-field flex-1 min-w-0 flex items-center truncate font-mono !text-meta select-text"
                    title={destination}
                  >
                    {destination || "Next to the project folder"}
                  </span>
                  <Button variant="secondary" icon={FolderOpen} onClick={onChooseDestination}>
                    Change…
                  </Button>
                </div>,
              )}
            </fieldset>
          </div>

          {exportJob && exportJob.state !== "idle" && (
            <div className="mx-6 mb-6 space-y-2 rounded-control border border-studio-800 bg-studio-850 p-4">
              <div className="flex items-center justify-between text-label">
                <span className="flex items-center gap-2 font-medium capitalize text-studio-100">
                  {exportJob.state === "completed" && <CheckCircle2 className="w-4 h-4 text-success" />}
                  {exportJob.state === "completed" ? "Exported" : exportJob.state}
                </span>
                <span className="font-mono tabular-nums text-studio-400">{Math.round(progress * 100)}%</span>
              </div>
              <div className="h-1.5 rounded-full bg-studio-800 overflow-hidden">
                <div
                  className={`h-full transition-[width] ${exportJob.state === "failed" ? "bg-danger" : exportJob.state === "completed" ? "bg-success" : "bg-accent-hover"}`}
                  style={{ width: `${progress * 100}%` }}
                />
              </div>
              {exportJob.failure && (
                <p role="alert" className="text-label text-danger-fg">
                  {exportJob.failure.message}
                </p>
              )}
              {exportJob.state === "completed" && exportJob.outputPath && (
                <div className="flex items-center gap-2">
                  <span className="flex-1 min-w-0 truncate font-mono text-meta text-studio-300 select-text" title={exportJob.outputPath}>
                    {exportJob.outputPath}
                  </span>
                  <Button size="sm" variant="secondary" icon={FolderOpen} onClick={() => void api.showInFinder(exportJob.outputPath!)}>
                    Show in {FILE_MANAGER}
                  </Button>
                </div>
              )}
            </div>
          )}
        </div>

        <div className="px-6 py-4 border-t border-studio-800 flex items-center justify-end gap-2">
          {exporting ? (
            <Button variant="danger" icon={XCircle} onClick={onCancel}>
              Cancel export
            </Button>
          ) : (
            <>
              <Button variant="ghost" onClick={onClose}>
                Close
              </Button>
              <Button variant="primary" size="lg" icon={Download} disabled={custom && !mbpsValid} onClick={start}>
                Export video
              </Button>
            </>
          )}
        </div>
      </div>
    </div>
  );
};
