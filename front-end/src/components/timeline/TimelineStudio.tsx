import React, { useEffect, useLayoutEffect, useRef, useState } from "react";
import {
  ZoomIn,
  ZoomOut,
  Scissors,
  X,
  RotateCcw,
  Video,
  MonitorPlay,
  Plus,
  Eye,
  EyeOff,
  Volume2,
  VolumeX,
  Trash2,
  Link2,
  Unlink,
  Film,
  AudioLines,
  Captions,
  Mic,
  Music,
  Camera,
  Monitor,
  Magnet,
  RefreshCw,
  ChevronUp,
  ChevronDown,
  ArrowLeftToLine,
  ArrowRightToLine,
  ScanSearch,
} from "lucide-react";
import { Badge, Button, IconButton, cn } from "../ui";
import { useProjectStore } from "../../stores/projectStore";
import { useTimeline } from "../../hooks/useTimeline";
import { WaveformRenderer } from "../waveform/WaveformRenderer";
import { MEDIA_DRAG_TYPE, currentMediaDrag } from "../media/MediaPanel";
import { placedChapters } from "../chapters/ChaptersPanel";
import { TrackHeaderButtons } from "../audio/TrackHeaderButtons";
import { api } from "../../lib/ipc";
import { hotkeyHint, useHotkeyStore, type HotkeyAction } from "../../stores/hotkeyStore";
import { saveTrackMix } from "../../lib/trackMix";
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
  type EditedSpan as EditedRangeSpan,
  type CaptionEdit,
  type CaptionTrackView,
  type WaveformBucket,
  type OpenedProject,
  type OverlayClip,
  type ProjectZoom,
  type TrackEdit,
  type ZoomKeyframe,
  type MainTrack,
  DEFAULT_MAIN_TRACK,
  DEFAULT_ZOOM_SETTINGS,
} from "../../lib/types";
import {
  audioStreamCount,
  audioStreamName,
  audioTracks as audioTracksOf,
  clipEndUs,
  defaultClipUs,
  fitsOnTrack,
  isAudioTrack,
  laneRole,
  soundLanes,
  rowAtPoint,
  rowFromElement,
  sameRow,
  snapStart,
  trackLabel,
  trimClip,
  videoTracks as videoTracksOf,
  type TrackRow,
} from "../../lib/trackUtils";

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
/** Pointer travel, in pixels, before a Shift+drag on the ruler becomes a range selection. */
const RANGE_DRAG_PX = 4;

/** Track lane heights, per track type, remembered on this machine. */
const TRACK_HEIGHT_KEY = "aeroedits.trackHeights.v1";
const DEFAULT_TRACK_HEIGHT = 56;
const MIN_TRACK_HEIGHT = 28;
const MAX_TRACK_HEIGHT = 240;
/** Height of a video or audio track, and of the "new track" drop row. */
const OVERLAY_ROW_PX = 48;
/** Height of a lane showing the sound of the main sequence's imported clips. */
const LINKED_SOUND_ROW_PX = 32;
const NEW_TRACK_ROW_PX = 28;
/** The zoom track's row. */
const ZOOM_ROW_PX = 36;
/** The main track's (V1's) clip row. */
const MAIN_ROW_PX = 40;
/** A zoom is never dragged shorter than this. */
const MIN_ZOOM_US = 300_000;

/** A button on a track header. */
const HDR_BUTTON =
  "h-7 w-7 shrink-0 inline-flex items-center justify-center rounded-control text-studio-400 hover:text-studio-100 hover:bg-studio-800 disabled:opacity-40 transition-colors";

/** The colour strip on a track header: what kind of track it is. */
const TRACK_TONE = {
  video: "bg-video",
  audio: "bg-audio",
  caption: "bg-caption",
  zoom: "bg-zoom",
} as const;

const RECORDING_TRACK = {
  screen: { label: "Screen recording", icon: Monitor, tone: "video" },
  webcam: { label: "Camera", icon: Camera, tone: "video" },
  mic: { label: "Microphone", icon: Mic, tone: "audio" },
  system: { label: "System audio", icon: Volume2, tone: "audio" },
} as const;

/** A divider between groups of toolbar controls. */
const ToolbarDivider: React.FC = () => <span className="mx-1 h-5 w-px shrink-0 bg-studio-800" aria-hidden />;

/** A toolbar button that stays lit while its mode is on. */
const ToolbarToggle: React.FC<
  React.ComponentProps<typeof Button> & { on: boolean }
> = ({ on, className, ...rest }) => (
  <Button
    variant="ghost"
    aria-pressed={on}
    className={cn(on && "!bg-accent/15 !text-accent-fg !border-accent/40", className)}
    {...rest}
  />
);

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
/** A selected clip: one on the main sequence (by its place on the timeline), or on a track. */
type ClipRef = { kind: "main"; startUs: number; endUs: number } | { kind: "track"; clipId: string };
type EditedSpan = { startUs: number; endUs: number };

interface EdgeDrag {
  clip: TimelineClip;
  side: "start" | "end";
  startX: number;
  minDeltaUs: number;
  maxDeltaUs: number;
  deltaUs: number;
  pointerId: number;
}

/**
 * The editor's timeline. With `scope` it opens on that stretch of the edit (a short's) and
 * dims the rest; every edit still goes to the whole project.
 */
export const TimelineStudio: React.FC<{ scope?: { startUs: number; endUs: number } }> = ({ scope }) => {
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

  const { isPlaying, togglePlayPause, seekToUs } = useTimeline();
  const frameUs = Math.round(
    1e6 / (openedProject?.manifest.tracks.find((track) => track.trackType === "screen")?.fps || 30),
  );

  // Range selection (Shift+drag on the ruler, or mark in/out): Cam Focus, Normal view and
  // Delete / Keep only work on it.
  const [range, setRange] = useState<EditedSpan | null>(null);
  // Clip selection: click a clip, Shift/Ctrl+click to add or remove one.
  const [selectedClips, setSelectedClips] = useState<ClipRef[]>([]);
  const [editError, setEditError] = useState<string>();
  const bindings = useHotkeyStore((s) => s.bindings);
  const hint = (action: HotkeyAction) => hotkeyHint(bindings, action);
  const [editing, setEditing] = useState(false);
  const selectedZoomId = useProjectStore((s) => s.selectedZoomId);
  const setSelectedZoomId = useProjectStore((s) => s.setSelectedZoomId);
  const setTimelineSelection = useProjectStore((s) => s.setTimelineSelection);
  const setSelectedOverlayClipId = useProjectStore((s) => s.setSelectedOverlayClipId);
  // The project's auto-zoom settings: the same in every window.
  const zoomSettings = openedProject?.zoomSettings ?? DEFAULT_ZOOM_SETTINGS;
  const [zoomBusy, setZoomBusy] = useState(false);
  const dragging = useRef<{
    mode: DragMode;
    zoomId: string;
    startX: number;
    originStart: number;
    originEnd: number;
  } | null>(null);
  /** The zoom being dragged on the zoom track, and how far. */
  const [zoomDrag, setZoomDrag] = useState<{ barId: string; mode: DragMode; deltaUs: number } | null>(null);
  const suppressSeek = useRef(false);
  const [trackHeights, setTrackHeights] = useState(loadTrackHeights);
  /** A lane's height: as resized (remembered per kind of lane), else `fallback`. */
  const trackHeight = (trackType: string, fallback = DEFAULT_TRACK_HEIGHT) => trackHeights[trackType] ?? fallback;
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
  /** The grip under a lane header: drag to resize every lane of its kind, double-click to reset. */
  const resizeGrip = (key: string, label: string, fallback = DEFAULT_TRACK_HEIGHT) => (
    <div
      role="separator"
      aria-orientation="horizontal"
      aria-label={`Resize ${label}`}
      title="Drag to resize, double-click to reset"
      className="absolute left-0 right-0 -bottom-1.5 h-3 z-10 cursor-ns-resize group flex items-center justify-center"
      onPointerDown={(event) => {
        if (event.button !== 0) return;
        event.preventDefault();
        event.currentTarget.setPointerCapture(event.pointerId);
        trackResize.current = { trackType: key, startY: event.clientY, startHeight: trackHeight(key, fallback) };
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
      onDoubleClick={() => setTrackHeight(key, null)}
    >
      <span className="h-1 w-10 rounded-full bg-studio-700 group-hover:bg-accent-hover transition-colors" />
    </div>
  );
  useEffect(() => {
    setRange(null);
    setSelectedClips([]);
    setEditError(undefined);
  }, [openedProject?.projectHandle]);
  /** Runs one edit; resolves to whether it was applied. */
  const runEdit = async (work: (project: OpenedProject) => Promise<OpenedProject>) => {
    if (!openedProject || editing) return false;
    setEditing(true); setEditError(undefined);
    try { applyOpenedProject(await work(openedProject)); return true; }
    catch (err) { setEditError(String(err)); return false; }
    finally { setEditing(false); }
  };
  const tracksEdit = (edit: TrackEdit) =>
    runEdit((project) => api.projectTracksEdit(project.projectHandle, project.revision, edit));
  /** Delete / Keep only on the range: every track loses the same time. */
  const editRange = async (trim: boolean) => {
    if (!openedProject || editing || !range) return;
    const { startUs, endUs } = range;
    const cuts = trim ? [
      ...(startUs > 0 ? [{ startUs: 0, endUs: startUs }] : []),
      ...(endUs < durationUs ? [{ startUs: endUs, endUs: durationUs }] : []),
    ] : [{startUs, endUs}];
    if (!cuts.length) return;
    if (await tracksEdit({ kind: "rippleDelete", ranges: cuts, allTracks: true })) {
      setRange(null);
    }
  };
  /** Every clip under `atUs` on the tracks beside V1. */
  const trackClipsAt = (atUs: number) =>
    overlayTracks.flatMap((t) => t.clips.filter((c) => c.startUs < atUs && atUs < clipEndUs(c)).map((c) => c.id));
  // S: selected clips split alone; with nothing selected, V1 and every track split together.
  const splitAtPlayhead = () => {
    const at = currentTimeUs;
    if (selectedClips.length > 0) {
      const main = selectedMain.some((r) => r.startUs < at && at < r.endUs);
      const clipIds = selectedTrackIds.filter((id) => trackClipsAt(at).includes(id));
      if (!main && clipIds.length === 0) {
        setEditError("Put the playhead over a selected clip to split it, or deselect (Ctrl+D) to split everything.");
        return;
      }
      return tracksEdit({ kind: "split", atUs: at, main, clipIds });
    }
    return tracksEdit({ kind: "split", atUs: at, main: at > 0 && at < durationUs, clipIds: trackClipsAt(at) });
  };
  /** Puts cut media back; with `shiftAtUs`, the other tracks move right with it. */
  const restoreCut = (startUs: number, endUs: number, grow: "end" | "start" = "end", shiftAtUs?: number) =>
    tracksEdit({ kind: "restore", ranges: [{ startUs, endUs }], grow, shiftTracksAt: shiftAtUs ?? null });
  // Q and E: ripple-delete from the playhead to the previous or next edit point. A selected
  // clip is trimmed alone; with nothing selected, every track loses the same time.
  const rippleTrim = (side: "previous" | "next") => {
    const at = currentTimeUs;
    if (selectedClips.length > 1) {
      setEditError("Select one clip to trim, or deselect (Ctrl+D) to trim every track.");
      return;
    }
    const trackId = selectedTrackIds[0];
    if (trackId) {
      return tracksEdit({ kind: "rippleTrimClip", clipId: trackId, side: side === "previous" ? "start" : "end", atUs: at });
    }
    let cut: EditedRangeSpan;
    let allTracks = true;
    const clip = selectedMain[0];
    if (clip) {
      if (!(clip.startUs < at && at < clip.endUs)) {
        setEditError("Put the playhead inside the selected clip to trim it.");
        return;
      }
      cut = side === "previous" ? { startUs: clip.startUs, endUs: at } : { startUs: at, endUs: clip.endUs };
      allTracks = false;
    } else {
      const points = [0, durationUs, ...edges, ...overlayTracks.flatMap((t) => t.clips.flatMap((c) => [c.startUs, clipEndUs(c)]))];
      const before = Math.max(...points.filter((p) => p < at));
      const after = Math.min(...points.filter((p) => p > at && p <= durationUs));
      if (side === "previous" ? !Number.isFinite(before) : !Number.isFinite(after)) return;
      cut = side === "previous" ? { startUs: before, endUs: at } : { startUs: at, endUs: after };
    }
    return tracksEdit({ kind: "rippleDelete", ranges: [cut], allTracks }).then((applied) => {
      // Q pulls the later media back to the cut's start; the playhead follows it.
      if (applied && side === "previous") seekToUs(cut.startUs);
      return applied;
    });
  };
  const undo = () => {
    if (openedProject?.undoAvailable) void runEdit((project) => api.projectUndo(project.projectHandle, project.revision));
  };
  const redo = () => {
    if (openedProject?.redoAvailable) void runEdit((project) => api.projectRedo(project.projectHandle, project.revision));
  };

  const selectRange = (startUs: number, endUs: number) => {
    const a = Math.max(0, Math.min(startUs, endUs));
    const b = Math.min(durationUs, Math.max(startUs, endUs));
    setRange(b > a ? { startUs: a, endUs: b } : null);
  };

  const retained = openedProject?.retainedIntervals ?? [];
  const clips = buildClips(retained, openedProject?.splitPointsUs);
  const overlayTracks = openedProject?.overlayTracks ?? [];
  const videoTracks = videoTracksOf(overlayTracks);
  const audioTracks = audioTracksOf(overlayTracks);
  // V1 as a track: magnetic (cuts close up, moves insert) or not (gaps, overwrite), and where
  // it sits among the video tracks: those above it list first, those below after its rows.
  const mainTrack = openedProject?.mainTrack ?? DEFAULT_MAIN_TRACK;
  const magnetic = mainTrack.magnetic;
  const mainPosition = Math.min(mainTrack.position, videoTracks.length);
  const videoAbove = videoTracks.slice(mainPosition).reverse();
  const videoBelow = videoTracks.slice(0, mainPosition).reverse();
  const trackOfClip = (clipId: string) => overlayTracks.find((t) => t.clips.some((c) => c.id === clipId));
  const findTrackClip = (clipId: string) => trackOfClip(clipId)?.clips.find((c) => c.id === clipId);

  const selectedMain = selectedClips.filter((ref): ref is Extract<ClipRef, { kind: "main" }> => ref.kind === "main");
  const selectedTrackIds = selectedClips.flatMap((ref) => (ref.kind === "track" ? [ref.clipId] : []));
  const clipSelected = (clip: EditedSpan) =>
    selectedMain.some((ref) => ref.startUs === clip.startUs && ref.endUs === clip.endUs);
  const trackClipSelected = (clipId: string) => selectedTrackIds.includes(clipId);
  /** Clicking selects one clip; Shift/Ctrl/Cmd+click adds it to (or takes it out of) the selection. */
  const pickClip = (ref: ClipRef, additive: boolean) => {
    const same = (other: ClipRef) =>
      ref.kind === "main"
        ? other.kind === "main" && other.startUs === ref.startUs && other.endUs === ref.endUs
        : other.kind === "track" && other.clipId === ref.clipId;
    setSelectedClips((current) =>
      additive ? (current.some(same) ? current.filter((o) => !same(o)) : [...current, ref]) : [ref],
    );
    if (ref.kind === "track") setSelectedOverlayClipId(ref.clipId);
    else if (!additive) setSelectedOverlayClipId(undefined);
    setSelectedZoomId(undefined);
  };
  const clearSelection = () => {
    setRange(null);
    setSelectedClips([]);
  };
  // After an edit, keep the selected clips that still exist; a pending span selects the clips
  // inside it (a moved block lands somewhere new).
  const pendingSelect = useRef<EditedSpan | null>(null);
  useEffect(() => {
    const pending = pendingSelect.current;
    pendingSelect.current = null;
    if (pending) {
      setSelectedClips(
        clips
          .filter((clip) => clip.startUs >= pending.startUs && clip.endUs <= pending.endUs)
          .map((clip) => ({ kind: "main", startUs: clip.startUs, endUs: clip.endUs })),
      );
      return;
    }
    setSelectedClips((current) => {
      const kept = current.filter((ref) =>
        ref.kind === "main"
          ? clips.some((clip) => clip.startUs === ref.startUs && clip.endUs === ref.endUs)
          : !!findTrackClip(ref.clipId),
      );
      return kept.length === current.length ? current : kept;
    });
    setRange((current) => (current && current.endUs > durationUs ? null : current));
  }, [openedProject?.revision]);

  // The selected main clips, when they sit side by side: they move as one block.
  const selectedBlock = (() => {
    if (selectedMain.length === 0 || selectedTrackIds.length > 0) return null;
    const sorted = [...selectedMain].sort((a, b) => a.startUs - b.startUs);
    for (let i = 1; i < sorted.length; i++) if (sorted[i].startUs !== sorted[i - 1].endUs) return null;
    return { startUs: sorted[0].startUs, endUs: sorted[sorted.length - 1].endUs };
  })();
  // What Cam Focus, Normal view and the Zoom panel act on: the range, else the selected clips.
  const selection = range ?? selectedBlock;
  useEffect(() => {
    setTimelineSelection(selection);
  }, [selection?.startUs, selection?.endUs, setTimelineSelection]);
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

  // Video tracks above the main sequence (V2, V3, ...) and audio tracks below it (A1, A2, ...).
  const selectedOverlayClipId = useProjectStore((s) => s.selectedOverlayClipId);
  const assetOf = (assetId: string) => openedProject?.mediaAssets?.find((asset) => asset.id === assetId);
  useEffect(() => {
    if (selectedOverlayClipId && !overlayTracks.some((t) => t.clips.some((c) => c.id === selectedOverlayClipId))) {
      setSelectedOverlayClipId(undefined);
    }
  }, [openedProject?.revision, selectedOverlayClipId]);
  const editTracks = (edit: TrackEdit) =>
    runEdit((project) => api.projectTracksEdit(project.projectHandle, project.revision, edit));
  /** Runs a track edit on `row`; the "new track" row first adds a track on top (its own undo step). */
  const editOnRow = (row: TrackRow, makeEdit: (trackId: string) => TrackEdit) =>
    runEdit(async (project) => {
      let current = project;
      let trackId: string;
      if (row.kind === "track") {
        trackId = row.trackId;
      } else if (row.kind === "new") {
        current = await api.projectTracksEdit(current.projectHandle, current.revision, {
          kind: "addTrack",
          audio: row.audio,
        });
        const added = (current.overlayTracks ?? []).filter((t) => isAudioTrack(t) === row.audio);
        trackId = added[added.length - 1]?.id ?? "";
      } else {
        throw new Error("Clips on V1 are inserted, not placed");
      }
      try {
        return await api.projectTracksEdit(current.projectHandle, current.revision, makeEdit(trackId));
      } catch (err) {
        // The new track stays; show it while reporting why the clip could not go on it.
        if (current !== project) applyOpenedProject(current);
        throw err;
      }
    });
  /** Edges a dragged block snaps to: the playhead, the main clips and other track clips. */
  const snapPoints = (ignoreId?: string) => [
    currentTimeUs,
    ...edges,
    ...overlayTracks.flatMap((track) =>
      track.clips.filter((clip) => clip.id !== ignoreId).flatMap((clip) => [clip.startUs, clipEndUs(clip)]),
    ),
  ];
  // Read in event handlers only, after the timeline scale below is known.
  const snapUs = () => (pxPerUs > 0 ? SNAP_PX / pxPerUs : 0);
  /** Whether a block of `lengthUs` can land at `startUs` on `row`; `kinds` are the track kinds it may go on. */
  const placeable = (
    row: TrackRow,
    startUs: number,
    lengthUs: number,
    kinds: { video: boolean; audio: boolean },
    ignoreId?: string,
  ) => {
    if (startUs >= durationUs) return false;
    if (row.kind === "new") return row.audio ? kinds.audio : kinds.video;
    if (row.kind !== "track") return false;
    const track = overlayTracks.find((t) => t.id === row.trackId);
    if (!(isAudioTrack(track) ? kinds.audio : kinds.video)) return false;
    return fitsOnTrack(track, startUs, lengthUs, ignoreId);
  };
  /** Where media from the bin may go: sound onto audio tracks, pictures onto video tracks. */
  const assetKinds = (asset: { kind: string } & Parameters<typeof audioStreamCount>[0]) => ({
    video: asset?.kind !== "audio",
    audio: audioStreamCount(asset) > 0,
  });

  // Dropping a Media panel item on the main track inserts it at the nearest clip edge; on a
  // track above, it lands where it is dropped.
  const [mediaDropUs, setMediaDropUs] = useState<number | null>(null);
  const [mediaGhost, setMediaGhost] = useState<{ row: TrackRow; startUs: number; durationUs: number; valid: boolean } | null>(null);
  const nearestEdge = (clientX: number) => {
    const pointerUs = clientXToUs(clientX);
    if (!magnetic) {
      const near = snapPoints().reduce((best, p) => (Math.abs(p - pointerUs) < Math.abs(best - pointerUs) ? p : best), Infinity);
      return Math.max(0, Math.abs(near - pointerUs) <= snapUs() ? near : Math.round(pointerUs));
    }
    return edges.reduce((best, edge) => (Math.abs(edge - pointerUs) < Math.abs(best - pointerUs) ? edge : best), edges[0] ?? 0);
  };
  const onMediaDragOver = (event: React.DragEvent<HTMLDivElement>) => {
    if (!event.dataTransfer.types.includes(MEDIA_DRAG_TYPE) || !openedProject) return;
    event.preventDefault();
    event.dataTransfer.dropEffect = "copy";
    const row = rowFromElement(event.target as Element);
    const asset = assetOf(currentMediaDrag() ?? "");
    if (row && row.kind !== "main" && asset) {
      const durationUs = defaultClipUs(asset);
      const startUs = snapStart(clientXToUs(event.clientX), durationUs, snapPoints(), snapUs());
      setMediaDropUs(null);
      setMediaGhost({ row, startUs, durationUs, valid: placeable(row, startUs, durationUs, assetKinds(asset)) });
    } else {
      setMediaGhost(null);
      setMediaDropUs(nearestEdge(event.clientX));
    }
  };
  const onMediaDrop = (event: React.DragEvent<HTMLDivElement>) => {
    const assetId = event.dataTransfer.getData(MEDIA_DRAG_TYPE);
    const ghost = mediaGhost;
    setMediaDropUs(null);
    setMediaGhost(null);
    if (!assetId || !openedProject) return;
    event.preventDefault();
    if (ghost) {
      if (!ghost.valid) {
        setEditError("That media cannot go there: the track is the wrong kind, or another clip is in the way.");
        return;
      }
      void editOnRow(ghost.row, (trackId) => ({ kind: "placeMedia", assetId, trackId, startUs: ghost.startUs }));
      return;
    }
    const target = nearestEdge(event.clientX);
    void tracksEdit({ kind: "insertMedia", assetId, targetUs: target });
  };
  const jumpToEdit = (direction: -1 | 1) => {
    const target =
      direction < 0
        ? [...edges].reverse().find((edge) => edge < currentTimeUs)
        : edges.find((edge) => edge > currentTimeUs && edge < durationUs);
    if (target !== undefined) seekToUs(target);
  };

  /** Delete: the selected clips in one undo step (V1 clips close up, track clips go), else the range. */
  const deleteSelection = async () => {
    if (selectedClips.length === 0) {
      if (range) await editRange(false);
      else if (selectedZoomId) deleteZoom(selectedZoomId);
      return;
    }
    const ranges = selectedMain.map(({ startUs, endUs }) => ({ startUs, endUs }));
    const clipIds = [...selectedTrackIds];
    if (await tracksEdit({ kind: "deleteSelection", ranges, clipIds })) {
      setSelectedClips([]);
      setSelectedOverlayClipId(undefined);
    }
  };

  /**
   * U: a selected picture with its sound attached is unlinked (its sound goes onto audio
   * tracks); a picture selected together with its unlinked sound is relinked.
   */
  const toggleLink = () => {
    const sounds = selectedTrackIds.filter((id) => findTrackClip(id)?.audioStream !== undefined);
    const pictures: ({ kind: "main"; startUs: number; endUs: number; assetId: string; unlinked: boolean } | { kind: "track"; clipId: string; assetId: string; unlinked: boolean })[] = [
      ...selectedMain.flatMap((ref) => {
        const clip = clips.find((c) => c.startUs === ref.startUs && c.endUs === ref.endUs);
        return clip?.media ? [{ ...ref, assetId: clip.media, unlinked: !!clip.audioUnlinked }] : [];
      }),
      ...selectedTrackIds.flatMap((clipId) => {
        const clip = findTrackClip(clipId);
        return clip && clip.audioStream === undefined
          ? [{ kind: "track" as const, clipId, assetId: clip.assetId, unlinked: !!clip.audioUnlinked }]
          : [];
      }),
    ];
    if (pictures.length !== 1) {
      setEditError(
        selectedMain.length > 0 && pictures.length === 0
          ? "The recording's sound is on its own tracks already. Unlink works on imported clips."
          : "Select one imported clip (and, to relink, its sound) and press U.",
      );
      return;
    }
    const picture = pictures[0];
    if (!picture.unlinked) {
      if (audioStreamCount(assetOf(picture.assetId)) === 0) {
        setEditError("That clip has no sound to unlink.");
        return;
      }
      void editTracks(
        picture.kind === "main"
          ? { kind: "unlinkMain", startUs: picture.startUs, endUs: picture.endUs }
          : { kind: "unlinkClip", clipId: picture.clipId },
      );
      return;
    }
    if (sounds.length === 0) {
      setEditError("Its sound is unlinked. Select the clip and its sound together, then press U to relink.");
      return;
    }
    void editTracks(
      picture.kind === "main"
        ? { kind: "relinkMain", startUs: picture.startUs, endUs: picture.endUs, audioClipIds: sounds }
        : { kind: "relinkClip", clipId: picture.clipId, audioClipIds: sounds },
    );
  };
  const linkState = (() => {
    const one = selectedMain.length + selectedTrackIds.filter((id) => findTrackClip(id)?.audioStream === undefined).length === 1;
    if (!one) return null;
    const main = selectedMain[0] && clips.find((c) => c.startUs === selectedMain[0].startUs && c.endUs === selectedMain[0].endUs);
    if (main) return main.media ? (main.audioUnlinked ? "unlinked" : "linked") : null;
    const clip = selectedTrackIds.map(findTrackClip).find((c) => c && c.audioStream === undefined);
    return clip ? (clip.audioUnlinked ? "unlinked" : "linked") : null;
  })();

  /** I and O: the range starts or ends at the playhead. */
  const markIn = () => selectRange(currentTimeUs, range && range.endUs > currentTimeUs ? range.endUs : durationUs);
  const markOut = () => selectRange(range && range.startUs < currentTimeUs ? range.startUs : 0, currentTimeUs);

  const actions = useRef<Partial<Record<HotkeyAction, () => void>>>({});
  actions.current = {
    playPause: togglePlayPause,
    split: () => (cueAtSelection ? splitCue() : void splitAtPlayhead()),
    rippleTrimPrevious: () => void rippleTrim("previous"),
    rippleTrimNext: () => void rippleTrim("next"),
    deleteSelection: () => (cueAtSelection ? hideCue() : void deleteSelection()),
    deselect: () => {
      setSelectedOverlayClipId(undefined);
      setSelectedCue(null);
      clearSelection();
    },
    selectAll: () => {
      setRange(null);
      setSelectedClips([
        ...clips.map((clip): ClipRef => ({ kind: "main", startUs: clip.startUs, endUs: clip.endUs })),
        ...overlayTracks.flatMap((t) => t.clips.map((c): ClipRef => ({ kind: "track", clipId: c.id }))),
      ]);
    },
    deselectAll: () => {
      setSelectedOverlayClipId(undefined);
      setSelectedClips([]);
    },
    toggleLink,
    markIn,
    markOut,
    undo,
    redo,
    stepBack: () => seekToUs(currentTimeUs - frameUs),
    stepForward: () => seekToUs(currentTimeUs + frameUs),
    stepBackLong: () => seekToUs(currentTimeUs - 1_000_000),
    stepForwardLong: () => seekToUs(currentTimeUs + 1_000_000),
    previousEdit: () => jumpToEdit(-1),
    nextEdit: () => jumpToEdit(1),
    zoomIn: () => zoomRef.current(2),
    zoomOut: () => zoomRef.current(0.5),
    goToStart: () => seekToUs(0),
    goToEnd: () => seekToUs(Number.MAX_SAFE_INTEGER),
  };
  // Actions that act once per press; the rest (stepping, zoom) repeat while the key is held.
  const ONCE: HotkeyAction[] = ["playPause", "split", "rippleTrimPrevious", "rippleTrimNext", "toggleLink", "deleteSelection", "selectAll", "deselectAll"];
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement | null;
      if (target?.closest("input, textarea, select, [contenteditable='true']")) return;
      const action = useHotkeyStore.getState().actionFor(event);
      if (!action) return;
      event.preventDefault();
      if (event.repeat && ONCE.includes(action)) return;
      actions.current[action]?.();
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);

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

  // A scoped timeline frames its stretch, with a little room either side.
  useEffect(() => {
    if (!scope || viewportPx <= 0 || durationUs <= 0) return;
    const length = Math.max(1, scope.endUs - scope.startUs);
    const pad = length * 0.05;
    const zoom = Math.min(MAX_TIMELINE_ZOOM, Math.max(MIN_TIMELINE_ZOOM, durationUs / (length * 1.1)));
    const left = Math.max(0, scope.startUs - pad);
    if (Math.abs(zoom - timelineZoom) > 1e-6) {
      zoomAnchor.current = { timeUs: left, offsetPx: 0 };
      setTimelineZoom(zoom);
    } else if (scrollRef.current) {
      scrollRef.current.scrollLeft = left * pxPerUs;
    }
  }, [scope?.startUs, scope?.endUs, viewportPx, durationUs]);

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
  const rangeDrag = useRef<{ startX: number; active: boolean } | null>(null);
  const onRulerPointerDown = (event: React.PointerEvent<HTMLDivElement>) => {
    if (event.button !== 0 || !openedProject || durationUs <= 0) return;
    event.currentTarget.setPointerCapture(event.pointerId);
    if (event.shiftKey) {
      rangeDrag.current = { startX: event.clientX, active: false };
      return;
    }
    scrub.current = { frame: null, targetUs: 0 };
    scrubTo(event.clientX);
  };
  const onRulerPointerMove = (event: React.PointerEvent<HTMLDivElement>) => {
    const drag = rangeDrag.current;
    if (drag) {
      if (!drag.active && Math.abs(event.clientX - drag.startX) < RANGE_DRAG_PX) return;
      drag.active = true;
      selectRange(clientXToUs(drag.startX), clientXToUs(event.clientX));
      return;
    }
    scrubTo(event.clientX);
  };
  const onRulerPointerUp = () => {
    if (rangeDrag.current) {
      rangeDrag.current = null;
      return;
    }
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
    // A selected clip is trimmed alone; otherwise the other tracks keep in step.
    const allTracks = !clipSelected(clip);
    const ripple = (startUs: number, endUs: number) =>
      tracksEdit({ kind: "rippleDelete", ranges: [{ startUs, endUs }], allTracks });
    if (side === "start") {
      void (deltaUs > 0
        ? ripple(clip.startUs, clip.startUs + deltaUs)
        : restoreCut(clip.sourceStartUs + deltaUs, clip.sourceStartUs, "start", allTracks || !magnetic ? clip.startUs : undefined));
      return;
    }
    void (deltaUs < 0
      ? ripple(clip.endUs + deltaUs, clip.endUs)
      : restoreCut(sourceEndUs, sourceEndUs + deltaUs, "end", allTracks || !magnetic ? clip.endUs : undefined));
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


  const patchPersisted = (zoom: ProjectZoom, sourceStartUs: number, sourceEndUs: number) => {
    if (!openedProject) return;
    const duration = sourceEndUs - sourceStartUs;
    if (duration < 3) {
      setEditError("Zoom range is too short.");
      return;
    }
    // The auto-zoom transition, as long as in and out both fit.
    const transitionUs = Math.max(1, Math.min(Math.round(zoomSettings.transitionMs * 1000), Math.floor((duration - 1) / 2)));
    void persistZoom(() =>
      api.projectZoomUpdate(openedProject.projectHandle, openedProject.revision, {
        ...zoom,
        sourceStartUs,
        sourceEndUs,
        transitionUs,
      }),
    );
  };

  const clientXToUs = (clientX: number) => {
    const rect = timelineTrackRef.current?.getBoundingClientRect();
    if (!rect || rect.width <= 0) return 0;
    return Math.round(Math.max(0, Math.min(1, (clientX - rect.left) / rect.width)) * durationUs);
  };
  const onTrackPointerDown = () => {
    // A drag that ended on a handle never delivers its click here, so clear the flag it left.
    suppressSeek.current = false;
  };

  /** Clicking a clip selects it; Shift/Ctrl+click adds or removes it. The playhead stays put. */
  const onClipClick = (event: React.MouseEvent, clip: TimelineClip) => {
    event.stopPropagation();
    if (suppressSeek.current) {
      suppressSeek.current = false;
      return;
    }
    setRange(null);
    setSelectedCue(null);
    pickClip(
      { kind: "main", startUs: clip.startUs, endUs: clip.endUs },
      event.shiftKey || event.ctrlKey || event.metaKey,
    );
  };

  // Drag a clip (or the selection that holds it) to another clip edge to reorder.
  type ClipMove = {
    pointerId: number;
    startX: number;
    startY: number;
    range: { startUs: number; endUs: number };
    /** The clips that move: the selection when the pressed clip is in it. */
    ranges: EditedRangeSpan[];
    active: boolean;
    targetUs: number | null;
    /** Pointer time minus the range start at the press. */
    grabUs: number;
    /** Imported media dragged up onto a track: where it would land. */
    lift: { row: TrackRow; startUs: number; valid: boolean } | null;
  };
  const clipMoveRef = useRef<ClipMove | null>(null);
  const [clipMove, setClipMove] = useState<ClipMove | null>(null);
  const beginClipMove = (event: React.PointerEvent<HTMLElement>, clip: TimelineClip) => {
    if (event.button !== 0 || editing || !openedProject) return;
    // The track would otherwise start a range selection from this press.
    event.stopPropagation();
    suppressSeek.current = false;
    const ranges =
      clipSelected(clip) && selectedMain.length > 1
        ? [...selectedMain].sort((a, b) => a.startUs - b.startUs).map(({ startUs, endUs }) => ({ startUs, endUs }))
        : [{ startUs: clip.startUs, endUs: clip.endUs }];
    const range = { startUs: ranges[0].startUs, endUs: ranges[ranges.length - 1].endUs };
    // Captured at once: a drag straight up to another track leaves the block within a few pixels.
    event.currentTarget.setPointerCapture(event.pointerId);
    clipMoveRef.current = {
      pointerId: event.pointerId,
      startX: event.clientX,
      startY: event.clientY,
      range,
      ranges,
      active: false,
      targetUs: null,
      grabUs: clientXToUs(event.clientX) - range.startUs,
      lift: null,
    };
  };
  const moveClipMove = (event: React.PointerEvent<HTMLElement>) => {
    const move = clipMoveRef.current;
    if (!move || move.pointerId !== event.pointerId) return;
    if (!move.active) {
      if (Math.max(Math.abs(event.clientX - move.startX), Math.abs(event.clientY - move.startY)) < CLIP_DRAG_PX) return;
      move.active = true;
    }
    const pointerUs = clientXToUs(event.clientX);
    // One whole imported clip can go up onto a track; the recording stays on V1.
    const row = rowAtPoint(event.clientX, event.clientY);
    const media = clips.find(
      (clip) => clip.media && clip.startUs === move.range.startUs && clip.endUs === move.range.endUs,
    );
    if (row && row.kind !== "main") {
      const length = move.range.endUs - move.range.startUs;
      const startUs = snapStart(pointerUs - move.grabUs, length, snapPoints(), snapUs());
      move.targetUs = null;
      move.lift = media
        ? { row, startUs, valid: placeable(row, startUs, length, { video: true, audio: false }) }
        : { row, startUs, valid: false };
      setClipMove({ ...move });
      return;
    }
    move.lift = null;
    if (!magnetic) {
      const length = move.ranges.reduce((sum, r) => sum + r.endUs - r.startUs, 0);
      const start = Math.max(0, snapStart(pointerUs - move.grabUs, length, snapPoints(), snapUs()));
      move.targetUs = start === move.range.startUs && move.ranges.length === 1 ? null : start;
      setClipMove({ ...move });
      return;
    }
    const candidates = edges.filter((edge) => move.ranges.every((r) => edge <= r.startUs || edge >= r.endUs));
    const nearest = candidates.reduce(
      (best, edge) => (Math.abs(edge - pointerUs) < Math.abs(best - pointerUs) ? edge : best),
      candidates[0] ?? 0,
    );
    const stays = move.ranges.length === 1 && (nearest === move.range.startUs || nearest === move.range.endUs);
    move.targetUs = stays ? null : nearest;
    setClipMove({ ...move });
  };
  const endClipMove = (event: React.PointerEvent<HTMLElement>) => {
    const move = clipMoveRef.current;
    if (!move || move.pointerId !== event.pointerId) return;
    clipMoveRef.current = null;
    setClipMove(null);
    if (!move.active) return; // A plain click: onClick selects and seeks.
    suppressSeek.current = true; // The click that ends a drag must not select or seek.
    const lift = move.lift;
    if (lift) {
      const { startUs, endUs } = move.range;
      const isMedia = clips.some((clip) => clip.media && clip.startUs === startUs && clip.endUs === endUs);
      if (!isMedia) {
        setEditError("The recording stays on V1. Imported media can move to the tracks above.");
      } else if (!lift.valid) {
        setEditError("It can't go there: pictures go on video tracks, and another clip may be in the way.");
      } else {
        clearSelection();
        void editOnRow(lift.row, (trackId) => ({ kind: "liftFromMain", startUs, endUs, trackId, atUs: lift.startUs }));
      }
      return;
    }
    const target = move.targetUs;
    if (target === null) return;
    if (!magnetic) {
      // Not magnetic: the clips land at the target over what is there, leaving gaps behind.
      const length = move.ranges.reduce((sum, r) => sum + r.endUs - r.startUs, 0);
      pendingSelect.current = { startUs: target, endUs: target + length };
      void tracksEdit({ kind: "placeMain", ranges: move.ranges, startUs: target }).then((applied) => {
        if (!applied) pendingSelect.current = null;
      });
      return;
    }
    // The moved clips land together at the target, in timeline order, and stay selected.
    const length = move.ranges.reduce((sum, r) => sum + r.endUs - r.startUs, 0);
    const before = move.ranges.filter((r) => r.endUs <= target).reduce((sum, r) => sum + r.endUs - r.startUs, 0);
    const movedStart = target - before;
    pendingSelect.current = { startUs: movedStart, endUs: movedStart + length };
    const work = tracksEdit({ kind: "moveMain", ranges: move.ranges, targetUs: target });
    void work.then((applied) => {
      if (!applied) pendingSelect.current = null;
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

  // Clips on the tracks above: drag to move along or between tracks (or down into the main
  // sequence), drag an edge to trim.
  type OverlayDrag = {
    pointerId: number;
    startX: number;
    startY: number;
    active: boolean;
    clip: OverlayClip;
    trackId: string;
    grabUs: number;
    mode: "move" | "start" | "end";
    row: TrackRow | null;
    /** The clip as it would be after the drop. */
    preview: OverlayClip;
    mainUs: number | null;
    valid: boolean;
  };
  const overlayDragRef = useRef<OverlayDrag | null>(null);
  const [overlayDrag, setOverlayDrag] = useState<OverlayDrag | null>(null);
  const beginOverlayDrag = (
    event: React.PointerEvent<HTMLElement>,
    clip: OverlayClip,
    trackId: string,
    mode: OverlayDrag["mode"],
  ) => {
    if (event.button !== 0 || editing || !openedProject) return;
    event.stopPropagation();
    suppressSeek.current = false;
    event.currentTarget.setPointerCapture(event.pointerId);
    overlayDragRef.current = {
      pointerId: event.pointerId,
      startX: event.clientX,
      startY: event.clientY,
      active: false,
      clip,
      trackId,
      // From the grabbed edge (or the clip start, when moving) to the pointer.
      grabUs: clientXToUs(event.clientX) - (mode === "end" ? clipEndUs(clip) : clip.startUs),
      mode,
      row: { kind: "track", trackId },
      preview: clip,
      mainUs: null,
      valid: true,
    };
  };
  const moveOverlayDrag = (event: React.PointerEvent<HTMLElement>) => {
    const drag = overlayDragRef.current;
    if (!drag || drag.pointerId !== event.pointerId) return;
    event.stopPropagation();
    if (!drag.active) {
      const moved = Math.max(Math.abs(event.clientX - drag.startX), Math.abs(event.clientY - drag.startY));
      if (moved < (drag.mode === "move" ? CLIP_DRAG_PX : 2)) return;
      drag.active = true;
    }
    const pointerUs = clientXToUs(event.clientX);
    const track = overlayTracks.find((t) => t.id === drag.trackId);
    if (drag.mode !== "move") {
      const edgeAtPress = drag.mode === "start" ? drag.clip.startUs : clipEndUs(drag.clip);
      const edgeUs = snapStart(pointerUs - drag.grabUs, 0, snapPoints(drag.clip.id), snapUs());
      drag.preview = trimClip(drag.clip, drag.mode, edgeUs - edgeAtPress, assetOf(drag.clip.assetId), track);
      drag.valid = true;
      setOverlayDrag({ ...drag });
      return;
    }
    drag.row = rowAtPoint(event.clientX, event.clientY) ?? drag.row;
    const sound = drag.clip.audioStream !== undefined;
    const multi = trackClipSelected(drag.clip.id) && selectedTrackIds.length > 1;
    if (multi) {
      // Several clips move along their own tracks by the same amount.
      const startUs = snapStart(pointerUs - drag.grabUs, drag.clip.durationUs, snapPoints(drag.clip.id), snapUs());
      drag.row = { kind: "track", trackId: drag.trackId };
      drag.mainUs = null;
      drag.preview = { ...drag.clip, startUs };
      const delta = startUs - drag.clip.startUs;
      drag.valid = selectedTrackIds.every((id) => (findTrackClip(id)?.startUs ?? 0) + delta >= 0);
    } else if (drag.row?.kind === "main" && !sound) {
      drag.mainUs = nearestEdge(event.clientX);
      drag.valid = true;
    } else {
      drag.mainUs = null;
      const startUs = snapStart(pointerUs - drag.grabUs, drag.clip.durationUs, snapPoints(drag.clip.id), snapUs());
      drag.preview = { ...drag.clip, startUs };
      drag.valid =
        !!drag.row &&
        placeable(drag.row, startUs, drag.clip.durationUs, { video: !sound, audio: sound }, drag.clip.id);
    }
    setOverlayDrag({ ...drag });
  };
  const endOverlayDrag = (event: React.PointerEvent<HTMLElement>) => {
    const drag = overlayDragRef.current;
    if (!drag || drag.pointerId !== event.pointerId) return;
    event.stopPropagation();
    overlayDragRef.current = null;
    setOverlayDrag(null);
    if (!drag.active) return; // A plain click: onClick selects it.
    suppressSeek.current = true;
    const { clip, preview, row } = drag;
    if (drag.mode !== "move") {
      if (preview.startUs !== clip.startUs || preview.durationUs !== clip.durationUs) {
        void editTracks({ kind: "updateClip", clip: preview, trackId: drag.trackId });
      }
      return;
    }
    if (!row) return;
    if (!drag.valid) {
      setEditError(
        drag.clip.audioStream !== undefined
          ? "Sound clips go on audio tracks, where there is room."
          : "Pictures go on video tracks, where there is room.",
      );
      return;
    }
    if (trackClipSelected(clip.id) && selectedTrackIds.length > 1) {
      const deltaUs = preview.startUs - clip.startUs;
      if (deltaUs !== 0) void tracksEdit({ kind: "moveClips", clipIds: selectedTrackIds, deltaUs });
    } else if (row.kind === "main" && drag.mainUs !== null) {
      setSelectedOverlayClipId(undefined);
      void editTracks({ kind: "dropToMain", clipId: clip.id, targetUs: drag.mainUs });
    } else if (row.kind === "track" && row.trackId === drag.trackId && preview.startUs === clip.startUs) {
      return;
    } else {
      void editOnRow(row, (trackId) => ({ kind: "updateClip", clip: preview, trackId }));
    }
  };
  const overlayDragHandlers = (clip: OverlayClip, trackId: string, mode: OverlayDrag["mode"]) => ({
    onPointerDown: (event: React.PointerEvent<HTMLElement>) => beginOverlayDrag(event, clip, trackId, mode),
    onPointerMove: moveOverlayDrag,
    onPointerUp: endOverlayDrag,
    onPointerCancel: () => {
      overlayDragRef.current = null;
      setOverlayDrag(null);
    },
  });
  const onOverlayClipClick = (event: React.MouseEvent, clip: OverlayClip) => {
    event.stopPropagation();
    if (suppressSeek.current) {
      suppressSeek.current = false;
      return;
    }
    setRange(null);
    setSelectedCue(null);
    pickClip({ kind: "track", clipId: clip.id }, event.shiftKey || event.ctrlKey || event.metaKey);
  };

  /** Where a dragged block would land on `row`, if anywhere: a clip moving or trimming, imported media lifted off V1, or media from the bin. */
  const ghostOn = (row: TrackRow) => {
    if (overlayDrag?.active && sameRow(overlayDrag.mode === "move" ? overlayDrag.row : { kind: "track", trackId: overlayDrag.trackId }, row)) {
      return { startUs: overlayDrag.preview.startUs, durationUs: overlayDrag.preview.durationUs, valid: overlayDrag.valid };
    }
    if (clipMove?.lift && sameRow(clipMove.lift.row, row)) {
      return {
        startUs: clipMove.lift.startUs,
        durationUs: clipMove.range.endUs - clipMove.range.startUs,
        valid: clipMove.lift.valid,
      };
    }
    if (mediaGhost && sameRow(mediaGhost.row, row)) return mediaGhost;
    return null;
  };
  const renderGhost = (row: TrackRow) => {
    const ghost = ghostOn(row);
    if (!ghost || durationUs <= 0) return null;
    return (
      <div
        className={`absolute top-1 bottom-1 rounded-md border-2 border-dashed z-30 pointer-events-none ${
          ghost.valid ? "border-accent-fg bg-accent/20" : "border-danger bg-danger/25"
        }`}
        style={{
          left: `${(ghost.startUs / durationUs) * 100}%`,
          width: `${(ghost.durationUs / durationUs) * 100}%`,
        }}
      />
    );
  };

  // What is being dragged decides which "new track" row appears: none shows otherwise.
  const draggingKinds = (() => {
    if (overlayDrag?.active && overlayDrag.mode === "move") {
      const sound = overlayDrag.clip.audioStream !== undefined;
      return { video: !sound, audio: sound };
    }
    if (clipMove?.active) return { video: true, audio: false };
    if (mediaGhost || mediaDropUs !== null) {
      const asset = assetOf(currentMediaDrag() ?? "");
      return asset ? assetKinds(asset) : { video: true, audio: true };
    }
    return null;
  })();

  // Waveforms of imported sound, per file and stream, over the file's own time.
  const [soundWaves, setSoundWaves] = useState<Record<string, WaveformBucket[]>>({});
  const soundAssets = (openedProject?.mediaAssets ?? []).filter((a) => audioStreamCount(a) > 0);
  const soundKey = soundAssets.map((a) => `${a.id}:${audioStreamCount(a)}`).join(",");
  useEffect(() => {
    if (!openedProject) return;
    let active = true;
    for (const asset of soundAssets) {
      for (let stream = 0; stream < audioStreamCount(asset); stream++) {
        const key = `msound-${stream}-${asset.id}`;
        if (soundWaves[key]) continue;
        void api
          .projectWaveform(openedProject.projectHandle, key, 0, asset.durationUs, 512)
          .then((page) => {
            if (active && !page.cancelled) setSoundWaves((current) => ({ ...current, [key]: page.buckets }));
          })
          .catch((err) => console.warn("[Timeline] Sound waveform failed:", err));
      }
    }
    return () => {
      active = false;
    };
  }, [openedProject?.projectHandle, soundKey]);
  /** The waveform of the part of a file's stream a clip plays, behind its label. */
  const renderSoundWave = (assetId: string, stream: number, inUs: number, lengthUs: number, startUs: number) => {
    const buckets = soundWaves[`msound-${stream}-${assetId}`];
    if (!buckets?.length) return null;
    return (
      <div className="absolute inset-0 px-0.5 py-0.5 pointer-events-none opacity-80">
        <WaveformRenderer
          buckets={buckets}
          startUs={inUs}
          endUs={inUs + lengthUs}
          currentTimeUs={currentTimeUs - startUs + inUs}
          activeBarColor="#34d399"
          barColor="#065f46"
          className="w-full h-full"
        />
      </div>
    );
  };

  // Sound of the clips on the video tracks above, per track and stream, while linked.
  const trackSoundLanes = videoTracks.flatMap((track) => {
    const streams = Math.max(
      0,
      ...track.clips.filter((c) => !c.audioUnlinked).map((c) => audioStreamCount(assetOf(c.assetId))),
    );
    return Array.from({ length: streams }, (_, stream) => ({ track, stream }));
  });

  // Sound of the imported clips on V1, one lane per audio stream (while linked).
  const linkedSoundLanes = Math.max(
    0,
    ...clips.filter((clip) => clip.media && !clip.gap && !clip.audioUnlinked).map((clip) => audioStreamCount(assetOf(clip.media!))),
  );

  // The caption track: the captioned transcript's cues, editable in place.
  const captionsVersion = useProjectStore((s) => s.captionsVersion);
  const bumpCaptions = useProjectStore((s) => s.bumpCaptions);
  const [captionTrack, setCaptionTrack] = useState<CaptionTrackView>({ cues: [] });
  const [selectedCue, setSelectedCue] = useState<number | null>(null);
  const [cueText, setCueText] = useState<{ index: number; text: string } | null>(null);
  const [cueDrag, setCueDrag] = useState<{
    index: number;
    side: "start" | "end" | "move";
    startX: number;
    deltaUs: number;
    pointerId: number;
  } | null>(null);
  useEffect(() => {
    if (!openedProject) return;
    let active = true;
    void api
      .projectCaptionCues(openedProject.projectHandle)
      .then((view) => {
        if (active) setCaptionTrack(view);
      })
      .catch(() => {
        if (active) setCaptionTrack({ cues: [] });
      });
    return () => {
      active = false;
    };
  }, [openedProject?.projectHandle, openedProject?.revision, captionsVersion, openedProject?.captions?.trackId]);
  useEffect(() => setSelectedCue(null), [openedProject?.projectHandle]);
  const editCaption = async (change: CaptionEdit) => {
    const trackId = captionTrack.trackId;
    if (!openedProject || !trackId) return false;
    try {
      setCaptionTrack(await api.transcriptCaptionEdit(openedProject.projectHandle, trackId, change));
      setEditError(undefined);
      bumpCaptions();
      return true;
    } catch (err) {
      setEditError(String(err));
      return false;
    }
  };
  const cueAtSelection = selectedCue !== null ? captionTrack.cues[selectedCue] : undefined;
  /** S on a selected caption: a new caption starts at the first word at or after the playhead. */
  const splitCue = () => {
    const cue = cueAtSelection;
    if (!cue) return;
    const index = cue.wordStartsUs.findIndex((start, i) => i > 0 && start >= currentTimeUs);
    if (index <= 0) {
      setEditError("Put the playhead between two words of the caption to split it.");
      return;
    }
    void editCaption({ kind: "split", wordId: cue.wordIds[index] });
  };
  const mergeCue = () => {
    if (!cueAtSelection || selectedCue === 0) return;
    void editCaption({ kind: "merge", wordId: cueAtSelection.wordIds[0] });
  };
  const hideCue = () => {
    if (!cueAtSelection) return;
    void editCaption({ kind: "hide", wordIds: cueAtSelection.wordIds, hidden: true });
    setSelectedCue(null);
  };
  const endCueDrag = () => {
    const drag = cueDrag;
    setCueDrag(null);
    if (!drag || Math.abs(drag.deltaUs) < 10_000) return;
    const cue = captionTrack.cues[drag.index];
    if (!cue) return;
    const startUs = drag.side === "end" ? cue.startUs : Math.max(0, cue.startUs + drag.deltaUs);
    const endUs = drag.side === "start" ? cue.endUs : cue.endUs + drag.deltaUs;
    void editCaption({ kind: "retime", wordIds: cue.wordIds, startUs, endUs });
  };
  const cueDragHandlers = (index: number, side: "start" | "end" | "move") => ({
    onPointerDown: (event: React.PointerEvent<HTMLElement>) => {
      if (event.button !== 0 || cueText) return;
      event.stopPropagation();
      event.currentTarget.setPointerCapture(event.pointerId);
      setSelectedCue(index);
      setCueDrag({ index, side, startX: event.clientX, deltaUs: 0, pointerId: event.pointerId });
    },
    onPointerMove: (event: React.PointerEvent<HTMLElement>) => {
      if (!cueDrag || cueDrag.pointerId !== event.pointerId || pxPerUs <= 0) return;
      event.stopPropagation();
      const deltaUs = Math.round((event.clientX - cueDrag.startX) / pxPerUs);
      if (deltaUs !== cueDrag.deltaUs) setCueDrag({ ...cueDrag, deltaUs });
    },
    onPointerUp: (event: React.PointerEvent<HTMLElement>) => {
      event.stopPropagation();
      suppressSeek.current = true;
      endCueDrag();
    },
    onPointerCancel: () => setCueDrag(null),
  });

  const renderCaptionLane = () => (
    <div data-track-row="captions" className="relative rounded-md bg-studio-850/30" style={{ height: trackHeight("lane:captions", OVERLAY_ROW_PX) }}>
      {durationUs > 0 &&
        captionTrack.cues.map((cue, index) => {
          const selected = index === selectedCue;
          const drag = cueDrag?.index === index ? cueDrag : null;
          const startUs = drag && drag.side !== "end" ? cue.startUs + drag.deltaUs : cue.startUs;
          const endUs = drag && drag.side !== "start" ? cue.endUs + drag.deltaUs : cue.endUs;
          const typing = cueText?.index === index;
          return (
            <div
              key={`${cue.wordIds[0]}-${index}`}
              role="button"
              aria-label={`Caption: ${cue.text}`}
              aria-pressed={selected}
              className={`absolute top-1 bottom-1 rounded-control border overflow-hidden flex items-center px-2 cursor-grab ${
                selected
                  ? "bg-caption-fill border-accent-fg ring-2 ring-accent-hover/70 z-10"
                  : "bg-caption-fill/90 border-caption/60 hover:border-caption"
              }`}
              style={{
                left: `${(Math.max(0, startUs) / durationUs) * 100}%`,
                width: `${(Math.max(1, endUs - startUs) / durationUs) * 100}%`,
                minWidth: typing ? 160 : undefined,
              }}
              title={`${cue.text}\nDouble-click to edit the text, drag to move it, drag an edge to retime. With it selected: ${
                "S splits at the playhead, Delete hides it"
              }.`}
              onClick={(event) => {
                event.stopPropagation();
                setSelectedCue(index);
                setSelectedClips([]);
              }}
              onDoubleClick={(event) => {
                event.stopPropagation();
                setCueText({ index, text: cue.text });
              }}
              {...cueDragHandlers(index, "move")}
            >
              {typing ? (
                <input
                  autoFocus
                  aria-label="Caption text"
                  value={cueText.text}
                  onChange={(e) => setCueText({ index, text: e.target.value })}
                  onPointerDown={(e) => e.stopPropagation()}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") {
                      const text = cueText.text.trim();
                      setCueText(null);
                      if (text && text !== cue.text) void editCaption({ kind: "setText", wordIds: cue.wordIds, text });
                    } else if (e.key === "Escape") {
                      setCueText(null);
                    }
                  }}
                  onBlur={() => setCueText(null)}
                  className="w-full bg-studio-950 text-meta text-white px-1.5 h-6 rounded outline-none border border-caption"
                />
              ) : (
                <span className="text-meta text-studio-100 truncate pointer-events-none">{cue.text}</span>
              )}
              {!typing &&
                (["start", "end"] as const).map((side) => (
                  <div
                    key={side}
                    role="separator"
                    aria-label={`Retime the caption ${side}`}
                    className={`absolute inset-y-0 w-1.5 cursor-ew-resize opacity-0 hover:opacity-100 hover:bg-caption/80 ${
                      side === "start" ? "left-0" : "right-0"
                    }`}
                    onClick={(event) => event.stopPropagation()}
                    {...cueDragHandlers(index, side)}
                  />
                ))}
            </div>
          );
        })}
    </div>
  );

  const renderNewTrackRow = (audio: boolean) => {
    const shown = audio ? draggingKinds?.audio : draggingKinds?.video;
    return (
      <div
        data-track-row={audio ? "new-audio" : "new"}
        // The top row: the playhead and overlays before it are absolute, so drop the gap
        // space-y would put above it and keep the lanes level with their headers.
        className={`relative rounded-md flex items-center px-2 text-meta transition-colors ${audio ? "" : "!mt-0"} ${
          shown ? "border border-dashed border-accent/60 bg-accent/5 text-studio-300" : "border border-transparent text-transparent"
        }`}
        style={{ height: NEW_TRACK_ROW_PX }}
      >
        <span className="pointer-events-none truncate">
          {shown ? `Drop here for a new ${audio ? "audio" : "video"} track` : ""}
        </span>
        {renderGhost({ kind: "new", audio })}
      </div>
    );
  };

  const renderTrackLane = (track: (typeof overlayTracks)[number]) => {
    const audio = isAudioTrack(track);
    const label = trackLabel(overlayTracks, track.id);
    return (
      <div
        key={track.id}
        data-track-row={`track:${track.id}`}
        className={`relative rounded-md bg-studio-850/30 ${track.hidden || (audio && track.muted) ? "opacity-50" : ""}`}
        style={{ height: trackHeight(audio ? "lane:audio" : "lane:video", OVERLAY_ROW_PX) }}
      >
        {durationUs > 0 &&
          track.clips.map((clip) => {
            const dragging = overlayDrag?.active && overlayDrag.clip.id === clip.id;
            const selected = trackClipSelected(clip.id);
            const asset = assetOf(clip.assetId);
            const name =
              clip.audioStream !== undefined
                ? `${asset?.name ?? "Missing media"} · ${audioStreamName(asset, clip.audioStream)}`
                : (asset?.name ?? "Missing media");
            const tint = audio
              ? selected
                ? "bg-audio/35 border-accent-fg ring-2 ring-accent-hover/70"
                : "bg-audio/20 border-audio/50 hover:border-audio-fg"
              : selected
                ? "bg-video/45 border-accent-fg ring-2 ring-accent-hover/70"
                : "bg-video/30 border-video/60 hover:border-video-fg";
            return (
              <div
                key={clip.id}
                role="button"
                aria-label={`${name} on ${label}`}
                aria-pressed={selected}
                className={`absolute top-1 bottom-1 rounded-control border overflow-hidden flex items-center gap-1.5 px-2 cursor-grab ${tint} ${
                  dragging && overlayDrag?.mode === "move" ? "opacity-40" : ""
                }`}
                style={{
                  left: `${(clip.startUs / durationUs) * 100}%`,
                  width: `${(clip.durationUs / durationUs) * 100}%`,
                }}
                title={`${name}: ${(clip.durationUs / 1e6).toFixed(2)}s on ${label}. Click to select (Shift/Ctrl+click for more), drag to move${
                  audio ? " between audio tracks" : " along or between tracks (down to V1 inserts it)"
                }, drag an edge to trim.${clip.link ? " Unlinked sound: select it with its picture and press U to relink." : ""}`}
                onClick={(event) => onOverlayClipClick(event, clip)}
                {...overlayDragHandlers(clip, track.id, "move")}
              >
                {audio && clip.audioStream !== undefined &&
                  renderSoundWave(clip.assetId, clip.audioStream, clip.inUs, clip.durationUs, clip.startUs)}
                {audio ? (
                  <AudioLines className="relative w-3.5 h-3.5 shrink-0 text-white/75 pointer-events-none" aria-label="Audio" />
                ) : (
                  <Film className="relative w-3.5 h-3.5 shrink-0 text-white/75 pointer-events-none" aria-label="Video" />
                )}
                {(clip.link || clip.audioUnlinked) && (
                  <Unlink className="relative w-3.5 h-3.5 shrink-0 text-white/75 pointer-events-none" aria-label="Sound unlinked" />
                )}
                <span className="relative text-meta font-medium text-white/90 truncate pointer-events-none">
                  {name}
                </span>
                {(["start", "end"] as const).map((side) => (
                  <div
                    key={side}
                    role="separator"
                    aria-label={`Trim ${name} ${side}`}
                    className={`absolute inset-y-0 w-1.5 cursor-ew-resize opacity-0 hover:opacity-100 ${
                      audio ? "hover:bg-audio-fg/80" : "hover:bg-video-fg/80"
                    } ${side === "start" ? "left-0" : "right-0"}`}
                    onClick={(event) => event.stopPropagation()}
                    {...overlayDragHandlers(clip, track.id, side)}
                  />
                ))}
              </div>
            );
          })}
        {renderGhost({ kind: "track", trackId: track.id })}
      </div>
    );
  };

  // What a lane carries, marked on its header: speech or background sound; screen or webcam.
  const lanesByMix = soundLanes(openedProject);
  const roleFlag = (laneId: string) => {
    const lane = lanesByMix.find((l) => l.id === laneId);
    if (!lane) return null;
    const role = laneRole(openedProject, lane);
    return (
      <button
        type="button"
        aria-label={`${lane.label}: ${role === "mic" ? "speech" : "background sound"}`}
        title={
          role === "mic"
            ? "Speech: transcribed, captioned, and what background sound ducks under. Click to mark as background."
            : "Background sound (music, game, desktop). Click to mark as speech."
        }
        onClick={() => void saveTrackMix({ [lane.id]: { role: role === "mic" ? "background" : "mic" } }).catch((err) => setEditError(String(err)))}
        className={cn(HDR_BUTTON, role === "mic" ? "!text-audio-fg" : "!text-studio-300")}
      >
        {role === "mic" ? <Mic className="w-4 h-4" /> : <Music className="w-4 h-4" />}
      </button>
    );
  };
  const pictureFlag = (track: (typeof overlayTracks)[number]) => {
    // Unmarked, each file keeps its own role; a click cycles: screen, webcam, unmarked.
    const next = track.role === undefined || track.role === null ? "screen" : track.role === "screen" ? "webcam" : null;
    const label = track.role === "webcam" ? "webcam" : track.role === "screen" ? "screen" : "each file's own role";
    return (
      <button
        type="button"
        disabled={editing}
        aria-label={`${trackLabel(overlayTracks, track.id)}: ${label}`}
        title={
          track.role === "webcam"
            ? "Webcam: its clips show in the webcam bubble. Click to unmark."
            : track.role === "screen"
              ? "Screen: its clips fill the frame over the video. Click to mark as webcam."
              : "Unmarked: each file's own role. Click to mark this track as the screen."
        }
        onClick={() => void editTracks({ kind: "setTrackRole", trackId: track.id, role: next })}
        className={cn(HDR_BUTTON, track.role ? "!text-video-fg" : "!text-studio-500")}
      >
        {track.role === "webcam" ? <Camera className="w-4 h-4" /> : <Monitor className="w-4 h-4" />}
      </button>
    );
  };

  /** Up/down in the stack: video tracks and V1 among themselves, audio tracks among theirs. */
  const orderButtons = (trackId: string, label: string) => (
    <div className="flex flex-col">
      {([true, false] as const).map((up) => (
        <button
          key={up ? "up" : "down"}
          disabled={editing}
          aria-label={`Move ${label} ${up ? "up" : "down"}`}
          title={`Move ${label} ${up ? "up" : "down"}${
            trackId === "main" || videoTracks.some((t) => t.id === trackId) ? (up ? ": it draws over more" : ": it draws under more") : ""
          }`}
          onClick={() => void editTracks({ kind: "moveTrack", trackId, up })}
          className="h-3.5 w-5 inline-flex items-center justify-center rounded text-studio-500 hover:text-studio-100 hover:bg-studio-800 disabled:opacity-40"
        >
          {up ? <ChevronUp className="w-3.5 h-3.5" /> : <ChevronDown className="w-3.5 h-3.5" />}
        </button>
      ))}
    </div>
  );
  const setMainTrack = (change: Partial<Pick<MainTrack, "magnetic" | "hidden" | "muted">>) => {
    const next = { ...mainTrack, ...change };
    void editTracks({ kind: "setMainTrack", magnetic: next.magnetic, hidden: next.hidden, muted: next.muted });
  };
  /**
   * One track header: a colour strip for the kind of track, an icon, the name (and a detail
   * line when the row is tall enough), and its buttons. It is exactly as tall as its lane.
   */
  const trackHeaderRow = (row: {
    key: string;
    height: number;
    tone: keyof typeof TRACK_TONE;
    icon: React.ComponentType<{ className?: string }>;
    name: React.ReactNode;
    meta?: React.ReactNode;
    title?: string;
    grip?: React.ReactNode;
    actions?: React.ReactNode;
    dim?: boolean;
  }) => {
    const Icon = row.icon;
    return (
      <div
        key={row.key}
        className="group/header relative flex items-center gap-2 pl-3 pr-1.5 border-b border-studio-800/60 hover:bg-studio-850/60"
        style={{ height: row.height }}
        title={row.title}
      >
        <span className={cn("absolute left-0 top-1.5 bottom-1.5 w-[3px] rounded-r", TRACK_TONE[row.tone])} aria-hidden />
        {row.grip}
        <Icon className={cn("w-4 h-4 shrink-0", row.dim ? "text-studio-600" : "text-studio-400")} aria-hidden />
        <div className={cn("min-w-0 flex-1", row.height >= 40 ? "leading-tight" : "flex items-baseline gap-2")}>
          <div className={cn("text-label font-medium truncate", row.dim ? "text-studio-500" : "text-studio-100")}>{row.name}</div>
          {row.meta && <div className="text-meta text-studio-500 truncate">{row.meta}</div>}
        </div>
        {row.actions && <div className="flex items-center gap-0.5 shrink-0">{row.actions}</div>}
      </div>
    );
  };

  const mainHeader = trackHeaderRow({
    key: "main",
    height: trackHeight("lane:main", MAIN_ROW_PX),
    grip: resizeGrip("lane:main", "the main track (V1)", MAIN_ROW_PX),
    tone: "video",
    icon: Film,
    name: "V1 · Main",
    meta: magnetic ? "Magnetic" : "Free placement",
    dim: mainTrack.hidden,
    actions: (
      <>
        {orderButtons("main", "V1")}
        <button
          type="button"
          disabled={editing || !openedProject}
          aria-pressed={mainTrack.hidden}
          aria-label={`${mainTrack.hidden ? "Show" : "Hide"} V1`}
          title={mainTrack.hidden ? "Show V1" : "Hide V1: black where no other track draws"}
          onClick={() => setMainTrack({ hidden: !mainTrack.hidden })}
          className={cn(HDR_BUTTON, mainTrack.hidden && "text-danger-fg")}
        >
          {mainTrack.hidden ? <EyeOff className="w-4 h-4" /> : <Eye className="w-4 h-4" />}
        </button>
        <button
          type="button"
          disabled={editing || !openedProject}
          aria-pressed={mainTrack.muted}
          aria-label={`${mainTrack.muted ? "Unmute" : "Mute"} V1`}
          title={mainTrack.muted ? "Unmute V1" : "Mute V1: the recording's and its clips' sound"}
          onClick={() => setMainTrack({ muted: !mainTrack.muted })}
          className={cn(HDR_BUTTON, mainTrack.muted && "text-danger-fg")}
        >
          {mainTrack.muted ? <VolumeX className="w-4 h-4" /> : <Volume2 className="w-4 h-4" />}
        </button>
      </>
    ),
  });

  const renderTrackHeader = (track: (typeof overlayTracks)[number]) => {
    const audio = isAudioTrack(track);
    const label = trackLabel(overlayTracks, track.id);
    return trackHeaderRow({
      key: track.id,
      height: trackHeight(audio ? "lane:audio" : "lane:video", OVERLAY_ROW_PX),
      tone: audio ? "audio" : "video",
      icon: audio ? AudioLines : Film,
      name: label,
      meta: `${track.clips.length} clip${track.clips.length === 1 ? "" : "s"}`,
      dim: track.hidden || (audio && track.muted),
      grip: resizeGrip(audio ? "lane:audio" : "lane:video", audio ? "audio tracks" : "video tracks", OVERLAY_ROW_PX),
      actions: (
        <>
          {orderButtons(track.id, label)}
          {audio ? roleFlag(track.id) : pictureFlag(track)}
          {!audio && (
            <button
              type="button"
              disabled={editing}
              aria-pressed={track.hidden}
              aria-label={`${track.hidden ? "Show" : "Hide"} ${label}`}
              title={track.hidden ? "Show this track" : "Hide this track"}
              onClick={() => void editTracks({ kind: "setTrack", trackId: track.id, hidden: !track.hidden, muted: track.muted })}
              className={cn(HDR_BUTTON, track.hidden && "text-danger-fg")}
            >
              {track.hidden ? <EyeOff className="w-4 h-4" /> : <Eye className="w-4 h-4" />}
            </button>
          )}
          <button
            type="button"
            disabled={editing}
            aria-pressed={track.muted}
            aria-label={`${track.muted ? "Unmute" : "Mute"} ${label}`}
            title={track.muted ? "Unmute this track" : "Mute this track"}
            onClick={() => void editTracks({ kind: "setTrack", trackId: track.id, hidden: track.hidden, muted: !track.muted })}
            className={cn(HDR_BUTTON, track.muted && "text-danger-fg")}
          >
            {track.muted ? <VolumeX className="w-4 h-4" /> : <Volume2 className="w-4 h-4" />}
          </button>
          <button
            type="button"
            disabled={editing}
            aria-label={`Remove ${label}`}
            title="Remove this track and its clips (Undo brings it back)"
            onClick={() => void editTracks({ kind: "removeTrack", trackId: track.id })}
            className={cn(HDR_BUTTON, "hover:!text-danger-fg hover:!bg-danger/15")}
          >
            <Trash2 className="w-4 h-4" />
          </button>
        </>
      ),
    });
  };

  const addTrackButton = (audio: boolean) => (
    <div className="px-2 flex items-center" style={{ height: NEW_TRACK_ROW_PX }}>
      <Button
        size="sm"
        variant="ghost"
        icon={Plus}
        disabled={!openedProject || editing || (audio ? audioTracks : videoTracks).length >= 8}
        onClick={() => void editTracks({ kind: "addTrack", audio })}
        className="text-studio-400"
        title={
          audio
            ? "Add an audio track below the others"
            : "Add a video track above the others. Clips on higher tracks draw over the ones below."
        }
      >
        {audio ? "Add audio track" : "Add video track"}
      </Button>
    </div>
  );

  const handleTimelineClick = (e: React.MouseEvent<HTMLDivElement>) => {
    if (suppressSeek.current) {
      suppressSeek.current = false;
      return;
    }
    if (!timelineTrackRef.current) return;
    // Clips stop their clicks, so a click that lands here hit empty track space: deselect.
    if (!(e.shiftKey || e.ctrlKey || e.metaKey)) {
      clearSelection();
      setSelectedCue(null);
      setSelectedOverlayClipId(undefined);
    }
    const rect = timelineTrackRef.current.getBoundingClientRect();
    const clickX = e.clientX - rect.left;
    const progress = Math.max(0, Math.min(1, clickX / rect.width));
    seekToUs(progress * durationUs);
  };

  /** Where a zoom may go: between the zooms either side of it (zooms never overlap). */
  const zoomRoom = (bar: ZoomKeyframe) => {
    const others = zoomKeyframes.filter((k) => !k.pending && k.zoomId !== bar.zoomId);
    const before = others.filter((k) => k.endUs <= bar.tUs).map((k) => k.endUs);
    const after = others.filter((k) => k.tUs >= bar.endUs).map((k) => k.tUs);
    return { from: Math.max(0, ...before), to: Math.min(durationUs, ...after) };
  };
  /** A zoom's edited span while being dragged `deltaUs` by `mode`. */
  const draggedZoomSpan = (bar: ZoomKeyframe, mode: DragMode, deltaUs: number): [number, number] => {
    const { from, to } = zoomRoom(bar);
    const length = bar.endUs - bar.tUs;
    if (mode === "move") {
      const snapped = snapStart(bar.tUs + deltaUs, length, snapPoints(), snapUs());
      const start = Math.max(Math.min(from, bar.tUs), Math.min(Math.max(to, bar.endUs) - length, snapped));
      return [start, start + length];
    }
    if (mode === "start") {
      return [Math.max(Math.min(from, bar.tUs), Math.min(bar.endUs - MIN_ZOOM_US, bar.tUs + deltaUs)), bar.endUs];
    }
    return [bar.tUs, Math.min(Math.max(to, bar.endUs), Math.max(bar.tUs + MIN_ZOOM_US, bar.endUs + deltaUs))];
  };

  const beginDrag = (event: React.PointerEvent, bar: ZoomKeyframe, mode: DragMode) => {
    if (!openedProject || zoomBusy || event.button !== 0) return;
    event.preventDefault();
    event.stopPropagation();
    setSelectedZoomId(bar.zoomId);
    setSelectedClips([]);
    setRange(null);
    setSelectedCue(null);
    if (bar.pending) return; // A suggestion is accepted (double-click) before it moves.
    dragging.current = {
      mode,
      zoomId: bar.zoomId,
      startX: event.clientX,
      originStart: bar.sourceStartUs,
      originEnd: bar.sourceEndUs,
    };
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
  };

  const onBarPointerMove = (event: React.PointerEvent, bar: ZoomKeyframe) => {
    const drag = dragging.current;
    if (!drag || drag.zoomId !== bar.zoomId || !timelineTrackRef.current || durationUs <= 0) return;
    event.stopPropagation();
    const rect = timelineTrackRef.current.getBoundingClientRect();
    if (rect.width <= 0) return;
    const deltaUs = ((event.clientX - drag.startX) / rect.width) * durationUs;
    if (Math.abs(event.clientX - drag.startX) < 2 && !zoomDrag) return;
    suppressSeek.current = true;
    setZoomDrag({ barId: bar.id, mode: drag.mode, deltaUs });
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
    const [start, end] = draggedZoomSpan(bar, moved.mode, moved.deltaUs);
    // A zoom on an imported file's clock moves along that file's clips.
    const toSource = (editedUs: number) => editedToSourceUs(openedProject.retainedIntervals, editedUs, zoom.media);
    let sourceStart = drag.originStart;
    let sourceEnd = drag.originEnd;
    if (moved.mode === "move") {
      const mapped = toSource(Math.min(durationUs - 1, start));
      if (mapped == null) {
        setEditError("A zoom moves along its own recording: drop it over that recording's clips.");
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
    if (!openedProject) return;
    const pending = zoomKeyframes.some((k) => k.zoomId === zoomId && k.pending);
    setSelectedZoomId(undefined);
    void persistZoom(() =>
      pending
        ? api.projectZoomDismiss(openedProject.projectHandle, openedProject.revision, [zoomId])
        : api.projectZoomDelete(openedProject.projectHandle, openedProject.revision, zoomId),
    );
  };
  const acceptZoom = (zoomId: string) => {
    if (!openedProject) return;
    void persistZoom(() =>
      api.projectZoomAccept(openedProject.projectHandle, openedProject.revision, [zoomId]),
    );
  };
  const addZoomHere = () => {
    if (!openedProject) return;
    const startUs = selection ? selection.startUs : Math.max(0, currentTimeUs - 600_000);
    const endUs = selection ? selection.endUs : Math.min(durationUs, Math.max(startUs + 2_000_000, currentTimeUs + 1_400_000));
    void persistZoom(() =>
      api.projectZoomAdd(openedProject.projectHandle, openedProject.revision, {
        editedStartUs: startUs,
        editedEndUs: endUs,
        centerX: 0.5,
        centerY: 0.5,
        scale: zoomSettings.clickScale,
      }),
    );
  };
  const reloadZooms = () => {
    if (!openedProject) return;
    void persistZoom(() =>
      api.projectZoomReload(openedProject.projectHandle, openedProject.revision),
    );
  };

  const zoomHeader = trackHeaderRow({
    key: "zooms",
    height: trackHeight("lane:zooms", ZOOM_ROW_PX),
    grip: resizeGrip("lane:zooms", "the zoom track", ZOOM_ROW_PX),
    tone: "zoom",
    icon: ScanSearch,
    name: "Zooms",
    meta: `${openedProject?.zooms?.length ?? 0}`,
    actions: (
      <>
        <button
          type="button"
          disabled={!openedProject || zoomBusy || durationUs < 3}
          aria-label="Add a zoom"
          title={selection ? "Add a zoom over the selection" : "Add a zoom at the playhead (it fits between the zooms there)"}
          onClick={addZoomHere}
          className={HDR_BUTTON}
        >
          <Plus className="w-4 h-4" />
        </button>
        <button
          type="button"
          disabled={!openedProject || zoomBusy}
          aria-label="Reload zooms from the recording"
          title="Reload zooms from the recording with your auto-zoom settings. Zooms you added or changed stay; Undo brings the old ones back."
          onClick={reloadZooms}
          className={HDR_BUTTON}
        >
          <RefreshCw className="w-4 h-4" />
        </button>
      </>
    ),
  });

  const zoomLane = (
    <div className="relative rounded-md bg-studio-850/30" style={{ height: trackHeight("lane:zooms", ZOOM_ROW_PX) }} data-track-row="zooms">
      {durationUs > 0 &&
        zoomKeyframes.map((k) => {
          const selected = k.zoomId === selectedZoomId;
          const [startUs, endUs] =
            zoomDrag && zoomDrag.barId === k.id ? draggedZoomSpan(k, zoomDrag.mode, zoomDrag.deltaUs) : [k.tUs, k.endUs];
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
              style={{ left: `${(startUs / durationUs) * 100}%`, width: `${Math.max(((endUs - startUs) / durationUs) * 100, 0.4)}%` }}
              title={
                k.pending
                  ? `Suggested ${kind.toLowerCase()} · ${k.scale.toFixed(1)}× · ${seconds}s. Double-click to add it; Delete dismisses it.`
                  : `${kind} · ${k.scale.toFixed(1)}× · ${seconds}s. Drag to move, drag an edge to retime, Delete removes it. Zooms never overlap.`
              }
              onClick={(event) => {
                event.stopPropagation();
                if (suppressSeek.current) suppressSeek.current = false;
              }}
              onDoubleClick={(event) => {
                event.stopPropagation();
                if (k.pending) acceptZoom(k.zoomId);
                else seekToUs(k.tUs);
              }}
              onPointerDown={(event) => beginDrag(event, k, "move")}
              onPointerMove={(event) => onBarPointerMove(event, k)}
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

  const progress = durationUs > 0 ? currentTimeUs / durationUs : 0;
  const pendingCount = pendingZoomSuggestions.length;
  const persistedCount = openedProject?.zooms?.length ?? 0;

  return (
    <div className="relative flex flex-col h-full bg-studio-900 select-none">
      {/* Toolbar: transport, editing tools, modes, camera, and the timeline's zoom */}
      <div className="h-11 shrink-0 flex items-center gap-1 px-2 border-b border-studio-800 bg-studio-900 overflow-x-auto overflow-y-hidden">
        {/* Play, the time and quality sit on the preview's bar. */}
        <Button
          variant="ghost"
          icon={Scissors}
          disabled={!openedProject || editing || durationUs === 0}
          onClick={() => void splitAtPlayhead()}
          title={`Split at the playhead${hint("split")}`}
        >
          Split
        </Button>
        <IconButton
          icon={ArrowLeftToLine}
          label={`Ripple delete from the playhead back to the previous edit${hint("rippleTrimPrevious")}`}
          disabled={!openedProject || editing || durationUs === 0}
          onClick={() => void rippleTrim("previous")}
        />
        <IconButton
          icon={ArrowRightToLine}
          label={`Ripple delete from the playhead to the next edit${hint("rippleTrimNext")}`}
          disabled={!openedProject || editing || durationUs === 0}
          onClick={() => void rippleTrim("next")}
        />
        <Button
          variant="ghost"
          icon={linkState === "unlinked" ? Link2 : Unlink}
          disabled={!openedProject || editing || !linkState}
          onClick={toggleLink}
          aria-pressed={linkState === "unlinked"}
          title={
            linkState === "unlinked"
              ? `Relink: select this clip and its sound, then press this${hint("toggleLink")}`
              : `Unlink the selected clip's sound onto audio tracks, to move or trim it on its own${hint("toggleLink")}`
          }
        >
          {linkState === "unlinked" ? "Relink" : "Unlink"}
        </Button>

        <ToolbarDivider />
        <ToolbarToggle
          on={magnetic}
          icon={Magnet}
          disabled={!openedProject || editing}
          onClick={() => setMainTrack({ magnetic: !magnetic })}
          title={
            magnetic
              ? "Magnetic V1 (on): cuts close up and moved clips insert. Turn off to leave gaps and place clips anywhere."
              : "Magnetic V1 (off): cuts leave gaps (black) and moved clips land where you drop them. Turn on to close up."
          }
        >
          Magnetic
        </ToolbarToggle>
        {cutMarkers.length > 0 && openedProject && (
          <Button
            variant="ghost"
            icon={RotateCcw}
            disabled={editing}
            onClick={() =>
              void tracksEdit({
                kind: "restore",
                // Exactly what was cut: recorder pauses never come back.
                ranges: (openedProject.removedIntervals ?? []).map(({ startUs, endUs }) => ({ startUs, endUs })),
                grow: "end",
                shiftTracksAt: null,
              })
            }
            title={`Put all ${cutMarkers.length} cut${cutMarkers.length === 1 ? "" : "s"} back on the timeline`}
          >
            Restore {cutMarkers.length} cut{cutMarkers.length === 1 ? "" : "s"}
          </Button>
        )}

        <ToolbarDivider />
        <ToolbarToggle
          on={selectionFocused}
          icon={Video}
          disabled={!openedProject || editing || !selection || !!openedProject?.shortView}
          onClick={toggleCamFocus}
          title={
            selectionFocused
              ? "Remove Cam Focus from the selection"
              : "Make the webcam fill the frame over the selection (turns Auto Webcam on). Click a focus block on the webcam lane to select it."
          }
        >
          Cam Focus
        </ToolbarToggle>
        <ToolbarToggle
          on={selectionInNormalView}
          icon={MonitorPlay}
          disabled={!openedProject || editing || !selection || !!openedProject?.shortView}
          onClick={toggleNormalView}
          title={
            selectionInNormalView
              ? "These clips use normal view. Click to let Auto Webcam go full frame here again."
              : "Keep the normal view (webcam bubble) over the selected clips, even when Auto Webcam would go full frame"
          }
        >
          Normal view
        </ToolbarToggle>

        <span className="flex-1 min-w-2" />
        {(pendingCount > 0 || zoomDiagnostics.length > 0 || persistedCount > 0) && (
          <Badge tone="zoom" className="hidden xl:inline-flex" title="Review and change zooms on the Zooms track or in the Zoom panel">
            {pendingCount > 0
              ? `${pendingCount} zoom suggestion${pendingCount === 1 ? "" : "s"}`
              : persistedCount > 0
                ? `${persistedCount} zoom${persistedCount === 1 ? "" : "s"}`
                : "No auto-zoom"}
          </Badge>
        )}
        <ToolbarDivider />
        <IconButton
          icon={ZoomOut}
          label={`Zoom the timeline out${hint("zoomOut")}`}
          disabled={!openedProject || timelineZoom <= MIN_TIMELINE_ZOOM}
          onClick={() => zoomTimeline(0.5)}
        />
        <Button
          variant="ghost"
          size="sm"
          disabled={!openedProject || timelineZoom <= MIN_TIMELINE_ZOOM}
          onClick={() => setTimelineZoom(MIN_TIMELINE_ZOOM)}
          title="Fit the whole video in view"
        >
          Fit
        </Button>
        <IconButton
          icon={ZoomIn}
          label={`Zoom the timeline in${hint("zoomIn")}`}
          disabled={!openedProject || timelineZoom >= MAX_TIMELINE_ZOOM}
          onClick={() => zoomTimeline(2)}
        />
      </div>

      {openedProject && (range || editError) && (
        <div className="absolute bottom-3 right-3 z-50 flex flex-col items-end gap-1.5 pointer-events-none">
          {editError && (
            <div
              role="alert"
              className="pointer-events-auto flex items-start gap-2 max-w-md rounded-panel border border-danger/40 bg-studio-850 px-3 py-2 text-label text-danger-fg shadow-popover"
            >
              <span>{editError}</span>
              <button
                aria-label="Dismiss"
                onClick={() => setEditError(undefined)}
                className="text-danger-fg/70 hover:text-studio-100"
              >
                <X className="w-3.5 h-3.5" />
              </button>
            </div>
          )}
          {range && (
            <div className="pointer-events-auto flex items-center gap-1 rounded-panel border border-studio-700 bg-studio-850 pl-3 pr-1 py-1 text-label text-studio-200 shadow-popover">
              <span className="font-mono tabular-nums text-studio-300 mr-1">
                {((range.endUs - range.startUs) / 1e6).toFixed(2)}s range
              </span>
              <button
                disabled={editing}
                onClick={() => void editRange(false)}
                className="h-7 px-2 rounded-control text-danger-fg hover:bg-danger/15 disabled:opacity-40"
                title={`Cut the range and close the gap${hint("deleteSelection")}`}
              >
                Delete
              </button>
              <button
                disabled={editing}
                onClick={() => void editRange(true)}
                className="h-7 px-2 rounded-control text-accent-fg hover:bg-accent/15 disabled:opacity-40"
                title="Cut everything outside the range"
              >
                Keep only
              </button>
              <button
                aria-label="Clear range"
                onClick={() => setRange(null)}
                className="h-7 w-7 inline-flex items-center justify-center rounded-control text-studio-400 hover:text-studio-100 hover:bg-studio-800"
                title={`Clear the range${hint("deselect")}`}
              >
                <X className="w-3 h-3" />
              </button>
            </div>
          )}
        </div>
      )}

      {/* Tracks: headers on the left, lanes on the right */}
      {/* Scrolls up and down; both columns grow to the full height of the tracks, so a short
          timeline panel scrolls the lanes with their headers instead of cutting them off. */}
      <div className="flex-1 min-h-0 overflow-x-hidden overflow-y-auto">
      <div className="flex min-h-full">
        {/* Track headers, pinned beside their lanes (each as tall as its lane) */}
        <div className="w-64 shrink-0 flex flex-col border-r border-studio-800 bg-studio-900">
          <div className="h-8 shrink-0 border-b border-studio-800 px-3 flex items-center text-meta font-semibold uppercase tracking-wide text-studio-500">
            Tracks
          </div>

          <div className="flex-1 space-y-2 py-2">
            {captionTrack.trackId &&
              trackHeaderRow({
                key: "captions",
                height: trackHeight("lane:captions", OVERLAY_ROW_PX),
                tone: "caption",
                icon: Captions,
                name: "Captions",
                meta: openedProject?.captions?.enabled ? `${captionTrack.cues.length} shown` : "Off in export",
                grip: resizeGrip("lane:captions", "the captions track", OVERLAY_ROW_PX),
                actions: cueAtSelection && (
                  <>
                    <Button size="sm" variant="ghost" onClick={splitCue} title={`Split the caption at the playhead${hint("split")}`}>
                      Split
                    </Button>
                    <Button
                      size="sm"
                      variant="ghost"
                      onClick={mergeCue}
                      disabled={selectedCue === 0}
                      title="Join this caption to the one before it"
                    >
                      Merge
                    </Button>
                    <Button
                      size="sm"
                      variant="ghost"
                      onClick={hideCue}
                      className="text-danger-fg hover:!bg-danger/15"
                      title={`Hide this caption (the sound stays)${hint("deleteSelection")}`}
                    >
                      Hide
                    </Button>
                  </>
                ),
              })}
            {zoomHeader}
            {addTrackButton(false)}
            {videoAbove.map(renderTrackHeader)}
            {mainHeader}
            {tracks.map((track) => {
              const kind = RECORDING_TRACK[track.trackType];
              // "3/4 segments available": only worth showing when some are missing.
              const segments = /(\d+)\/(\d+) segments/.exec(track.name);
              const missing = segments && Number(segments[1]) < Number(segments[2]);
              return trackHeaderRow({
                key: track.id,
                height: trackHeight(track.trackType),
                tone: kind.tone,
                icon: kind.icon,
                name: kind.label,
                meta: missing ? (
                  <span className="text-suggest-fg">{`${segments![1]} of ${segments![2]} segments available`}</span>
                ) : undefined,
                title: track.name,
                grip: resizeGrip(track.trackType, `the ${track.name} track`),
                actions: (
                  <>
                    {roleFlag(track.id)}
                    <TrackHeaderButtons track={track} />
                  </>
                ),
              });
            })}
            {videoBelow.map(renderTrackHeader)}
            {Array.from({ length: linkedSoundLanes }, (_, stream) =>
              trackHeaderRow({
                key: `linked-${stream}`,
                height: trackHeight("lane:sound", LINKED_SOUND_ROW_PX),
                tone: "audio",
                icon: AudioLines,
                name: `V1 sound ${stream + 1}`,
                title: "The sound of the imported clips on V1. Select a clip and press U to unlink it onto an audio track.",
                grip: resizeGrip("lane:sound", "sound lanes", LINKED_SOUND_ROW_PX),
                actions: roleFlag(`main-sound-${stream + 1}`),
              }),
            )}
            {trackSoundLanes.map(({ track, stream }) =>
              trackHeaderRow({
                key: `tsound-${track.id}-${stream}`,
                height: trackHeight("lane:sound", LINKED_SOUND_ROW_PX),
                tone: "audio",
                icon: AudioLines,
                name: `${trackLabel(overlayTracks, track.id)} sound ${stream + 1}`,
                title: `The sound of the clips on ${trackLabel(overlayTracks, track.id)}. Select a clip and press U to unlink it.`,
                grip: resizeGrip("lane:sound", "sound lanes", LINKED_SOUND_ROW_PX),
                actions: roleFlag(`${track.id}-sound-${stream + 1}`),
              }),
            )}
            {audioTracks.map(renderTrackHeader)}
            {addTrackButton(true)}
          </div>
        </div>

        {/* Right Track Lanes & Playhead */}
        <div ref={scrollRef} className="timeline-scroll flex-1 overflow-x-auto overflow-y-hidden relative">
          <div className="flex flex-col h-full min-w-full" style={{ width: `${timelineZoom * 100}%` }}>
          {/* Time Ruler: drag to scrub */}
          <div
            className="h-8 shrink-0 border-b border-studio-800 bg-studio-900 relative cursor-ew-resize overflow-hidden"
            onPointerDown={onRulerPointerDown}
            onPointerMove={onRulerPointerMove}
            onPointerUp={onRulerPointerUp}
            onPointerCancel={onRulerPointerUp}
            title="Drag to scrub. Shift+drag to select a range."
          >
            {range && durationUs > 0 && (
              <div
                className="absolute top-0 bottom-0 bg-accent/25 border-x border-accent-hover pointer-events-none"
                style={{
                  left: `${(range.startUs / durationUs) * 100}%`,
                  width: `${((range.endUs - range.startUs) / durationUs) * 100}%`,
                }}
              />
            )}
            {rulerTicks.map((tickUs) => (
              <div
                key={tickUs}
                className="absolute top-3 bottom-0 border-l border-studio-700 pl-1.5 -mt-3 pt-2 text-meta font-mono tabular-nums text-studio-400 pointer-events-none"
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
                  <div className="h-full border-l border-studio-300/70" />
                  <span className="absolute top-0.5 left-1 max-w-[180px] truncate rounded bg-studio-700 px-1.5 text-meta text-studio-100">
                    {chapter.title}
                  </span>
                </div>
              ))}
            <div
              className="absolute top-0 bottom-0 w-0.5 -translate-x-1/2 bg-accent-hover pointer-events-none"
              style={{ left: `${progress * 100}%` }}
            >
              <div className="absolute top-0 left-1/2 -translate-x-1/2 h-2.5 w-3 rounded-b-sm bg-accent-hover" />
            </div>
          </div>

          {/* Interactive Track Area */}
          <div
            ref={timelineTrackRef}
            onDragOver={onMediaDragOver}
            onDragLeave={() => {
              setMediaDropUs(null);
              setMediaGhost(null);
            }}
            onDrop={onMediaDrop}
            onClick={handleTimelineClick}
            onPointerDown={onTrackPointerDown}
            className="flex-1 relative cursor-pointer py-2 space-y-2 bg-studio-950/40"
          >
            {/* Playhead Vertical Line */}
            <div
              className="absolute top-0 bottom-0 w-0.5 -translate-x-1/2 bg-accent-hover z-30 pointer-events-none shadow-[0_0_6px_rgb(var(--accent-hover)/0.5)]"
              style={{ left: `${progress * 100}%` }}
            />
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

            {/* Outside the scoped stretch (a short's), dimmed */}
            {scope && durationUs > 0 && (
              <>
                <div
                  className="absolute top-0 bottom-0 left-0 bg-black/55 z-20 pointer-events-none"
                  style={{ width: `${(Math.max(0, scope.startUs) / durationUs) * 100}%` }}
                />
                <div
                  className="absolute top-0 bottom-0 right-0 bg-black/55 z-20 pointer-events-none"
                  style={{ width: `${(Math.max(0, durationUs - scope.endUs) / durationUs) * 100}%` }}
                />
              </>
            )}

            {/* Range (Shift+drag on the ruler, or mark in/out) */}
            {range && durationUs > 0 && (
              <div
                className="absolute top-0 bottom-0 bg-accent/[0.08] border-x border-accent-hover/60 z-10 pointer-events-none"
                style={{
                  left: `${(range.startUs / durationUs) * 100}%`,
                  width: `${((range.endUs - range.startUs) / durationUs) * 100}%`,
                }}
              />
            )}

            {/* Clip move: the dragged range dims and a bar marks where it will land */}
            {clipMove?.active && durationUs > 0 && (
              <>
                <div
                  className="absolute top-0 bottom-0 bg-accent/10 border-x border-dashed border-accent-fg/70 z-30 pointer-events-none"
                  style={{
                    left: `${(clipMove.range.startUs / durationUs) * 100}%`,
                    width: `${((clipMove.range.endUs - clipMove.range.startUs) / durationUs) * 100}%`,
                  }}
                />
                {clipMove.targetUs !== null && (
                  <div
                    className="absolute top-0 bottom-0 w-1 -translate-x-1/2 bg-accent-fg shadow-[0_0_8px_rgb(var(--accent-fg)/0.8)] z-40 pointer-events-none"
                    style={{ left: `${(clipMove.targetUs / durationUs) * 100}%` }}
                  >
                    <span className="absolute -top-0.5 left-1.5 text-meta font-medium text-accent-fg bg-studio-950 border border-accent/50 rounded px-1.5 whitespace-nowrap">
                      {magnetic ? "Move here" : "Place here"}
                    </span>
                  </div>
                )}
              </>
            )}

            {overlayDrag?.active && overlayDrag.mainUs !== null && durationUs > 0 && (
              <div
                className="absolute top-0 bottom-0 w-1 -translate-x-1/2 bg-accent-fg shadow-[0_0_8px_rgb(var(--accent-fg)/0.8)] z-40 pointer-events-none"
                style={{ left: `${(overlayDrag.mainUs / durationUs) * 100}%` }}
              >
                <span className="absolute -top-0.5 left-1.5 text-meta font-medium text-accent-fg bg-studio-950 border border-accent/50 rounded px-1.5 whitespace-nowrap">
                  Insert into V1
                </span>
              </div>
            )}

            {mediaDropUs !== null && durationUs > 0 && (
              <div
                className="absolute top-0 bottom-0 w-1 -translate-x-1/2 bg-accent-fg shadow-[0_0_8px_rgb(var(--accent-fg)/0.8)] z-40 pointer-events-none"
                style={{ left: `${(mediaDropUs / durationUs) * 100}%` }}
              >
                <span className="absolute -top-0.5 left-1.5 text-meta font-medium text-accent-fg bg-studio-950 border border-accent/50 rounded px-1.5 whitespace-nowrap">
                  Insert here
                </span>
              </div>
            )}

            {/* Captions from the transcript, on top: edit, retime, split, merge or hide them here */}
            {captionTrack.trackId && renderCaptionLane()}

            {/* Zoom track: zooms as clips of their own */}
            {zoomLane}

            {/* New video track: a drop row that shows only while something can be dropped on it */}
            {renderNewTrackRow(false)}

            {/* Video tracks above the main sequence, top track first */}
            {videoAbove.map(renderTrackLane)}

            {/* Clip lane (V1): edges come from cuts and splits; markers restore cuts */}
            <div className="relative rounded-md bg-studio-850/30" style={{ height: trackHeight("lane:main", MAIN_ROW_PX) }} data-track-row="main">
              {durationUs === 0 && (
                <div className="absolute inset-0 flex items-center rounded-md border border-dashed border-studio-600 px-3 text-label text-studio-400 pointer-events-none">
                  Drag media from the Media panel here to start the main video (V1)
                </div>
              )}
              {durationUs > 0 && clips.map((clip, index) => {
                if (clip.gap) return null;
                const selected = clipSelected(clip);
                return (
                  <button
                    key={`${clip.sourceStartUs}-${index}`}
                    className={`absolute top-2.5 bottom-0.5 rounded-control border text-meta font-medium text-left px-1.5 truncate ${
                      clip.media
                        ? selected
                          ? "bg-video/55 border-accent-fg text-white ring-2 ring-accent-hover/70"
                          : "bg-video/40 border-video-fg/50 text-white hover:bg-video/50"
                        : selected
                          ? "bg-video/45 border-accent-fg text-white ring-2 ring-accent-hover/70"
                          : "bg-video/25 border-video/60 text-video-fg hover:bg-video/35"
                    }`}
                    style={{
                      left: `${(clip.startUs / durationUs) * 100}%`,
                      width: `${((clip.endUs - clip.startUs) / durationUs) * 100}%`,
                    }}
                    title={`Clip ${index + 1}${clip.media ? ` (${mediaName(clip)})` : ""}: ${((clip.endUs - clip.startUs) / 1e6).toFixed(2)}s. Click to select, Shift/Ctrl+click to add to the selection, drag to move it.${clip.audioUnlinked ? " Its sound is unlinked onto an audio track." : ""}`}
                    onClick={(event) => onClipClick(event, clip)}
                    {...clipMoveHandlers(clip)}
                  >
                    <Film className="inline w-3.5 h-3.5 mr-1 -mt-0.5 opacity-75" aria-label="Video" />
                    {clip.audioUnlinked && <Unlink className="inline w-3.5 h-3.5 mr-1 -mt-0.5" aria-label="Sound unlinked" />}
                    {clip.media ? `${index + 1} · ${mediaName(clip)}` : index + 1}
                  </button>
                );
              })}
              {/* Clip edge handles: the end handle sits left of the edge, the start handle right of it. */}
              {durationUs > 0 && !editing && clips.flatMap((clip, index) =>
                (clip.gap ? [] : (["start", "end"] as const)).map((side) => {
                  const edgePct = ((side === "start" ? clip.startUs : clip.endUs) / durationUs) * 100;
                  return (
                    <div
                      key={`${side}-${clip.sourceStartUs}-${index}`}
                      role="separator"
                      aria-label={`Trim clip ${index + 1} ${side}`}
                      className={`absolute top-2.5 bottom-0.5 w-2 z-30 cursor-ew-resize hover:bg-accent-fg/70 ${
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
                    className={`absolute top-2.5 bottom-0.5 z-20 pointer-events-none rounded-control border flex items-center justify-center text-meta font-mono tabular-nums ${
                      trimming
                        ? "bg-danger/40 border-danger text-white"
                        : "bg-accent/30 border-dashed border-accent-fg text-white"
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
                    void restoreCut(marker.sourceStartUs, marker.sourceEndUs, marker.grow, marker.editedUs);
                  }}
                >
                  <span className="w-3 h-2 shrink-0 rounded-sm bg-danger group-hover:bg-danger-fg" />
                  <span className="w-0.5 flex-1 bg-danger group-hover:bg-danger-fg" />
                </button>
              ))}
            </div>

            {/* Individual Lanes: one block per clip, so every cut and split shows as a gap. */}
            {tracks.map((track) => (
              <div
                key={track.id}
                data-track-row="main"
                className="relative"
                style={{ height: trackHeight(track.trackType) }}
              >
                {durationUs > 0 && clips.map((clip, index) => {
                  const selected = clipSelected(clip);
                  const audio = track.trackType === "mic" || track.trackType === "system";
                  return (
                    <div
                      key={`${clip.sourceStartUs}-${index}`}
                      className={`absolute top-0 bottom-0 rounded-control border overflow-hidden flex items-center ${
                        clip.media
                          ? "bg-video/20 border-video-fg/40 hover:border-video-fg/70"
                          : audio
                            ? "bg-audio/10 border-audio/35 hover:border-audio/70"
                            : "bg-video/20 border-video/45 hover:border-video-fg/70"
                      } ${selected ? "!border-accent-fg ring-2 ring-accent-hover/70" : ""}`}
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
                          <span className="px-2 text-meta font-medium text-video-fg truncate pointer-events-none">
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
                            activeBarColor={track.trackType === "mic" ? "#10b981" : "#0d9e74"}
                            barColor={track.trackType === "mic" ? "#17634d" : "#134d40"}
                            className="w-full h-full"
                            heightPx={trackHeight(track.trackType)}
                          />
                        </div>
                      ) : (
                        !audio && (
                          <span className="px-2 text-meta text-video-fg/80 truncate pointer-events-none">
                            {index + 1}
                          </span>
                        )
                      )}
                    </div>
                  );
                })}

                {track.waveform && track.waveform.buckets.length === 0 && (
                  <span className="absolute inset-0 flex items-center px-4 text-meta text-studio-400 pointer-events-none">Waveform unavailable</span>
                )}

                {!track.waveform && (track.trackType === "mic" || track.trackType === "system") && (
                  <span className="absolute inset-0 flex items-center px-4 text-meta text-studio-400 pointer-events-none">Loading waveform…</span>
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
                        className={`absolute top-1 bottom-1 rounded-control border z-10 cursor-pointer hover:ring-2 hover:ring-zoom-fg/70 ${
                          segment.enabled && openedProject?.webcamFocus?.enabled
                            ? "bg-zoom/30 border-zoom/80"
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
                        className="absolute top-1 bottom-1 rounded-control border border-studio-300/60 bg-studio-600/40 z-10 pointer-events-none flex items-center justify-center"
                        style={{
                          left: `${(edited.startUs / durationUs) * 100}%`,
                          width: `${((edited.endUs - edited.startUs) / durationUs) * 100}%`,
                        }}
                        title="Normal view: the webcam stays in its bubble here"
                      >
                        <span className="text-meta text-studio-100 truncate px-1">Normal view</span>
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
                        className="absolute top-0 bottom-0 bg-danger/20 border-x border-danger/60 flex items-center justify-center z-10 pointer-events-none"
                        style={{
                          left: `${cutStartProg * 100}%`,
                          width: `${cutWidthProg * 100}%`,
                        }}
                      >
                        <span className="text-meta text-danger-fg font-semibold uppercase tracking-wide">
                          CUT
                        </span>
                      </div>
                    );
                  })}
              </div>
            ))}

            {/* Video tracks below V1 */}
            {videoBelow.map(renderTrackLane)}

            {/* V1's imported clips' sound, one lane per audio stream: part of the clip until unlinked */}
            {Array.from({ length: linkedSoundLanes }, (_, stream) => (
              <div key={`linked-${stream}`} data-track-row="main" className="relative" style={{ height: trackHeight("lane:sound", LINKED_SOUND_ROW_PX) }}>
                {durationUs > 0 &&
                  clips.map((clip, index) => {
                    if (!clip.media || clip.gap || clip.audioUnlinked) return null;
                    const asset = assetOf(clip.media);
                    if (stream >= audioStreamCount(asset)) return null;
                    const selected = clipSelected(clip);
                    return (
                      <div
                        key={`${clip.sourceStartUs}-${index}`}
                        className={`absolute top-0.5 bottom-0.5 rounded-control border overflow-hidden flex items-center ${
                          selected
                            ? "bg-audio/30 border-accent-fg ring-2 ring-accent-hover/70"
                            : "bg-audio/15 border-audio/40 hover:border-audio-fg/70"
                        }`}
                        style={{
                          left: `calc(${(clip.startUs / durationUs) * 100}% + 1px)`,
                          width: `max(1px, calc(${((clip.endUs - clip.startUs) / durationUs) * 100}% - 2px))`,
                        }}
                        title={`${asset?.name ?? "Media"} · ${audioStreamName(asset, stream)}: linked to its picture on V1. Select the clip and press U to unlink its sound.`}
                        onClick={(event) => onClipClick(event, clip)}
                        {...clipMoveHandlers(clip)}
                      >
                        {renderSoundWave(clip.media, stream, clip.sourceStartUs, clip.endUs - clip.startUs, clip.startUs)}
                        <span className="relative flex items-center gap-1.5 px-2 text-meta text-white/85 truncate pointer-events-none">
                          <AudioLines className="w-3.5 h-3.5 shrink-0" aria-label="Audio" />
                          {audioStreamName(asset, stream)}
                        </span>
                      </div>
                    );
                  })}
              </div>
            ))}

            {/* The linked sound of the video tracks' clips */}
            {trackSoundLanes.map(({ track, stream }) => (
              <div key={`tsound-${track.id}-${stream}`} className="relative" style={{ height: trackHeight("lane:sound", LINKED_SOUND_ROW_PX) }}>
                {durationUs > 0 &&
                  track.clips.map((clip) => {
                    const asset = assetOf(clip.assetId);
                    if (clip.audioUnlinked || stream >= audioStreamCount(asset)) return null;
                    const selected = trackClipSelected(clip.id);
                    return (
                      <div
                        key={clip.id}
                        className={`absolute top-0.5 bottom-0.5 rounded-control border overflow-hidden flex items-center cursor-pointer ${
                          selected
                            ? "bg-audio/30 border-accent-fg ring-2 ring-accent-hover/70"
                            : "bg-audio/15 border-audio/40 hover:border-audio-fg/70"
                        }`}
                        style={{
                          left: `${(clip.startUs / durationUs) * 100}%`,
                          width: `${(clip.durationUs / durationUs) * 100}%`,
                        }}
                        title={`${asset?.name ?? "Media"} · ${audioStreamName(asset, stream)}: linked to its picture on ${trackLabel(overlayTracks, track.id)}. Select it and press U to unlink.`}
                        onPointerDown={(event) => event.stopPropagation()}
                        onClick={(event) => onOverlayClipClick(event, clip)}
                      >
                        {renderSoundWave(clip.assetId, stream, clip.inUs, clip.durationUs, clip.startUs)}
                        <span className="relative flex items-center gap-1.5 px-2 text-meta text-white/85 truncate pointer-events-none">
                          <AudioLines className="w-3.5 h-3.5 shrink-0" aria-label="Audio" />
                          {audioStreamName(asset, stream)}
                        </span>
                      </div>
                    );
                  })}
              </div>
            ))}

            {/* Audio tracks: sound unlinked from its picture, or placed on its own */}
            {audioTracks.map(renderTrackLane)}
            {renderNewTrackRow(true)}

          </div>
          </div>
        </div>
      </div>
      </div>
    </div>
  );
};
