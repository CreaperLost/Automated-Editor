import React, { useEffect, useState } from "react";
import { Captions } from "lucide-react";
import { Button } from "../ui";
import { api } from "../../lib/ipc";
import type { CaptionEdit, CaptionTrackView } from "../../lib/types";
import { useProjectStore } from "../../stores/projectStore";
import { HeaderRow, ResizeGrip, type TimelineView } from "./timelineShared";

/**
 * The caption track: the captioned transcript's cues, editable in place. Double-click to edit
 * text, drag to move, drag an edge to retime; with one selected, S splits it at the playhead,
 * Delete hides it.
 */
export function useCaptionLane({
  view,
  height,
  onResize,
  onError,
  onPicked,
  suppressClick,
  hint,
}: {
  view: TimelineView;
  height: number;
  onResize: (height: number | null) => void;
  onError: (message?: string) => void;
  onPicked: () => void;
  suppressClick: React.MutableRefObject<boolean>;
  hint: (action: "split" | "deleteSelection") => string;
}) {
  const openedProject = useProjectStore((s) => s.openedProject);
  const captionsVersion = useProjectStore((s) => s.captionsVersion);
  const bumpCaptions = useProjectStore((s) => s.bumpCaptions);
  const [track, setTrack] = useState<CaptionTrackView>({ cues: [] });
  const [selected, setSelected] = useState<number | null>(null);
  const [typing, setTyping] = useState<{ index: number; text: string } | null>(null);
  const [drag, setDrag] = useState<{ index: number; side: "start" | "end" | "move"; startX: number; deltaUs: number; pointerId: number } | null>(null);
  const { durationUs } = view;

  useEffect(() => {
    if (!openedProject) return;
    let active = true;
    void api
      .projectCaptionCues(openedProject.projectHandle)
      .then((next) => active && setTrack(next))
      .catch(() => active && setTrack({ cues: [] }));
    return () => {
      active = false;
    };
  }, [openedProject?.projectHandle, openedProject?.revision, captionsVersion, openedProject?.captions?.trackId]);
  useEffect(() => setSelected(null), [openedProject?.projectHandle]);

  const edit = async (change: CaptionEdit) => {
    if (!openedProject || !track.trackId) return false;
    try {
      setTrack(await api.transcriptCaptionEdit(openedProject.projectHandle, track.trackId, change));
      onError(undefined);
      bumpCaptions();
      return true;
    } catch (err) {
      onError(String(err));
      return false;
    }
  };
  const cue = selected !== null ? track.cues[selected] : undefined;
  /** A new caption starts at the first word at or after the playhead. */
  const splitCue = () => {
    if (!cue) return;
    const index = cue.wordStartsUs.findIndex((start, i) => i > 0 && start >= view.nowUs());
    if (index <= 0) {
      onError("Put the playhead between two words of the caption to split it.");
      return;
    }
    void edit({ kind: "split", wordId: cue.wordIds[index] });
  };
  const mergeCue = () => {
    if (!cue || selected === 0) return;
    void edit({ kind: "merge", wordId: cue.wordIds[0] });
  };
  const hideCue = () => {
    if (!cue) return;
    void edit({ kind: "hide", wordIds: cue.wordIds, hidden: true });
    setSelected(null);
  };
  const endDrag = () => {
    const current = drag;
    setDrag(null);
    if (!current || Math.abs(current.deltaUs) < 10_000) return;
    const moved = track.cues[current.index];
    if (!moved) return;
    const startUs = current.side === "end" ? moved.startUs : Math.max(0, moved.startUs + current.deltaUs);
    const endUs = current.side === "start" ? moved.endUs : moved.endUs + current.deltaUs;
    void edit({ kind: "retime", wordIds: moved.wordIds, startUs, endUs });
  };
  const dragHandlers = (index: number, side: "start" | "end" | "move") => ({
    onPointerDown: (event: React.PointerEvent<HTMLElement>) => {
      if (event.button !== 0 || typing) return;
      event.stopPropagation();
      event.currentTarget.setPointerCapture(event.pointerId);
      setSelected(index);
      onPicked();
      setDrag({ index, side, startX: event.clientX, deltaUs: 0, pointerId: event.pointerId });
    },
    onPointerMove: (event: React.PointerEvent<HTMLElement>) => {
      if (!drag || drag.pointerId !== event.pointerId || view.pxPerUs <= 0) return;
      event.stopPropagation();
      const deltaUs = Math.round((event.clientX - drag.startX) / view.pxPerUs);
      if (deltaUs !== drag.deltaUs) setDrag({ ...drag, deltaUs });
    },
    onPointerUp: (event: React.PointerEvent<HTMLElement>) => {
      event.stopPropagation();
      suppressClick.current = true;
      endDrag();
    },
    onPointerCancel: () => setDrag(null),
  });

  const shown = !!track.trackId;
  const header = shown ? (
    <HeaderRow
      key="captions"
      height={height}
      tone="caption"
      icon={Captions}
      name="Captions"
      meta={openedProject?.captions?.enabled ? `${track.cues.length} shown` : "Off in export"}
      grip={<ResizeGrip label="the captions track" height={height} onResize={onResize} />}
      actions={
        cue && (
          <>
            <Button size="sm" variant="ghost" onClick={splitCue} title={`Split the caption at the playhead${hint("split")}`}>
              Split
            </Button>
            <Button size="sm" variant="ghost" onClick={mergeCue} disabled={selected === 0} title="Join this caption to the one before it">
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
        )
      }
    />
  ) : null;

  const lane = shown ? (
    <div key="captions" data-track-row="captions" className="relative rounded-md bg-studio-850/30" style={{ height }}>
      {durationUs > 0 &&
        track.cues.map((item, index) => {
          const isSelected = index === selected;
          const dragged = drag?.index === index ? drag : null;
          const startUs = dragged && dragged.side !== "end" ? item.startUs + dragged.deltaUs : item.startUs;
          const endUs = dragged && dragged.side !== "start" ? item.endUs + dragged.deltaUs : item.endUs;
          const editing = typing?.index === index;
          return (
            <div
              key={`${item.wordIds[0]}-${index}`}
              role="button"
              aria-label={`Caption: ${item.text}`}
              aria-pressed={isSelected}
              className={`absolute top-1 bottom-1 rounded-control border overflow-hidden flex items-center px-2 cursor-grab ${
                isSelected
                  ? "bg-caption-fill border-accent-fg ring-2 ring-accent-hover/70 z-10"
                  : "bg-caption-fill/90 border-caption/60 hover:border-caption"
              }`}
              style={{
                left: view.pct(Math.max(0, startUs)),
                width: view.pct(Math.max(1, endUs - startUs)),
                minWidth: editing ? 160 : undefined,
              }}
              title={`${item.text}\nDouble-click to edit the text, drag to move it, drag an edge to retime. With it selected: S splits at the playhead, Delete hides it.`}
              onClick={(event) => {
                event.stopPropagation();
                setSelected(index);
                onPicked();
              }}
              onDoubleClick={(event) => {
                event.stopPropagation();
                setTyping({ index, text: item.text });
              }}
              {...dragHandlers(index, "move")}
            >
              {editing ? (
                <input
                  autoFocus
                  aria-label="Caption text"
                  value={typing.text}
                  onChange={(e) => setTyping({ index, text: e.target.value })}
                  onPointerDown={(e) => e.stopPropagation()}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") {
                      const text = typing.text.trim();
                      setTyping(null);
                      if (text && text !== item.text) void edit({ kind: "setText", wordIds: item.wordIds, text });
                    } else if (e.key === "Escape") {
                      setTyping(null);
                    }
                  }}
                  onBlur={() => setTyping(null)}
                  className="w-full bg-studio-950 text-meta text-white px-1.5 h-6 rounded outline-none border border-caption"
                />
              ) : (
                <span className="text-meta text-studio-100 truncate pointer-events-none">{item.text}</span>
              )}
              {!editing &&
                (["start", "end"] as const).map((side) => (
                  <div
                    key={side}
                    role="separator"
                    aria-label={`Retime the caption ${side}`}
                    className={`absolute inset-y-0 w-1.5 cursor-ew-resize opacity-0 hover:opacity-100 hover:bg-caption/80 ${
                      side === "start" ? "left-0" : "right-0"
                    }`}
                    onClick={(event) => event.stopPropagation()}
                    {...dragHandlers(index, side)}
                  />
                ))}
            </div>
          );
        })}
    </div>
  ) : null;

  return {
    header,
    lane,
    /** A caption is selected: S and Delete act on it. */
    active: !!cue,
    splitCue,
    hideCue,
    clear: () => setSelected(null),
  };
}
