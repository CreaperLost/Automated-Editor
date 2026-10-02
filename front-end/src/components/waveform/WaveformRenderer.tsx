import React, { useEffect, useRef } from "react";
import { WaveformBucket } from "../../lib/types";

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

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    const dpr = window.devicePixelRatio || 1;
    const rect = canvas.getBoundingClientRect();

    canvas.width = rect.width * dpr;
    canvas.height = rect.height * dpr;
    ctx.scale(dpr, dpr);
    ctx.clearRect(0, 0, rect.width, rect.height);

    const spanUs = endUs - startUs;
    if (buckets.length === 0 || spanUs <= 0 || rect.width <= 0) return;

    const pxPerUs = rect.width / spanUs;
    const centerY = rect.height / 2;

    for (const bucket of buckets) {
      if (bucket.endUs <= startUs || bucket.startUs >= endUs) continue;
      const x0 = (Math.max(bucket.startUs, startUs) - startUs) * pxPerUs;
      const x1 = (Math.min(bucket.endUs, endUs) - startUs) * pxPerUs;
      // Bars fill 70% of their slot so neighbouring buckets stay readable.
      const barWidth = Math.max(1, (x1 - x0) * 0.7);
      if (bucket.gap) {
        ctx.fillStyle = gapColor;
        ctx.globalAlpha = 0.35;
        ctx.fillRect(x0, 2, barWidth, rect.height - 4);
        ctx.globalAlpha = 1;
        continue;
      }

      // Zero amplitude is a 1px hairline, not a decorative min-height bar.
      const barHeight = Math.max(1, bucket.peak * (rect.height * 0.85));
      const y = centerY - barHeight / 2;
      ctx.fillStyle = bucket.startUs < currentTimeUs ? activeBarColor : barColor;
      ctx.beginPath();
      ctx.roundRect(x0, y, barWidth, barHeight, 1);
      ctx.fill();
    }
  }, [buckets, startUs, endUs, currentTimeUs, barColor, activeBarColor, gapColor, heightPx]);

  return <canvas ref={canvasRef} className={`w-full h-full block ${className}`} />;
};
