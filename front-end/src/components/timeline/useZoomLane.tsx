import React, { useEffect, useRef, useState } from "react";
import { Plus, RefreshCw, ScanSearch } from "lucide-react";
import { api } from "../../lib/ipc";
import { snapStart, sourceAt } from "../../lib/sequence";
import { DEFAULT_ZOOM_SETTINGS, type OpenedProject, type ProjectZoom, type ZoomKeyframe } from "../../lib/types";
import { useProjectStore } from "../../stores/projectStore";
import { HDR_BUTTON, HeaderRow, ResizeGrip, type TimelineView } from "./timelineShared";

type DragMode = "move" | "start" | "end";
/** A zoom is never dragged shorter than this. */
const MIN_ZOOM_US = 300_000;

/**
 * The zoom track: zooms as clips of their own, each on its recording's clock. Drag to move,
 * drag an edge to retime; a suggestion is added with a double-click.
 */
export function useZoomLane({
  view,
  height,
  onResize,
  snapPoints,
  selection,
  onError,
  onPicked,
  suppressClick,
}: {
  view: TimelineView;
  height: number;
  onResize: (height: number | null) => void;
  snapPoints: () => number[];
  selection: { startUs: number; endUs: number } | null;
  onError: (message?: string) => void;
  /** A zoom was picked: other selections clear. */
  onPicked: () => void;
  suppressClick: React.MutableRefObject<boolean>;
}) {
  const openedProject = useProjectStore((s) => s.openedProject);
  const zoomKeyframes = useProjectStore((s) => s.zoomKeyframes);
  const applyOpenedProject = useProjectStore((s) => s.applyOpenedProject);
  const applyZoomGeneration = useProjectStore((s) => s.applyZoomGeneration);
  const selectedZoomId = useProjectStore((s) => s.selectedZoomId);
  const setSelectedZoomId = useProjectStore((s) => s.setSelectedZoomId);
  const zoomSettings = openedProject?.zoomSettings ?? DEFAULT_ZOOM_SETTINGS;
  const [busy, setBusy] = useState(false);
  const dragging = useRef<{ mode: DragMode; zoomId: string; startX: number; originStart: number; originEnd: number } | null>(null);
  const [zoomDrag, setZoomDrag] = useState<{ barId: string; mode: DragMode; deltaUs: number } | null>(null);
  const { durationUs } = view;

  // Suggestions from every recording's mouse data, placed where its screen plays.
  useEffect(() => {
    if (!openedProject) return;
    let active = true;
    void api
      .projectZoomSuggestions(openedProject.projectHandle)
      .then((generation) => {
        if (active) applyZoomGeneration(generation);
      })
      .catch((err) => {
        if (active) applyZoomGeneration({ suggestions: [], diagnostics: [String(err)] });
      });
    return () => {
      active = false;
    };
  }, [openedProject?.projectHandle, openedProject?.revision, applyZoomGeneration]);

  useEffect(() => {
    if (selectedZoomId && !zoomKeyframes.some((bar) => bar.zoomId === selectedZoomId)) setSelectedZoomId(undefined);
  }, [zoomKeyframes, selectedZoomId, setSelectedZoomId]);

  const persist = async (work: (project: OpenedProject) => Promise<OpenedProject>) => {
    if (!openedProject || busy) return;
    setBusy(true);
    onError(undefined);
    try {
      applyOpenedProject(await work(openedProject));
    } catch (err) {
      onError(String(err));
    } finally {
      setBusy(false);
    }
  };

  const patchPersisted = (zoom: ProjectZoom, sourceStartUs: number, sourceEndUs: number) => {
    const duration = sourceEndUs - sourceStartUs;
    if (duration < 3) {
      onError("Zoom range is too short.");
      return;
    }
    // The auto-zoom transition, as long as in and out both fit.
    const transitionUs = Math.max(1, Math.min(Math.round(zoomSettings.transitionMs * 1000), Math.floor((duration - 1) / 2)));
    void persist((project) =>
      api.projectZoomUpdate(project.projectHandle, project.revision, { ...zoom, sourceStartUs, sourceEndUs, transitionUs }),
    );
  };

  /** Where a zoom may go: between the zooms either side of it (zooms never overlap). */
  const zoomRoom = (bar: ZoomKeyframe) => {
    const others = zoomKeyframes.filter((k) => !k.pending && k.zoomId !== bar.zoomId);
    const before = others.filter((k) => k.endUs <= bar.tUs).map((k) => k.endUs);
    const after = others.filter((k) => k.tUs >= bar.endUs).map((k) => k.tUs);
    return { from: Math.max(0, ...before), to: Math.min(durationUs, ...after) };
  };
  const draggedSpan = (bar: ZoomKeyframe, mode: DragMode, deltaUs: number): [number, number] => {
    const { from, to } = zoomRoom(bar);
    const length = bar.endUs - bar.tUs;
    if (mode === "move") {
      const snapped = snapStart(bar.tUs + deltaUs, length, snapPoints(), view.snapUs());
      const start = Math.max(Math.min(from, bar.tUs), Math.min(Math.max(to, bar.endUs) - length, snapped));
      return [start, start + length];
    }
    if (mode === "start") return [Math.max(Math.min(from, bar.tUs), Math.min(bar.endUs - MIN_ZOOM_US, bar.tUs + deltaUs)), bar.endUs];
    return [bar.tUs, Math.min(Math.max(to, bar.endUs), Math.max(bar.tUs + MIN_ZOOM_US, bar.endUs + deltaUs))];
  };

  const beginDrag = (event: React.PointerEvent, bar: ZoomKeyframe, mode: DragMode) => {
    if (!openedProject || busy || event.button !== 0) return;
    event.preventDefault();
    event.stopPropagation();
    setSelectedZoomId(bar.zoomId);
    onPicked();
    if (bar.pending) return; // A suggestion is accepted (double-click) before it moves.
    dragging.current = { mode, zoomId: bar.zoomId, startX: event.clientX, originStart: bar.sourceStartUs, originEnd: bar.sourceEndUs };
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
  };
  const moveDrag = (event: React.PointerEvent, bar: ZoomKeyframe) => {
    const drag = dragging.current;
    if (!drag || drag.zoomId !== bar.zoomId || view.pxPerUs <= 0) return;
    event.stopPropagation();
    if (Math.abs(event.clientX - drag.startX) < 2 && !zoomDrag) return;
    suppressClick.current = true;
    setZoomDrag({ barId: bar.id, mode: drag.mode, deltaUs: (event.clientX - drag.startX) / view.pxPerUs });
  };
  const endDrag = (event: React.PointerEvent, bar: ZoomKeyframe) => {
    const drag = dragging.current;
    dragging.current = null;
    const moved = zoomDrag;
    setZoomDrag(null);
    if (!drag || drag.zoomId !== bar.zoomId || !openedProject || !moved || Math.abs(moved.deltaUs) < 1_000) return;
    event.stopPropagation();
    const zoom = openedProject.zooms?.find((item) => item.id === bar.zoomId);
    if (!zoom) return;
    const [start, end] = draggedSpan(bar, moved.mode, moved.deltaUs);
    // A zoom moves along its own recording's screen.
    const asset = zoom.media ?? openedProject.assets.find((a) => a.kind === "recording")?.id;
    const toSource = (editedUs: number) => (asset ? sourceAt(openedProject, asset, editedUs) : null);
    let sourceStart = drag.originStart;
    let sourceEnd = drag.originEnd;
    if (moved.mode === "move") {
      const mapped = toSource(Math.min(durationUs - 1, start));
      if (mapped == null) {
        onError("A zoom moves along its own recording: drop it over that recording's screen.");
        return;
      }
      sourceStart = mapped;
      sourceEnd = mapped + (drag.originEnd - drag.originStart);
    } else if (moved.mode === "start") {
      const mapped = toSource(start);
      if (mapped == null) return;
      sourceStart = mapped;
    } else {
      const mapped = toSource(Math.max(start + 1, end) - 1);
      if (mapped == null) return;
      sourceEnd = mapped + 1;
    }
    if (sourceEnd <= sourceStart + 2) return;
    patchPersisted(zoom, sourceStart, sourceEnd);
  };

  const deleteZoom = (zoomId: string) => {
    const pending = zoomKeyframes.some((k) => k.zoomId === zoomId && k.pending);
    setSelectedZoomId(undefined);
    void persist((project) =>
      pending
        ? api.projectZoomDismiss(project.projectHandle, project.revision, [zoomId])
        : api.projectZoomDelete(project.projectHandle, project.revision, zoomId),
    );
  };
  const acceptZoom = (zoomId: string) =>
    void persist((project) => api.projectZoomAccept(project.projectHandle, project.revision, [zoomId]));
  const addZoomHere = () => {
    const currentTimeUs = view.nowUs();
    const startUs = selection ? selection.startUs : Math.max(0, currentTimeUs - 600_000);
    const endUs = selection ? selection.endUs : Math.min(durationUs, Math.max(startUs + 2_000_000, currentTimeUs + 1_400_000));
    void persist((project) =>
      api.projectZoomAdd(project.projectHandle, project.revision, {
        editedStartUs: startUs,
        editedEndUs: endUs,
        centerX: 0.5,
        centerY: 0.5,
        scale: zoomSettings.clickScale,
      }),
    );
  };
  const reloadZooms = () => void persist((project) => api.projectZoomReload(project.projectHandle, project.revision));

  const header = (
    <HeaderRow
      key="zooms"
      height={height}
      grip={<ResizeGrip label="the zoom track" height={height} onResize={onResize} />}
      tone="zoom"
      icon={ScanSearch}
      name="Zooms"
      meta={`${openedProject?.zooms?.length ?? 0}`}
      actions={
        <>
          <button
            type="button"
            disabled={!openedProject || busy || durationUs < 3}
            aria-label="Add a zoom"
            title={selection ? "Add a zoom over the selection" : "Add a zoom at the playhead (it fits between the zooms there)"}
            onClick={addZoomHere}
            className={HDR_BUTTON}
          >
            <Plus className="w-4 h-4" />
          </button>
          <button
            type="button"
            disabled={!openedProject || busy}
            aria-label="Reload zooms from the recording"
            title="Reload zooms from the recordings with your auto-zoom settings. Zooms you added or changed stay; Undo brings the old ones back."
            onClick={reloadZooms}
            className={HDR_BUTTON}
          >
            <RefreshCw className="w-4 h-4" />
          </button>
        </>
      }
    />
  );

  const lane = (
    <div key="zooms" className="relative rounded-md bg-studio-850/30" style={{ height }} data-track-row="zooms">
      {durationUs > 0 &&
        zoomKeyframes.map((k) => {
          const selected = k.zoomId === selectedZoomId;
          const [startUs, endUs] = zoomDrag && zoomDrag.barId === k.id ? draggedSpan(k, zoomDrag.mode, zoomDrag.deltaUs) : [k.tUs, k.endUs];
          const seconds = ((endUs - startUs) / 1e6).toFixed(1);
          const kind = k.source === "manual" ? "Manual zoom" : k.origin === "dwell" ? "Hover zoom" : "Click zoom";
          return (
            <div
              key={k.id}
              role="button"
              aria-label={`${k.pending ? "Suggested" : kind} ${k.scale.toFixed(1)}×`}
              aria-pressed={selected}
              className={`group absolute top-1 bottom-1 rounded-control border text-meta font-medium px-1.5 flex items-center overflow-hidden ${
                zoomDrag?.barId === k.id ? "cursor-grabbing z-30" : k.pending ? "cursor-pointer" : "cursor-grab"
              } ${
                k.pending
                  ? `border-dashed ${selected ? "border-accent-fg bg-zoom/25 text-zoom-fg ring-2 ring-accent-hover/70" : "border-zoom/70 bg-zoom/10 text-zoom-fg hover:bg-zoom/20"}`
                  : selected
                    ? "border-accent-fg bg-zoom/40 text-white ring-2 ring-accent-hover/70"
                    : "border-zoom/60 bg-zoom/25 text-zoom-fg hover:bg-zoom/35 hover:border-zoom-fg"
              }`}
              style={{ left: view.pct(startUs), width: `max(4px, ${view.pct(endUs - startUs)})` }}
              title={
                k.pending
                  ? `Suggested ${kind.toLowerCase()} · ${k.scale.toFixed(1)}× · ${seconds}s. Double-click to add it; Delete dismisses it.`
                  : `${kind} · ${k.scale.toFixed(1)}× · ${seconds}s. Drag to move, drag an edge to retime, Delete removes it. Zooms never overlap.`
              }
              onClick={(event) => {
                event.stopPropagation();
                if (suppressClick.current) suppressClick.current = false;
              }}
              onDoubleClick={(event) => {
                event.stopPropagation();
                if (k.pending) acceptZoom(k.zoomId);
                else view.seekToUs(k.tUs);
              }}
              onPointerDown={(event) => beginDrag(event, k, "move")}
              onPointerMove={(event) => moveDrag(event, k)}
              onPointerUp={(event) => endDrag(event, k)}
              onPointerCancel={() => {
                dragging.current = null;
                setZoomDrag(null);
              }}
            >
              <span className="truncate pointer-events-none">
                {k.scale.toFixed(1)}×{endUs - startUs > 1_500_000 ? ` · ${seconds}s` : ""}
              </span>
              {!k.pending && (
                <>
                  <span
                    aria-label="Resize zoom start"
                    className="absolute inset-y-0 left-0 w-1.5 cursor-ew-resize opacity-0 group-hover:opacity-100 bg-white/40"
                    onPointerDown={(event) => beginDrag(event, k, "start")}
                  />
                  <span
                    aria-label="Resize zoom end"
                    className="absolute inset-y-0 right-0 w-1.5 cursor-ew-resize opacity-0 group-hover:opacity-100 bg-white/40"
                    onPointerDown={(event) => beginDrag(event, k, "end")}
                  />
                </>
              )}
            </div>
          );
        })}
    </div>
  );

  return { header, lane, deleteZoom, selectedZoomId };
}
