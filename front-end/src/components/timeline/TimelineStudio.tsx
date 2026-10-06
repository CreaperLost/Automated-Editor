import React, { useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import {
  ArrowLeftToLine,
  ArrowRightToLine,
  AudioLines,
  Camera,
  ChevronDown,
  ChevronUp,
  Eye,
  EyeOff,
  Film,
  Image as ImageIcon,
  Layers,
  Link2,
  Lock,
  Unlock as LockOpen,
  Magnet,
  Mic,
  Monitor,
  MonitorPlay,
  Music,
  Plus,
  RotateCcw,
  Scissors,
  Trash2,
  Unlink,
  Video,
  Volume2,
  VolumeX,
  X,
  ZoomIn,
  ZoomOut,
} from "lucide-react";
import { Badge, Button, IconButton, cn } from "../ui";
import { useProjectStore } from "../../stores/projectStore";
import { useTimeline } from "../../hooks/useTimeline";
import { ClipWaveform } from "../waveform/ClipWaveform";
import { MEDIA_DRAG_TYPE, currentMediaDrag } from "../media/MediaPanel";
import { placedChapters } from "../chapters/ChaptersPanel";
import { api } from "../../lib/ipc";
import { pausePlayback, shuttleForward } from "../../lib/playbackControl";
import { hotkeyHint, useHotkeyStore, type HotkeyAction } from "../../stores/hotkeyStore";
import {
  assetById,
  clipEnd,
  clipName,
  clipRole,
  cutJoins,
  defaultClipUs,
  formatRulerLabel,
  rulerStepUs,
  sequenceEdges,
  snapStart,
  streamKey,
  streamOf,
  trackLabel,
  trackNumber,
  withPartners,
} from "../../lib/sequence";
import {
  DEFAULT_WEBCAM_FOCUS,
  type Clip,
  type EditedSpan,
  type OpenedProject,
  type Role,
  type SeqTrack,
  type SequenceEdit,
  type WaveformOverview,
} from "../../lib/types";
import {
  CLIP_DRAG_PX,
  HDR_BUTTON,
  HeaderRow,
  MAX_TIMELINE_ZOOM,
  MIN_TIMELINE_ZOOM,
  NEW_TRACK_ROW_PX,
  RANGE_DRAG_PX,
  ResizeGrip,
  SNAP_PX,
  ToolbarDivider,
  useLaneHeights,
  type TimelineView,
} from "./timelineShared";
import { useZoomLane } from "./useZoomLane";
import { useCaptionLane } from "./useCaptionLane";

type SourceRange = [start: number, end: number];

/** The playhead now, read when needed: the timeline itself does not re-render as it moves. */
const nowUs = () => useProjectStore.getState().currentTimeUs;

/**
 * Something drawn at the playhead. Only this re-renders as the playhead moves, not the
 * timeline with all its clips.
 */
const AtPlayhead: React.FC<{ spanUs: number; children: (left: string, timeUs: number) => React.ReactNode }> = ({
  spanUs,
  children,
}) => {
  const timeUs = useProjectStore((s) => s.currentTimeUs);
  return <>{children(`${(Math.min(timeUs, spanUs) / spanUs) * 100}%`, timeUs)}</>;
};

/** While playing, pages the timeline so the playhead stays in view. */
const FollowPlayhead: React.FC<{ scrollRef: React.RefObject<HTMLDivElement | null>; pxPerUs: number; zoom: number }> = ({
  scrollRef,
  pxPerUs,
  zoom,
}) => {
  const timeUs = useProjectStore((s) => (s.isPlaying ? s.currentTimeUs : -1));
  useEffect(() => {
    const element = scrollRef.current;
    if (timeUs < 0 || !element || pxPerUs <= 0 || zoom <= 1) return;
    const x = timeUs * pxPerUs;
    if (x < element.scrollLeft || x > element.scrollLeft + element.clientWidth - 24) {
      element.scrollLeft = Math.max(0, x - element.clientWidth * 0.1);
    }
  }, [timeUs, pxPerUs, zoom, scrollRef]);
  return null;
};

/** `ranges` minus `cut`, both as [start, end) ranges. */
function subtractRanges(ranges: SourceRange[], cut: SourceRange[]): SourceRange[] {
  return ranges.flatMap(([start, end]) => {
    let pieces: SourceRange[] = [[start, end]];
    for (const [cutStart, cutEnd] of cut) {
      pieces = pieces.flatMap(([a, b]): SourceRange[] =>
        cutEnd <= a || cutStart >= b
          ? [[a, b]]
          : [...(cutStart > a ? [[a, cutStart] as SourceRange] : []), ...(cutEnd < b ? [[cutEnd, b] as SourceRange] : [])],
      );
    }
    return pieces;
  });
}

/** A row a clip or media can be dropped on: a track, or a new track at the top or bottom. */
type Row = { kind: "track"; trackId: string } | { kind: "new"; audio: boolean };

/** Rows carry `data-track-row`: "track:<id>", "new-video" or "new-audio". */
function rowFromElement(element: Element | null): Row | null {
  const value = element?.closest("[data-track-row]")?.getAttribute("data-track-row");
  if (!value) return null;
  if (value === "new-video" || value === "new-audio") return { kind: "new", audio: value === "new-audio" };
  return value.startsWith("track:") ? { kind: "track", trackId: value.slice(6) } : null;
}

/** Where each row is on screen (the track rows and the new-track rows). */
type RowBox = { row: Row; rect: DOMRect };

/** The rows' boxes, read when a drag starts. */
function rowBoxes(): RowBox[] {
  return [...document.querySelectorAll("[data-track-row]")].flatMap((element) => {
    const row = rowFromElement(element);
    return row ? [{ row, rect: element.getBoundingClientRect() }] : [];
  });
}

/**
 * The row under a screen point, also while the pointer is captured by a dragged clip. By the
 * rows' boxes: asking the page what is under the point hit-tests every clip (thousands on a
 * long timeline, tens of ms a pointer move).
 */
function rowAtPoint(boxes: RowBox[], x: number, y: number): Row | null {
  const hit = boxes.find(({ rect }) => x >= rect.left && x < rect.right && y >= rect.top && y < rect.bottom);
  return hit?.row ?? null;
}

/** A clip being dragged: moved (with the clips that go with it) or trimmed at one edge. */
interface ClipDrag {
  pointerId: number;
  startX: number;
  startY: number;
  active: boolean;
  mode: "move" | "start" | "end";
  anchor: Clip;
  anchorTrack: SeqTrack;
  /** The clips that move or trim together. */
  ids: string[];
  /** The same, for lookups while dragging. */
  idSet: Set<string>;
  /** Where the earliest of them starts: a move stops at the timeline's start. */
  earliestUs: number;
  /** Clip edges to snap to, the dragged clips' own left out; worked out when the drag starts. */
  snapEdges: number[];
  /** Where the rows are, and when that was read (it is read again now and then, in case
   * the timeline scrolled). */
  rows: RowBox[];
  rowsAt: number;
  /** Pointer time minus the anchor's edge (or start, when moving) at the press. */
  grabUs: number;
  /** Moving: how far; trimming: where the edge goes. */
  deltaUs: number;
  edgeUs: number;
  row: Row | null;
}

/** What the timeline does for a clip's clicks and drags; read through a ref, so a clip
 * does not re-render because the timeline made new handlers. */
interface ClipActions {
  pick: (clip: Clip, event: React.MouseEvent) => void;
  begin: (event: React.PointerEvent<HTMLElement>, trackId: string, clip: Clip, mode: ClipDrag["mode"]) => void;
  move: (event: React.PointerEvent<HTMLElement>) => void;
  end: (event: React.PointerEvent<HTMLElement>) => void;
  cancel: () => void;
  suppressClick: React.MutableRefObject<boolean>;
}

/** What a clip shows, worked out once per revision rather than on every render. */
interface ClipInfo {
  name: string;
  role?: Role;
  /** "Screen", "Speech"…: what it plays as. */
  roleLabel?: string;
  image: boolean;
  missing: boolean;
  /** An unlinked clip of media with several streams (it shows the unlink mark). */
  unlinked: boolean;
}

interface ClipBlockProps {
  clip: Clip;
  trackId: string;
  trackNo: string;
  audio: boolean;
  locked: boolean;
  showName: boolean;
  spanUs: number;
  info: ClipInfo;
  selected: boolean;
  moving: boolean;
  shade: { from: number; to: number; shrinking: boolean } | null;
  /** A sound clip's waveform, and the part of the clip (timeline time) it is drawn over: the
   *  part near the view, so a long clip zoomed in stays sharp. */
  waveform?: WaveformOverview;
  waveFromUs: number;
  waveToUs: number;
  actions: React.MutableRefObject<ClipActions>;
}

/**
 * One clip on its lane. Memoized: selecting, dragging or trimming re-renders only the clips
 * it changes, not every clip on the timeline.
 */
const ClipBlock = React.memo(function ClipBlock({
  clip,
  trackId,
  trackNo,
  audio,
  locked,
  showName,
  spanUs,
  info,
  selected,
  moving,
  shade,
  waveform,
  waveFromUs,
  waveToUs,
  actions,
}: ClipBlockProps) {
  const pct = (us: number) => `${(us / spanUs) * 100}%`;
  const { name, role, missing } = info;
  const Icon = audio ? (role === "mic" ? Mic : AudioLines) : info.image ? ImageIcon : role === "webcam" ? Camera : Film;
  const tint = missing
    ? "bg-danger/15 border-danger/50"
    : audio
      ? selected
        ? "bg-audio/45 border-white ring-2 ring-accent-hover z-10"
        : "bg-audio/20 border-audio/50 hover:border-audio-fg"
      : selected
        ? "bg-video/65 border-white ring-2 ring-accent-hover z-10"
        : "bg-video/30 border-video/60 hover:border-video-fg";
  const drag = (mode: ClipDrag["mode"]) => ({
    onPointerDown: (event: React.PointerEvent<HTMLElement>) => actions.current.begin(event, trackId, clip, mode),
    onPointerMove: (event: React.PointerEvent<HTMLElement>) => actions.current.move(event),
    onPointerUp: (event: React.PointerEvent<HTMLElement>) => actions.current.end(event),
    onPointerCancel: () => actions.current.cancel(),
  });
  // A click (not a drag) anywhere on the clip, its edges too, selects it: a narrow clip is
  // mostly edge.
  const click = (event: React.MouseEvent) => {
    event.stopPropagation();
    const suppress = actions.current.suppressClick;
    if (suppress.current) {
      suppress.current = false;
      return;
    }
    actions.current.pick(clip, event);
  };
  return (
    <>
      <div
        role="button"
        data-clip
        aria-label={`${name} on ${trackNo}`}
        aria-pressed={selected}
        className={cn(
          "absolute top-1 bottom-1 rounded-control border overflow-hidden flex items-center gap-1.5 px-2",
          locked ? "cursor-not-allowed" : "cursor-grab",
          tint,
          moving && "opacity-40",
        )}
        style={{ left: pct(clip.startUs), width: `max(2px, ${pct(clip.durationUs)})` }}
        title={`${name}${missing ? " (missing)" : ""}: ${(clip.durationUs / 1e6).toFixed(2)}s${
          info.roleLabel ? ` · ${info.roleLabel}` : ""
        }. Click to select (with what it is linked to; Alt+click alone, Shift/Ctrl+click to add or remove), drag to move, drag an edge to trim. Drag over empty space to select several.`}
        onClick={click}
        {...drag("move")}
      >
        {audio ? (
          <>
            {/* Sound shows as its waveform; its name is a small tag in the corner. */}
            {waveform && waveToUs > waveFromUs && (
              <div
                className="absolute inset-y-0.5 pointer-events-none"
                style={{
                  left: `${((waveFromUs - clip.startUs) / clip.durationUs) * 100}%`,
                  width: `${((waveToUs - waveFromUs) / clip.durationUs) * 100}%`,
                }}
              >
                <ClipWaveform
                  overview={waveform}
                  fromUs={clip.inUs + (waveFromUs - clip.startUs)}
                  toUs={clip.inUs + (waveToUs - clip.startUs)}
                  speech={role === "mic"}
                />
              </div>
            )}
            {showName && (
              <span className="absolute top-0.5 left-1 max-w-[calc(100%-0.5rem)] flex items-center gap-1 px-1 rounded-sm bg-studio-950/55 text-[10px] leading-4 font-medium text-white/85 truncate pointer-events-none">
                {info.unlinked && <Unlink className="w-2.5 h-2.5 shrink-0" aria-label="Unlinked" />}
                <span className="truncate">{name}</span>
              </span>
            )}
          </>
        ) : (
          <>
            <Icon className="relative w-3.5 h-3.5 shrink-0 text-white/75 pointer-events-none" aria-hidden />
            {info.unlinked && <Unlink className="relative w-3 h-3 shrink-0 text-white/60 pointer-events-none" aria-label="Unlinked" />}
            <span className="relative text-meta font-medium text-white/90 truncate pointer-events-none">{showName ? name : ""}</span>
          </>
        )}
        {!locked &&
          (["start", "end"] as const).map((side) => (
            <div
              key={side}
              role="separator"
              aria-label={`Trim ${name} ${side}`}
              className={cn(
                "absolute inset-y-0 w-1.5 cursor-ew-resize opacity-0 hover:opacity-100",
                audio ? "hover:bg-audio-fg/80" : "hover:bg-video-fg/80",
                side === "start" ? "left-0" : "right-0",
              )}
              onClick={click}
              {...drag(side)}
            />
          ))}
      </div>
      {shade && (
        <div
          className={cn(
            "absolute top-1 bottom-1 z-20 pointer-events-none rounded-control border flex items-center justify-center text-meta font-mono tabular-nums",
            shade.shrinking ? "bg-danger/40 border-danger text-white" : "bg-accent/30 border-dashed border-accent-fg text-white",
          )}
          style={{ left: pct(shade.from), width: `max(2px, ${pct(shade.to - shade.from)})` }}
        >
          <span className="px-1 bg-studio-950/80 rounded whitespace-nowrap">
            {shade.shrinking ? "−" : "+"}
            {((shade.to - shade.from) / 1e6).toFixed(2)}s
          </span>
        </div>
      )}
    </>
  );
});

/** The cuts on one track, each a mark whose cap puts the cut time back. Memoized: the marks
 * redraw only when the cuts, the scale or the busy state change. */
const CutMarks = React.memo(function CutMarks({
  joins,
  spanUs,
  editing,
  restore,
}: {
  joins: { clipId: string; atUs: number; gapUs: number }[];
  spanUs: number;
  editing: boolean;
  restore: React.MutableRefObject<(clipId: string) => void>;
}) {
  return (
    <>
      {joins.map((join) => {
        const left = `${(join.atUs / spanUs) * 100}%`;
        return (
          <React.Fragment key={`join-${join.clipId}`}>
            {/* The line marks the cut; only its cap restores, so the clip edges stay draggable. */}
            <span className="absolute top-0 bottom-0 w-px -translate-x-1/2 bg-danger/70 z-10 pointer-events-none" style={{ left }} />
            <button
              disabled={editing}
              aria-label="Restore cut"
              className="absolute top-0 h-2.5 w-3.5 -translate-x-1/2 z-30 rounded-b-sm bg-danger hover:bg-danger-fg disabled:opacity-40"
              style={{ left }}
              title={`Restore the ${(join.gapUs / 1e6).toFixed(2)}s cut here (everything after moves along)`}
              onPointerDown={(event) => event.stopPropagation()}
              onClick={(event) => {
                event.stopPropagation();
                restore.current(join.clipId);
              }}
            />
          </React.Fragment>
        );
      })}
    </>
  );
});

const ROLE_LABEL: Record<Role, string> = {
  screen: "Screen",
  webcam: "Camera",
  overlay: "Overlay",
  mic: "Speech",
  background: "Background",
};

/** The editor's timeline: video tracks over audio tracks, each holding clips, like Premiere. */
export const TimelineStudio: React.FC = () => {
  const openedProject = useProjectStore((s) => s.openedProject);
  const pendingZoomSuggestions = useProjectStore((s) => s.pendingZoomSuggestions);
  const zoomDiagnostics = useProjectStore((s) => s.zoomDiagnostics);
  const durationUs = useProjectStore((s) => s.durationUs);
  const applyOpenedProject = useProjectStore((s) => s.applyOpenedProject);
  const selectedClipIds = useProjectStore((s) => s.selectedClipIds);
  const setSelectedClipIds = useProjectStore((s) => s.setSelectedClipIds);
  const setTimelineSelection = useProjectStore((s) => s.setTimelineSelection);
  const setSelectedZoomId = useProjectStore((s) => s.setSelectedZoomId);
  const waveforms = useProjectStore((s) => s.waveforms);
  const setWaveform = useProjectStore((s) => s.setWaveform);
  const { togglePlayPause, seekToUs } = useTimeline();
  const bindings = useHotkeyStore((s) => s.bindings);
  const hint = (action: HotkeyAction) => hotkeyHint(bindings, action);
  const lanes = useLaneHeights();

  const sequence = openedProject?.sequence ?? { tracks: [], magnetic: true };
  const magnetic = sequence.magnetic;
  const videoTracks = sequence.tracks.filter((t) => t.kind === "video");
  const audioTracks = sequence.tracks.filter((t) => t.kind === "audio");
  const frameUs = Math.round(1e6 / (openedProject?.fps || 30));
  const edges = useMemo(() => sequenceEdges(sequence), [openedProject?.revision, openedProject?.shortView]);
  // Clips by id with their tracks, and every clip edge: built once per sequence, so drags and
  // selections do not search the timeline for each clip.
  const clipIndex = useMemo(() => {
    const index = new Map<string, { track: SeqTrack; clip: Clip }>();
    for (const track of sequence.tracks) for (const clip of track.clips) index.set(clip.id, { track, clip });
    return index;
  }, [sequence]);
  const clipEdges = useMemo(() => sequence.tracks.flatMap((t) => t.clips.flatMap((c) => [c.startUs, clipEnd(c)])), [sequence]);
  const previousInfo = useRef(new Map<string, ClipInfo>());
  const clipInfo = useMemo(() => {
    const info = new Map<string, ClipInfo>();
    // Unchanged info keeps its object, so the clip's memo holds across edits.
    const keep = (id: string, next: ClipInfo) => {
      const was = previousInfo.current.get(id);
      const same =
        was &&
        was.name === next.name &&
        was.role === next.role &&
        was.roleLabel === next.roleLabel &&
        was.image === next.image &&
        was.missing === next.missing &&
        was.unlinked === next.unlinked;
      info.set(id, same ? was : next);
    };
    for (const track of sequence.tracks) {
      for (const clip of track.clips) {
        const asset = assetById(openedProject, clip.asset);
        const stream = streamOf(openedProject, clip);
        const role = clipRole(openedProject, track, clip);
        keep(clip.id, {
          name: clipName(openedProject, clip),
          role,
          roleLabel: stream ? ROLE_LABEL[role ?? stream.role] : undefined,
          image: asset?.kind === "image",
          missing: !asset || !!asset.missing,
          unlinked: !clip.link && !!asset && asset.streams.length > 1,
        });
      }
    }
    previousInfo.current = info;
    return info;
  }, [openedProject]);

  // Range selection (Shift+drag on the ruler, or mark in/out).
  const [range, setRange] = useState<EditedSpan | null>(null);
  const [editError, setEditError] = useState<string>();
  const [editing, setEditing] = useState(false);
  const suppressClick = useRef(false);

  useEffect(() => {
    setRange(null);
    setEditError(undefined);
  }, [openedProject?.projectHandle]);
  useEffect(() => {
    setRange((current) => (current && current.endUs > durationUs ? null : current));
  }, [openedProject?.revision, durationUs]);

  /** Runs one edit; resolves to whether it was applied. */
  const runEdit = async (work: (project: OpenedProject) => Promise<OpenedProject>) => {
    if (!openedProject || editing) return false;
    setEditing(true);
    setEditError(undefined);
    try {
      applyOpenedProject(await work(openedProject));
      return true;
    } catch (err) {
      setEditError(String(err));
      return false;
    } finally {
      setEditing(false);
    }
  };
  const edit = (change: SequenceEdit) =>
    runEdit((project) => api.projectSequenceEdit(project.projectHandle, project.revision, change));
  /** Adds a track of a kind, then runs an edit that needs its id (two undo steps). */
  const editOnNewTrack = (audio: boolean, makeEdit: (trackId: string) => SequenceEdit) =>
    runEdit(async (project) => {
      const withTrack = await api.projectSequenceEdit(project.projectHandle, project.revision, {
        kind: "addTrack",
        trackKind: audio ? "audio" : "video",
      });
      const added = withTrack.sequence.tracks.filter((t) => t.kind === (audio ? "audio" : "video"));
      const trackId = audio ? added[added.length - 1]?.id : added[added.length - 1]?.id;
      try {
        return await api.projectSequenceEdit(withTrack.projectHandle, withTrack.revision, makeEdit(trackId ?? ""));
      } catch (err) {
        // The new track stays; show it while reporting why the clip could not go on it.
        applyOpenedProject(withTrack);
        throw err;
      }
    });

  // ---- The view: scale, scroll, zoom --------------------------------------------------------
  const lanesRef = useRef<HTMLDivElement | null>(null);
  const scrollRef = useRef<HTMLDivElement | null>(null);
  const [timelineZoom, setTimelineZoom] = useState(1);
  const [viewportPx, setViewportPx] = useState(0);
  // Only clips near the view are drawn: a long timeline cut at every pause has thousands, and
  // drawing them all made every edit slow. The drawn window moves in steps of half a view, so
  // scrolling does not redraw the timeline on every frame.
  const [windowStep, setWindowStep] = useState(0);
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
  useEffect(() => {
    const element = scrollRef.current;
    if (!element) return;
    let frame = 0;
    let timer = 0;
    const update = () => {
      cancelAnimationFrame(frame);
      window.clearTimeout(timer);
      frame = 0;
      timer = 0;
      const step = Math.max(1, element.clientWidth / 2);
      setWindowStep(Math.floor(element.scrollLeft / step));
    };
    // Once a frame; a timer too, for a window that is not drawing frames just now.
    const onScroll = () => {
      if (frame || timer) return;
      frame = requestAnimationFrame(update);
      timer = window.setTimeout(update, 100);
    };
    element.addEventListener("scroll", onScroll, { passive: true });
    update();
    return () => {
      element.removeEventListener("scroll", onScroll);
      cancelAnimationFrame(frame);
      window.clearTimeout(timer);
    };
  }, [openedProject?.projectHandle]);
  // A little room after the last clip, so it can be dragged past the end.
  const spanUs = Math.max(durationUs * 1.05, 10_000_000);
  const contentPx = viewportPx * timelineZoom;
  const pxPerUs = spanUs > 0 ? contentPx / spanUs : 0;
  const pct = (us: number) => `${(us / spanUs) * 100}%`;
  const clientXToUs = (clientX: number) => {
    const rect = lanesRef.current?.getBoundingClientRect();
    if (!rect || rect.width <= 0) return 0;
    return Math.round(Math.max(0, Math.min(1, (clientX - rect.left) / rect.width)) * spanUs);
  };
  const snapUs = () => (pxPerUs > 0 ? SNAP_PX / pxPerUs : 0);
  const view: TimelineView = { durationUs: spanUs, pxPerUs, nowUs, pct, clientXToUs, snapUs, seekToUs };

  const zoomTimeline = (factor: number, anchor?: { timeUs: number; offsetPx: number }) => {
    const next = Math.min(MAX_TIMELINE_ZOOM, Math.max(MIN_TIMELINE_ZOOM, timelineZoom * factor));
    if (next === timelineZoom) return;
    const element = scrollRef.current;
    zoomAnchor.current =
      anchor ?? (element && pxPerUs > 0 ? { timeUs: nowUs(), offsetPx: nowUs() * pxPerUs - element.scrollLeft } : null);
    setTimelineZoom(next);
  };
  useLayoutEffect(() => {
    const anchor = zoomAnchor.current;
    const element = scrollRef.current;
    zoomAnchor.current = null;
    if (!anchor || !element || pxPerUs <= 0) return;
    element.scrollLeft = Math.max(0, anchor.timeUs * pxPerUs - anchor.offsetPx);
  }, [timelineZoom, pxPerUs]);
  const zoomRef = useRef(zoomTimeline);
  zoomRef.current = zoomTimeline;
  useEffect(() => {
    const element = scrollRef.current;
    if (!element) return;
    // The wheel scrolls the timeline sideways (Shift + wheel: the tracks up and down, as over
    // the track headers); Ctrl/Cmd + wheel zooms around the cursor. A trackpad's sideways swipe
    // scrolls as it always does. The listener is not passive so it can take the wheel over.
    const onWheel = (event: WheelEvent) => {
      if (!(event.ctrlKey || event.metaKey)) {
        const unit = event.deltaMode === 1 ? 32 : event.deltaMode === 2 ? element.clientWidth : 1;
        // Shift: up and down. Browsers turn Shift + wheel into sideways, so either axis counts.
        if (event.shiftKey) {
          const tracks = element.parentElement?.closest(".overflow-y-auto") as HTMLElement | null;
          if (!tracks) return;
          event.preventDefault();
          tracks.scrollTop += (event.deltaY || event.deltaX) * unit;
          return;
        }
        const sideways = Math.abs(event.deltaX) > Math.abs(event.deltaY);
        if (sideways || event.deltaY === 0) return;
        // Nothing to scroll sideways (the whole video fits): the wheel scrolls the tracks.
        if (element.scrollWidth <= element.clientWidth) return;
        event.preventDefault();
        element.scrollLeft += event.deltaY * unit;
        return;
      }
      event.preventDefault();
      const rect = element.getBoundingClientRect();
      const offsetPx = event.clientX - rect.left;
      const width = element.scrollWidth;
      const timeUs = width > 0 ? ((element.scrollLeft + offsetPx) / width) * spanUs : 0;
      zoomRef.current(event.deltaY < 0 ? 1.25 : 0.8, { timeUs, offsetPx });
    };
    element.addEventListener("wheel", onWheel, { passive: false });
    return () => element.removeEventListener("wheel", onWheel);
  }, [openedProject?.projectHandle, spanUs]);

  const rulerStep = rulerStepUs(pxPerUs);
  const rulerTicks = pxPerUs > 0 ? Array.from({ length: Math.floor(spanUs / rulerStep) + 1 }, (_, i) => i * rulerStep) : [];

  // ---- Selection ----------------------------------------------------------------------------
  const selected = new Set(selectedClipIds);
  const selectClips = (ids: string[]) => {
    setSelectedClipIds(ids);
    setSelectedZoomId(undefined);
    captions.clear();
  };
  /** Click: the clip and its linked partners (Alt: just the clip); Shift/Ctrl adds or removes them. */
  const pickClip = (clip: Clip, event: React.MouseEvent | React.PointerEvent) => {
    const group = event.altKey ? [clip.id] : withPartners(sequence, [clip.id]);
    if (event.shiftKey || event.ctrlKey || event.metaKey) {
      const on = selected.has(clip.id);
      selectClips(on ? selectedClipIds.filter((id) => !group.includes(id)) : [...new Set([...selectedClipIds, ...group])]);
    } else {
      selectClips(group);
    }
    setRange(null);
  };
  const clearSelection = () => {
    setRange(null);
    setSelectedClipIds([]);
  };
  /** Click on a track's name: all its clips; Shift/Ctrl adds or removes them. */
  const pickTrack = (track: SeqTrack, event: React.MouseEvent) => {
    if (track.locked) return;
    const ids = track.clips.map((c) => c.id);
    if (event.shiftKey || event.ctrlKey || event.metaKey) {
      const all = ids.length > 0 && ids.every((id) => selected.has(id));
      selectClips(all ? selectedClipIds.filter((id) => !ids.includes(id)) : [...new Set([...selectedClipIds, ...ids])]);
    } else {
      selectClips(ids);
    }
    setRange(null);
  };

  // ---- Marquee: drag over empty space to select the clips the box touches -------------------
  const marqueeRef = useRef<{
    pointerId: number;
    startX: number;
    startY: number;
    active: boolean;
    /** Shift/Ctrl: the box adds to what was selected. */
    base: string[];
    /** Alt: only the clips touched, not their linked partners. */
    alone: boolean;
    rows: RowBox[];
    frame: number;
    x: number;
    y: number;
  } | null>(null);
  const [marquee, setMarquee] = useState<{ x0: number; y0: number; x1: number; y1: number } | null>(null);
  const onMarqueeDown = (event: React.PointerEvent<HTMLDivElement>) => {
    // A drag that ended on a handle never delivers its click here: clear its flag.
    suppressClick.current = false;
    const target = event.target as HTMLElement;
    const empty =
      target === event.currentTarget || target.hasAttribute("data-track-row") || target.hasAttribute("data-lanes-stack");
    if (event.button !== 0 || !empty || target.closest("[data-clip]")) return;
    marqueeRef.current = {
      pointerId: event.pointerId,
      startX: event.clientX,
      startY: event.clientY,
      active: false,
      base: event.shiftKey || event.ctrlKey || event.metaKey ? selectedClipIds : [],
      alone: event.altKey,
      rows: [],
      frame: 0,
      x: event.clientX,
      y: event.clientY,
    };
  };
  const marqueeSelect = () => {
    const m = marqueeRef.current;
    if (!m) return;
    m.frame = 0;
    const [left, right] = [Math.min(m.startX, m.x), Math.max(m.startX, m.x)];
    const [top, bottom] = [Math.min(m.startY, m.y), Math.max(m.startY, m.y)];
    const fromUs = clientXToUs(left);
    const toUs = clientXToUs(right);
    const rows = new Set(
      m.rows.flatMap(({ row, rect }) => (row.kind === "track" && rect.bottom > top && rect.top < bottom ? [row.trackId] : [])),
    );
    const touched = sequence.tracks
      .filter((t) => rows.has(t.id) && !t.locked)
      .flatMap((t) => t.clips.filter((c) => c.startUs < toUs && clipEnd(c) > fromUs).map((c) => c.id));
    const ids = m.alone ? touched : withPartners(sequence, touched);
    setSelectedClipIds(m.base.length > 0 ? [...new Set([...m.base, ...ids])] : ids);
    setMarquee({ x0: m.startX, y0: m.startY, x1: m.x, y1: m.y });
  };
  const onMarqueeMove = (event: React.PointerEvent<HTMLDivElement>) => {
    const m = marqueeRef.current;
    if (!m || m.pointerId !== event.pointerId) return;
    m.x = event.clientX;
    m.y = event.clientY;
    if (!m.active) {
      if (Math.hypot(m.x - m.startX, m.y - m.startY) < CLIP_DRAG_PX) return;
      m.active = true;
      m.rows = rowBoxes();
      event.currentTarget.setPointerCapture(event.pointerId);
      setRange(null);
      setSelectedZoomId(undefined);
      captions.clear();
    }
    // One selection pass a frame, however fast the pointer moves.
    if (!m.frame) m.frame = requestAnimationFrame(marqueeSelect);
  };
  const onMarqueeUp = () => {
    const m = marqueeRef.current;
    marqueeRef.current = null;
    if (!m?.active) return;
    if (m.frame) cancelAnimationFrame(m.frame);
    marqueeRef.current = m;
    marqueeSelect();
    marqueeRef.current = null;
    setMarquee(null);
    // The click that ends the drag neither seeks nor clears what the box selected.
    suppressClick.current = true;
  };
  const selectedClips = selectedClipIds.flatMap((id) => {
    const found = clipIndex.get(id);
    return found ? [found.clip] : [];
  });
  // The selected clips' span: what Cam Focus, Normal view and the Zoom panel act on without a range.
  const selectedSpan =
    selectedClips.length > 0
      ? { startUs: Math.min(...selectedClips.map((c) => c.startUs)), endUs: Math.max(...selectedClips.map(clipEnd)) }
      : null;
  const selection = range ?? selectedSpan;
  useEffect(() => {
    setTimelineSelection(selection);
  }, [selection?.startUs, selection?.endUs, setTimelineSelection]);
  const selectRange = (startUs: number, endUs: number) => {
    const a = Math.max(0, Math.min(startUs, endUs));
    const b = Math.min(durationUs, Math.max(startUs, endUs));
    setRange(b > a ? { startUs: a, endUs: b } : null);
    if (b > a) setSelectedClipIds([]);
  };

  // ---- Snapping -----------------------------------------------------------------------------
  const snapPoints = (edges: number[] = clipEdges) => [nowUs(), 0].concat(edges);

  // ---- Edits --------------------------------------------------------------------------------
  /** S: the selected clips split at the playhead; with none selected, every track. */
  const splitAtPlayhead = () => {
    const at = nowUs();
    if (selectedClipIds.length > 0 && !selectedClips.some((c) => c.startUs < at && at < clipEnd(c))) {
      setEditError("Put the playhead over a selected clip to split it, or deselect (Ctrl+D) to split every track.");
      return;
    }
    return edit({ kind: "split", atUs: at, clipIds: selectedClipIds });
  };
  /**
   * Q and E: ripple-delete from the playhead to the previous or next edit point on every track;
   * with one clip (and its partners) selected, that clip alone is trimmed to the playhead.
   */
  const rippleTrim = (side: "previous" | "next") => {
    const at = nowUs();
    const groups = new Set(selectedClips.map((c) => c.link ?? c.id));
    if (groups.size > 1) {
      setEditError("Select one clip to trim, or deselect (Ctrl+D) to trim every track.");
      return;
    }
    const clip = selectedClips[0];
    if (clip) {
      if (!(clip.startUs < at && at < clipEnd(clip))) {
        setEditError("Put the playhead inside the selected clip to trim it.");
        return;
      }
      return edit({ kind: "trimClip", clipId: clip.id, edge: side === "previous" ? "start" : "end", toUs: at, ripple: true }).then(
        (applied) => {
          if (applied && side === "previous") seekToUs(clip.startUs);
        },
      );
    }
    const previous = [...edges].reverse().find((e) => e < at);
    return edit({ kind: "rippleTrim", atUs: at, side }).then((applied) => {
      // Q pulls what comes after back to the edit point; the playhead follows it.
      if (applied && side === "previous" && previous !== undefined) seekToUs(previous);
    });
  };
  /** Delete: the selected clips (closing up when magnetic), else the range, a zoom or a caption. */
  const deleteSelection = (ripple?: boolean) => {
    if (captions.active) return captions.hideCue();
    if (selectedClipIds.length > 0) {
      void edit({ kind: "delete", clipIds: selectedClipIds, ripple: ripple ?? null }).then((applied) => {
        if (applied) setSelectedClipIds([]);
      });
      return;
    }
    if (range) {
      void edit({ kind: "deleteRange", ranges: [range], ripple: ripple ?? null }).then((applied) => applied && setRange(null));
      return;
    }
    if (zooms.selectedZoomId) zooms.deleteZoom(zooms.selectedZoomId);
  };
  /** Keep only the range: everything else goes. */
  const keepOnlyRange = () => {
    if (!range) return;
    const cuts = [
      ...(range.startUs > 0 ? [{ startUs: 0, endUs: range.startUs }] : []),
      ...(range.endUs < durationUs ? [{ startUs: range.endUs, endUs: durationUs }] : []),
    ];
    if (cuts.length) void edit({ kind: "deleteRange", ranges: cuts, ripple: true }).then((applied) => applied && setRange(null));
  };
  /** U: selected clips that are one linked set unlink; otherwise the selection links into one set. */
  const linkState = (() => {
    if (selectedClips.length === 0) return null;
    const links = new Set(selectedClips.map((c) => c.link ?? `!${c.id}`));
    if (links.size === 1 && selectedClips[0].link) return "linked";
    return selectedClips.length >= 2 ? "unlinked" : null;
  })();
  const toggleLink = () => {
    if (linkState === "linked") void edit({ kind: "unlink", clipIds: selectedClipIds });
    else if (linkState === "unlinked") void edit({ kind: "link", clipIds: selectedClipIds });
    else setEditError("Select clips to link them, or a linked clip to unlink it.");
  };
  const undo = () => {
    if (openedProject?.undoAvailable) void runEdit((p) => api.projectUndo(p.projectHandle, p.revision));
  };
  const redo = () => {
    if (openedProject?.redoAvailable) void runEdit((p) => api.projectRedo(p.projectHandle, p.revision));
  };
  const jumpToEdit = (direction: -1 | 1) => {
    const currentTimeUs = nowUs();
    const target =
      direction < 0 ? [...edges].reverse().find((e) => e < currentTimeUs) : edges.find((e) => e > currentTimeUs && e <= durationUs);
    if (target !== undefined) seekToUs(target);
  };
  const markIn = () => {
    const at = nowUs();
    selectRange(at, range && range.endUs > at ? range.endUs : durationUs);
  };
  const markOut = () => {
    const at = nowUs();
    selectRange(range && range.startUs < at ? range.startUs : 0, at);
  };

  // Restoring cut time: every join between two pieces of one stretch of a source.
  // Worked out once per revision: they only change with the sequence.
  const joinsByTrack = useMemo(() => {
    const byTrack = new Map<string, { clipId: string; atUs: number; gapUs: number }[]>();
    if (!openedProject) return byTrack;
    for (const track of openedProject.sequence.tracks) if (!track.locked) byTrack.set(track.id, cutJoins(openedProject, track));
    return byTrack;
  }, [openedProject]);
  const cutCount = useMemo(() => new Set([...joinsByTrack.values()].flat().map((j) => j.atUs)).size, [joinsByTrack]);
  const restoreCut = useRef((_clipId: string) => {});
  restoreCut.current = (clipId) => void edit({ kind: "restoreCuts", clipIds: [clipId] });

  // ---- Webcam focus and normal view ---------------------------------------------------------
  const focus = openedProject?.webcamFocus ?? DEFAULT_WEBCAM_FOCUS;
  const focusAsset = focus.media ?? openedProject?.assets.find((a) => a.kind === "recording")?.id;
  const cameraClips = useMemo(
    () =>
      (openedProject?.sequence.tracks ?? [])
        .filter((t) => t.kind === "video")
        .flatMap((track) =>
          track.clips.filter((c) => c.asset === focusAsset && clipRole(openedProject, track, c) === "webcam").map((clip) => ({ track, clip })),
        ),
    [openedProject, focusAsset],
  );
  const normalView: SourceRange[] = (focus.normalView ?? []).map((r) => [r.sourceStartUs, r.sourceEndUs]);
  const selectionSource: SourceRange[] = selection
    ? cameraClips.flatMap(({ clip }): SourceRange[] => {
        const start = Math.max(selection.startUs, clip.startUs);
        const end = Math.min(selection.endUs, clipEnd(clip));
        if (end <= start) return [];
        const offset = clip.inUs - clip.startUs;
        return [[start + offset, end + offset]];
      })
    : [];
  const selectionInNormalView = selectionSource.length > 0 && subtractRanges(selectionSource, normalView).length === 0;
  const toggleNormalView = () => {
    if (selectionSource.length === 0) return;
    const next = selectionInNormalView ? subtractRanges(normalView, selectionSource) : [...normalView, ...selectionSource];
    void runEdit((project) =>
      api.projectWebcamFocusUpdate(project.projectHandle, project.revision, {
        ...(project.webcamFocus ?? DEFAULT_WEBCAM_FOCUS),
        media: focusAsset,
        normalView: next.map(([sourceStartUs, sourceEndUs]) => ({ sourceStartUs, sourceEndUs })),
      }),
    );
  };
  const focusEdited: SourceRange[] = focus.enabled
    ? (focus.segments ?? []).filter((s) => s.enabled).flatMap((s) => (s.editedRanges ?? []).map((r): SourceRange => [r.startUs, r.endUs]))
    : [];
  const selectionFocused = !!selection && subtractRanges([[selection.startUs, selection.endUs]], focusEdited).length === 0;
  const toggleCamFocus = () => {
    if (!selection) return;
    const { startUs, endUs } = selection;
    void runEdit((project) =>
      selectionFocused
        ? api.projectWebcamFocusRemove(project.projectHandle, project.revision, startUs, endUs)
        : api.projectWebcamFocusAdd(project.projectHandle, project.revision, startUs, endUs),
    );
  };

  // ---- Lanes on top: captions and zooms ------------------------------------------------------
  const zooms = useZoomLane({
    view,
    height: lanes.height("lane:zooms"),
    onResize: (h) => lanes.setHeight("lane:zooms", h),
    snapPoints: () => snapPoints(),
    selection,
    onError: setEditError,
    onPicked: () => {
      setSelectedClipIds([]);
      setRange(null);
      captions.clear();
    },
    suppressClick,
  });
  const captions = useCaptionLane({
    view,
    height: lanes.height("lane:captions"),
    onResize: (h) => lanes.setHeight("lane:captions", h),
    onError: setEditError,
    onPicked: () => {
      setSelectedClipIds([]);
      setSelectedZoomId(undefined);
    },
    suppressClick,
    hint,
  });

  // ---- Keyboard -----------------------------------------------------------------------------
  const actions = useRef<Partial<Record<HotkeyAction, () => void>>>({});
  actions.current = {
    playPause: togglePlayPause,
    shuttleForward,
    pause: pausePlayback,
    split: () => (captions.active ? captions.splitCue() : void splitAtPlayhead()),
    rippleTrimPrevious: () => void rippleTrim("previous"),
    rippleTrimNext: () => void rippleTrim("next"),
    deleteSelection: () => deleteSelection(),
    rippleDelete: () => deleteSelection(true),
    deselect: () => {
      captions.clear();
      clearSelection();
      setSelectedZoomId(undefined);
    },
    selectAll: () => {
      setRange(null);
      setSelectedClipIds(sequence.tracks.filter((t) => !t.locked).flatMap((t) => t.clips.map((c) => c.id)));
    },
    deselectAll: () => setSelectedClipIds([]),
    toggleLink,
    markIn,
    markOut,
    undo,
    redo,
    stepBack: () => seekToUs(nowUs() - frameUs),
    stepForward: () => seekToUs(nowUs() + frameUs),
    stepBackLong: () => seekToUs(nowUs() - 1_000_000),
    stepForwardLong: () => seekToUs(nowUs() + 1_000_000),
    previousEdit: () => jumpToEdit(-1),
    nextEdit: () => jumpToEdit(1),
    zoomIn: () => zoomRef.current(2),
    zoomOut: () => zoomRef.current(0.5),
    goToStart: () => seekToUs(0),
    goToEnd: () => seekToUs(Number.MAX_SAFE_INTEGER),
  };
  // Actions that act once per press; the rest (stepping, zoom) repeat while the key is held.
  const ONCE: HotkeyAction[] = ["playPause", "shuttleForward", "pause", "split", "rippleTrimPrevious", "rippleTrimNext", "toggleLink", "deleteSelection", "rippleDelete", "selectAll", "deselectAll"];
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

  // ---- Ruler: scrub, Shift+drag for a range --------------------------------------------------
  const scrub = useRef<{ frame: number | null; targetUs: number } | null>(null);
  const scrubTo = (clientX: number) => {
    const state = scrub.current;
    if (!state) return;
    state.targetUs = Math.min(durationUs, clientXToUs(clientX));
    if (state.frame !== null) return;
    state.frame = requestAnimationFrame(() => {
      if (!scrub.current) return;
      scrub.current.frame = null;
      seekToUs(scrub.current.targetUs);
    });
  };
  const rangeDrag = useRef<{ startX: number; active: boolean } | null>(null);
  const onRulerPointerDown = (event: React.PointerEvent<HTMLDivElement>) => {
    if (event.button !== 0 || !openedProject) return;
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

  // ---- Waveforms: each sound once, over its own time; clips draw their part ------------------
  const soundKeys = useMemo(() => {
    const keys = new Map<string, number>();
    for (const track of audioTracks) {
      for (const clip of track.clips) {
        const asset = assetById(openedProject, clip.asset);
        if (asset) keys.set(streamKey(clip.asset, clip.stream), asset.durationUs);
      }
    }
    return keys;
  }, [openedProject?.revision, openedProject?.projectHandle, openedProject?.shortView]);
  useEffect(() => {
    if (!openedProject) return;
    let active = true;
    for (const key of soundKeys.keys()) {
      if (waveforms[key]) continue;
      void api
        .projectWaveformOverview(openedProject.projectHandle, key)
        .then((overview) => active && setWaveform(key, overview))
        .catch((err) => console.warn("[Timeline] Waveform failed:", err));
    }
    return () => {
      active = false;
    };
  }, [openedProject?.projectHandle, soundKeys]);

  // ---- Dragging clips: move (along and between tracks) or trim an edge -----------------------
  const dragRef = useRef<ClipDrag | null>(null);
  const dragFrame = useRef<number | null>(null);
  const [drag, setDrag] = useState<ClipDrag | null>(null);
  const beginClipDrag = (event: React.PointerEvent<HTMLElement>, track: SeqTrack, clip: Clip, mode: ClipDrag["mode"]) => {
    if (event.button !== 0 || editing || !openedProject || track.locked) return;
    event.stopPropagation();
    suppressClick.current = false;
    event.currentTarget.setPointerCapture(event.pointerId);
    // The selection moves when the pressed clip is in it; otherwise the clip and its partners.
    const ids =
      mode === "move" && selected.has(clip.id)
        ? selectedClipIds
        : event.altKey
          ? [clip.id]
          : withPartners(sequence, [clip.id]);
    const edge = mode === "end" ? clipEnd(clip) : clip.startUs;
    const idSet = new Set(ids);
    const moving = ids.flatMap((id) => clipIndex.get(id)?.clip ?? []);
    dragRef.current = {
      rows: rowBoxes(),
      rowsAt: performance.now(),
      idSet,
      earliestUs: Math.min(...moving.map((c) => c.startUs)),
      snapEdges: sequence.tracks.flatMap((t) => t.clips.filter((c) => !idSet.has(c.id)).flatMap((c) => [c.startUs, clipEnd(c)])),
      pointerId: event.pointerId,
      startX: event.clientX,
      startY: event.clientY,
      active: false,
      mode,
      anchor: clip,
      anchorTrack: track,
      ids,
      grabUs: clientXToUs(event.clientX) - edge,
      deltaUs: 0,
      edgeUs: edge,
      row: { kind: "track", trackId: track.id },
    };
  };
  const moveClipDrag = (event: React.PointerEvent<HTMLElement>) => {
    const current = dragRef.current;
    if (!current || current.pointerId !== event.pointerId) return;
    event.stopPropagation();
    if (!current.active) {
      const moved = Math.max(Math.abs(event.clientX - current.startX), Math.abs(event.clientY - current.startY));
      if (moved < (current.mode === "move" ? CLIP_DRAG_PX : 2)) return;
      current.active = true;
    }
    const pointerUs = clientXToUs(event.clientX);
    if (current.mode === "move") {
      const start = snapStart(pointerUs - current.grabUs, current.anchor.durationUs, snapPoints(current.snapEdges), snapUs());
      current.deltaUs = Math.max(-current.earliestUs, start - current.anchor.startUs);
      if (performance.now() - current.rowsAt > 250) {
        current.rows = rowBoxes();
        current.rowsAt = performance.now();
      }
      current.row = rowAtPoint(current.rows, event.clientX, event.clientY) ?? current.row;
    } else {
      current.edgeUs = Math.max(0, snapStart(pointerUs - current.grabUs, 0, snapPoints(current.snapEdges), snapUs()));
    }
    // Pointer events come faster than frames: the timeline redraws once a frame at most.
    if (dragFrame.current === null) {
      dragFrame.current = requestAnimationFrame(() => {
        dragFrame.current = null;
        if (dragRef.current) setDrag({ ...dragRef.current });
      });
    }
  };
  const endClipDrag = (event: React.PointerEvent<HTMLElement>) => {
    const current = dragRef.current;
    if (!current || current.pointerId !== event.pointerId) return;
    event.stopPropagation();
    dragRef.current = null;
    setDrag(null);
    if (!current.active) return; // A plain click: onClick selects it.
    suppressClick.current = true;
    if (current.mode !== "move") {
      const before = current.mode === "start" ? current.anchor.startUs : clipEnd(current.anchor);
      if (Math.abs(current.edgeUs - before) >= 1_000) {
        void edit({ kind: "trimClip", clipId: current.anchor.id, edge: current.mode, toUs: current.edgeUs });
      }
      return;
    }
    const row = current.row;
    const kind = current.anchorTrack.kind;
    if (row?.kind === "new") {
      if (row.audio !== (kind === "audio")) {
        setEditError(kind === "audio" ? "Sound goes on audio tracks." : "Pictures go on video tracks.");
        return;
      }
      void editOnNewTrack(row.audio, (trackId) => ({
        kind: "moveClips",
        clipIds: current.ids,
        deltaUs: current.deltaUs,
        trackId,
        anchorId: current.anchor.id,
      }));
      return;
    }
    const target = row?.kind === "track" ? sequence.tracks.find((t) => t.id === row.trackId) : undefined;
    const trackId = target && target.kind === kind && target.id !== current.anchorTrack.id ? target.id : null;
    if (target && target.kind !== kind) {
      setEditError(kind === "audio" ? "Sound goes on audio tracks." : "Pictures go on video tracks.");
      return;
    }
    if (current.deltaUs === 0 && !trackId) return;
    void edit({ kind: "moveClips", clipIds: current.ids, deltaUs: current.deltaUs, trackId, anchorId: current.anchor.id });
  };

  /** Where each moving clip lands while dragging: its track (shifted with the anchor's) and start. */
  const dragGhosts = (() => {
    if (!drag?.active || drag.mode !== "move") return [];
    const kind = drag.anchorTrack.kind;
    const ofKind = sequence.tracks.filter((t) => t.kind === kind);
    const anchorIndex = ofKind.findIndex((t) => t.id === drag.anchorTrack.id);
    let offset = 0;
    if (drag.row?.kind === "track") {
      const index = ofKind.findIndex((t) => t.id === (drag.row as { trackId: string }).trackId);
      if (index >= 0) offset = index - anchorIndex;
    } else if (drag.row?.kind === "new" && drag.row.audio === (kind === "audio")) {
      offset = ofKind.length - anchorIndex;
    }
    return drag.ids.flatMap((id) => {
      const found = clipIndex.get(id);
      if (!found) return [];
      const own = sequence.tracks.filter((t) => t.kind === found.track.kind);
      const index = own.findIndex((t) => t.id === found.track.id) + (found.track.kind === kind ? offset : 0);
      const row: Row = index >= own.length ? { kind: "new", audio: found.track.kind === "audio" } : { kind: "track", trackId: own[Math.max(0, index)]?.id ?? "" };
      return [{ row, startUs: found.clip.startUs + drag.deltaUs, durationUs: found.clip.durationUs }];
    });
  })();
  /** The trimmed or extended part while an edge is dragged, on each clip that trims. */
  const trimShade = (clip: Clip) => {
    if (!drag?.active || drag.mode === "move" || !drag.idSet.has(clip.id)) return null;
    const edgeBefore = drag.mode === "start" ? drag.anchor.startUs : clipEnd(drag.anchor);
    const mine = drag.mode === "start" ? clip.startUs : clipEnd(clip);
    if (mine !== edgeBefore) return null;
    const delta = drag.edgeUs - edgeBefore;
    if (delta === 0) return null;
    const shrinking = drag.mode === "start" ? delta > 0 : delta < 0;
    return { from: Math.min(edgeBefore, drag.edgeUs), to: Math.max(edgeBefore, drag.edgeUs), shrinking };
  };

  // ---- Dropping media from the Media panel ---------------------------------------------------
  const [mediaGhost, setMediaGhost] = useState<{ row: Row; startUs: number; durationUs: number } | null>(null);
  const [insertAtUs, setInsertAtUs] = useState<number | null>(null);
  const onMediaDragOver = (event: React.DragEvent<HTMLDivElement>) => {
    if (!event.dataTransfer.types.includes(MEDIA_DRAG_TYPE) || !openedProject) return;
    event.preventDefault();
    event.dataTransfer.dropEffect = "copy";
    const asset = assetById(openedProject, currentMediaDrag() ?? "");
    const row = rowFromElement(event.target as Element);
    if (!asset || !row) {
      setMediaGhost(null);
      setInsertAtUs(null);
      return;
    }
    const length = defaultClipUs(asset);
    const startUs = snapStart(clientXToUs(event.clientX), length, snapPoints(), snapUs());
    setMediaGhost({ row, startUs, durationUs: length });
    // Magnetic: landing on a clip makes room for it.
    const track = row.kind === "track" ? sequence.tracks.find((t) => t.id === row.trackId) : undefined;
    const occupied = !!track?.clips.some((c) => c.startUs < startUs + length && startUs < clipEnd(c));
    setInsertAtUs(magnetic && occupied ? startUs : null);
  };
  const onMediaDrop = (event: React.DragEvent<HTMLDivElement>) => {
    const assetId = event.dataTransfer.getData(MEDIA_DRAG_TYPE);
    const ghost = mediaGhost;
    setMediaGhost(null);
    setInsertAtUs(null);
    if (!assetId || !openedProject) return;
    event.preventDefault();
    const atUs = ghost?.startUs ?? snapStart(clientXToUs(event.clientX), 0, snapPoints(), snapUs());
    if (ghost?.row.kind === "new") {
      void editOnNewTrack(ghost.row.audio, (trackId) => ({ kind: "placeAsset", assetId, atUs, trackId }));
      return;
    }
    const track = ghost?.row.kind === "track" ? sequence.tracks.find((t) => t.id === (ghost.row as { trackId: string }).trackId) : undefined;
    if (track?.locked) {
      setEditError("That track is locked.");
      return;
    }
    void edit({ kind: "placeAsset", assetId, atUs, trackId: track?.id ?? null });
  };
  const dragKinds = (() => {
    if (drag?.active && drag.mode === "move") return { video: drag.anchorTrack.kind === "video", audio: drag.anchorTrack.kind === "audio" };
    if (mediaGhost) return { video: true, audio: true };
    return null;
  })();

  const ghostsOn = (row: Row) => {
    const same = (other: Row) =>
      other.kind === row.kind && (row.kind === "new" ? (other as { audio: boolean }).audio === row.audio : (other as { trackId: string }).trackId === row.trackId);
    const ghosts = dragGhosts.filter((g) => same(g.row));
    if (mediaGhost && same(mediaGhost.row)) ghosts.push(mediaGhost);
    return ghosts.map((ghost, i) => (
      <div
        key={`ghost-${i}`}
        className="absolute top-1 bottom-1 rounded-md border-2 border-dashed border-accent-fg bg-accent/20 z-30 pointer-events-none"
        style={{ left: pct(ghost.startUs), width: pct(ghost.durationUs) }}
      />
    ));
  };

  // ---- Clicking empty space ------------------------------------------------------------------
  const handleLanesClick = (event: React.MouseEvent<HTMLDivElement>) => {
    if (suppressClick.current) {
      suppressClick.current = false;
      return;
    }
    // Clips stop their clicks, so a click that lands here hit empty space: deselect and seek.
    if (!(event.shiftKey || event.ctrlKey || event.metaKey)) {
      clearSelection();
      captions.clear();
      setSelectedZoomId(undefined);
    }
    seekToUs(Math.min(durationUs, clientXToUs(event.clientX)));
  };

  // ---- Track headers -------------------------------------------------------------------------
  const setTrack = (track: SeqTrack, change: Partial<Pick<SeqTrack, "name" | "hidden" | "muted" | "locked" | "role">>) => {
    const next = { ...track, ...change };
    void edit({
      kind: "setTrack",
      trackId: track.id,
      name: next.name ?? "",
      hidden: !!next.hidden,
      muted: !!next.muted,
      locked: !!next.locked,
      role: next.role ?? null,
    });
  };
  const [renaming, setRenaming] = useState<{ trackId: string; name: string } | null>(null);
  const roleCycle = (track: SeqTrack): (Role | undefined)[] =>
    track.kind === "video" ? [undefined, "screen", "webcam", "overlay"] : [undefined, "mic", "background"];
  const roleButton = (track: SeqTrack) => {
    const cycle = roleCycle(track);
    const next = cycle[(cycle.indexOf(track.role) + 1) % cycle.length];
    const Icon =
      track.role === "webcam" ? Camera : track.role === "overlay" ? Layers : track.role === "mic" ? Mic : track.role === "background" ? Music : track.kind === "video" ? Monitor : AudioLines;
    const current = track.role ? ROLE_LABEL[track.role] : "Each clip's own";
    return (
      <button
        type="button"
        disabled={editing}
        aria-label={`${trackNumber(sequence, track.id)} plays as: ${current}`}
        title={`Plays as: ${current}. Click for ${next ? ROLE_LABEL[next] : "each clip's own role"}.${
          track.kind === "video"
            ? " Screen gets the canvas layout, zooms and cursor; Camera goes in the bubble; Overlay covers the canvas."
            : " Speech is transcribed, captioned and ducks background sound."
        }`}
        onClick={() => setTrack(track, { role: next })}
        className={cn(HDR_BUTTON, track.role ? (track.kind === "video" ? "!text-video-fg" : "!text-audio-fg") : "!text-studio-500")}
      >
        <Icon className="w-4 h-4" />
      </button>
    );
  };
  const trackHeader = (track: SeqTrack) => {
    const audio = track.kind === "audio";
    const number = trackNumber(sequence, track.id);
    const label = trackLabel(sequence, track);
    const height = lanes.height(audio ? "lane:audio" : "lane:video");
    const kindIndex = (audio ? audioTracks : videoTracks).findIndex((t) => t.id === track.id);
    const kindCount = (audio ? audioTracks : videoTracks).length;
    // Video tracks list top track first: "up" draws over more.
    const canUp = audio ? kindIndex > 0 : kindIndex < kindCount - 1;
    const canDown = audio ? kindIndex < kindCount - 1 : kindIndex > 0;
    const isRenaming = renaming?.trackId === track.id;
    const roleMeta = track.role ? ROLE_LABEL[track.role] : `${track.clips.length} clip${track.clips.length === 1 ? "" : "s"}`;
    return (
      <HeaderRow
        key={track.id}
        height={height}
        tone={audio ? "audio" : "video"}
        icon={audio ? AudioLines : Film}
        dim={track.hidden || track.muted}
        grip={
          <ResizeGrip
            label={audio ? "audio tracks" : "video tracks"}
            height={height}
            onResize={(h) => lanes.setHeight(audio ? "lane:audio" : "lane:video", h)}
          />
        }
        name={
          isRenaming ? (
            <input
              autoFocus
              aria-label={`Name ${number}`}
              value={renaming.name}
              maxLength={40}
              onChange={(e) => setRenaming({ trackId: track.id, name: e.target.value })}
              onKeyDown={(e) => {
                if (e.key === "Enter") {
                  setRenaming(null);
                  if (renaming.name.trim() !== (track.name ?? "")) setTrack(track, { name: renaming.name.trim() });
                } else if (e.key === "Escape") setRenaming(null);
              }}
              onBlur={() => setRenaming(null)}
              className="w-full bg-studio-950 text-label text-white px-1.5 h-6 rounded outline-none border border-accent"
            />
          ) : (
            <span
              className="cursor-pointer"
              onClick={(event) => pickTrack(track, event)}
              onDoubleClick={() => setRenaming({ trackId: track.id, name: track.name ?? "" })}
              title="Click to select its clips (Shift+click to add or remove them). Double-click to rename."
            >
              {track.name ? `${number} · ${label}` : number}
            </span>
          )
        }
        meta={track.locked ? "Locked" : roleMeta}
        actions={
          <>
            <div className="flex flex-col">
              {([true, false] as const).map((up) => (
                <button
                  key={up ? "up" : "down"}
                  disabled={editing || !(up ? canUp : canDown)}
                  aria-label={`Move ${number} ${up ? "up" : "down"}`}
                  title={`Move ${number} ${up ? "up" : "down"}${audio ? "" : up ? ": it draws over more" : ": it draws under more"}`}
                  onClick={() => void edit({ kind: "moveTrack", trackId: track.id, up })}
                  className="h-3.5 w-5 inline-flex items-center justify-center rounded text-studio-500 hover:text-studio-100 hover:bg-studio-800 disabled:opacity-30"
                >
                  {up ? <ChevronUp className="w-3.5 h-3.5" /> : <ChevronDown className="w-3.5 h-3.5" />}
                </button>
              ))}
            </div>
            {roleButton(track)}
            <button
              type="button"
              disabled={editing}
              aria-pressed={!!track.locked}
              aria-label={`${track.locked ? "Unlock" : "Lock"} ${number}`}
              title={track.locked ? "Unlock this track" : "Lock this track: edits and ripples leave it alone"}
              onClick={() => setTrack(track, { locked: !track.locked })}
              className={cn(HDR_BUTTON, track.locked && "!text-suggest-fg")}
            >
              {track.locked ? <Lock className="w-4 h-4" /> : <LockOpen className="w-4 h-4" />}
            </button>
            {audio ? (
              <button
                type="button"
                disabled={editing}
                aria-pressed={!!track.muted}
                aria-label={`${track.muted ? "Unmute" : "Mute"} ${number}`}
                title={track.muted ? "Unmute this track" : "Mute this track in preview and export"}
                onClick={() => setTrack(track, { muted: !track.muted })}
                className={cn(HDR_BUTTON, track.muted && "text-danger-fg")}
              >
                {track.muted ? <VolumeX className="w-4 h-4" /> : <Volume2 className="w-4 h-4" />}
              </button>
            ) : (
              <button
                type="button"
                disabled={editing}
                aria-pressed={!!track.hidden}
                aria-label={`${track.hidden ? "Show" : "Hide"} ${number}`}
                title={track.hidden ? "Show this track" : "Hide this track in preview and export"}
                onClick={() => setTrack(track, { hidden: !track.hidden })}
                className={cn(HDR_BUTTON, track.hidden && "text-danger-fg")}
              >
                {track.hidden ? <EyeOff className="w-4 h-4" /> : <Eye className="w-4 h-4" />}
              </button>
            )}
            <button
              type="button"
              disabled={editing || !!track.locked}
              aria-label={`Remove ${number}`}
              title={track.locked ? "Unlock this track to remove it" : "Remove this track and its clips (Undo brings it back)"}
              onClick={() => void edit({ kind: "removeTrack", trackId: track.id })}
              className={cn(HDR_BUTTON, "hidden group-hover/header:inline-flex focus-visible:inline-flex hover:!text-danger-fg hover:!bg-danger/15")}
            >
              <Trash2 className="w-4 h-4" />
            </button>
          </>
        }
      />
    );
  };
  const addTrackButton = (audio: boolean) => (
    <div key={audio ? "add-audio" : "add-video"} className="px-2 flex items-center" style={{ height: NEW_TRACK_ROW_PX }}>
      <Button
        size="sm"
        variant="ghost"
        icon={Plus}
        disabled={!openedProject || editing || (audio ? audioTracks : videoTracks).length >= 16}
        onClick={() => void edit({ kind: "addTrack", trackKind: audio ? "audio" : "video" })}
        className="text-studio-400"
        title={audio ? "Add an audio track below the others" : "Add a video track above the others: it draws over the ones below"}
      >
        {audio ? "Add audio track" : "Add video track"}
      </Button>
    </div>
  );

  // ---- Lanes ---------------------------------------------------------------------------------
  const newTrackRow = (audio: boolean) => {
    const shown = audio ? dragKinds?.audio : dragKinds?.video;
    return (
      <div
        key={audio ? "new-audio" : "new-video"}
        data-track-row={audio ? "new-audio" : "new-video"}
        className={cn(
          "relative rounded-md flex items-center px-2 text-meta transition-colors",
          shown ? "border border-dashed border-accent/60 bg-accent/5 text-studio-300" : "border border-transparent text-transparent",
        )}
        style={{ height: NEW_TRACK_ROW_PX }}
      >
        <span className="pointer-events-none truncate">{shown ? `Drop here for a new ${audio ? "audio" : "video"} track` : ""}</span>
        {ghostsOn({ kind: "new", audio })}
      </div>
    );
  };

  // Clips call back through this ref, so their handlers never change between renders.
  const clipActions = useRef<ClipActions>(null as unknown as ClipActions);
  clipActions.current = {
    pick: pickClip,
    begin: (event, trackId, clip, mode) => {
      const track = sequence.tracks.find((t) => t.id === trackId);
      if (track) beginClipDrag(event, track, clip, mode);
    },
    move: moveClipDrag,
    end: endClipDrag,
    cancel: () => {
      dragRef.current = null;
      setDrag(null);
    },
    suppressClick,
  };
  const clipBlock = (track: SeqTrack, trackNo: string, clip: Clip, index: number) => {
    const audio = track.kind === "audio";
    return (
      <ClipBlock
        key={clip.id}
        clip={clip}
        trackId={track.id}
        trackNo={trackNo}
        audio={audio}
        locked={!!track.locked}
        showName={index === 0 || clip.durationUs * pxPerUs > 60}
        spanUs={spanUs}
        info={clipInfo.get(clip.id) ?? { name: clipName(openedProject, clip), image: false, missing: true, unlinked: false }}
        selected={selected.has(clip.id)}
        moving={!!(drag?.active && drag.mode === "move" && drag.idSet.has(clip.id))}
        shade={trimShade(clip)}
        waveform={audio ? waveforms[streamKey(clip.asset, clip.stream)] : undefined}
        waveFromUs={audio ? Math.max(clip.startUs, drawnUs.from) : 0}
        waveToUs={audio ? Math.min(clipEnd(clip), drawnUs.to) : 0}
        actions={clipActions}
      />
    );
  };

  // The time drawn: the visible stretch with half a view or more either side (all of it
  // before the timeline has a size).
  const drawnUs = (() => {
    if (!(pxPerUs > 0) || viewportPx <= 0) return { from: 0, to: Number.MAX_SAFE_INTEGER };
    const fromPx = (windowStep - 1) * (viewportPx / 2);
    return { from: Math.max(0, fromPx / pxPerUs), to: (fromPx + viewportPx * 2.5) / pxPerUs };
  })();
  /** The clips of a track (in order) that overlap the drawn time, with their positions. */
  const inView = (clips: Clip[]) => {
    let lo = 0;
    let hi = clips.length;
    while (lo < hi) {
      const mid = (lo + hi) >> 1;
      if (clipEnd(clips[mid]) <= drawnUs.from) lo = mid + 1;
      else hi = mid;
    }
    const out: { clip: Clip; index: number }[] = [];
    for (let i = lo; i < clips.length && clips[i].startUs < drawnUs.to; i++) out.push({ clip: clips[i], index: i });
    return out;
  };
  const joinsInView = useMemo(() => {
    const cache = new Map<unknown, unknown>();
    return <T extends { atUs: number }>(joins: T[]): T[] => {
      const hit = cache.get(joins);
      if (hit) return hit as T[];
      const kept = joins.filter((j) => j.atUs >= drawnUs.from && j.atUs <= drawnUs.to);
      // The same list while the window holds, so the marks' memo holds too.
      const result = kept.length === joins.length ? joins : kept;
      cache.set(joins, result);
      return result;
    };
  }, [drawnUs.from, drawnUs.to]);

  const trackLane = (track: SeqTrack) => {
    const audio = track.kind === "audio";
    const trackNo = trackNumber(sequence, track.id);
    const trackJoins = joinsByTrack.get(track.id);
    const hasCamera = cameraClips.some((c) => c.track.id === track.id);
    return (
      <div
        key={track.id}
        data-track-row={`track:${track.id}`}
        className={cn("relative rounded-md bg-studio-850/30", (track.hidden || track.muted) && "opacity-50", track.locked && "bg-[repeating-linear-gradient(135deg,transparent_0_6px,rgb(255_255_255/0.025)_6px_12px)]")}
        style={{ height: lanes.height(audio ? "lane:audio" : "lane:video") }}
      >
        {inView(track.clips).map(({ clip, index }) => clipBlock(track, trackNo, clip, index))}
        {/* Cut time between two pieces of one stretch: click to put it back */}
        {trackJoins && trackJoins.length > 0 && (
          <CutMarks joins={joinsInView(trackJoins)} spanUs={spanUs} editing={editing} restore={restoreCut} />
        )}
        {/* Auto webcam layout: where the camera fills the frame, and where it keeps its bubble */}
        {hasCamera &&
          (focus.segments ?? []).flatMap((segment) =>
            (segment.editedRanges ?? []).map((r, i) => (
              <div
                key={`${segment.id}-${i}`}
                role="button"
                onPointerDown={(event) => event.stopPropagation()}
                onClick={(event) => {
                  event.stopPropagation();
                  selectRange(r.startUs, r.endUs);
                }}
                className={cn(
                  "absolute top-0 h-2 rounded-b z-10 cursor-pointer",
                  segment.enabled && focus.enabled ? "bg-zoom/80 hover:bg-zoom-fg" : "bg-studio-500/50",
                )}
                style={{ left: pct(r.startUs), width: pct(r.endUs - r.startUs) }}
                title={`Camera ${segment.enabled ? "fills the frame" : "focus off"} (${segment.source}). Click to select it; Cam Focus removes it.`}
              />
            )),
          )}
        {hasCamera &&
          (focus.normalView ?? []).flatMap((r) =>
            (r.editedRanges ?? []).map((edited, i) => (
              <div
                key={`normal-${r.sourceStartUs}-${i}`}
                className="absolute bottom-0 h-1.5 rounded-t bg-studio-300/60 z-10 pointer-events-none"
                style={{ left: pct(edited.startUs), width: pct(edited.endUs - edited.startUs) }}
                title="Normal view: the camera stays in its bubble here"
              />
            )),
          )}
        {ghostsOn({ kind: "track", trackId: track.id })}
      </div>
    );
  };

  const pendingCount = pendingZoomSuggestions.length;
  const persistedCount = openedProject?.zooms?.length ?? 0;
  const marker = (atUs: number, label: string) => (
    <div className="absolute top-0 bottom-0 w-1 -translate-x-1/2 bg-accent-fg shadow-[0_0_8px_rgb(var(--accent-fg)/0.8)] z-40 pointer-events-none" style={{ left: pct(atUs) }}>
      <span className="absolute -top-0.5 left-1.5 text-meta font-medium text-accent-fg bg-studio-950 border border-accent/50 rounded px-1.5 whitespace-nowrap">{label}</span>
    </div>
  );
  const dragInsertAt =
    drag?.active && drag.mode === "move" && magnetic && dragGhosts.some((g) => {
      if (g.row.kind !== "track") return false;
      const track = sequence.tracks.find((t) => t.id === (g.row as { trackId: string }).trackId);
      return !!track?.clips.some((c) => !drag.idSet.has(c.id) && c.startUs < g.startUs + g.durationUs && g.startUs < clipEnd(c));
    })
      ? Math.min(...dragGhosts.map((g) => g.startUs))
      : null;
  const empty = sequence.tracks.length === 0;

  return (
    <div className="relative flex flex-col h-full bg-studio-900 select-none">
      {/* Toolbar: editing tools, modes, camera, and the timeline's zoom */}
      <div className="h-11 shrink-0 flex items-center gap-1 px-2 border-b border-studio-800 bg-studio-900 overflow-x-auto overflow-y-hidden">
        <Button
          variant="ghost"
          icon={Scissors}
          disabled={!openedProject || editing || durationUs === 0}
          onClick={() => void splitAtPlayhead()}
          title={`Split at the playhead: the selected clips, or every track${hint("split")}`}
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
          icon={linkState === "linked" ? Unlink : Link2}
          disabled={!openedProject || editing || !linkState}
          onClick={toggleLink}
          title={
            linkState === "linked"
              ? `Unlink: move and trim these clips on their own${hint("toggleLink")}`
              : `Link the selected clips so they move and trim together${hint("toggleLink")}`
          }
        >
          {linkState === "linked" ? "Unlink" : "Link"}
        </Button>

        <ToolbarDivider />
        <Button
          variant="ghost"
          icon={Magnet}
          aria-pressed={magnetic}
          disabled={!openedProject || editing}
          className={cn(magnetic && "!bg-accent/15 !text-accent-fg !border-accent/40")}
          onClick={() => void edit({ kind: "setMagnetic", magnetic: !magnetic })}
          title={
            magnetic
              ? "Magnetic (on): cuts close up on every track, and clips dropped onto others make room. Turn off to leave gaps and overwrite."
              : "Magnetic (off): cuts and moves leave gaps, and dropped clips replace what is under them. Turn on to close up."
          }
        >
          Magnetic
        </Button>
        {cutCount > 0 && (
          <Button
            variant="ghost"
            icon={RotateCcw}
            disabled={editing}
            onClick={() => void edit({ kind: "restoreCuts", clipIds: [] })}
            title={`Put all ${cutCount} cut${cutCount === 1 ? "" : "s"} back on the timeline`}
          >
            Restore {cutCount} cut{cutCount === 1 ? "" : "s"}
          </Button>
        )}

        <ToolbarDivider />
        <Button
          variant="ghost"
          icon={Video}
          aria-pressed={selectionFocused}
          className={cn(selectionFocused && "!bg-accent/15 !text-accent-fg !border-accent/40")}
          disabled={!openedProject || editing || !selection || !!openedProject?.shortView || cameraClips.length === 0}
          onClick={toggleCamFocus}
          title={selectionFocused ? "Remove Cam Focus from the selection" : "Make the camera fill the frame over the selection (turns Auto Webcam on)"}
        >
          Cam Focus
        </Button>
        <Button
          variant="ghost"
          icon={MonitorPlay}
          aria-pressed={selectionInNormalView}
          className={cn(selectionInNormalView && "!bg-accent/15 !text-accent-fg !border-accent/40")}
          disabled={!openedProject || editing || selectionSource.length === 0 || !!openedProject?.shortView}
          onClick={toggleNormalView}
          title={
            selectionInNormalView
              ? "Here the camera keeps its bubble. Click to let Auto Webcam go full frame again."
              : "Keep the camera in its bubble over the selection, even when Auto Webcam would go full frame"
          }
        >
          Normal view
        </Button>

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
        <Button variant="ghost" size="sm" disabled={!openedProject || timelineZoom <= MIN_TIMELINE_ZOOM} onClick={() => setTimelineZoom(MIN_TIMELINE_ZOOM)} title="Fit the whole video in view">
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
            <div role="alert" className="pointer-events-auto flex items-start gap-2 max-w-md rounded-panel border border-danger/40 bg-studio-850 px-3 py-2 text-label text-danger-fg shadow-popover">
              <span>{editError}</span>
              <button aria-label="Dismiss" onClick={() => setEditError(undefined)} className="text-danger-fg/70 hover:text-studio-100">
                <X className="w-3.5 h-3.5" />
              </button>
            </div>
          )}
          {range && (
            <div className="pointer-events-auto flex items-center gap-1 rounded-panel border border-studio-700 bg-studio-850 pl-3 pr-1 py-1 text-label text-studio-200 shadow-popover">
              <span className="font-mono tabular-nums text-studio-300 mr-1">{((range.endUs - range.startUs) / 1e6).toFixed(2)}s range</span>
              <button
                disabled={editing}
                onClick={() => deleteSelection()}
                className="h-7 px-2 rounded-control text-danger-fg hover:bg-danger/15 disabled:opacity-40"
                title={`Cut the range from every track${magnetic ? " and close the gap" : ""}${hint("deleteSelection")}`}
              >
                Delete
              </button>
              <button disabled={editing} onClick={keepOnlyRange} className="h-7 px-2 rounded-control text-accent-fg hover:bg-accent/15 disabled:opacity-40" title="Cut everything outside the range">
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

      {/* Tracks: headers on the left, lanes on the right; both scroll up and down together */}
      <div className="flex-1 min-h-0 overflow-x-hidden overflow-y-auto">
        <div className="flex min-h-full">
          <div className="w-64 shrink-0 flex flex-col border-r border-studio-800 bg-studio-900">
            <div className="h-8 shrink-0 border-b border-studio-800 px-3 flex items-center text-meta font-semibold uppercase tracking-wide text-studio-500">Tracks</div>
            <div className="flex-1 space-y-2 py-2">
              {captions.header}
              {zooms.header}
              {addTrackButton(false)}
              {[...videoTracks].reverse().map(trackHeader)}
              {audioTracks.map(trackHeader)}
              {addTrackButton(true)}
            </div>
          </div>

          <div ref={scrollRef} className="timeline-scroll flex-1 overflow-x-auto overflow-y-hidden relative">
            <div className="flex flex-col h-full min-w-full" style={{ width: `${timelineZoom * 100}%` }}>
              {/* Time ruler: drag to scrub, Shift+drag to select a range */}
              <div
                className="h-8 shrink-0 border-b border-studio-800 bg-studio-900 relative cursor-ew-resize overflow-hidden"
                onPointerDown={onRulerPointerDown}
                onPointerMove={onRulerPointerMove}
                onPointerUp={onRulerPointerUp}
                onPointerCancel={onRulerPointerUp}
                title="Drag to scrub. Shift+drag to select a range."
              >
                {range && <div className="absolute top-0 bottom-0 bg-accent/25 border-x border-accent-hover pointer-events-none" style={{ left: pct(range.startUs), width: pct(range.endUs - range.startUs) }} />}
                {durationUs > 0 && <div className="absolute top-0 bottom-0 bg-studio-950/60 pointer-events-none" style={{ left: pct(durationUs), right: 0 }} />}
                {rulerTicks.map((tickUs) => (
                  <div
                    key={tickUs}
                    className="absolute top-3 bottom-0 border-l border-studio-700 pl-1.5 -mt-3 pt-2 text-meta font-mono tabular-nums text-studio-400 pointer-events-none"
                    style={{ left: pct(tickUs) }}
                  >
                    {formatRulerLabel(tickUs, rulerStep)}
                  </div>
                ))}
                {/* Chapter markers, in playback order, the first at 0:00 like the export */}
                {placedChapters(openedProject?.chapters ?? []).map(({ chapter, atUs }) => (
                  <div key={chapter.id} className="absolute top-0 bottom-0 pointer-events-none z-10" style={{ left: pct(atUs) }} title={chapter.title}>
                    <div className="h-full border-l border-studio-300/70" />
                    <span className="absolute top-0.5 left-1 max-w-[180px] truncate rounded bg-studio-700 px-1.5 text-meta text-studio-100">{chapter.title}</span>
                  </div>
                ))}
                <AtPlayhead spanUs={spanUs}>
                  {(left) => (
                    <div className="absolute top-0 bottom-0 w-0.5 -translate-x-1/2 bg-accent-hover pointer-events-none" style={{ left }}>
                      <div className="absolute top-0 left-1/2 -translate-x-1/2 h-2.5 w-3 rounded-b-sm bg-accent-hover" />
                    </div>
                  )}
                </AtPlayhead>
              </div>

              <div
                ref={lanesRef}
                onDragOver={onMediaDragOver}
                onDragLeave={() => {
                  setMediaGhost(null);
                  setInsertAtUs(null);
                }}
                onDrop={onMediaDrop}
                onClick={handleLanesClick}
                onPointerDown={onMarqueeDown}
                onPointerMove={onMarqueeMove}
                onPointerUp={onMarqueeUp}
                onPointerCancel={onMarqueeUp}
                className="flex-1 relative cursor-pointer py-2 bg-studio-950/40"
              >
                {marquee &&
                  (() => {
                    const rect = lanesRef.current?.getBoundingClientRect();
                    if (!rect) return null;
                    return (
                      <div
                        className="absolute z-50 pointer-events-none rounded-sm border border-accent-hover bg-accent/15"
                        style={{
                          left: Math.min(marquee.x0, marquee.x1) - rect.left,
                          top: Math.min(marquee.y0, marquee.y1) - rect.top,
                          width: Math.abs(marquee.x1 - marquee.x0),
                          height: Math.abs(marquee.y1 - marquee.y0),
                        }}
                      />
                    );
                  })()}
                <AtPlayhead spanUs={spanUs}>
                  {(left, timeUs) => (
                    <>
                      <div
                        className="absolute top-0 bottom-0 w-0.5 -translate-x-1/2 bg-accent-hover z-30 pointer-events-none shadow-[0_0_6px_rgb(var(--accent-hover)/0.5)]"
                        style={{ left }}
                      />
                      {/* Playhead grab strip: drag the playhead itself without touching the selection. */}
                      <div
                  role="slider"
                  aria-label="Playhead"
                  aria-valuemin={0}
                  aria-valuemax={durationUs}
                  aria-valuenow={timeUs}
                  className="absolute top-0 bottom-0 w-3 -translate-x-1/2 z-40 cursor-ew-resize"
                  style={{ left }}
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
                    </>
                  )}
                </AtPlayhead>
                <FollowPlayhead scrollRef={scrollRef} pxPerUs={pxPerUs} zoom={timelineZoom} />
                {range && <div className="absolute top-0 bottom-0 bg-accent/[0.08] border-x border-accent-hover/60 z-10 pointer-events-none" style={{ left: pct(range.startUs), width: pct(range.endUs - range.startUs) }} />}
                {durationUs > 0 && <div className="absolute top-0 bottom-0 bg-studio-950/40 pointer-events-none" style={{ left: pct(durationUs), right: 0 }} />}
                {dragInsertAt !== null && marker(dragInsertAt, "Insert here")}
                {insertAtUs !== null && marker(insertAtUs, "Insert here")}

                <div data-lanes-stack className="space-y-2">
                  {captions.lane}
                  {zooms.lane}
                  {newTrackRow(false)}
                  {[...videoTracks].reverse().map(trackLane)}
                  {audioTracks.map(trackLane)}
                  {newTrackRow(true)}
                </div>
                {empty && (
                  <div data-track-row="new-video" className="mx-2 h-20 flex items-center rounded-md border border-dashed border-studio-600 px-4 text-label text-studio-400">
                    Drag media from the Media panel here to start the video
                  </div>
                )}
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};
