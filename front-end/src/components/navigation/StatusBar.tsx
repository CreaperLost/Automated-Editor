import React from "react";
import { CheckCircle2, Folder, Loader2, XCircle } from "lucide-react";
import { useProjectStore } from "../../stores/projectStore";
import { useSettingsStore } from "../../stores/settingsStore";
import { formatBinding, useHotkeyStore, type HotkeyAction } from "../../stores/hotkeyStore";
import type { ExportStatus } from "../../lib/types";
import { FILE_MANAGER } from "../../lib/projectActions";
import { Kbd } from "../ui";

function formatDuration(us: number): string {
  const total = Math.max(0, Math.round(us / 1_000_000));
  const hours = Math.floor(total / 3600);
  const minutes = Math.floor((total % 3600) / 60);
  const seconds = total % 60;
  const two = (n: number) => String(n).padStart(2, "0");
  return hours > 0 ? `${hours}:${two(minutes)}:${two(seconds)}` : `${two(minutes)}:${two(seconds)}`;
}

/** The shortcuts worth knowing at a glance; the rest are in File › Keyboard shortcuts. */
const HINTS: { action: HotkeyAction; label: string }[] = [
  { action: "playPause", label: "Play" },
  { action: "split", label: "Split" },
  { action: "deleteSelection", label: "Delete" },
  { action: "undo", label: "Undo" },
];

/// The strip along the bottom of the window: what the video is, export progress, and the main
/// keyboard shortcuts (as they are set, not as they ship).
export const StatusBar: React.FC<{
  exportJob?: ExportStatus;
  onOpenExport: () => void;
  onShowExport: (path: string) => void;
}> = ({ exportJob, onOpenExport, onShowExport }) => {
  const project = useProjectStore((s) => s.openedProject);
  const aspect = useSettingsStore((s) => s.canvas.aspectRatio);
  const bindings = useHotkeyStore((s) => s.bindings);
  if (!project) return null;

  const running = exportJob?.state === "queued" || exportJob?.state === "running";
  const percent =
    exportJob && exportJob.progressDenominator > 0
      ? Math.round((exportJob.progressNumerator / exportJob.progressDenominator) * 100)
      : 0;

  return (
    <footer className="h-statusbar shrink-0 flex items-center gap-4 border-t border-studio-800 bg-studio-900 px-3 text-meta text-studio-400 select-none">
      <span className="truncate">
        {formatDuration(project.editedDurationUs)} edited · Canvas {aspect}
        {project.shorts && project.shorts.length > 0 ? ` · ${project.shorts.length} shorts` : ""}
      </span>

      {exportJob && exportJob.state !== "idle" && (
        <span className="flex items-center gap-1.5 min-w-0">
          {running && (
            <button type="button" onClick={onOpenExport} className="flex items-center gap-1.5 text-accent-fg hover:underline">
              <Loader2 className="w-3.5 h-3.5 animate-spin" /> Exporting {percent}%
            </button>
          )}
          {exportJob.state === "completed" && (
            <>
              <CheckCircle2 className="w-3.5 h-3.5 text-success" />
              <span className="text-studio-300">Exported</span>
              {exportJob.outputPath && (
                <button
                  type="button"
                  onClick={() => onShowExport(exportJob.outputPath!)}
                  className="inline-flex items-center gap-1 text-accent-fg hover:underline"
                >
                  <Folder className="w-3.5 h-3.5" /> Show in {FILE_MANAGER}
                </button>
              )}
            </>
          )}
          {(exportJob.state === "failed" || exportJob.state === "cancelled") && (
            <button type="button" onClick={onOpenExport} className="flex items-center gap-1.5 text-danger-fg hover:underline">
              <XCircle className="w-3.5 h-3.5" /> Export {exportJob.state}
            </button>
          )}
        </span>
      )}

      <span className="flex-1" />
      <span className="hidden lg:flex items-center gap-3">
        {HINTS.map(({ action, label }) =>
          bindings[action][0] ? (
            <span key={action} className="flex items-center gap-1.5">
              <Kbd>{formatBinding(bindings[action][0])}</Kbd>
              {label}
            </span>
          ) : null,
        )}
      </span>
    </footer>
  );
};
