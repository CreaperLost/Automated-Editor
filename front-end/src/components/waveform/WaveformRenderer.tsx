import React, { useEffect, useRef } from "react";
import { WaveformBucket } from "../../lib/types";

/**
 * The most backing pixels a waveform canvas gets across (and down). A zoomed-in clip can be
 * hundreds of thousands of CSS pixels wide; its waveform has a few hundred bars, so past this
 * the extra pixels add nothing but memory (three canvases of them). The canvas is stretched.
 */
const MAX_BACKING_PX = 4096;

interface WaveformRendererProps {
  buckets: WaveformBucket[];
  /** Edited-timeline window this canvas shows; buckets outside it are skipped. */
  startUs: number;
  endUs: number;
  currentTimeUs: number;
  className?: string;
  barColor?: string;
  activeBarColor?: string;
  gapColor?: string;
  /** Redraws when the lane is resized. */
  heightPx?: number;
}

export const WaveformRenderer: React.FC<WaveformRendererProps> = ({
  buckets,
  startUs,
  endUs,
  currentTimeUs,
  className = "w-full h-12",
  barColor = "#3f3f46",
  activeBarColor = "#10b981",
  gapColor = "#57534e",
  heightPx,
}) => {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  // The waveform drawn twice, in its normal and played colours, at the canvas's pixel size.
  // Playback only moves where the played copy is cut off, so it redraws nothing else.
  const layers = useRef<{ base: HTMLCanvasElement; played: HTMLCanvasElement; width: number; height: number } | null>(null);
  const [size, setSize] = React.useState({ width: 0, height: 0 });

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
  }, [heightPx]);

  // The geometry: only when the buckets, window, colours or size change.
  useEffect(() => {
    const canvas = canvasRef.current;
    const { width, height } = size;
    if (!canvas || width <= 0 || height <= 0) {
      layers.current = null;
      return;
    }
    const dpr = window.devicePixelRatio || 1;
    canvas.width = Math.max(1, Math.min(MAX_BACKING_PX, Math.round(width * dpr)));
    canvas.height = Math.max(1, Math.min(MAX_BACKING_PX, Math.round(height * dpr)));
    // Drawing is in CSS pixels, scaled to however many backing pixels there are.
    const sx = canvas.width / width;
    const sy = canvas.height / height;
    const paint = (color: string) => {
      const layer = document.createElement("canvas");
      layer.width = canvas.width;
      layer.height = canvas.height;
      const ctx = layer.getContext("2d");
      if (!ctx) return layer;
      ctx.scale(sx, sy);
      const spanUs = endUs - startUs;
      if (buckets.length === 0 || spanUs <= 0) return layer;
      const pxPerUs = width / spanUs;
      const centerY = height / 2;
      for (const bucket of buckets) {
        if (bucket.endUs <= startUs || bucket.startUs >= endUs) continue;
        const x0 = (Math.max(bucket.startUs, startUs) - startUs) * pxPerUs;
        const x1 = (Math.min(bucket.endUs, endUs) - startUs) * pxPerUs;
        // Bars fill 70% of their slot so neighbouring buckets stay readable.
        const barWidth = Math.max(1, (x1 - x0) * 0.7);
        if (bucket.gap) {
          ctx.fillStyle = gapColor;
          ctx.globalAlpha = 0.35;
          ctx.fillRect(x0, 2, barWidth, height - 4);
          ctx.globalAlpha = 1;
          continue;
        }
        // Zero amplitude is a 1px hairline, not a decorative min-height bar.
        const barHeight = Math.max(1, bucket.peak * (height * 0.85));
        ctx.fillStyle = color;
        ctx.beginPath();
        ctx.roundRect(x0, centerY - barHeight / 2, barWidth, barHeight, 1);
        ctx.fill();
      }
      return layer;
    };
    layers.current = { base: paint(barColor), played: paint(activeBarColor), width, height };
  }, [buckets, startUs, endUs, barColor, activeBarColor, gapColor, size]);

  // Every playback tick: the normal copy, then the played copy up to the playhead.
  useEffect(() => {
    const canvas = canvasRef.current;
    const drawn = layers.current;
    const ctx = canvas?.getContext("2d");
    if (!canvas || !drawn || !ctx) return;
    ctx.setTransform(1, 0, 0, 1, 0, 0);
    ctx.clearRect(0, 0, canvas.width, canvas.height);
    ctx.drawImage(drawn.base, 0, 0);
    const spanUs = endUs - startUs;
    if (spanUs <= 0) return;
    const played = Math.max(0, Math.min(1, (currentTimeUs - startUs) / spanUs)) * canvas.width;
    if (played > 0) ctx.drawImage(drawn.played, 0, 0, played, canvas.height, 0, 0, played, canvas.height);
  }, [currentTimeUs, startUs, endUs, buckets, barColor, activeBarColor, gapColor, size]);

  return <canvas ref={canvasRef} className={`w-full h-full block ${className}`} />;
};
