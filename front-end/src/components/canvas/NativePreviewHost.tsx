import React, { useEffect, useRef, useState } from "react";
import { useProjectStore } from "../../stores/projectStore";
import { api, isTauriEnvironment } from "../../lib/ipc";
import { PreviewHitMode, PreviewStatus } from "../../lib/types";
import { usePreviewQualityStore } from "../../stores/previewQualityStore";
import { restoreSeeThrough, seeThroughPath } from "./seeThrough";

type Picture = ImageBitmap | HTMLImageElement;

/** Decodes a JPEG off the main thread where the webview can, with an <img> as the fallback. */
async function decodeFrame(blob: Blob): Promise<Picture> {
  if (typeof createImageBitmap === "function") return createImageBitmap(blob);
  const url = URL.createObjectURL(blob);
  try {
    const image = new Image();
    image.src = url;
    await image.decode();
    return image;
  } finally {
    URL.revokeObjectURL(url);
  }
}

function closePicture(picture: Picture | null) {
  if (picture && "close" in picture) picture.close();
}

/**
 * Pulls the playback engine's frames and draws them on `canvasRef`, measuring the rate drawn.
 * Each request waits for the next frame, so it arrives as soon as it is stored; decoding runs
 * off the main thread while the next frame is already being fetched. `accept` can turn away
 * frames that are not for this view (a window showing a short skips the video's).
 */
export function useEngineFrames(
  canvasRef: React.RefObject<HTMLCanvasElement | null>,
  active: boolean,
  restartKey?: unknown,
  accept?: () => boolean,
) {
  const setMeasuredFps = usePreviewQualityStore(s => s.setMeasuredFps);
  // Each request waits for the next frame, so it arrives as soon as it is stored; decoding
  // runs off the main thread while the next frame is already being fetched.
  useEffect(() => {
    if (!active) return;
    let cancelled = false;
    let lastSeq = 0;
    let latest: Blob | null = null;
    let drawing = false;
    let drawn = 0;
    let windowStart = performance.now();
    const sleep = (ms: number) => new Promise(resolve => setTimeout(resolve, ms));
    const draw = async () => {
      if (drawing) return;
      drawing = true;
      try {
        while (latest && !cancelled) {
          const blob = latest;
          latest = null;
          const picture = await decodeFrame(blob).catch(() => null);
          const canvas = canvasRef.current;
          const context = canvas?.getContext("2d");
          if (!picture || cancelled || !canvas || !context) {
            closePicture(picture);
            continue;
          }
          if (canvas.width !== picture.width || canvas.height !== picture.height) {
            canvas.width = picture.width;
            canvas.height = picture.height;
          }
          context.drawImage(picture, 0, 0);
          closePicture(picture);
          drawn += 1;
        }
      } finally {
        drawing = false;
      }
    };
    const measure = window.setInterval(() => {
      const now = performance.now();
      const fps = Math.round((drawn * 1000) / Math.max(1, now - windowStart));
      drawn = 0;
      windowStart = now;
      setMeasuredFps(fps > 0 ? fps : null);
    }, 1000);
    const pull = async () => {
      while (!cancelled) {
        if (document.hidden) {
          await sleep(250);
          continue;
        }
        let buffer: ArrayBuffer;
        try {
          buffer = await api.previewFrame(lastSeq, 250);
        } catch {
          await sleep(100);
          continue;
        }
        if (cancelled || buffer.byteLength <= 8) continue;
        lastSeq = Number(new DataView(buffer).getBigUint64(0, true));
        if (accept && !accept()) continue;
        latest = new Blob([buffer.slice(8)], { type: "image/jpeg" });
        void draw();
      }
    };
    void pull();
    return () => {
      cancelled = true;
      window.clearInterval(measure);
      setMeasuredFps(null);
    };
  }, [active, restartKey, setMeasuredFps]);

}

interface NativePreviewHostProps {
  windowLabel?: string;
  hitMode?: PreviewHitMode;
  className?: string;
  fitAspectRatio?: number;
  showStatus?: boolean;
}

export function NativePreviewHost({
  windowLabel = "main",
  hitMode = "consume",
  className = "w-full max-w-4xl aspect-video",
  fitAspectRatio,
  showStatus = true,
}: NativePreviewHostProps) {
  const previewAvailable = useProjectStore(s => s.previewAvailable);
  const playbackError = useProjectStore(s => s.playbackError);
  const hostRef = useRef<HTMLDivElement | null>(null);
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const [status, setStatus] = useState<PreviewStatus | null>(null);
  const [error, setError] = useState<string>();
  const webview = status?.attached === true && status.surface === "webview";
  const webviewGeneration = webview ? status.generation : undefined;
  const underlay = status?.attached === true && status.surface === "underlay";
  // Read by the layout effect, which outlives status changes.
  const underlayRef = useRef(false);
  const markDirtyRef = useRef<() => void>();

  // Without a native child view, pull JPEG frames from the backend and draw them on a canvas.
  useEngineFrames(canvasRef, webviewGeneration !== undefined, webviewGeneration);

  // Drawn in the window under the page: make the page see-through over the preview (on the
  // next layout), or paint it again when frames go back to the webview.
  useEffect(() => {
    underlayRef.current = underlay;
    if (!underlay) restoreSeeThrough();
    markDirtyRef.current?.();
  }, [underlay]);
  useEffect(() => restoreSeeThrough, []);

  // Drawn in the window, the page sees no frames: the engine counts the rate drawn.
  const setMeasuredFps = usePreviewQualityStore((s) => s.setMeasuredFps);
  useEffect(() => {
    if (!underlay) return;
    const poll = window.setInterval(() => {
      void api.previewStatus().then((next) => {
        setMeasuredFps(next.presentedFps ? Math.round(next.presentedFps) : null);
      }).catch(() => undefined);
    }, 1000);
    return () => {
      window.clearInterval(poll);
      setMeasuredFps(null);
    };
  }, [underlay, setMeasuredFps]);

  // The engine went back to the webview (the window could not draw): it says so here.
  useEffect(() => {
    if (!isTauriEnvironment()) return;
    let unlisten: (() => void) | undefined;
    let disposed = false;
    void import("@tauri-apps/api/event").then(({ listen }) =>
      listen<PreviewStatus>("preview-status", (event) => {
        setStatus((current) =>
          current && current.generation === event.payload.generation ? event.payload : current,
        );
      }).then((fn) => {
        if (disposed) fn();
        else unlisten = fn;
      }),
    );
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  // Tells the backend where the preview is on screen (and whether it can be seen). Measuring
  // the host and its ancestors forces layout, so it runs only when something may have moved:
  // a resize of the host or an ancestor, a scroll, the window showing or hiding, and a slow
  // check for anything else (a sibling growing).
  useEffect(() => {
    let cancelled = false;
    let revision = 0;
    let generation: number | undefined;
    let animation = 0;
    let lastGeometry = "";
    let sending = false;
    let dirty = true;
    const update = () => {
      animation = 0;
      const el = hostRef.current;
      if (cancelled || !el || generation === undefined || sending || !dirty) return;
      dirty = false;
      const rect = el.getBoundingClientRect();
      if (rect.width <= 0 || rect.height <= 0) return;
      let left = Math.max(0, rect.left), top = Math.max(0, rect.top);
      let right = Math.min(window.innerWidth, rect.right), bottom = Math.min(window.innerHeight, rect.bottom);
      let ancestor = el.parentElement;
      while (ancestor) {
        const style = getComputedStyle(ancestor);
        const bounds = ancestor.getBoundingClientRect();
        if (/(auto|scroll|hidden|clip)/.test(style.overflowX)) { left = Math.max(left, bounds.left); right = Math.min(right, bounds.right); }
        if (/(auto|scroll|hidden|clip)/.test(style.overflowY)) { top = Math.max(top, bounds.top); bottom = Math.min(bottom, bounds.bottom); }
        ancestor = ancestor.parentElement;
      }
      const clip: [number, number, number, number] = [left - rect.left, top - rect.top, Math.max(0, right - left), Math.max(0, bottom - top)];
      const visible = !document.hidden && clip[2] > 0 && clip[3] > 0;
      // Drawn in the window: the frame fits in the host's content box, inside its border.
      const style = underlayRef.current ? getComputedStyle(el) : null;
      const drawn = style ? {
        frame: [rect.left + el.clientLeft, rect.top + el.clientTop, el.clientWidth, el.clientHeight] as [number, number, number, number],
        frameRadius: Math.max(0, (parseFloat(style.borderTopLeftRadius) || 0) - el.clientLeft),
        fills: seeThroughPath(el),
      } : {};
      const viewport = { windowLabel, x: rect.left, y: rect.top, width: rect.width, height: rect.height,
        backingScale: window.devicePixelRatio || 1, visible, occluded: !visible, generation, clip, ...drawn };
      const key = JSON.stringify(viewport);
      if (key === lastGeometry) return;
      sending = true;
      void api.previewLayout({ ...viewport, revision: ++revision })
        .then(next => { if (!cancelled) { lastGeometry = key; setStatus(next); setError(undefined); } })
        .catch(err => { if (!cancelled) { lastGeometry = ""; setError(String(err)); } })
        .finally(() => {
          sending = false;
          // Something moved while this was on its way: send that too.
          if (dirty) schedule();
        });
    };
    const schedule = () => {
      if (!animation && !cancelled) animation = requestAnimationFrame(update);
    };
    const markDirty = () => {
      dirty = true;
      schedule();
    };
    markDirtyRef.current = markDirty;
    const resized = new ResizeObserver(markDirty);
    for (let el: HTMLElement | null = hostRef.current; el; el = el.parentElement) resized.observe(el);
    window.addEventListener("resize", markDirty);
    window.addEventListener("scroll", markDirty, true);
    document.addEventListener("visibilitychange", markDirty);
    const check = window.setInterval(markDirty, 250);
    void api.previewAttach(windowLabel, hitMode).then(attached => {
      generation = attached.generation;
      if (cancelled) {
        void api.previewDetach(windowLabel, generation).catch(() => undefined);
        return;
      }
      setStatus(attached); setError(undefined);
      markDirty();
    }).catch(err => { if (!cancelled) setError(String(err)); });
    return () => {
      cancelled = true;
      markDirtyRef.current = undefined;
      cancelAnimationFrame(animation);
      resized.disconnect();
      window.removeEventListener("resize", markDirty);
      window.removeEventListener("scroll", markDirty, true);
      document.removeEventListener("visibilitychange", markDirty);
      window.clearInterval(check);
      if (generation !== undefined) {
        void api.previewDetach(windowLabel, generation).catch(() => undefined);
      }
    };
  }, [windowLabel, hitMode]);

  return (
    <div className={`w-full flex flex-col items-center gap-2 ${fitAspectRatio ? "h-full min-h-0" : ""}`}>
      <div
        className={
          fitAspectRatio
            ? "w-full flex-1 min-h-0 flex items-center justify-center overflow-hidden"
            : "contents"
        }
      >
        <div
          ref={hostRef}
          data-native-preview-host
          className={`pointer-events-none rounded-xl border border-studio-800 bg-black/50 ${fitAspectRatio ? "w-full h-full min-h-0" : `shrink-0 ${className}`}`}
          data-content-aspect-ratio={fitAspectRatio}
        >
          {webview && (
            <canvas
              ref={canvasRef}
              className="w-full h-full object-contain rounded-xl"
            />
          )}
        </div>
      </div>
      {showStatus && <p className="text-xs text-studio-400 max-w-md text-center shrink-0">
        {error || playbackError
          ? error || playbackError
          : status?.attached
            ? previewAvailable
              ? ""
              : "Loading project preview…"
            : "Preview surface is not attached."}
      </p>}
    </div>
  );
}
