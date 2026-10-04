import React, { useRef, useState } from "react";
import { cn } from "../ui";

/** Pointer distance, in pixels, inside which a dragged edge or clip snaps to a point. */
export const SNAP_PX = 8;
/** Pointer travel, in pixels, before a press on a clip becomes a drag. */
export const CLIP_DRAG_PX = 6;
/** Pointer travel, in pixels, before a Shift+drag on the ruler becomes a range selection. */
export const RANGE_DRAG_PX = 4;

export const MIN_TIMELINE_ZOOM = 1;
export const MAX_TIMELINE_ZOOM = 64;

/** Lane heights by kind of lane, remembered on this machine. */
const LANE_HEIGHT_KEY = "aeroedits.laneHeights.v2";
export const MIN_LANE_PX = 28;
export const MAX_LANE_PX = 240;
export const LANE_DEFAULTS = {
  "lane:video": 52,
  "lane:audio": 52,
  "lane:zooms": 36,
  "lane:captions": 44,
} as const;
export type LaneKind = keyof typeof LANE_DEFAULTS;
/** The drop row for a new track, shown while something is dragged. */
export const NEW_TRACK_ROW_PX = 28;

/** A button on a track header. */
export const HDR_BUTTON =
  "h-7 w-7 shrink-0 inline-flex items-center justify-center rounded-control text-studio-400 hover:text-studio-100 hover:bg-studio-800 disabled:opacity-40 transition-colors";

/** The colour strip on a track header: what kind of track it is. */
export const TRACK_TONE = {
  video: "bg-video",
  audio: "bg-audio",
  caption: "bg-caption",
  zoom: "bg-zoom",
} as const;

function loadLaneHeights(): Partial<Record<LaneKind, number>> {
  try {
    const stored = JSON.parse(window.localStorage.getItem(LANE_HEIGHT_KEY) ?? "null");
    if (stored && typeof stored === "object") return stored;
  } catch {
    // Storage can be unavailable; default heights still work.
  }
  return {};
}

/** Lane heights per kind of lane, resizable and remembered. */
export function useLaneHeights() {
  const [heights, setHeights] = useState(loadLaneHeights);
  const height = (kind: LaneKind) => heights[kind] ?? LANE_DEFAULTS[kind];
  const setHeight = (kind: LaneKind, value: number | null) =>
    setHeights((current) => {
      const next = { ...current };
      if (value === null) delete next[kind];
      else next[kind] = Math.round(Math.max(MIN_LANE_PX, Math.min(MAX_LANE_PX, value)));
      try {
        window.localStorage.setItem(LANE_HEIGHT_KEY, JSON.stringify(next));
      } catch {
        // Not remembering the height is harmless.
      }
      return next;
    });
  return { height, setHeight };
}

/** The grip under a lane header: drag to resize every lane of its kind, double-click to reset. */
export const ResizeGrip: React.FC<{
  label: string;
  height: number;
  onResize: (height: number | null) => void;
}> = ({ label, height, onResize }) => {
  const drag = useRef<{ startY: number; startHeight: number } | null>(null);
  return (
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
        drag.current = { startY: event.clientY, startHeight: height };
      }}
      onPointerMove={(event) => {
        if (drag.current) onResize(drag.current.startHeight + event.clientY - drag.current.startY);
      }}
      onPointerUp={() => {
        drag.current = null;
      }}
      onPointerCancel={() => {
        drag.current = null;
      }}
      onDoubleClick={() => onResize(null)}
    >
      <span className="h-1 w-10 rounded-full bg-studio-700 group-hover:bg-accent-hover transition-colors" />
    </div>
  );
};

/**
 * One track header: a colour strip for the kind of track, an icon, the name (and a detail
 * line when the row is tall enough), and its buttons. It is exactly as tall as its lane.
 */
export const HeaderRow: React.FC<{
  height: number;
  tone: keyof typeof TRACK_TONE;
  icon: React.ComponentType<{ className?: string }>;
  name: React.ReactNode;
  meta?: React.ReactNode;
  title?: string;
  grip?: React.ReactNode;
  actions?: React.ReactNode;
  dim?: boolean;
  selected?: boolean;
}> = ({ height, tone, icon: Icon, name, meta, title, grip, actions, dim, selected }) => (
  <div
    className={cn(
      "group/header relative flex items-center gap-2 pl-3 pr-1.5 border-b border-studio-800/60 hover:bg-studio-850/60",
      selected && "bg-studio-850/60",
    )}
    style={{ height }}
    title={title}
  >
    <span className={cn("absolute left-0 top-1.5 bottom-1.5 w-[3px] rounded-r", TRACK_TONE[tone])} aria-hidden />
    {grip}
    <Icon className={cn("w-4 h-4 shrink-0", dim ? "text-studio-600" : "text-studio-400")} aria-hidden />
    <div className={cn("min-w-0 flex-1", height >= 40 ? "leading-tight" : "flex items-baseline gap-2")}>
      <div className={cn("text-label font-medium truncate", dim ? "text-studio-500" : "text-studio-100")}>{name}</div>
      {meta && <div className="text-meta text-studio-500 truncate">{meta}</div>}
    </div>
    {actions && <div className="flex items-center gap-0.5 shrink-0">{actions}</div>}
  </div>
);

/** What the timeline's lanes need to place things: the scale, and the pointer in time. */
export interface TimelineView {
  durationUs: number;
  pxPerUs: number;
  currentTimeUs: number;
  /** Percent of the lane for a time. */
  pct: (us: number) => string;
  clientXToUs: (clientX: number) => number;
  /** How far, in time, the snap distance is at this zoom. */
  snapUs: () => number;
  seekToUs: (us: number) => void;
}

/** A divider between groups of toolbar controls. */
export const ToolbarDivider: React.FC = () => <span className="mx-1 h-5 w-px shrink-0 bg-studio-800" aria-hidden />;
