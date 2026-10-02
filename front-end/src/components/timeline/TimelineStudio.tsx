import React, { useEffect, useLayoutEffect, useRef, useState } from "react";
import {
  Play,
  Pause,
  SkipBack,
  ZoomIn,
  ZoomOut,
  Scissors,
  X,
  RotateCcw,
  Video,
  MonitorPlay,
} from "lucide-react";
import { useProjectStore } from "../../stores/projectStore";
import { useTimeline } from "../../hooks/useTimeline";
import { WaveformRenderer } from "../waveform/WaveformRenderer";
import { MEDIA_DRAG_TYPE } from "../media/MediaPanel";
import { placedChapters } from "../chapters/ChaptersPanel";
import { useZoomSettingsStore, zoomConfigFor } from "../../stores/zoomSettingsStore";
import { TrackHeaderButtons } from "../audio/TrackHeaderButtons";
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
import {
  DEFAULT_WEBCAM_FOCUS,
  type OpenedProject,
  type ProjectZoom,
  type ZoomKeyframe,
} from "../../lib/types";

type SourceRange = [start: number, end: number];

/** `ranges` minus `cut`, both as [start, end) source ranges. */
function subtractRanges(ranges: SourceRange[], cut: SourceRange[]): SourceRange[] {
  return ranges.flatMap(([start, end]) => {
    let pieces: SourceRange[] = [[start, end]];
    for (const [cutStart, cutEnd] of cut) {
      pieces = pieces.flatMap(([a, b]): SourceRange[] =>
        cutEnd <= a || cutStart >= b
          ? [[a, b]]
          : [
              ...(cutStart > a ? [[a, cutStart] as SourceRange] : []),
              ...(cutEnd < b ? [[cutEnd, b] as SourceRange] : []),
            ],
      );
    }
    return pieces;
  });
}

type DragMode = "move" | "start" | "end";

const MIN_TIMELINE_ZOOM = 1;
const MAX_TIMELINE_ZOOM = 64;
/** Pointer distance, in pixels, inside which a dragged clip edge snaps to the playhead. */
const SNAP_PX = 8;
/** Pointer travel, in pixels, before a press on a clip becomes a drag that reorders it. */
const CLIP_DRAG_PX = 8;
/** Pointer travel, in pixels, before a press on the track becomes a range selection instead of a seek. */
const RANGE_DRAG_PX = 6;

/** Track lane heights, per track type, remembered on this machine. */
const TRACK_HEIGHT_KEY = "aeroedits.trackHeights.v1";
const DEFAULT_TRACK_HEIGHT = 56;
const MIN_TRACK_HEIGHT = 28;
const MAX_TRACK_HEIGHT = 240;

function loadTrackHeights(): Record<string, number> {
  try {
    const stored = JSON.parse(window.localStorage.getItem(TRACK_HEIGHT_KEY) ?? "null");
    if (stored && typeof stored === "object") return stored;
  } catch {
    // Storage can be unavailable; default heights still work.
  }
  return {};
}

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
  const selectedZoomId = useProjectStore((s) => s.selectedZoomId);
  const setSelectedZoomId = useProjectStore((s) => s.setSelectedZoomId);
  const setTimelineSelection = useProjectStore((s) => s.setTimelineSelection);
  const autoZoomOptions = useZoomSettingsStore((s) => s.options);
  const [zoomBusy, setZoomBusy] = useState(false);
  const dragging = useRef<{
    mode: DragMode;
    zoomId: string;
    startX: number;
    originStart: number;
    originEnd: number;
  } | null>(null);
  const suppressSeek = useRef(false);
  const [trackHeights, setTrackHeights] = useState(loadTrackHeights);
  const trackHeight = (trackType: string) => trackHeights[trackType] ?? DEFAULT_TRACK_HEIGHT;
  const setTrackHeight = (trackType: string, height: number | null) =>
    setTrackHeights((current) => {
      const next = { ...current };
      if (height === null) delete next[trackType];
      else next[trackType] = Math.round(Math.max(MIN_TRACK_HEIGHT, Math.min(MAX_TRACK_HEIGHT, height)));
      try {
        window.localStorage.setItem(TRACK_HEIGHT_KEY, JSON.stringify(next));
      } catch {
        // Not remembering the height is harmless.
      }
      return next;
    });
  const trackResize = useRef<{ trackType: string; startY: number; startHeight: number } | null>(null);
  useEffect(() => { setRangeStart("0"); setRangeEnd(String(durationUs / 1e6)); setEditError(undefined); }, [openedProject?.projectHandle, durationUs]);
  /** Runs one edit; resolves to whether it was applied. */
  const runEdit = async (work: (project: OpenedProject) => Promise<OpenedProject>) => {
    if (!openedProject || editing) return false;
    setEditing(true); setEditError(undefined);
    try { applyOpenedProject(await work(openedProject)); return true; }
    catch (err) { setEditError(String(err)); return false; }
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
  const restoreCut = (startUs: number, endUs: number, grow: "end" | "start" = "end") =>
    runEdit((project) => api.projectRestoreCuts(project.projectHandle, project.revision, [{ startUs, endUs }], grow));
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
  useEffect(() => {
    setTimelineSelection(selection);
  }, [selection?.startUs, selection?.endUs, setTimelineSelection]);

  const retained = openedProject?.retainedIntervals ?? [];
  const clips = buildClips(retained, openedProject?.splitPointsUs);
  const cutMarkers = buildCutMarkers(retained, openedProject?.removedIntervals);
  const edges = clipEdges(clips);

  // Per-clip "normal view": the webcam keeps its bubble over these source ranges.
  const focus = openedProject?.webcamFocus ?? DEFAULT_WEBCAM_FOCUS;
  const normalView: SourceRange[] = (focus.normalView ?? []).map((r) => [r.sourceStartUs, r.sourceEndUs]);
  const selectionSource: SourceRange[] = selection
    ? clips.flatMap((clip): SourceRange[] => {
        if (clip.media) return [];
        const start = Math.max(selection.startUs, clip.startUs);
        const end = Math.min(selection.endUs, clip.endUs);
        if (end <= start) return [];
        const offset = clip.sourceStartUs - clip.startUs;
        return [[start + offset, end + offset]];
      })
    : [];
  const selectionInNormalView =
    selectionSource.length > 0 && subtractRanges(selectionSource, normalView).length === 0;
  const toggleNormalView = () => {
    if (selectionSource.length === 0) return;
    const next = selectionInNormalView
      ? subtractRanges(normalView, selectionSource)
      : [...normalView, ...selectionSource];
    void runEdit((project) =>
      api.projectWebcamFocusUpdate(project.projectHandle, project.revision, {
        ...(project.webcamFocus ?? DEFAULT_WEBCAM_FOCUS),
        normalView: next.map(([sourceStartUs, sourceEndUs]) => ({ sourceStartUs, sourceEndUs })),
      }),
    );
  };

  // Cam Focus toggles: on adds focus over the selection (never stacking), on again removes it.
  const focusEdited: SourceRange[] = focus.enabled
    ? (focus.segments ?? [])
        .filter((segment) => segment.enabled)
        .flatMap((segment) => (segment.editedRanges ?? []).map((r): SourceRange => [r.startUs, r.endUs]))
    : [];
  const selectionFocused =
    !!selection && subtractRanges([[selection.startUs, selection.endUs]], focusEdited).length === 0;
  const toggleCamFocus = () => {
    if (!selection) return;
    const { startUs, endUs } = selection;
    void runEdit((project) =>
      selectionFocused
        ? api.projectWebcamFocusRemove(project.projectHandle, project.revision, startUs, endUs)
        : api.projectWebcamFocusAdd(project.projectHandle, project.revision, startUs, endUs),
    );
  };

  const mediaName = (clip: TimelineClip) =>
    clip.media ? openedProject?.mediaAssets?.find((asset) => asset.id === clip.media)?.name ?? "Media" : null;

  // Dropping a Media panel item on the track area inserts it at the nearest clip edge.
  const [mediaDropUs, setMediaDropUs] = useState<number | null>(null);
  const nearestEdge = (clientX: number) => {
    const pointerUs = clientXToUs(clientX);
    return edges.reduce((best, edge) => (Math.abs(edge - pointerUs) < Math.abs(best - pointerUs) ? edge : best), edges[0] ?? 0);
  };
  const onMediaDragOver = (event: React.DragEvent<HTMLDivElement>) => {
    if (!event.dataTransfer.types.includes(MEDIA_DRAG_TYPE) || !openedProject) return;
    event.preventDefault();
    event.dataTransfer.dropEffect = "copy";
    setMediaDropUs(nearestEdge(event.clientX));
  };
  const onMediaDrop = (event: React.DragEvent<HTMLDivElement>) => {
    const assetId = event.dataTransfer.getData(MEDIA_DRAG_TYPE);
    setMediaDropUs(null);
    if (!assetId || !openedProject) return;
    event.preventDefault();
    const target = nearestEdge(event.clientX);
    void runEdit((project) => api.projectMediaInsert(project.projectHandle, project.revision, assetId, target));
  };
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
          : api.projectRestoreCuts(
              project.projectHandle,
              project.revision,
              [{ startUs: clip.sourceStartUs + deltaUs, endUs: clip.sourceStartUs }],
              "start",
            );
      }
      return deltaUs < 0
        ? api.projectRippleCuts(project.projectHandle, project.revision, [
            { startUs: clip.endUs + deltaUs, endUs: clip.endUs },
          ])
        : api.projectRestoreCuts(
            project.projectHandle,
            project.revision,
            [{ startUs: sourceEndUs, endUs: sourceEndUs + deltaUs }],
            "end",
          );
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
        .projectWaveform(openedProject.projectHandle, track.descriptor.id, 0, durationUs, 512)
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
      .projectZoomSuggestions(openedProject.projectHandle, zoomConfigFor(autoZoomOptions))
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
  }, [openedProject?.projectHandle, openedProject?.revision, applyZoomGeneration, autoZoomOptions]);

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
    // A drag that ended on a handle never delivers its click here, so clear the flag it left.
    suppressSeek.current = false;
    if (event.button !== 0 || !openedProject || durationUs <= 0) return;
    rangeDrag.current = { startX: event.clientX, active: false };
  };
  const onTrackPointerMove = (event: React.PointerEvent<HTMLDivElement>) => {
    const drag = rangeDrag.current;
    if (!drag) return;
    if (!drag.active) {
      if (Math.abs(event.clientX - drag.startX) < RANGE_DRAG_PX) return;
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

  /** Clicking a clip block, in any lane, selects it and moves the playhead to the click point. */
  const onClipClick = (event: React.MouseEvent, clip: TimelineClip) => {
    event.stopPropagation();
    if (suppressSeek.current) {
      suppressSeek.current = false;
      return;
    }
    const extend = (event.shiftKey || event.ctrlKey || event.metaKey) && selection;
    if (extend) {
      selectRange(Math.min(selection.startUs, clip.startUs), Math.max(selection.endUs, clip.endUs));
    } else if (!selection || clip.startUs < selection.startUs || clip.endUs > selection.endUs) {
      // A click inside the current selection only moves the playhead.
      selectRange(clip.startUs, clip.endUs);
    }
    seekToUs(clientXToUs(event.clientX));
  };
  const clipSelected = (clip: TimelineClip) =>
    !!selection && clip.startUs >= selection.startUs && clip.endUs <= selection.endUs;

  // Drag a clip (or the selection that holds it) to another clip edge to reorder.
  type ClipMove = {
    pointerId: number;
    startX: number;
    range: { startUs: number; endUs: number };
    active: boolean;
    targetUs: number | null;
  };
  const clipMoveRef = useRef<ClipMove | null>(null);
  const [clipMove, setClipMove] = useState<ClipMove | null>(null);
  const beginClipMove = (event: React.PointerEvent<HTMLElement>, clip: TimelineClip) => {
    if (event.button !== 0 || editing || !openedProject) return;
    // The track would otherwise start a range selection from this press.
    event.stopPropagation();
    suppressSeek.current = false;
    const range = clipSelected(clip) && selection ? selection : { startUs: clip.startUs, endUs: clip.endUs };
    clipMoveRef.current = { pointerId: event.pointerId, startX: event.clientX, range, active: false, targetUs: null };
  };
  const moveClipMove = (event: React.PointerEvent<HTMLElement>) => {
    const move = clipMoveRef.current;
    if (!move || move.pointerId !== event.pointerId) return;
    if (!move.active) {
      if (Math.abs(event.clientX - move.startX) < CLIP_DRAG_PX) return;
      move.active = true;
      event.currentTarget.setPointerCapture(event.pointerId);
    }
    const pointerUs = clientXToUs(event.clientX);
    const candidates = edges.filter((edge) => edge <= move.range.startUs || edge >= move.range.endUs);
    const nearest = candidates.reduce(
      (best, edge) => (Math.abs(edge - pointerUs) < Math.abs(best - pointerUs) ? edge : best),
      candidates[0] ?? 0,
    );
    move.targetUs = nearest === move.range.startUs || nearest === move.range.endUs ? null : nearest;
    setClipMove({ ...move });
  };
  const endClipMove = (event: React.PointerEvent<HTMLElement>) => {
    const move = clipMoveRef.current;
    if (!move || move.pointerId !== event.pointerId) return;
    clipMoveRef.current = null;
    setClipMove(null);
    if (!move.active) return; // A plain click: onClick selects and seeks.
    suppressSeek.current = true; // The click that ends a drag must not select or seek.
    const target = move.targetUs;
    if (target === null) return;
    const { startUs, endUs } = move.range;
    const length = endUs - startUs;
    void runEdit((project) =>
      api.projectMoveRange(project.projectHandle, project.revision, startUs, endUs, target),
    ).then((applied) => {
      if (!applied) return;
      const movedStart = target < startUs ? target : target - length;
      selectRange(movedStart, movedStart + length);
    });
  };
  const clipMoveHandlers = (clip: TimelineClip) => ({
    onPointerDown: (event: React.PointerEvent<HTMLElement>) => beginClipMove(event, clip),
    onPointerMove: moveClipMove,
    onPointerUp: endClipMove,
    onPointerCancel: () => {
      clipMoveRef.current = null;
      setClipMove(null);
    },
  });

  const handleTimelineClick = (e: React.MouseEvent<HTMLDivElement>) => {
    if (suppressSeek.current) {
      suppressSeek.current = false;
      return;
    }
    if (!timelineTrackRef.current) return;
    // Clips stop their clicks, so a click that lands here hit empty track space: deselect.
    clearSelection();
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
            disabled={!openedProject || editing || !selection}
            onClick={toggleCamFocus}
            aria-pressed={selectionFocused}
            className={`flex items-center space-x-1.5 px-3 py-1.5 rounded-md border text-xs font-medium transition-colors disabled:opacity-40 ${
              selectionFocused
                ? "bg-amber-500/35 border-amber-300/70 text-amber-50"
                : "bg-amber-500/15 hover:bg-amber-500/25 border-amber-400/30 text-amber-200"
            }`}
            title={
              selectionFocused
                ? "Remove Cam Focus from the selection"
                : "Make the webcam fill the frame over the selection (turns Auto Webcam on). Click a focus block on the webcam lane to select it."
            }
          >
            <Video className="w-3.5 h-3.5" />
            <span>Cam Focus</span>
          </button>
          <button
            disabled={!openedProject || editing || !selection}
            onClick={toggleNormalView}
            aria-pressed={selectionInNormalView}
            className={`flex items-center space-x-1.5 px-3 py-1.5 rounded-md border text-xs font-medium transition-colors disabled:opacity-40 ${
              selectionInNormalView
                ? "bg-sky-500/30 border-sky-300/60 text-sky-100"
                : "bg-sky-500/10 hover:bg-sky-500/20 border-sky-400/30 text-sky-200"
            }`}
            title={
              selectionInNormalView
                ? "These clips use normal view. Click to let Auto Webcam go full frame here again."
                : "Keep the normal view (webcam bubble) over the selected clips, even when Auto Webcam would go full frame"
            }
          >
            <MonitorPlay className="w-3.5 h-3.5" />
            <span>Normal view</span>
          </button>
          {(pendingCount > 0 || zoomDiagnostics.length > 0 || persistedCount > 0) && (
            <span className="text-[11px] text-indigo-300/80" title="Review and change zooms in the Zoom panel">
              {pendingCount > 0
                ? `${pendingCount} zoom suggestion${pendingCount === 1 ? "" : "s"} in the Zoom panel`
                : persistedCount > 0
                  ? `${persistedCount} zoom${persistedCount === 1 ? "" : "s"}`
                  : "No auto-zoom (see the Zoom panel)"}
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
        {selection && (
          <button
            onClick={clearSelection}
            className="flex items-center gap-1 px-1.5 py-0.5 rounded border border-studio-700 text-studio-200 hover:bg-studio-800"
            title="Deselect (Esc, or click empty track space)"
          >
            <X className="w-3 h-3" />
            Clear selection
          </button>
        )}
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
        <span className="text-studio-500">Drag on the timeline to select, drag a clip to move it, drag a clip edge to trim it. S splits, Q/E ripple-trim to the previous/next edit, Delete removes the selection, Esc deselects.</span>
        {editError && <span role="alert" className="text-rose-300">{editError}</span>}
      </div>}

      {/* Multi-Track Workspace */}
      <div className="flex-1 flex min-h-0 overflow-x-hidden overflow-y-auto">
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
                className="relative px-3 flex items-center justify-between border-b border-studio-800/40 hover:bg-studio-850/50"
                style={{ height: trackHeight(track.trackType) }}
              >
                <div
                  role="separator"
                  aria-orientation="horizontal"
                  aria-label={`Resize the ${track.name} track`}
                  title="Drag to resize the track, double-click to reset"
                  className="absolute left-0 right-0 -bottom-1.5 h-3 z-10 cursor-ns-resize group flex items-center justify-center"
                  onPointerDown={(event) => {
                    if (event.button !== 0) return;
                    event.preventDefault();
                    event.currentTarget.setPointerCapture(event.pointerId);
                    trackResize.current = {
                      trackType: track.trackType,
                      startY: event.clientY,
                      startHeight: trackHeight(track.trackType),
                    };
                  }}
                  onPointerMove={(event) => {
                    const resize = trackResize.current;
                    if (!resize) return;
                    setTrackHeight(resize.trackType, resize.startHeight + event.clientY - resize.startY);
                  }}
                  onPointerUp={() => {
                    trackResize.current = null;
                  }}
                  onPointerCancel={() => {
                    trackResize.current = null;
                  }}
                  onDoubleClick={() => setTrackHeight(track.trackType, null)}
                >
                  <span className="h-1 w-10 rounded-full bg-studio-700 group-hover:bg-teal-400 transition-colors" />
                </div>
                <div className="truncate">
                  <div className="text-xs font-medium text-studio-200 truncate">{track.name}</div>
                  <div className="text-[10px] uppercase font-mono text-studio-400">
                    {track.trackType}
                  </div>
                </div>

                <TrackHeaderButtons track={track} />
              </div>
            ))}
          </div>
        </div>

        {/* Right Track Lanes & Playhead */}
        <div ref={scrollRef} className="timeline-scroll flex-1 overflow-x-auto overflow-y-hidden relative">
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
            {/* Chapter markers, in playback order, the first at 0:00 like the export */}
            {durationUs > 0 &&
              placedChapters(openedProject?.chapters ?? []).map(({ chapter, atUs }) => (
                <div
                  key={chapter.id}
                  className="absolute top-0 bottom-0 pointer-events-none z-10"
                  style={{ left: `${(atUs / durationUs) * 100}%` }}
                  title={chapter.title}
                >
                  <div className="h-full border-l border-violet-400/80" />
                  <span className="absolute top-0 left-0.5 max-w-[160px] truncate rounded-sm bg-violet-900/80 px-1 text-[9px] leading-[14px] text-violet-100">
                    {chapter.title}
                  </span>
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
            onDragOver={onMediaDragOver}
            onDragLeave={() => setMediaDropUs(null)}
            onDrop={onMediaDrop}
            onClick={handleTimelineClick}
            onPointerDown={onTrackPointerDown}
            onPointerMove={onTrackPointerMove}
            onPointerUp={onTrackPointerUp}
            onPointerCancel={onTrackPointerUp}
            className="flex-1 relative cursor-pointer py-2 space-y-2"
          >
            {/* Playhead Vertical Line */}
            <div
              className="absolute top-0 bottom-0 w-0.5 bg-indigo-500 z-30 pointer-events-none"
              style={{ left: `${progress * 100}%` }}
            >
              {/* Playhead Top Scrubber Cap */}
              <div className="w-3 h-3 bg-indigo-500 rounded-sm transform -translate-x-1/2 -top-1 absolute rotate-45 shadow-md shadow-indigo-500/50" />
            </div>
            {/* Playhead grab strip: drag the playhead itself without touching the selection. */}
            <div
              role="slider"
              aria-label="Playhead"
              aria-valuemin={0}
              aria-valuemax={durationUs}
              aria-valuenow={currentTimeUs}
              className="absolute top-0 bottom-0 w-3 -translate-x-1/2 z-40 cursor-ew-resize"
              style={{ left: `${progress * 100}%` }}
              title="Drag to move the playhead"
              onPointerDown={(event) => {
                event.stopPropagation();
                onRulerPointerDown(event);
              }}
              onPointerMove={(event) => {
                event.stopPropagation();
                onRulerPointerMove(event);
              }}
              onPointerUp={(event) => {
                event.stopPropagation();
                onRulerPointerUp();
              }}
              onPointerCancel={onRulerPointerUp}
              onClick={(event) => event.stopPropagation()}
            />

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

            {/* Clip move: the dragged range dims and a bar marks where it will land */}
            {clipMove?.active && durationUs > 0 && (
              <>
                <div
                  className="absolute top-0 bottom-0 bg-teal-300/10 border-x border-dashed border-teal-200/70 z-30 pointer-events-none"
                  style={{
                    left: `${(clipMove.range.startUs / durationUs) * 100}%`,
                    width: `${((clipMove.range.endUs - clipMove.range.startUs) / durationUs) * 100}%`,
                  }}
                />
                {clipMove.targetUs !== null && (
                  <div
                    className="absolute top-0 bottom-0 w-1 -translate-x-1/2 bg-amber-300 shadow-[0_0_8px_rgba(252,211,77,0.8)] z-40 pointer-events-none"
                    style={{ left: `${(clipMove.targetUs / durationUs) * 100}%` }}
                  >
                    <span className="absolute -top-0.5 left-1.5 text-[9px] font-mono text-amber-100 bg-studio-950/90 rounded px-1 whitespace-nowrap">
                      Move here
                    </span>
                  </div>
                )}
              </>
            )}

            {mediaDropUs !== null && durationUs > 0 && (
              <div
                className="absolute top-0 bottom-0 w-1 -translate-x-1/2 bg-fuchsia-300 shadow-[0_0_8px_rgba(240,171,252,0.8)] z-40 pointer-events-none"
                style={{ left: `${(mediaDropUs / durationUs) * 100}%` }}
              >
                <span className="absolute -top-0.5 left-1.5 text-[9px] font-mono text-fuchsia-50 bg-studio-950/90 rounded px-1 whitespace-nowrap">
                  Insert here
                </span>
              </div>
            )}

            {/* Clip lane: edges come from cuts and splits; markers restore cuts */}
            <div className="h-8 relative">
              {durationUs > 0 && clips.map((clip, index) => {
                const selected = clipSelected(clip);
                return (
                  <button
                    key={`${clip.sourceStartUs}-${index}`}
                    className={`absolute top-2.5 bottom-0 rounded border text-[9px] font-mono text-left px-1 truncate ${
                      selected
                        ? "bg-teal-500/40 border-teal-200 text-white"
                        : clip.media
                          ? "bg-fuchsia-500/20 border-fuchsia-300/50 text-fuchsia-100 hover:bg-fuchsia-500/30"
                          : "bg-teal-500/15 border-teal-400/40 text-teal-200 hover:bg-teal-500/25"
                    }`}
                    style={{
                      left: `${(clip.startUs / durationUs) * 100}%`,
                      width: `${((clip.endUs - clip.startUs) / durationUs) * 100}%`,
                    }}
                    title={`Clip ${index + 1}${clip.media ? ` (${mediaName(clip)})` : ""}: ${((clip.endUs - clip.startUs) / 1e6).toFixed(2)}s. Click to select and move the playhead, Shift+click to extend, drag to move it.`}
                    onClick={(event) => onClipClick(event, clip)}
                    {...clipMoveHandlers(clip)}
                  >
                    {clip.media ? `${index + 1} · ${mediaName(clip)}` : index + 1}
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
                    void restoreCut(marker.sourceStartUs, marker.sourceEndUs, marker.grow);
                  }}
                >
                  <span className="w-2.5 h-2 shrink-0 rounded-sm bg-rose-400 group-hover:bg-rose-200" />
                  <span className="w-0.5 flex-1 bg-rose-400 group-hover:bg-rose-200" />
                </button>
              ))}
            </div>

            {/* Individual Lanes: one block per clip, so every cut and split shows as a gap. */}
            {tracks.map((track) => (
              <div
                key={track.id}
                className="relative"
                style={{ height: trackHeight(track.trackType) }}
              >
                {durationUs > 0 && clips.map((clip, index) => {
                  const selected = clipSelected(clip);
                  const audio = track.trackType === "mic" || track.trackType === "system";
                  return (
                    <div
                      key={`${clip.sourceStartUs}-${index}`}
                      className={`absolute top-0 bottom-0 rounded-md border overflow-hidden flex items-center ${
                        selected
                          ? "bg-studio-700/80 border-teal-200 ring-1 ring-teal-200/60"
                          : clip.media
                            ? "bg-fuchsia-500/15 border-fuchsia-300/40 hover:border-fuchsia-200/70"
                            : audio
                              ? "bg-studio-800/70 border-studio-700 hover:border-studio-500"
                              : "bg-indigo-500/15 border-indigo-400/30 hover:border-indigo-300/60"
                      }`}
                      style={{
                        left: `calc(${(clip.startUs / durationUs) * 100}% + 1px)`,
                        width: `max(1px, calc(${((clip.endUs - clip.startUs) / durationUs) * 100}% - 2px))`,
                      }}
                      title={`${track.name}, clip ${index + 1}: ${((clip.endUs - clip.startUs) / 1e6).toFixed(2)}s. Drag to move it.`}
                      onClick={(event) => onClipClick(event, clip)}
                      {...clipMoveHandlers(clip)}
                    >
                      {clip.media ? (
                        // Imported media replaces the recording here: name it on the screen lane.
                        track.trackType === "screen" && (
                          <span className="px-1.5 text-[9px] font-mono text-fuchsia-100/80 truncate pointer-events-none">
                            {mediaName(clip)}
                          </span>
                        )
                      ) : track.waveform && track.waveform.buckets.length > 0 ? (
                        <div className="w-full h-full px-0.5 py-1">
                          <WaveformRenderer
                            buckets={track.waveform.buckets}
                            startUs={clip.startUs}
                            endUs={clip.endUs}
                            currentTimeUs={currentTimeUs}
                            activeBarColor={track.trackType === "mic" ? "#10b981" : "#6366f1"}
                            className="w-full h-full"
                            heightPx={trackHeight(track.trackType)}
                          />
                        </div>
                      ) : (
                        !audio && (
                          <span className="px-1.5 text-[9px] font-mono text-indigo-200/70 truncate pointer-events-none">
                            {index + 1}
                          </span>
                        )
                      )}
                    </div>
                  );
                })}

                {track.waveform && track.waveform.buckets.length === 0 && (
                  <span className="absolute inset-0 flex items-center px-4 text-xs text-studio-400 pointer-events-none">Waveform unavailable</span>
                )}

                {!track.waveform && (track.trackType === "mic" || track.trackType === "system") && (
                  <span className="absolute inset-0 flex items-center px-4 text-xs text-studio-400 pointer-events-none">Loading waveform…</span>
                )}

                {/* Auto webcam layout: where the webcam fills the frame */}
                {track.trackType === "webcam" && durationUs > 0 &&
                  (openedProject?.webcamFocus?.segments ?? []).flatMap((segment) =>
                    (segment.editedRanges ?? []).map((range, index) => (
                      <div
                        key={`${segment.id}-${index}`}
                        role="button"
                        onPointerDown={(event) => event.stopPropagation()}
                        onClick={(event) => {
                          event.stopPropagation();
                          selectRange(range.startUs, range.endUs);
                        }}
                        className={`absolute top-1 bottom-1 rounded-sm border z-10 cursor-pointer hover:ring-1 hover:ring-amber-200 ${
                          segment.enabled && openedProject?.webcamFocus?.enabled
                            ? "bg-amber-400/30 border-amber-300/70"
                            : "border-dashed border-studio-500/60"
                        }`}
                        style={{
                          left: `${(range.startUs / durationUs) * 100}%`,
                          width: `${((range.endUs - range.startUs) / durationUs) * 100}%`,
                        }}
                        title={`Webcam ${segment.enabled ? "fills the frame" : "focus off"} (${segment.source}). Click to select it, then press Cam Focus to remove it.`}
                      />
                    )),
                  )}

                {/* Normal view: the webcam keeps its bubble here whatever Auto Webcam finds */}
                {track.trackType === "webcam" && durationUs > 0 &&
                  (focus.normalView ?? []).flatMap((range) =>
                    (range.editedRanges ?? []).map((edited, index) => (
                      <div
                        key={`normal-${range.sourceStartUs}-${index}`}
                        className="absolute top-1 bottom-1 rounded-sm border border-sky-300/70 bg-sky-500/20 z-10 pointer-events-none flex items-center justify-center"
                        style={{
                          left: `${(edited.startUs / durationUs) * 100}%`,
                          width: `${((edited.endUs - edited.startUs) / durationUs) * 100}%`,
                        }}
                        title="Normal view: the webcam stays in its bubble here"
                      >
                        <span className="text-[9px] font-mono text-sky-100 truncate px-1">Normal</span>
                      </div>
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
