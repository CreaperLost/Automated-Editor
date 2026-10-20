import React from "react";
import type { Layout } from "../../lib/types";

interface Box {
  x: number;
  y: number;
  w: number;
  h: number;
  r: number;
}

/** Where the screen and the camera go, roughly as `layouts::arrange` places them. */
function boxes(layout: Layout, w: number, h: number): { screen?: Box; camera?: Box } {
  const short = Math.min(w, h);
  const landscape = w >= h;
  // The screen fitted (16:9) or filling its part.
  const fitted = (x: number, y: number, bw: number, bh: number, fill: boolean): Box => {
    if (fill) return { x, y, w: bw, h: bh, r: 0 };
    const sw = Math.min(bw, (bh * 16) / 9) * 0.9;
    const sh = (sw * 9) / 16;
    return { x: x + (bw - sw) / 2, y: y + (bh - sh) / 2, w: sw, h: sh, r: short * 0.03 };
  };
  const fills = (fit: "auto" | "fit" | "fill") => (fit === "auto" ? !landscape : fit === "fill");
  switch (layout.kind) {
    case "screenOnly":
      return { screen: fitted(0, 0, w, h, fills(layout.screen)) };
    case "cameraOnly": {
      const f = layout.sizePct / 100;
      return { camera: { x: (w * (1 - f)) / 2, y: (h * (1 - f)) / 2, w: w * f, h: h * f, r: f < 1 ? short * 0.03 : 0 } };
    }
    case "bubble": {
      const size = (short * layout.sizePct) / 100;
      const round = layout.shape !== "rect";
      const bw = size;
      const bh = round ? size : (size * 9) / 16;
      const margin = short * 0.02;
      const cells: Record<string, [number, number]> = {
        topLeft: [0, 0], top: [1, 0], topRight: [2, 0],
        left: [0, 1], center: [1, 1], right: [2, 1],
        bottomLeft: [0, 2], bottom: [1, 2], bottomRight: [2, 2],
      };
      const place = (cell: number, room: number) => (cell === 0 ? margin : cell === 1 ? room / 2 : room - margin);
      const [col, row] = cells[layout.spot] ?? [-1, -1];
      const x = col < 0 ? ((w - bw) * layout.customX) / 100 : place(col, w - bw);
      const y = row < 0 ? ((h - bh) * layout.customY) / 100 : place(row, h - bh);
      const r = layout.shape === "circle" ? bw / 2 : layout.shape === "squircle" ? bw * 0.3 : (Math.min(bw, bh) * layout.roundnessPct) / 100;
      return { screen: fitted(0, 0, w, h, fills(layout.screen)), camera: { x, y, w: bw, h: bh, r } };
    }
    case "sideBySide": {
      const side = landscape ? w : h;
      const cam = (side * layout.cameraPct) / 100;
      const first = layout.cameraFirst;
      const fill = fills(layout.screen);
      const margin = layout.overlap ? short * 0.04 : 0;
      const along = layout.overlap
        ? first
          ? { cam: [margin, cam - margin], screen: [cam / 2, side - cam / 2] }
          : { cam: [side - cam, cam - margin], screen: [0, side - cam / 2] }
        : first
          ? { cam: [0, cam], screen: [cam, side - cam] }
          : { cam: [side - cam, cam], screen: [0, side - cam] };
      const r = layout.overlap ? short * 0.03 : 0;
      return landscape
        ? {
            screen: fitted(along.screen[0], 0, along.screen[1], h, fill),
            camera: { x: along.cam[0], y: margin, w: along.cam[1], h: h - 2 * margin, r },
          }
        : {
            screen: fitted(0, along.screen[0], w, along.screen[1], fill),
            camera: { x: margin, y: along.cam[0], w: w - 2 * margin, h: along.cam[1], r },
          };
    }
  }
}

/// A small drawing of a layout: the screen in grey-blue, the camera in the accent colour.
export const LayoutThumb: React.FC<{ layout: Layout; portrait?: boolean; className?: string }> = React.memo(
  ({ layout, portrait, className }) => {
    const [w, h] = portrait ? [27, 48] : [48, 27];
    const { screen, camera } = boxes(layout, w, h);
    return (
      <svg viewBox={`0 0 ${w} ${h}`} className={className} aria-hidden>
        <rect x={0} y={0} width={w} height={h} rx={1.5} className="fill-studio-800" />
        {screen && <rect x={screen.x} y={screen.y} width={screen.w} height={screen.h} rx={screen.r} className="fill-video/70" />}
        {camera && <rect x={camera.x} y={camera.y} width={camera.w} height={camera.h} rx={camera.r} className="fill-accent" />}
      </svg>
    );
  },
);
LayoutThumb.displayName = "LayoutThumb";
