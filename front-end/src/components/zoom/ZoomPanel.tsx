import React, { useState } from "react";
import { Check, Crosshair, Plus, RefreshCw, RotateCcw, Trash2, X } from "lucide-react";
import { useProjectStore } from "../../stores/projectStore";
import { api } from "../../lib/ipc";
import { DEFAULT_ZOOM_SETTINGS, type OpenedProject, type ProjectZoom, type ZoomSettings, type ZoomSuggestion } from "../../lib/types";
import { InspectorSection } from "../inspector/InspectorSection";
import { Notice, Switch } from "../ui";

function formatTime(us: number): string {
  const total = us / 1_000_000;
  const minutes = Math.floor(total / 60);
  const seconds = total - minutes * 60;
  return `${minutes}:${seconds.toFixed(1).padStart(4, "0")}`;
}

const ORIGIN_LABEL = { click: "Click", dwell: "Hover", cluster: "Clicks" } as const;
/** One click, one undoable edit: preset amounts rather than a slider for saved zooms. */
const SCALES = [1.25, 1.5, 1.8, 2, 2.5, 3] as const;

/** Edited start and end of a zoom, or null when all of it was cut. */
function editedSpan(zoom: { editedRanges: { startUs: number; endUs: number }[] }): [number, number] | null {
  const ranges = zoom.editedRanges ?? [];
  if (ranges.length === 0) return null;
  return [Math.min(...ranges.map((r) => r.startUs)), Math.max(...ranges.map((r) => r.endUs))];
}

function Slider(props: {
  label: string;
  value: number;
  min: number;
  max: number;
  step: number;
  format: (value: number) => string;
  onChange: (value: number) => void;
}) {
  return (
    <label className="block space-y-1.5">
      <div className="flex justify-between text-label">
        <span className="text-studio-400">{props.label}</span>
        <span className="font-mono tabular-nums text-meta text-studio-200">{props.format(props.value)}</span>
      </div>
      <input
        type="range"
        min={props.min}
        max={props.max}
        step={props.step}
        value={props.value}
        onChange={(event) => props.onChange(Number(event.target.value))}
        className="w-full h-1.5 cursor-pointer"
      />
    </label>
  );
}

/// Everything about zooms in one place: auto-zoom options, the suggestions to review, the zooms
/// on the timeline, and adding a manual one. The timeline's zoom lane still shows them and lets
/// you drag their timing.
export const ZoomPanel: React.FC = () => {
  const openedProject = useProjectStore((s) => s.openedProject);
  const applyOpenedProject = useProjectStore((s) => s.applyOpenedProject);
  const applyPlaybackStatus = useProjectStore((s) => s.applyPlaybackStatus);
  const pending = useProjectStore((s) => s.pendingZoomSuggestions);
  const diagnostics = useProjectStore((s) => s.zoomDiagnostics);
  const selectedZoomId = useProjectStore((s) => s.selectedZoomId);
  const setSelectedZoomId = useProjectStore((s) => s.setSelectedZoomId);
  const selection = useProjectStore((s) => s.timelineSelection);
  const durationUs = useProjectStore((s) => s.durationUs);
  // The project's settings, shown at once while a change is being saved.
  const saved = openedProject?.zoomSettings ?? DEFAULT_ZOOM_SETTINGS;
  const [draft, setDraft] = useState<ZoomSettings | null>(null);
  const settings = draft ?? saved;
  const saveTimer = React.useRef<number>();
  React.useEffect(() => () => window.clearTimeout(saveTimer.current), []);
  const setSettings = (patch: Partial<ZoomSettings>) => {
    const next = { ...settings, ...patch };
    setDraft(next);
    window.clearTimeout(saveTimer.current);
    saveTimer.current = window.setTimeout(() => {
      const project = useProjectStore.getState().openedProject;
      if (!project) return;
      api
        .projectZoomSettingsSet(project.projectHandle, project.revision, next)
        .then((updated) => {
          applyOpenedProject(updated);
          setDraft(null);
          setError(undefined);
        })
        .catch((err) => setError(String(err)));
    }, 350);
  };
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string>();

  const zooms = [...(openedProject?.zooms ?? [])].sort(
    (a, b) => (editedSpan(a)?.[0] ?? Infinity) - (editedSpan(b)?.[0] ?? Infinity),
  );
  const suggestions = [...pending].sort(
    (a, b) => (editedSpan(a)?.[0] ?? Infinity) - (editedSpan(b)?.[0] ?? Infinity),
  );

  const run = async (work: (project: OpenedProject) => Promise<OpenedProject>) => {
    const project = useProjectStore.getState().openedProject;
    if (!project || busy) return;
    setBusy(true);
    setError(undefined);
    try {
      applyOpenedProject(await work(project));
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  };

  const seek = (us: number) => {
    if (!openedProject) return;
    void api
      .playbackSeek(openedProject.projectHandle, us)
      .then(applyPlaybackStatus)
      .catch(() => undefined);
  };

  const focus = (zoom: ProjectZoom | ZoomSuggestion) => {
    setSelectedZoomId(zoom.id);
    const span = editedSpan(zoom);
    if (span) seek(span[0]);
  };

  const addZoom = () => {
    const currentTimeUs = useProjectStore.getState().currentTimeUs;
    const useSelection = selection && selection.endUs - selection.startUs >= 3;
    const startUs = useSelection ? selection.startUs : Math.max(0, currentTimeUs - 600_000);
    const endUs = useSelection
      ? selection.endUs
      : Math.min(durationUs, Math.max(startUs + 2_000_000, currentTimeUs + 1_400_000));
    void run((project) =>
      api.projectZoomAdd(project.projectHandle, project.revision, {
        editedStartUs: startUs,
        editedEndUs: endUs,
        centerX: 0.5,
        centerY: 0.5,
        scale: settings.clickScale,
      }),
    );
  };

  const optionsChanged = JSON.stringify(settings) !== JSON.stringify(DEFAULT_ZOOM_SETTINGS);
  const rowClass = (selected: boolean) =>
    `group flex items-center gap-2 rounded-md border px-2 py-1.5 cursor-pointer ${
      selected ? "border-accent-fg/70 bg-accent-hover/15" : "border-studio-800 bg-studio-850 hover:border-accent-hover/40"
    }`;

  if (!openedProject) return null;

  return (
    <div className="h-full overflow-y-auto bg-studio-900 text-label select-none">
      {error && (
        <Notice tone="danger" onDismiss={() => setError(undefined)}>
          {error}
        </Notice>
      )}

      <div className="p-3 border-b border-studio-800">
      <button
        type="button"
        disabled={busy || durationUs < 3}
        onClick={addZoom}
        className="w-full h-control-lg flex items-center justify-center gap-2 rounded-control bg-accent hover:bg-accent-hover border border-accent-hover/40 text-white text-body font-medium disabled:opacity-40 transition-colors"
        title="Zoom into the timeline selection, or around the playhead when nothing is selected"
      >
        <Plus className="w-4 h-4" />
        {selection ? "Add zoom on the selection" : "Add zoom at the playhead"}
      </button>
      </div>

      <InspectorSection
        id="zoom-auto"
        title="Auto-zoom"
        icon={RefreshCw}
        extra={
          optionsChanged ? (
            <button
              type="button"
              onClick={() => setSettings(DEFAULT_ZOOM_SETTINGS)}
              className="text-meta text-studio-400 hover:text-studio-200"
            >
              Reset
            </button>
          ) : undefined
        }
      >
        <p className="text-meta text-studio-500 leading-relaxed">
          A few strong zooms, not many: activity close together is one zoom, the camera follows the mouse through
          it, and only the strongest moments are kept. These settings belong to the project; changing an amount
          changes every automatic zoom.
        </p>
        <button
          type="button"
          disabled={busy}
          onClick={() => void run((project) => api.projectZoomReload(project.projectHandle, project.revision))}
          className="w-full flex items-center justify-center gap-1.5 px-2 py-1.5 rounded-md border border-accent-hover/40 text-accent-fg hover:bg-accent/25 disabled:opacity-40"
          title="Take the automatic zooms off and find the recording's zooms again with these settings (dismissed ones too). Zooms you added or changed stay. Undo brings the old ones back."
        >
          <RefreshCw className="w-3.5 h-3.5" /> Reload zooms from the recording
        </button>
        <Slider
          label="Zoom amount"
          value={settings.clickScale}
          min={1.1}
          max={4}
          step={0.05}
          format={(v) => `${v.toFixed(2)}×`}
          onChange={(clickScale) => setSettings({ clickScale })}
        />
        <Slider
          label="Hover zoom (mouse resting)"
          value={settings.hoverScale}
          min={1}
          max={4}
          step={0.05}
          format={(v) => (v <= 1 ? "Off" : `${v.toFixed(2)}×`)}
          onChange={(hoverScale) => setSettings({ hoverScale })}
        />
        <Slider
          label="Most zooms"
          value={settings.maxZooms}
          min={1}
          max={40}
          step={1}
          format={(v) => `${v}`}
          onChange={(maxZooms) => setSettings({ maxZooms })}
        />
        <Slider
          label="Join activity closer than"
          value={settings.mergeGapMs}
          min={0}
          max={10000}
          step={250}
          format={(v) => `${(v / 1000).toFixed(1)} s`}
          onChange={(mergeGapMs) => setSettings({ mergeGapMs })}
        />
        <Slider
          label="Transition"
          value={settings.transitionMs}
          min={100}
          max={3000}
          step={50}
          format={(v) => `${v} ms`}
          onChange={(transitionMs) => setSettings({ transitionMs })}
        />
        <Slider
          label="Shortest zoom"
          value={settings.minHoldMs}
          min={500}
          max={20000}
          step={100}
          format={(v) => `${(v / 1000).toFixed(1)} s`}
          onChange={(minHoldMs) => setSettings({ minHoldMs })}
        />
        <Switch
          checked={settings.follow}
          onChange={(follow) => setSettings({ follow })}
          label="Follow the mouse while zoomed"
        />
        {settings.follow && (
          <Slider
            label="Camera"
            value={settings.followMs}
            min={100}
            max={3000}
            step={50}
            format={(v) => (v < 450 ? "Snappy" : v < 1100 ? "Smooth" : "Calm") + ` · ${v} ms`}
            onChange={(followMs) => setSettings({ followMs })}
          />
        )}
      </InspectorSection>

      <InspectorSection
        id="zoom-suggestions"
        title={`Suggestions${suggestions.length ? ` (${suggestions.length})` : ""}`}
        icon={Crosshair}
      >
        {suggestions.length === 0 ? (
          <p className="text-meta text-studio-500">
            {diagnostics.length > 0 && zooms.length === 0
              ? `No auto-zoom: ${diagnostics[diagnostics.length - 1]}`
              : "Nothing new to review."}
          </p>
        ) : (
          <>
            <div className="flex gap-2">
              <button
                type="button"
                disabled={busy}
                onClick={() =>
                  void run((project) =>
                    api.projectZoomAccept(project.projectHandle, project.revision, suggestions.map((s) => s.id)),
                  )
                }
                className="flex-1 flex items-center justify-center gap-1 px-2 py-1 rounded-md bg-accent/30 hover:bg-accent/40 text-accent-fg disabled:opacity-40"
              >
                <Check className="w-3.5 h-3.5" /> Accept all
              </button>
              <button
                type="button"
                disabled={busy}
                onClick={() =>
                  void run((project) =>
                    api.projectZoomDismiss(project.projectHandle, project.revision, suggestions.map((s) => s.id)),
                  )
                }
                className="flex-1 flex items-center justify-center gap-1 px-2 py-1 rounded-md border border-studio-700 text-studio-300 hover:bg-studio-800 disabled:opacity-40"
              >
                <X className="w-3.5 h-3.5" /> Dismiss all
              </button>
            </div>
            <div className="space-y-1">
              {suggestions.map((s) => {
                const span = editedSpan(s);
                return (
                  <div key={s.id} className={rowClass(s.id === selectedZoomId)} onClick={() => focus(s)}>
                    <span className="font-mono text-studio-400 w-11 shrink-0">{span ? formatTime(span[0]) : "cut"}</span>
                    <span className="flex-1 min-w-0 truncate text-studio-200">
                      {ORIGIN_LABEL[s.origin]} · {s.scale.toFixed(1)}×
                      {span && <span className="text-studio-500"> · {((span[1] - span[0]) / 1e6).toFixed(1)}s</span>}
                    </span>
                    <button
                      type="button"
                      aria-label="Accept this zoom"
                      title="Accept"
                      disabled={busy}
                      onClick={(event) => {
                        event.stopPropagation();
                        void run((project) => api.projectZoomAccept(project.projectHandle, project.revision, [s.id]));
                      }}
                      className="p-1 rounded text-accent-fg hover:bg-accent/30 disabled:opacity-40"
                    >
                      <Check className="w-3.5 h-3.5" />
                    </button>
                    <button
                      type="button"
                      aria-label="Dismiss this zoom"
                      title="Dismiss"
                      disabled={busy}
                      onClick={(event) => {
                        event.stopPropagation();
                        void run((project) => api.projectZoomDismiss(project.projectHandle, project.revision, [s.id]));
                      }}
                      className="p-1 rounded text-studio-400 hover:text-white hover:bg-studio-700 disabled:opacity-40"
                    >
                      <X className="w-3.5 h-3.5" />
                    </button>
                  </div>
                );
              })}
            </div>
          </>
        )}
      </InspectorSection>

      <InspectorSection id="zoom-list" title={`Zooms${zooms.length ? ` (${zooms.length})` : ""}`} icon={Plus}>
        {zooms.length === 0 ? (
          <p className="text-meta text-studio-500">
            No zooms yet. Accept a suggestion or add one; drag a zoom on the timeline's zoom lane to retime it.
          </p>
        ) : (
          <div className="space-y-1">
            {zooms.map((zoom) => {
              const span = editedSpan(zoom);
              const selected = zoom.id === selectedZoomId;
              return (
                <div key={zoom.id} className="space-y-1.5">
                  <div className={rowClass(selected)} onClick={() => focus(zoom)}>
                    <span className="font-mono text-studio-400 w-11 shrink-0">{span ? formatTime(span[0]) : "cut"}</span>
                    <span className="flex-1 min-w-0 truncate text-studio-200">
                      {zoom.source === "manual" ? "Manual" : ORIGIN_LABEL[zoom.origin]} · {zoom.scale.toFixed(1)}×
                      {span && <span className="text-studio-500"> · {((span[1] - span[0]) / 1e6).toFixed(1)}s</span>}
                    </span>
                    <button
                      type="button"
                      aria-label="Delete this zoom"
                      title="Delete (undoable)"
                      disabled={busy}
                      onClick={(event) => {
                        event.stopPropagation();
                        void run((project) => api.projectZoomDelete(project.projectHandle, project.revision, zoom.id));
                      }}
                      className="p-1 rounded text-studio-500 hover:text-danger-fg hover:bg-studio-700 disabled:opacity-40"
                    >
                      <Trash2 className="w-3.5 h-3.5" />
                    </button>
                  </div>
                  {selected && settings.follow && (
                    <label className="pl-2 flex items-center gap-2 text-meta text-studio-300">
                      <input
                        type="checkbox"
                        checked={!zoom.fixed}
                        disabled={busy}
                        onChange={(e) =>
                          void run((project) =>
                            api.projectZoomUpdate(project.projectHandle, project.revision, { ...zoom, fixed: !e.target.checked }),
                          )
                        }
                        className=""
                      />
                      Follow the mouse (off: stays on its center)
                    </label>
                  )}
                  {selected && (
                    <div className="pl-2 pr-1 pb-1 flex items-center gap-1" role="group" aria-label="Zoom amount">
                      <span className="text-studio-400 mr-1">Zoom</span>
                      {SCALES.map((scale) => (
                        <button
                          key={scale}
                          type="button"
                          disabled={busy}
                          aria-pressed={Math.abs(zoom.scale - scale) < 0.01}
                          onClick={() =>
                            void run((project) =>
                              api.projectZoomUpdate(project.projectHandle, project.revision, { ...zoom, scale }),
                            )
                          }
                          className={`flex-1 py-0.5 rounded font-mono text-meta disabled:opacity-40 ${
                            Math.abs(zoom.scale - scale) < 0.01
                              ? "bg-accent text-white"
                              : "bg-studio-850 text-studio-300 hover:bg-studio-800"
                          }`}
                        >
                          {scale}×
                        </button>
                      ))}
                    </div>
                  )}
                </div>
              );
            })}
          </div>
        )}
      </InspectorSection>

      {diagnostics.length > 1 && (
        <details className="text-meta text-studio-500">
          <summary className="cursor-pointer">
            <RotateCcw className="inline w-3 h-3 mr-1" />
            Auto-zoom notes
          </summary>
          <ul className="mt-1 space-y-0.5 list-disc pl-4">
            {diagnostics.map((note, index) => (
              <li key={index}>{note}</li>
            ))}
          </ul>
        </details>
      )}
    </div>
  );
};
