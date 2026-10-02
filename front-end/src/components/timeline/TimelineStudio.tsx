import React, { useEffect, useLayoutEffect, useRef, useState } from "react";
import {
  Play,
  Pause,
  SkipBack,
  ZoomIn,
  ZoomOut,
  Scissors,
  Eye,
  Volume2,
  VolumeX,
  Plus,
  Check,
  X,
  Trash2,
  RotateCcw,
  Video,
} from "lucide-react";
import { useProjectStore } from "../../stores/projectStore";
import { useTimeline } from "../../hooks/useTimeline";
import { WaveformRenderer } from "../waveform/WaveformRenderer";
import { api } from "../../lib/ipc";
import {
  buildClips,
  buildCutMarkers,
  clipEdges,
  clipTrimLimits,
  editedToSourceUs,
  formatRulerLabel,
  rulerStepUs,
  type TimelineClip,
} from "../../lib/projectUtils";
import type { OpenedProject, ProjectZoom, ZoomKeyframe } from "../../lib/types";

type DragMode = "move" | "start" | "end";

const MIN_TIMELINE_ZOOM = 1;
const MAX_TIMELINE_ZOOM = 64;
/** Pointer distance, in pixels, inside which a dragged clip edge snaps to the playhead. */
const SNAP_PX = 8;

/** A clip edge being dragged: inward ripple-deletes, outward restores cut media. */
interface EdgeDrag {
  clip: TimelineClip;
  side: "start" | "end";
  startX: number;
  minDeltaUs: number;
  maxDeltaUs: number;
  deltaUs: number;
  pointerId: number;
}

export const TimelineStudio: React.FC = () => {
  const {
    openedProject,
    tracks,
    zoomKeyframes,
    pendingZoomSuggestions,
    zoomDiagnostics,
    currentTimeUs,
    durationUs,
    setIsSilenceModalOpen,
    setTrackWaveform,
    applyOpenedProject,
    applyZoomGeneration,
  } = useProjectStore();

  const { isPlaying, togglePlayPause, seekToUs, formattedTime, formattedDuration } = useTimeline();
  const frameUs = Math.round(
    1e6 / (openedProject?.manifest.tracks.find((track) => track.trackType === "screen")?.fps || 30),
  );

  const [rangeStart, setRangeStart] = useState("0");
  const [rangeEnd, setRangeEnd] = useState("0");
  const [editError, setEditError] = useState<string>();
  const [editing, setEditing] = useState(false);
  const [selectedZoomId, setSelectedZoomId] = useState<string>();
  const [zoomBusy, setZoomBusy] = useState(false);
  const dragging = useRef<{
    mode: DragMode;
    zoomId: string;
    startX: number;
    originStart: number;
    originEnd: number;
  } | null>(null);
  const suppressSeek = useRef(false);
  useEffect(() => { setRangeStart("0"); setRangeEnd(String(durationUs / 1e6)); setEditError(undefined); }, [openedProject?.projectHandle, durationUs]);
  const runEdit = async (work: (project: OpenedProject) => Promise<OpenedProject>) => {
    if (!openedProject || editing) return;
    setEditing(true); setEditError(undefined);
    try { applyOpenedProject(await work(openedProject)); }
    catch (err) { setEditError(String(err)); }
    finally { setEditing(false); }
  };
  const editRange = async (trim: boolean) => {
    if (!openedProject || editing) return;
    const startUs = Math.round(Number(rangeStart) * 1e6);
    const endUs = Math.round(Number(rangeEnd) * 1e6);
    if (!Number.isSafeInteger(startUs) || !Number.isSafeInteger(endUs) || startUs < 0 || startUs >= endUs || endUs > durationUs) {
      setEditError("Choose a start and end within the timeline, with start before end."); return;
    }
    const cuts = trim ? [
      ...(startUs > 0 ? [{ startUs: 0, endUs: startUs }] : []),
      ...(endUs < durationUs ? [{ startUs: endUs, endUs: durationUs }] : []),
    ] : [{startUs, endUs}];
    if (!cuts.length) return;
    await runEdit((project) => api.projectRippleCuts(project.projectHandle, project.revision, cuts));
  };
  const splitAtPlayhead = () =>
    runEdit((project) => api.projectSplit(project.projectHandle, project.revision, currentTimeUs));
  const restoreCut = (startUs: number, endUs: number) =>
    runEdit((project) => api.projectRestoreCuts(project.projectHandle, project.revision, [{ startUs, endUs }]));
  // Q and E: ripple-delete from the playhead to the previous or next edit point.
  const rippleTrim = (side: "previous" | "next") =>
    runEdit(async (project) => {
      const playheadUs = currentTimeUs;
      const next = await api.projectRippleTrim(project.projectHandle, project.revision, playheadUs, side);
      // Q pulls the later media back to the previous edit point; the playhead follows it.
      if (side === "previous") seekToUs(playheadUs - (project.editedDurationUs - next.editedDurationUs));
      return next;
    });
  const undo = () => {
    if (openedProject?.undoAvailable) void runEdit((project) => api.projectUndo(project.projectHandle, project.revision));
  };
  const redo = () => {
    if (openedProject?.redoAvailable) void runEdit((project) => api.projectRedo(project.projectHandle, project.revision));
  };

  // The range fields double as the timeline selection; the full range means nothing is selected.
  const selectedStartUs = Math.round(Number(rangeStart) * 1e6);
  const selectedEndUs = Math.round(Number(rangeEnd) * 1e6);
  const selection =
    Number.isSafeInteger(selectedStartUs) &&
    Number.isSafeInteger(selectedEndUs) &&
    selectedStartUs >= 0 &&
    selectedStartUs < selectedEndUs &&
    selectedEndUs <= durationUs &&
    !(selectedStartUs === 0 && selectedEndUs === durationUs)
      ? { startUs: selectedStartUs, endUs: selectedEndUs }
      : null;
  const selectRange = (startUs: number, endUs: number) => {
    setRangeStart(String(startUs / 1e6));
    setRangeEnd(String(endUs / 1e6));
  };
  const clearSelection = () => selectRange(0, durationUs);

  const retained = openedProject?.retainedIntervals ?? [];
  const clips = buildClips(retained, openedProject?.splitPointsUs);
  const cutMarkers = buildCutMarkers(retained, openedProject?.removedIntervals);
  const edges = clipEdges(clips);
  const jumpToEdit = (direction: -1 | 1) => {
    const target =
      direction < 0
        ? [...edges].reverse().find((edge) => edge < currentTimeUs)
        : edges.find((edge) => edge > currentTimeUs && edge < durationUs);
    if (target !== undefined) seekToUs(target);
  };

  const shortcuts = useRef({
    splitAtPlayhead,
    deleteSelection: () => {},
    clearSelection,
    rippleTrim,
    undo,
    redo,
    togglePlayPause,
    jumpToEdit,
    stepUs: (_offsetUs: number) => {},
    seekToUs,
  });
  shortcuts.current = {
    splitAtPlayhead,
    deleteSelection: () => { if (selection) void editRange(false); },
    clearSelection,
    rippleTrim,
    undo,
    redo,
    togglePlayPause,
    jumpToEdit,
    stepUs: (offsetUs: number) => seekToUs(currentTimeUs + offsetUs),
    seekToUs,
  };
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement | null;
      if (target?.closest("input, textarea, select, [contenteditable='true']")) return;
      const keys = shortcuts.current;
      if ((event.ctrlKey || event.metaKey) && !event.altKey) {
        const key = event.key.toLowerCase();
        if (key === "z") {
          event.preventDefault();
          if (event.shiftKey) keys.redo();
          else keys.undo();
        } else if (key === "y") {
          event.preventDefault();
          keys.redo();
        }
        return;
      }
      if (event.ctrlKey || event.metaKey || event.altKey) return;
      switch (event.key) {
        case "s":
        case "S":
          event.preventDefault();
          if (!event.repeat) void keys.splitAtPlayhead();
          break;
        case "q":
        case "Q":
          event.preventDefault();
          if (!event.repeat) void keys.rippleTrim("previous");
          break;
        case "e":
        case "E":
          event.preventDefault();
          if (!event.repeat) void keys.rippleTrim("next");
          break;
        case "Delete":
        case "Backspace":
          event.preventDefault();
          keys.deleteSelection();
          break;
        case "Escape":
          keys.clearSelection();
          break;
        case " ":
          event.preventDefault();
          if (!event.repeat) keys.togglePlayPause();
          break;
        case "ArrowLeft":
          event.preventDefault();
          keys.stepUs(-(event.shiftKey ? 1_000_000 : frameUs));
          break;
        case "ArrowRight":
          event.preventDefault();
          keys.stepUs(event.shiftKey ? 1_000_000 : frameUs);
          break;
        case "ArrowUp":
          event.preventDefault();
          keys.jumpToEdit(-1);
          break;
        case "ArrowDown":
          event.preventDefault();
          keys.jumpToEdit(1);
          break;
        case "=":
        case "+":
          event.preventDefault();
          zoomRef.current(2);
          break;
        case "-":
          event.preventDefault();
          zoomRef.current(0.5);
          break;
        case "Home":
          event.preventDefault();
          keys.seekToUs(0);
          break;
        case "End":
          event.preventDefault();
          keys.seekToUs(Number.MAX_SAFE_INTEGER);
          break;
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [frameUs]);

  const timelineTrackRef = useRef<HTMLDivElement | null>(null);
  const scrollRef = useRef<HTMLDivElement | null>(null);
  const [timelineZoom, setTimelineZoom] = useState(1);
  const [viewportPx, setViewportPx] = useState(0);
  // Keeps the time under the cursor (or the playhead) in place while zooming.
  const zoomAnchor = useRef<{ timeUs: number; offsetPx: number } | null>(null);

  useEffect(() => {
    const element = scrollRef.current;
    if (!element) return;
    const observer = new ResizeObserver(() => setViewportPx(element.clientWidth));
    observer.observe(element);
    setViewportPx(element.clientWidth);
    return () => observer.disconnect();
  }, [openedProject?.projectHandle]);
  useEffect(() => setTimelineZoom(1), [openedProject?.projectHandle]);

  const contentPx = viewportPx * timelineZoom;
  const pxPerUs = durationUs > 0 ? contentPx / durationUs : 0;

  const zoomTimeline = (factor: number, anchor?: { timeUs: number; offsetPx: number }) => {
    const next = Math.min(MAX_TIMELINE_ZOOM, Math.max(MIN_TIMELINE_ZOOM, timelineZoom * factor));
    if (next === timelineZoom) return;
    const element = scrollRef.current;
    zoomAnchor.current =
      anchor ??
      (element && pxPerUs > 0
        ? { timeUs: currentTimeUs, offsetPx: currentTimeUs * pxPerUs - element.scrollLeft }
        : null);
    setTimelineZoom(next);
  };
  useLayoutEffect(() => {
    const anchor = zoomAnchor.current;
    const element = scrollRef.current;
    zoomAnchor.current = null;
    if (!anchor || !element || pxPerUs <= 0) return;
    element.scrollLeft = Math.max(0, anchor.timeUs * pxPerUs - anchor.offsetPx);
  }, [timelineZoom, pxPerUs]);

  // Read by the wheel and keyboard listeners, which outlive this render.
  const zoomRef = useRef(zoomTimeline);
  zoomRef.current = zoomTimeline;
  useEffect(() => {
    const element = scrollRef.current;
    if (!element) return;
    // Ctrl/Cmd + wheel zooms around the cursor; the listener is not passive so it can stop page zoom.
    const onWheel = (event: WheelEvent) => {
      if (!(event.ctrlKey || event.metaKey)) return;
      event.preventDefault();
      const rect = element.getBoundingClientRect();
      const offsetPx = event.clientX - rect.left;
      const width = element.scrollWidth;
      const timeUs = width > 0 ? ((element.scrollLeft + offsetPx) / width) * durationUs : 0;
      zoomRef.current(event.deltaY < 0 ? 1.25 : 0.8, { timeUs, offsetPx });
    };
    element.addEventListener("wheel", onWheel, { passive: false });
    return () => element.removeEventListener("wheel", onWheel);
  }, [openedProject?.projectHandle, durationUs]);

  // While playing, page the view so the playhead stays visible.
  useEffect(() => {
    const element = scrollRef.current;
    if (!isPlaying || !element || pxPerUs <= 0 || timelineZoom <= 1) return;
    const x = currentTimeUs * pxPerUs;
    if (x < element.scrollLeft || x > element.scrollLeft + element.clientWidth - 24) {
      element.scrollLeft = Math.max(0, x - element.clientWidth * 0.1);
    }
  }, [isPlaying, currentTimeUs, pxPerUs, timelineZoom]);

  const rulerStep = rulerStepUs(pxPerUs);
  const rulerTicks =
    durationUs > 0 && pxPerUs > 0
      ? Array.from({ length: Math.floor(durationUs / rulerStep) + 1 }, (_, i) => i * rulerStep)
      : [];

  // Dragging on the ruler scrubs the playhead; seeks are sent at most once per frame.
  const scrub = useRef<{ frame: number | null; targetUs: number } | null>(null);
  const scrubTo = (clientX: number) => {
    const state = scrub.current;
    if (!state) return;
    state.targetUs = clientXToUs(clientX);
    if (state.frame !== null) return;
    state.frame = requestAnimationFrame(() => {
      if (!scrub.current) return;
      scrub.current.frame = null;
      seekToUs(scrub.current.targetUs);
    });
  };
  const onRulerPointerDown = (event: React.PointerEvent<HTMLDivElement>) => {
    if (event.button !== 0 || !openedProject || durationUs <= 0) return;
    event.currentTarget.setPointerCapture(event.pointerId);
    scrub.current = { frame: null, targetUs: 0 };
    scrubTo(event.clientX);
  };
  const onRulerPointerMove = (event: React.PointerEvent<HTMLDivElement>) => scrubTo(event.clientX);
  const onRulerPointerUp = () => {
    const state = scrub.current;
    scrub.current = null;
    if (state?.frame != null) {
      cancelAnimationFrame(state.frame);
      seekToUs(state.targetUs);
    }
  };

  const [edgeDrag, setEdgeDrag] = useState<EdgeDrag | null>(null);
  const beginEdgeDrag = (event: React.PointerEvent<HTMLDivElement>, clip: TimelineClip, side: "start" | "end") => {
    if (event.button !== 0 || !openedProject || editing) return;
    event.preventDefault();
    event.stopPropagation();
    const limits = clipTrimLimits(clip, side, retained, openedProject.removedIntervals);
    event.currentTarget.setPointerCapture(event.pointerId);
    setEdgeDrag({ clip, side, startX: event.clientX, ...limits, deltaUs: 0, pointerId: event.pointerId });
  };
  const moveEdgeDrag = (event: React.PointerEvent<HTMLDivElement>) => {
    if (!edgeDrag || event.pointerId !== edgeDrag.pointerId || pxPerUs <= 0) return;
    event.stopPropagation();
    const edgeUs = edgeDrag.side === "start" ? edgeDrag.clip.startUs : edgeDrag.clip.endUs;
    let deltaUs = Math.round((event.clientX - edgeDrag.startX) / pxPerUs);
    // Snap the edge onto the playhead when the pointer is close to it.
    if (Math.abs(edgeUs + deltaUs - currentTimeUs) * pxPerUs <= SNAP_PX) deltaUs = currentTimeUs - edgeUs;
    deltaUs = Math.min(edgeDrag.maxDeltaUs, Math.max(edgeDrag.minDeltaUs, deltaUs));
    if (deltaUs !== edgeDrag.deltaUs) setEdgeDrag({ ...edgeDrag, deltaUs });
  };
  const endEdgeDrag = (event: React.PointerEvent<HTMLDivElement>) => {
    if (!edgeDrag || event.pointerId !== edgeDrag.pointerId) return;
    event.stopPropagation();
    suppressSeek.current = true;
    const { clip, side, deltaUs } = edgeDrag;
    setEdgeDrag(null);
    if (Math.abs(deltaUs) < 1_000) return;
    const sourceEndUs = clip.sourceStartUs + (clip.endUs - clip.startUs);
    void runEdit((project) => {
      if (side === "start") {
        return deltaUs > 0
          ? api.projectRippleCuts(project.projectHandle, project.revision, [
              { startUs: clip.startUs, endUs: clip.startUs + deltaUs },
            ])
          : api.projectRestoreCuts(project.projectHandle, project.revision, [
              { startUs: clip.sourceStartUs + deltaUs, endUs: clip.sourceStartUs },
            ]);
      }
      return deltaUs < 0
        ? api.projectRippleCuts(project.projectHandle, project.revision, [
            { startUs: clip.endUs + deltaUs, endUs: clip.endUs },
          ])
        : api.projectRestoreCuts(project.projectHandle, project.revision, [
            { startUs: sourceEndUs, endUs: sourceEndUs + deltaUs },
          ]);
    });
  };

  useEffect(() => {
    if (!openedProject || durationUs <= 0) return;
    let active = true;
    const audioTracks = openedProject.tracks.filter(
      (track) =>
        track.descriptor.trackType === "mic_audio" || track.descriptor.trackType === "system_audio",
    );
    for (const track of audioTracks) {
      void api
        .projectWaveform(openedProject.projectHandle, track.descriptor.id, 0, durationUs, 256)
        .then((page) => {
          if (active && !page.cancelled) {
            setTrackWaveform(track.descriptor.id, page);
          }
        })
        .catch((err) => {
          console.warn("[Timeline] Waveform query failed:", err);
          if (active) {
            setTrackWaveform(track.descriptor.id, {
              trackId: track.descriptor.id,
              startUs: 0,
              endUs: durationUs,
              sampleRate: 0,
              channels: 0,
              channelPolicy: "max_energy",
              buckets: [],
              diagnostics: [String(err)],
              cancelled: false,
            });
          }
        });
    }
    return () => {
      active = false;
    };
  }, [openedProject?.projectHandle, openedProject?.revision, durationUs, setTrackWaveform]);

  useEffect(() => {
    if (!openedProject) return;
    let active = true;
    void api
      .projectZoomSuggestions(openedProject.projectHandle)
      .then((generation) => {
        if (active) applyZoomGeneration(generation);
      })
      .catch((err) => {
        console.warn("[Timeline] Zoom suggestions failed:", err);
        if (active) applyZoomGeneration({ suggestions: [], diagnostics: [String(err)] });
      });
    return () => {
      active = false;
    };
  }, [openedProject?.projectHandle, openedProject?.revision, applyZoomGeneration]);

  useEffect(() => {
    if (selectedZoomId && !zoomKeyframes.some((bar) => bar.zoomId === selectedZoomId)) {
      setSelectedZoomId(undefined);
    }
  }, [zoomKeyframes, selectedZoomId]);

  const persistZoom = async (work: () => Promise<OpenedProject>) => {
    if (!openedProject || zoomBusy) return;
    setZoomBusy(true);
    setEditError(undefined);
    try {
      applyOpenedProject(await work());
    } catch (err) {
      setEditError(String(err));
    } finally {
      setZoomBusy(false);
    }
  };

  const selectedBar = zoomKeyframes.find((bar) => bar.zoomId === selectedZoomId);
  const persistedSelected = openedProject?.zooms?.find((zoom) => zoom.id === selectedZoomId);

  const patchPersisted = (zoom: ProjectZoom, sourceStartUs: number, sourceEndUs: number) => {
    if (!openedProject) return;
    const duration = sourceEndUs - sourceStartUs;
    if (duration < 3) {
      setEditError("Zoom range is too short.");
      return;
    }
    const transitionUs = Math.min(Math.max(1, Math.floor(duration / 5)), 400_000, duration - 1);
    void persistZoom(() =>
      api.projectZoomUpdate(openedProject.projectHandle, openedProject.revision, {
        ...zoom,
        sourceStartUs,
        sourceEndUs,
        transitionUs,
      }),
    );
  };

  const rangeDrag = useRef<{ startX: number; active: boolean } | null>(null);
  const clientXToUs = (clientX: number) => {
    const rect = timelineTrackRef.current?.getBoundingClientRect();
    if (!rect || rect.width <= 0) return 0;
    return Math.round(Math.max(0, Math.min(1, (clientX - rect.left) / rect.width)) * durationUs);
  };
  const onTrackPointerDown = (event: React.PointerEvent<HTMLDivElement>) => {
    if (event.button !== 0 || !openedProject || durationUs <= 0) return;
    rangeDrag.current = { startX: event.clientX, active: false };
  };
  const onTrackPointerMove = (event: React.PointerEvent<HTMLDivElement>) => {
    const drag = rangeDrag.current;
    if (!drag) return;
    if (!drag.active) {
      if (Math.abs(event.clientX - drag.startX) < 4) return;
      // Capture only once it is a drag, so a plain click still reaches the clip under it.
      drag.active = true;
      event.currentTarget.setPointerCapture(event.pointerId);
    }
    const a = clientXToUs(drag.startX);
    const b = clientXToUs(event.clientX);
    if (a !== b) selectRange(Math.min(a, b), Math.max(a, b));
  };
  const onTrackPointerUp = () => {
    if (rangeDrag.current?.active) suppressSeek.current = true;
    rangeDrag.current = null;
  };

  const handleTimelineClick = (e: React.MouseEvent<HTMLDivElement>) => {
    if (suppressSeek.current) {
      suppressSeek.current = false;
      return;
    }
    if (!timelineTrackRef.current) return;
    const rect = timelineTrackRef.current.getBoundingClientRect();
    const clickX = e.clientX - rect.left;
    const progress = Math.max(0, Math.min(1, clickX / rect.width));
    seekToUs(progress * durationUs);
  };

  const beginDrag = (event: React.PointerEvent, bar: ZoomKeyframe, mode: DragMode) => {
    if (bar.pending || !openedProject || zoomBusy) return;
    event.preventDefault();
    event.stopPropagation();
    setSelectedZoomId(bar.zoomId);
    dragging.current = {
      mode,
      zoomId: bar.zoomId,
      startX: event.clientX,
      originStart: bar.sourceStartUs,
      originEnd: bar.sourceEndUs,
    };
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
  };

  const onBarPointerMove = (event: React.PointerEvent) => {
    const drag = dragging.current;
    if (!drag || !timelineTrackRef.current || durationUs <= 0) return;
    event.stopPropagation();
    suppressSeek.current = true;
  };

  const endDrag = (event: React.PointerEvent, bar: ZoomKeyframe) => {
    const drag = dragging.current;
    dragging.current = null;
    if (!drag || drag.zoomId !== bar.zoomId || !openedProject || !timelineTrackRef.current) return;
    event.stopPropagation();
    const rect = timelineTrackRef.current.getBoundingClientRect();
    if (rect.width <= 0) return;
    const deltaUs = ((event.clientX - drag.startX) / rect.width) * durationUs;
    if (Math.abs(deltaUs) < 1_000) return;
    const retained = openedProject.retainedIntervals;
    let sourceStart = drag.originStart;
    let sourceEnd = drag.originEnd;
    if (drag.mode === "move") {
      const editedStart = bar.tUs + deltaUs;
      const mapped = editedToSourceUs(retained, Math.max(0, Math.min(durationUs - 1, editedStart)));
      if (mapped == null) return;
      const duration = drag.originEnd - drag.originStart;
      sourceStart = mapped;
      sourceEnd = mapped + duration;
    } else if (drag.mode === "start") {
      const mapped = editedToSourceUs(retained, Math.max(0, Math.min(bar.endUs - 1, bar.tUs + deltaUs)));
      if (mapped == null) return;
      sourceStart = mapped;
    } else {
      const mapped = editedToSourceUs(
        retained,
        Math.max(bar.tUs + 1, Math.min(durationUs, bar.endUs + deltaUs)),
      );
      if (mapped == null) return;
      sourceEnd = mapped + 1;
    }
    if (sourceEnd <= sourceStart + 2) return;
    const zoom = openedProject.zooms?.find((item) => item.id === bar.zoomId);
    if (!zoom) return;
    patchPersisted(zoom, sourceStart, sourceEnd);
  };

  const progress = durationUs > 0 ? currentTimeUs / durationUs : 0;
  const pendingCount = pendingZoomSuggestions.length;
  const persistedCount = openedProject?.zooms?.length ?? 0;

  return (
    <div className="flex flex-col h-full bg-studio-900 border-t border-studio-800 select-none">
      {/* Timeline Toolbar */}
      <div className="h-12 px-6 flex items-center justify-between border-b border-studio-800 bg-studio-850">
        {/* Playback Controls & Timecode */}
        <div className="flex items-center space-x-4">
          <button
            disabled={!openedProject}
            onClick={() => seekToUs(0)}
            className="p-1.5 rounded-md hover:bg-studio-700 text-studio-300 transition-colors"
            title="Jump to Start"
          >
            <SkipBack className="w-4 h-4" />
          </button>

          <button
            disabled={!openedProject}
            onClick={togglePlayPause}
            className="p-2 rounded-lg bg-indigo-600 hover:bg-indigo-500 text-white transition-colors"
            title={isPlaying ? "Pause (Space)" : "Play (Space)"}
          >
            {isPlaying ? (
              <Pause className="w-4 h-4 fill-white" />
            ) : (
              <Play className="w-4 h-4 fill-white ml-0.5" />
            )}
          </button>

          <div className="font-mono text-sm tracking-wider text-studio-200">
            <span className="text-white font-semibold">{formattedTime}</span>
            <span className="text-studio-500 mx-1.5">/</span>
            <span className="text-studio-400">{formattedDuration}</span>
          </div>
        </div>

        {/* Action Tools: Silence Detection & Zoom Keyframe */}
        <div className="flex items-center space-x-3">
          <button
            disabled={!openedProject || editing || durationUs === 0}
            onClick={() => void splitAtPlayhead()}
            className="flex items-center space-x-1 px-2 py-1.5 rounded-md text-xs text-studio-200 hover:bg-studio-700 disabled:opacity-40"
            title="Split the clip at the playhead (S)"
          >
            <Scissors className="w-3.5 h-3.5" />
            <span>Split</span>
          </button>
          <button
            disabled={!openedProject || editing || durationUs === 0}
            onClick={() => void rippleTrim("previous")}
            className="px-2 py-1.5 rounded-md text-xs text-studio-200 hover:bg-studio-700 disabled:opacity-40"
            title="Ripple delete from the playhead back to the previous edit (Q)"
          >
            Trim ← Q
          </button>
          <button
            disabled={!openedProject || editing || durationUs === 0}
            onClick={() => void rippleTrim("next")}
            className="px-2 py-1.5 rounded-md text-xs text-studio-200 hover:bg-studio-700 disabled:opacity-40"
            title="Ripple delete from the playhead to the next edit (E)"
          >
            E → Trim
          </button>
          <button
            disabled={!openedProject?.undoAvailable || editing}
            onClick={undo}
            className="px-2 py-1.5 rounded-md text-xs text-studio-300 hover:bg-studio-700 disabled:opacity-40"
            title="Undo edit (Ctrl+Z)"
          >
            Undo
          </button>
          <button
            disabled={!openedProject?.redoAvailable || editing}
            onClick={redo}
            className="px-2 py-1.5 rounded-md text-xs text-studio-300 hover:bg-studio-700 disabled:opacity-40"
            title="Redo edit (Ctrl+Shift+Z)"
          >
            Redo
          </button>

          <button
            disabled={!openedProject}
            onClick={() => setIsSilenceModalOpen(true)}
            className="flex items-center space-x-1.5 px-3 py-1.5 rounded-md bg-emerald-600/20 hover:bg-emerald-600/30 border border-emerald-500/30 text-emerald-300 text-xs font-medium transition-colors"
            title="Detect and ripple-delete silence"
          >
            <Scissors className="w-3.5 h-3.5" />
            <span>AI Jump Cuts</span>
          </button>

          <button
            disabled={!openedProject || zoomBusy || durationUs < 3}
            onClick={() => {
              if (!openedProject) return;
              const selectedStart = Math.round(Number(rangeStart) * 1e6);
              const selectedEnd = Math.round(Number(rangeEnd) * 1e6);
              const useSelection =
                Number.isSafeInteger(selectedStart) &&
                Number.isSafeInteger(selectedEnd) &&
                selectedEnd - selectedStart >= 3 &&
                selectedEnd <= durationUs;
              const startUs = useSelection
                ? selectedStart
                : Math.max(0, currentTimeUs - 600_000);
              const endUs = useSelection
                ? selectedEnd
                : Math.min(durationUs, Math.max(startUs + 2_000_000, currentTimeUs + 1_400_000));
              void persistZoom(() =>
                api.projectZoomAdd(openedProject.projectHandle, openedProject.revision, {
                  editedStartUs: startUs,
                  editedEndUs: endUs,
                  centerX: 0.5,
                  centerY: 0.5,
                  scale: 1.8,
                }),
              );
            }}
            className="flex items-center space-x-1.5 px-3 py-1.5 rounded-md bg-indigo-600/20 hover:bg-indigo-600/30 border border-indigo-500/30 text-indigo-300 text-xs font-medium transition-colors disabled:opacity-40"
            title="Add a manual zoom on the selection, or around the playhead"
          >
            <Plus className="w-3.5 h-3.5" />
            <span>Add Zoom</span>
          </button>

          <button
            disabled={!openedProject || editing || !selection}
            onClick={() => {
              if (!selection) return;
              void runEdit((project) =>
                api.projectWebcamFocusAdd(project.projectHandle, project.revision, selection.startUs, selection.endUs),
              );
            }}
            className="flex items-center space-x-1.5 px-3 py-1.5 rounded-md bg-amber-500/15 hover:bg-amber-500/25 border border-amber-400/30 text-amber-200 text-xs font-medium transition-colors disabled:opacity-40"
            title="Make the webcam fill the frame over the selection (turns Auto Webcam on)"
          >
            <Video className="w-3.5 h-3.5" />
            <span>Cam Focus</span>
          </button>
          {pendingCount > 0 && (
            <>
              <button
                disabled={zoomBusy}
                onClick={() => {
                  if (!openedProject) return;
                  void persistZoom(() =>
                    api.projectZoomAccept(
                      openedProject.projectHandle,
                      openedProject.revision,
                      pendingZoomSuggestions.map((item) => item.id),
                    ),
                  );
                }}
                className="flex items-center space-x-1 px-2 py-1.5 rounded-md text-xs text-indigo-200 hover:bg-indigo-600/20 disabled:opacity-40"
                title="Accept all pending auto-zooms"
              >
                <Check className="w-3.5 h-3.5" />
                <span>Accept {pendingCount}</span>
              </button>
              {selectedBar?.pending && (
                <>
                  <button
                    disabled={zoomBusy}
                    onClick={() => {
                      if (!openedProject || !selectedZoomId) return;
                      void persistZoom(() =>
                        api.projectZoomAccept(openedProject.projectHandle, openedProject.revision, [
                          selectedZoomId,
                        ]),
                      );
                    }}
                    className="px-2 py-1.5 rounded-md text-xs text-indigo-200 hover:bg-indigo-600/20 disabled:opacity-40"
                  >
                    Accept selected
                  </button>
                  <button
                    disabled={zoomBusy}
                    onClick={() => {
                      if (!openedProject || !selectedZoomId) return;
                      void persistZoom(() =>
                        api.projectZoomDismiss(openedProject.projectHandle, openedProject.revision, [
                          selectedZoomId,
                        ]),
                      );
                    }}
                    className="flex items-center space-x-1 px-2 py-1.5 rounded-md text-xs text-studio-300 hover:bg-studio-700 disabled:opacity-40"
                  >
                    <X className="w-3.5 h-3.5" />
                    <span>Dismiss</span>
                  </button>
                </>
              )}
            </>
          )}
          {persistedSelected && (
            <>
              <label className="flex items-center space-x-1 text-[11px] text-studio-300">
                <span>Scale</span>
                <input
                  aria-label="Zoom scale"
                  type="number"
                  min={1}
                  max={8}
                  step={0.1}
                  value={persistedSelected.scale}
                  disabled={zoomBusy}
                  onChange={(event) => {
                    const scale = Number(event.target.value);
                    if (!Number.isFinite(scale) || scale < 1 || scale > 8 || !openedProject) return;
                    void persistZoom(() =>
                      api.projectZoomUpdate(openedProject.projectHandle, openedProject.revision, {
                        ...persistedSelected,
                        scale,
                      }),
                    );
                  }}
                  className="w-14 bg-studio-950 px-1 py-0.5 rounded"
                />
              </label>
              <button
                disabled={zoomBusy}
                onClick={() => {
                  if (!openedProject || !selectedZoomId) return;
                  void persistZoom(() =>
                    api.projectZoomDelete(openedProject.projectHandle, openedProject.revision, selectedZoomId),
                  );
                }}
                className="flex items-center space-x-1 px-2 py-1.5 rounded-md text-xs text-rose-300 hover:bg-rose-950/40 disabled:opacity-40"
                title="Delete this zoom"
              >
                <Trash2 className="w-3.5 h-3.5" />
                <span>Delete</span>
              </button>
            </>
          )}
          {pendingCount > 0 && (
            <span className="text-[11px] text-indigo-300/80">
              {pendingCount} pending auto-zoom{pendingCount === 1 ? "" : "s"}
            </span>
          )}
          {persistedCount > 0 && (
            <span className="text-[11px] text-studio-400">
              {persistedCount} saved
            </span>
          )}
          {zoomDiagnostics.length > 0 && pendingCount === 0 && persistedCount === 0 && (
            <span className="text-[11px] text-studio-400 truncate max-w-[220px]" title={zoomDiagnostics.join(" · ")}>
              No auto-zoom
            </span>
          )}

          <div className="h-4 w-px bg-studio-700 mx-1" />

          <button
            disabled={!openedProject || timelineZoom <= MIN_TIMELINE_ZOOM}
            onClick={() => zoomTimeline(0.5)}
            className="p-1.5 rounded hover:bg-studio-700 text-studio-400 disabled:opacity-40"
            title="Zoom the timeline out (-)"
          >
            <ZoomOut className="w-4 h-4" />
          </button>
          <button
            disabled={!openedProject || timelineZoom >= MAX_TIMELINE_ZOOM}
            onClick={() => zoomTimeline(2)}
            className="p-1.5 rounded hover:bg-studio-700 text-studio-400 disabled:opacity-40"
            title="Zoom the timeline in (+)"
          >
            <ZoomIn className="w-4 h-4" />
          </button>
        </div>
      </div>

      {openedProject && <div className="flex flex-wrap items-center gap-2 px-4 py-2 text-xs border-b border-studio-800">
        <label>Start (s) <input aria-label="Selection start in seconds" type="number" min="0" step="0.001" value={rangeStart} onChange={e => setRangeStart(e.target.value)} className="w-24 bg-studio-950 px-2 py-1 rounded" /></label>
        <button onClick={() => setRangeStart(String(currentTimeUs / 1e6))}>Set start here</button>
        <label>End (s) <input aria-label="Selection end in seconds" type="number" min="0" step="0.001" value={rangeEnd} onChange={e => setRangeEnd(e.target.value)} className="w-24 bg-studio-950 px-2 py-1 rounded" /></label>
        <button onClick={() => setRangeEnd(String(currentTimeUs / 1e6))}>Set end here</button>
        <button disabled={editing || durationUs === 0} onClick={() => void editRange(false)} className="text-rose-300 disabled:opacity-40" title="Cut the selection and close the gap (Delete)">Delete range</button>
        <button disabled={editing || durationUs === 0} onClick={() => void editRange(true)} className="text-teal-300 disabled:opacity-40">Keep range</button>
        {cutMarkers.length > 0 && (
          <button
            disabled={editing}
            onClick={() => void restoreCut(0, openedProject.sourceDurationUs)}
            className="flex items-center gap-1 text-amber-300 disabled:opacity-40"
            title="Put every cut back on the timeline"
          >
            <RotateCcw className="w-3 h-3" />
            Restore all {cutMarkers.length} cut{cutMarkers.length === 1 ? "" : "s"}
          </button>
        )}
        <span className="text-studio-500">Drag on the timeline to select, drag a clip edge to trim it. S splits, Q/E ripple-trim to the previous/next edit, Delete removes the selection.</span>
        {editError && <span role="alert" className="text-rose-300">{editError}</span>}
      </div>}

      {/* Multi-Track Workspace */}
      <div className="flex-1 flex overflow-hidden">
        {/* Left Track Headers */}
        <div className="w-56 border-r border-studio-800 bg-studio-900 shrink-0 flex flex-col">
          {/* Header spacer aligned with time ruler */}
          <div className="h-7 border-b border-studio-800 px-3 flex items-center text-[10px] font-semibold tracking-wider uppercase text-studio-400">
            Tracks
          </div>

          <div className="flex-1 space-y-2 py-2">
            <div className="h-8 px-3 flex items-end text-[10px] font-semibold tracking-wider uppercase text-studio-400">
              Clips
            </div>
            {tracks.map((track) => (
              <div
                key={track.id}
                className="h-14 px-3 flex items-center justify-between border-b border-studio-800/40 hover:bg-studio-850/50"
              >
                <div className="truncate">
                  <div className="text-xs font-medium text-studio-200 truncate">{track.name}</div>
                  <div className="text-[10px] uppercase font-mono text-studio-400">
                    {track.trackType}
                  </div>
                </div>

                <div className="flex items-center space-x-1">
                  <button disabled className="p-1 rounded text-studio-400 hover:text-white">
                    {track.muted ? <VolumeX className="w-3.5 h-3.5" /> : <Volume2 className="w-3.5 h-3.5" />}
                  </button>
                  <button disabled className="p-1 rounded text-studio-400 hover:text-white">
                    <Eye className="w-3.5 h-3.5" />
                  </button>
                </div>
              </div>
            ))}
          </div>
        </div>

        {/* Right Track Lanes & Playhead */}
        <div ref={scrollRef} className="flex-1 overflow-x-auto overflow-y-hidden relative">
          <div className="flex flex-col h-full min-w-full" style={{ width: `${timelineZoom * 100}%` }}>
          {/* Time Ruler: drag to scrub */}
          <div
            className="h-7 shrink-0 border-b border-studio-800 bg-studio-850/70 relative cursor-ew-resize overflow-hidden"
            onPointerDown={onRulerPointerDown}
            onPointerMove={onRulerPointerMove}
            onPointerUp={onRulerPointerUp}
            onPointerCancel={onRulerPointerUp}
            title="Drag to scrub"
          >
            {rulerTicks.map((tickUs) => (
              <div
                key={tickUs}
                className="absolute top-0 bottom-0 border-l border-studio-700 pl-1 text-[10px] font-mono text-studio-500 pointer-events-none"
                style={{ left: `${(tickUs / durationUs) * 100}%` }}
              >
                {formatRulerLabel(tickUs, rulerStep)}
              </div>
            ))}
            <div
              className="absolute top-0 bottom-0 w-0.5 bg-indigo-500 pointer-events-none"
              style={{ left: `${progress * 100}%` }}
            />
          </div>

          {/* Interactive Track Area */}
          <div
            ref={timelineTrackRef}
            onClick={handleTimelineClick}
            onPointerDown={onTrackPointerDown}
            onPointerMove={onTrackPointerMove}
            onPointerUp={onTrackPointerUp}
            onPointerCancel={onTrackPointerUp}
            className="flex-1 relative cursor-pointer py-2 space-y-2"
          >
            {/* Playhead Vertical Line */}
            <div
              className="absolute top-0 bottom-0 w-0.5 bg-indigo-500 z-30 pointer-events-none transition-all duration-75"
              style={{ left: `${progress * 100}%` }}
            >
              {/* Playhead Top Scrubber Cap */}
              <div className="w-3 h-3 bg-indigo-500 rounded-sm transform -translate-x-1/2 -top-1 absolute rotate-45 shadow-md shadow-indigo-500/50" />
            </div>

            {/* Zoom Keyframe Track overlay */}
            <div className="h-4 absolute top-0 left-0 right-0 z-20">
              {zoomKeyframes.map((k) => {
                const startProg = durationUs > 0 ? k.tUs / durationUs : 0;
                const widthProg = durationUs > 0 ? Math.max(0, (k.endUs - k.tUs) / durationUs) : 0;
                const selected = k.zoomId === selectedZoomId;
                const label = k.pending
                  ? `Pending auto-zoom ${k.scale}x (${k.origin ?? "click"})`
                  : `${k.source === "manual" ? "Manual" : "Saved"} zoom ${k.scale}x`;
                return (
                  <div
                    key={k.id}
                    className="absolute top-0.5 h-3 rounded-sm"
                    style={{
                      left: `${startProg * 100}%`,
                      width: `${Math.max(widthProg * 100, 0.4)}%`,
                    }}
                    title={label}
                    onClick={(event) => {
                      event.stopPropagation();
                      setSelectedZoomId(k.zoomId);
                    }}
                    onPointerDown={(event) => beginDrag(event, k, "move")}
                    onPointerMove={onBarPointerMove}
                    onPointerUp={(event) => endDrag(event, k)}
                  >
                    <div
                      className={`h-full rounded-sm border ${
                        k.pending
                          ? "bg-indigo-400/20 border-dashed border-indigo-300/80"
                          : "bg-indigo-400/60 border-indigo-200/90"
                      } ${selected ? "ring-1 ring-white/80" : ""}`}
                    />
                    {!k.pending && (
                      <>
                        <button
                          aria-label="Resize zoom start"
                          className="absolute inset-y-0 left-0 w-1.5 cursor-ew-resize"
                          onPointerDown={(event) => beginDrag(event, k, "start")}
                        />
                        <button
                          aria-label="Resize zoom end"
                          className="absolute inset-y-0 right-0 w-1.5 cursor-ew-resize"
                          onPointerDown={(event) => beginDrag(event, k, "end")}
                        />
                      </>
                    )}
                  </div>
                );
              })}
            </div>

            {/* Drag selection */}
            {selection && durationUs > 0 && (
              <div
                className="absolute top-0 bottom-0 bg-white/10 border-x border-white/60 z-10 pointer-events-none"
                style={{
                  left: `${(selection.startUs / durationUs) * 100}%`,
                  width: `${((selection.endUs - selection.startUs) / durationUs) * 100}%`,
                }}
              />
            )}

            {/* Clip lane: edges come from cuts and splits; markers restore cuts */}
            <div className="h-8 relative">
              {durationUs > 0 && clips.map((clip, index) => {
                const selected = selection?.startUs === clip.startUs && selection?.endUs === clip.endUs;
                return (
                  <button
                    key={`${clip.sourceStartUs}-${index}`}
                    className={`absolute top-2.5 bottom-0 rounded border text-[9px] font-mono text-left px-1 truncate ${
                      selected
                        ? "bg-teal-500/40 border-teal-200 text-white"
                        : "bg-teal-500/15 border-teal-400/40 text-teal-200 hover:bg-teal-500/25"
                    }`}
                    style={{
                      left: `${(clip.startUs / durationUs) * 100}%`,
                      width: `${((clip.endUs - clip.startUs) / durationUs) * 100}%`,
                    }}
                    title={`Clip ${index + 1}: ${((clip.endUs - clip.startUs) / 1e6).toFixed(2)}s. Click to select, Shift+click to extend.`}
                    onClick={(event) => {
                      event.stopPropagation();
                      if (event.shiftKey && selection) {
                        selectRange(Math.min(selection.startUs, clip.startUs), Math.max(selection.endUs, clip.endUs));
                      } else {
                        selectRange(clip.startUs, clip.endUs);
                      }
                    }}
                  >
                    {index + 1}
                  </button>
                );
              })}
              {/* Clip edge handles: the end handle sits left of the edge, the start handle right of it. */}
              {durationUs > 0 && !editing && clips.flatMap((clip, index) =>
                (["start", "end"] as const).map((side) => {
                  const edgePct = ((side === "start" ? clip.startUs : clip.endUs) / durationUs) * 100;
                  return (
                    <div
                      key={`${side}-${clip.sourceStartUs}-${index}`}
                      role="separator"
                      aria-label={`Trim clip ${index + 1} ${side}`}
                      className={`absolute top-2.5 bottom-0 w-1.5 z-30 cursor-ew-resize hover:bg-teal-200/70 ${
                        side === "start" ? "rounded-l" : "-translate-x-full rounded-r"
                      }`}
                      style={{ left: `${edgePct}%` }}
                      title={`Drag to trim clip ${index + 1} ${side === "start" ? "start" : "end"}`}
                      onPointerDown={(event) => beginEdgeDrag(event, clip, side)}
                      onPointerMove={moveEdgeDrag}
                      onPointerUp={endEdgeDrag}
                      onPointerCancel={() => setEdgeDrag(null)}
                      onClick={(event) => event.stopPropagation()}
                    />
                  );
                }),
              )}
              {edgeDrag && edgeDrag.deltaUs !== 0 && durationUs > 0 && (() => {
                const { clip, side, deltaUs } = edgeDrag;
                const edgeUs = side === "start" ? clip.startUs : clip.endUs;
                const trimming = side === "start" ? deltaUs > 0 : deltaUs < 0;
                const fromUs = Math.min(edgeUs, edgeUs + deltaUs);
                const toUs = Math.max(edgeUs, edgeUs + deltaUs);
                return (
                  <div
                    className={`absolute top-2.5 bottom-0 z-20 pointer-events-none rounded border flex items-center justify-center text-[9px] font-mono ${
                      trimming
                        ? "bg-rose-500/40 border-rose-300 text-rose-50"
                        : "bg-teal-300/30 border-dashed border-teal-100 text-teal-50"
                    }`}
                    style={{
                      left: `${(Math.max(0, fromUs) / durationUs) * 100}%`,
                      width: `${((Math.min(durationUs, toUs) - Math.max(0, fromUs)) / durationUs) * 100}%`,
                      minWidth: 2,
                    }}
                  >
                    <span className="px-1 bg-studio-950/80 rounded whitespace-nowrap">
                      {trimming ? "−" : "+"}
                      {(Math.abs(deltaUs) / 1e6).toFixed(2)}s
                    </span>
                  </div>
                );
              })()}
              {/* Cut markers: the cap above the clips restores the cut; the edge below it trims. */}
              {durationUs > 0 && cutMarkers.map((marker) => (
                <button
                  key={marker.sourceStartUs}
                  disabled={editing}
                  aria-label="Restore cut"
                  className="absolute top-0 bottom-0 w-3 -translate-x-1/2 z-20 flex flex-col items-center group disabled:opacity-40"
                  style={{ left: `${(marker.editedUs / durationUs) * 100}%` }}
                  title={`Restore ${((marker.sourceEndUs - marker.sourceStartUs) / 1e6).toFixed(2)}s cut`}
                  onPointerDown={(event) => event.stopPropagation()}
                  onClick={(event) => {
                    event.stopPropagation();
                    void restoreCut(marker.sourceStartUs, marker.sourceEndUs);
                  }}
                >
                  <span className="w-2.5 h-2 shrink-0 rounded-sm bg-rose-400 group-hover:bg-rose-200" />
                  <span className="w-0.5 flex-1 bg-rose-400 group-hover:bg-rose-200" />
                </button>
              ))}
            </div>

            {/* Individual Lanes */}
            {tracks.map((track) => (
              <div
                key={track.id}
                className="h-14 mx-2 rounded-lg bg-studio-800/60 border border-studio-750 relative overflow-hidden flex items-center"
              >
                {/* Waveform for audio tracks */}
                {track.waveform && track.waveform.buckets.length > 0 && (
                  <div className="w-full h-full px-2 py-1">
                    <WaveformRenderer
                      buckets={track.waveform.buckets}
                      currentTimeProgress={progress}
                      activeBarColor={track.trackType === "mic" ? "#10b981" : "#6366f1"}
                    />
                  </div>
                )}

                {track.waveform && track.waveform.buckets.length === 0 && (
                  <span className="px-4 text-xs text-studio-400">Waveform unavailable</span>
                )}

                {!track.waveform && (track.trackType === "mic" || track.trackType === "system") && (
                  <span className="px-4 text-xs text-studio-400">Loading waveform…</span>
                )}

                {!track.waveform && track.trackType !== "mic" && track.trackType !== "system" && (
                  <div className="mx-2 h-8 flex-1 rounded bg-indigo-500/15 border border-indigo-400/20" />
                )}

                {/* Auto webcam layout: where the webcam fills the frame */}
                {track.trackType === "webcam" && durationUs > 0 &&
                  (openedProject?.webcamFocus?.segments ?? []).flatMap((segment) =>
                    (segment.editedRanges ?? []).map((range, index) => (
                      <div
                        key={`${segment.id}-${index}`}
                        className={`absolute top-1 bottom-1 rounded-sm border z-10 pointer-events-none ${
                          segment.enabled && openedProject?.webcamFocus?.enabled
                            ? "bg-amber-400/30 border-amber-300/70"
                            : "border-dashed border-studio-500/60"
                        }`}
                        style={{
                          left: `${(range.startUs / durationUs) * 100}%`,
                          width: `${((range.endUs - range.startUs) / durationUs) * 100}%`,
                        }}
                        title={`Webcam ${segment.enabled ? "fills the frame" : "focus off"} (${segment.source})`}
                      />
                    )),
                  )}

                {/* Excluded intervals / Silence cuts overlay */}
                {track.intervals
                  .filter((int) => int.excluded)
                  .map((cut) => {
                    const cutStartProg = cut.startUs / durationUs;
                    const cutWidthProg = (cut.endUs - cut.startUs) / durationUs;
                    return (
                      <div
                        key={cut.id}
                        className="absolute top-0 bottom-0 bg-rose-950/70 border-x border-rose-500/50 backdrop-blur-[1px] flex items-center justify-center z-10 pointer-events-none"
                        style={{
                          left: `${cutStartProg * 100}%`,
                          width: `${cutWidthProg * 100}%`,
                        }}
                      >
                        <span className="text-[9px] font-mono text-rose-300 font-semibold uppercase tracking-wider">
                          CUT
                        </span>
                      </div>
                    );
                  })}
              </div>
            ))}
          </div>
          </div>
        </div>
      </div>
    </div>
  );
};
