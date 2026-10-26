import React, { useRef, useState } from "react";
import { LayoutGrid } from "lucide-react";
import { useProjectStore } from "../../stores/projectStore";
import { changeLayouts, selectSection, useLayoutsStore } from "../../stores/layoutsStore";
import { layoutKindName, type LayoutSection } from "../../lib/types";
import { cn } from "../ui";
import { HeaderRow, MIN_LANE_PX, type TimelineView } from "./timelineShared";

/** A section shorter than this on the timeline (after cuts) is not shown. Mirrors `MIN_SHOWN_US`. */
const MIN_SHOWN_US = 1_000_000;
/** How close, in pixels, a press must be to a section's edge to drag the edge. */
const EDGE_PX = 6;
/** While dragging an end, the preview shows this far inside it: its last frame. */
const LAST_FRAME_US = 40_000;

/** The colour of a section, by what it shows. */
function tone(section: LayoutSection): string {
  switch (section.layout?.kind) {
    case "cameraOnly":
      return "bg-accent/80 border-accent-hover";
    case "screenOnly":
      return "bg-video/70 border-video";
    case "sideBySide":
      return "bg-caption/70 border-caption";
    case "bubble":
      return "bg-zoom/70 border-zoom";
    default:
      return "bg-studio-600/50 border-studio-500";
  }
}

/** Where a section plays: from its first piece's start to its last piece's end. */
function span(section: LayoutSection): { startUs: number; endUs: number } | null {
  const ranges = section.editedRanges ?? [];
  if (ranges.length === 0) return null;
  return { startUs: ranges[0].startUs, endUs: ranges[ranges.length - 1].endUs };
}

/**
 * The Layouts track: the sections that change the base layout, where they play. Click one to
 * edit it in the inspector; drag an edge to move it (the preview follows the edge, saved on
 * release); double-click to go to it.
 */
export function useLayoutLane({ view }: { view: TimelineView }) {
  const openedProject = useProjectStore((s) => s.openedProject);
  const selectedId = useLayoutsStore((s) => s.selectedId);
  const [drag, setDrag] = useState<{ id: string; edge: "start" | "end"; startUs: number; endUs: number } | null>(null);
  const dragRef = useRef(drag);
  dragRef.current = drag;
  // The preview follows the dragged edge, one seek a frame at most.
  const seekFrame = useRef<number | null>(null);
  const seekTarget = useRef(0);
  const track = openedProject?.layouts;
  if (!openedProject || !track) return { header: null, lane: null };

  // Tall enough for the header's name and detail on two lines.
  const height = Math.max(MIN_LANE_PX, 40);
  const sections = track.sections.flatMap((section) => {
    const at = span(section);
    return at ? [{ section, ...at }] : [];
  });

  const header = (
    <HeaderRow
      key="layouts"
      height={height}
      tone="video"
      icon={LayoutGrid}
      name="Layouts"
      meta={`${layoutKindName(track.base)} base · ${sections.length} section${sections.length === 1 ? "" : "s"}`}
      title="Where the layout changes. Elsewhere the base layout shows. Click a section to edit it in the inspector, drag an edge to move it (the preview shows the edge)."
    />
  );

  const startDrag = (event: React.PointerEvent, section: LayoutSection, startUs: number, endUs: number) => {
    event.stopPropagation();
    if (event.button !== 0) return;
    selectSection(section.id);
    const box = event.currentTarget.getBoundingClientRect();
    const fromStart = event.clientX - box.left;
    const fromEnd = box.right - event.clientX;
    const edge = fromStart <= EDGE_PX ? "start" : fromEnd <= EDGE_PX ? "end" : null;
    if (!edge) return;
    event.currentTarget.setPointerCapture(event.pointerId);
    setDrag({ id: section.id, edge, startUs, endUs });
  };
  const moveDrag = (event: React.PointerEvent) => {
    const current = dragRef.current;
    if (!current) return;
    const at = Math.max(0, Math.min(view.durationUs, view.clientXToUs(event.clientX)));
    const next =
      current.edge === "start"
        ? { ...current, startUs: Math.min(at, current.endUs - MIN_SHOWN_US) }
        : { ...current, endUs: Math.max(at, current.startUs + MIN_SHOWN_US) };
    setDrag(next);
    seekTarget.current =
      next.edge === "start" ? next.startUs : Math.max(next.startUs, next.endUs - LAST_FRAME_US);
    if (seekFrame.current === null) {
      seekFrame.current = requestAnimationFrame(() => {
        seekFrame.current = null;
        view.seekToUs(Math.round(seekTarget.current));
      });
    }
  };
  const endDrag = () => {
    const done = dragRef.current;
    setDrag(null);
    if (seekFrame.current !== null) {
      cancelAnimationFrame(seekFrame.current);
      seekFrame.current = null;
    }
    if (!done) return;
    changeLayouts({ op: "retime", id: done.id, startUs: Math.round(done.startUs), endUs: Math.round(done.endUs) }, "Moving a section");
  };

  const lane = (
    <div
      key="layouts"
      data-track-row="layouts"
      className="relative rounded-md bg-studio-850/30"
      style={{ height }}
      onClick={() => selectSection(null)}
    >
      {view.durationUs > 0 &&
        sections.map(({ section, startUs, endUs }) => {
          const dragging = drag?.id === section.id ? drag : null;
          const from = dragging?.startUs ?? startUs;
          const to = dragging?.endUs ?? endUs;
          const tooShort = to - from < MIN_SHOWN_US;
          const name = section.layout ? (section.name ?? layoutKindName(section.layout)) : "Base";
          return (
            <div
              key={section.id}
              role="button"
              aria-pressed={selectedId === section.id}
              onPointerDown={(event) => startDrag(event, section, startUs, endUs)}
              onPointerMove={moveDrag}
              onPointerUp={endDrag}
              onPointerCancel={() => setDrag(null)}
              onClick={(event) => event.stopPropagation()}
              onDoubleClick={(event) => {
                event.stopPropagation();
                view.seekToUs(startUs);
              }}
              className={cn(
                "absolute top-1 bottom-1 rounded border overflow-hidden px-1.5 flex items-center cursor-pointer",
                "[&:hover]:brightness-110",
                tone(section),
                section.auto && "opacity-60 border-dashed",
                tooShort && "bg-[repeating-linear-gradient(135deg,transparent_0_4px,rgb(255_255_255/0.15)_4px_8px)]",
                selectedId === section.id && "ring-2 ring-white/80 z-10",
              )}
              style={{ left: view.pct(from), width: view.pct(Math.max(0, to - from)) }}
              title={`${name}${section.auto ? ` (picked: ${section.rule ?? "rule"})` : ""}${tooShort ? " — too short after cuts, not shown" : ""}. Drag an edge to move it.`}
            >
              <span className="text-meta font-medium text-white/90 truncate pointer-events-none">{name}</span>
            </div>
          );
        })}
    </div>
  );

  return { header, lane };
}
