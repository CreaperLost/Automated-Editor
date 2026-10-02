import React, { useRef, useState } from "react";
import { PANEL_LIMITS, PanelId, useLayoutStore } from "../../stores/layoutStore";

const KEYBOARD_STEP_PX = 16;

/// A draggable divider that resizes one panel of the editor shell.
///
/// `orientation="vertical"` is a vertical bar that changes a width; `"horizontal"` is a
/// horizontal bar that changes a height. `grow` says which pointer direction makes the
/// panel bigger: a panel to the right of / below the bar grows when the pointer moves
/// left / up, so it uses -1.
export const Splitter: React.FC<{
  panel: PanelId;
  orientation: "vertical" | "horizontal";
  grow: 1 | -1;
  /// Size the panel is currently drawn at.
  size: number;
  /// Largest size that still leaves room for the rest of the window.
  max: number;
  label: string;
}> = ({ panel, orientation, grow, size, max, label }) => {
  const setPanelSize = useLayoutStore((s) => s.setPanelSize);
  const setPanelCollapsed = useLayoutStore((s) => s.setPanelCollapsed);
  const resetPanel = useLayoutStore((s) => s.resetPanel);
  const collapsed = useLayoutStore((s) => s.panels[panel].collapsed);
  const drag = useRef<{ start: number; startSize: number; pointerId: number } | null>(null);
  const [dragging, setDragging] = useState(false);
  const { min, collapsible } = PANEL_LIMITS[panel];
  const vertical = orientation === "vertical";

  const apply = (raw: number) => {
    if (collapsible && raw < min / 2) {
      setPanelCollapsed(panel, true);
      return;
    }
    setPanelSize(panel, Math.min(Math.max(raw, min), Math.max(min, max)));
  };

  const onPointerDown = (event: React.PointerEvent<HTMLDivElement>) => {
    if (event.button !== 0) return;
    event.preventDefault();
    event.currentTarget.setPointerCapture(event.pointerId);
    drag.current = {
      start: vertical ? event.clientX : event.clientY,
      startSize: size,
      pointerId: event.pointerId,
    };
    setDragging(true);
    document.body.style.cursor = vertical ? "col-resize" : "row-resize";
  };

  const onPointerMove = (event: React.PointerEvent<HTMLDivElement>) => {
    const state = drag.current;
    if (!state || state.pointerId !== event.pointerId) return;
    const delta = (vertical ? event.clientX : event.clientY) - state.start;
    apply(state.startSize + grow * delta);
  };

  const endDrag = (event: React.PointerEvent<HTMLDivElement>) => {
    if (!drag.current || drag.current.pointerId !== event.pointerId) return;
    drag.current = null;
    setDragging(false);
    document.body.style.cursor = "";
  };

  const onKeyDown = (event: React.KeyboardEvent<HTMLDivElement>) => {
    const bigger = vertical
      ? grow === 1 ? "ArrowRight" : "ArrowLeft"
      : grow === 1 ? "ArrowDown" : "ArrowUp";
    const smaller = vertical
      ? grow === 1 ? "ArrowLeft" : "ArrowRight"
      : grow === 1 ? "ArrowUp" : "ArrowDown";
    if (event.key === bigger) {
      event.preventDefault();
      apply(collapsed ? min : size + KEYBOARD_STEP_PX);
    } else if (event.key === smaller) {
      event.preventDefault();
      if (!collapsed) apply(size - KEYBOARD_STEP_PX);
    } else if (event.key === "Enter" && collapsible) {
      event.preventDefault();
      setPanelCollapsed(panel, !collapsed);
    } else if (event.key === "Home") {
      event.preventDefault();
      resetPanel(panel);
    }
  };

  return (
    <div
      role="separator"
      tabIndex={0}
      aria-label={label}
      aria-orientation={vertical ? "vertical" : "horizontal"}
      aria-valuenow={Math.round(size)}
      aria-valuemin={collapsible ? 0 : min}
      aria-valuemax={Math.round(Math.max(min, max))}
      title={`${label}: drag to resize${collapsible ? ", drag all the way to hide" : ""}. Double-click to reset.`}
      onPointerDown={onPointerDown}
      onPointerMove={onPointerMove}
      onPointerUp={endDrag}
      onPointerCancel={endDrag}
      onDoubleClick={() => resetPanel(panel)}
      onKeyDown={onKeyDown}
      className={`group relative z-30 shrink-0 touch-none outline-none ${
        vertical ? "w-1.5 -mx-[3px] cursor-col-resize" : "h-1.5 -my-[3px] cursor-row-resize"
      }`}
    >
      <div
        className={`absolute transition-colors ${
          vertical ? "inset-y-0 left-1/2 w-px -translate-x-1/2" : "inset-x-0 top-1/2 h-px -translate-y-1/2"
        } ${
          dragging
            ? "bg-indigo-400"
            : "bg-transparent group-hover:bg-indigo-500/70 group-focus-visible:bg-indigo-500/70"
        } ${vertical ? "group-hover:w-[3px]" : "group-hover:h-[3px]"} ${dragging ? (vertical ? "w-[3px]" : "h-[3px]") : ""}`}
      />
    </div>
  );
};
