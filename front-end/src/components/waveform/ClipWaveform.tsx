import React, { useEffect, useRef, useState } from "react";
import type { WaveformOverview } from "../../lib/types";

/** A bar with no sound behind it (the backend's `OVERVIEW_GAP`). */
const GAP = 255;

/**
 * The most backing pixels the canvas gets across: it covers only the drawn part of a clip
 * (a few views wide), so this is rarely reached; past it the canvas is stretched.
 */
const MAX_BACKING_PX = 4096;

/** Levels this far below full scale and quieter draw as the flat line. */
const FLOOR_DB = 54;

/** A packed level (`round(sqrt(level) * 254)`) as a share of the half-height, on a dB scale:
 *  quiet noise stays visible above silence and loud sound stands out. */
function heightOf(packed: number): number {
  if (packed === 0 || packed === GAP) return 0;
  const level = (packed / 254) ** 2;
  return Math.max(0, Math.min(1, (20 * Math.log10(level) + FLOOR_DB) / FLOOR_DB));
}

interface ClipWaveformProps {
  overview: WaveformOverview;
  /** The sound's own time at the canvas's left and right edges. */
  fromUs: number;
  toUs: number;
  speech: boolean;
}

/**
 * A static waveform, like Premiere's: each column the loudest peak (lighter) and RMS (solid)
 * of the sound under it, mirrored about the middle. Pauses read as the flat line.
 */
export const ClipWaveform = React.memo(function ClipWaveform({ overview, fromUs, toUs, speech }: ClipWaveformProps) {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const [size, setSize] = useState({ width: 0, height: 0 });

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const measure = () => {
      const rect = canvas.getBoundingClientRect();
      setSize((current) =>
        current.width === rect.width && current.height === rect.height ? current : { width: rect.width, height: rect.height },
      );
    };
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(canvas);
    return () => observer.disconnect();
  }, []);

  useEffect(() => {
    const canvas = canvasRef.current;
    const ctx = canvas?.getContext("2d");
    const { width, height } = size;
    if (!canvas || !ctx || width <= 0 || height <= 0) return;
    const dpr = window.devicePixelRatio || 1;
    canvas.width = Math.max(1, Math.min(MAX_BACKING_PX, Math.round(width * dpr)));
    canvas.height = Math.max(1, Math.round(height * dpr));
    ctx.clearRect(0, 0, canvas.width, canvas.height);
    const { peaks, rms, durationUs } = overview;
    const spanUs = toUs - fromUs;
    if (peaks.length === 0 || durationUs <= 0 || spanUs <= 0) return;
    const columns = canvas.width;
    const middle = canvas.height / 2;
    const reach = middle - Math.max(1, dpr);
    const barsPerUs = peaks.length / durationUs;
    const peakColor = speech ? "rgba(110, 231, 183, 0.55)" : "rgba(94, 234, 212, 0.5)";
    const rmsColor = speech ? "rgba(209, 250, 229, 0.95)" : "rgba(204, 251, 241, 0.9)";
    // The silence line, so pauses show as a flat line rather than nothing.
    ctx.fillStyle = "rgba(167, 243, 208, 0.35)";
    ctx.fillRect(0, Math.floor(middle), columns, Math.max(1, Math.round(dpr / 2)));
    for (let x = 0; x < columns; x++) {
      const a = fromUs + (spanUs * x) / columns;
      const b = fromUs + (spanUs * (x + 1)) / columns;
      const first = Math.max(0, Math.floor(a * barsPerUs));
      const last = Math.min(peaks.length - 1, Math.max(first, Math.ceil(b * barsPerUs) - 1));
      let peak = 0;
      let level = 0;
      for (let i = first; i <= last; i++) {
        if (peaks[i] === GAP) continue;
        if (peaks[i] > peak) peak = peaks[i];
        if (rms[i] > level) level = rms[i];
      }
      const p = heightOf(peak) * reach;
      if (p > 0) {
        ctx.fillStyle = peakColor;
        ctx.fillRect(x, middle - p, 1, p * 2);
      }
      const r = heightOf(level) * reach;
      if (r > 0) {
        ctx.fillStyle = rmsColor;
        ctx.fillRect(x, middle - r, 1, r * 2);
      }
    }
  }, [overview, fromUs, toUs, speech, size]);

  return <canvas ref={canvasRef} className="block w-full h-full" />;
});
