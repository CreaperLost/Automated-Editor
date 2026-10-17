import React, { useEffect, useRef, useState } from "react";
import { Loader2, ScanFace } from "lucide-react";
import { api } from "../../lib/ipc";
import { useProjectStore } from "../../stores/projectStore";
import type { FaceTrackStatus, ShortLayout } from "../../lib/types";

type FramingPatch = Partial<Pick<ShortLayout, "cameraFraming" | "cameraOffsetX" | "cameraOffsetY">>;

const clamp = (v: number) => Math.max(-1, Math.min(1, v));

/** A face track that arrives while the short is paused shows at once: the frame is drawn again
 *  (only when this window has the preview engine). */
function redrawPausedPreview(projectHandle: string) {
  const state = useProjectStore.getState();
  if (state.isPlaying || !state.viewShort || state.playbackShortId !== state.viewShort) return;
  void api
    .playbackSeek(projectHandle, state.currentTimeUs)
    .then(state.applyPlaybackStatus)
    .catch(() => undefined);
}

/** Follows the face tracks while any is being found, then redraws the preview with it. */
function followDetection(projectHandle: string, next: FaceTrackStatus[], set: (t: FaceTrackStatus[]) => void, stopped: () => boolean) {
  set(next);
  if (!next.some((t) => t.state === "detecting")) return 0;
  return window.setTimeout(function poll() {
    void api
      .projectFaceTracks(projectHandle, false)
      .then((later) => {
        if (stopped()) return;
        set(later);
        if (later.some((t) => t.state === "detecting")) window.setTimeout(poll, 1000);
        else redrawPausedPreview(projectHandle);
      })
      .catch(() => undefined);
  }, 1000);
}

/** What the face tracks say, in a line. */
function trackLine(tracks: FaceTrackStatus[] | null): { text: string; busy?: boolean; retry?: boolean } {
  if (!tracks) return { text: "Checking the camera…", busy: true };
  if (tracks.length === 0) return { text: "This project has no camera: nothing to frame." };
  const detecting = tracks.find((t) => t.state === "detecting");
  if (detecting) return { text: `Finding the face… ${Math.round(detecting.progress * 100)}%`, busy: true };
  const failed = tracks.find((t) => t.state === "failed");
  if (failed) return { text: `Could not find the face: ${failed.error ?? "detection failed"}. The camera stays centred.`, retry: true };
  if (tracks.some((t) => t.state === "unavailable"))
    return { text: "Face detection is not in this build; the camera stays centred." };
  const ready = tracks.filter((t) => t.state === "ready");
  if (ready.length === tracks.length) {
    const share = Math.round(100 * Math.min(...ready.map((t) => t.faceShare ?? 0)));
    return share > 0
      ? { text: `Following the face (seen in ${share}% of the camera).` }
      : { text: "No face found in the camera; it stays centred.", retry: true };
  }
  return { text: "The face has not been looked for yet.", retry: true };
}

/**
 * A short's camera framing: Auto follows the speaker's face (found once, in the background),
 * Manual puts the crop where the pad says. The pad's dot is the offset: a nudge on top of the
 * face in Auto, the crop's place in Manual. Drag it; double-click to centre.
 */
export const CameraFramingControl: React.FC<{
  projectHandle: string;
  layout: ShortLayout;
  onChange: (patch: FramingPatch) => void;
  segmented: (active: boolean) => string;
}> = ({ projectHandle, layout, onChange, segmented }) => {
  const [tracks, setTracks] = useState<FaceTrackStatus[] | null>(null);
  const auto = layout.cameraFraming !== "manual";

  // Look for the face on first need (Auto), then follow the detection until it is done.
  const alive = useRef(true);
  useEffect(() => {
    alive.current = true;
    return () => {
      alive.current = false;
    };
  }, []);
  const look = (start: boolean) => {
    void api
      .projectFaceTracks(projectHandle, start)
      .then((next) => alive.current && followDetection(projectHandle, next, setTracks, () => !alive.current))
      .catch(() => alive.current && setTracks([]));
  };
  useEffect(() => look(auto), [projectHandle, auto]); // eslint-disable-line react-hooks/exhaustive-deps

  const retry = () => {
    setTracks(null);
    look(true);
  };

  const pad = useRef<HTMLDivElement | null>(null);
  const dragging = useRef(false);
  const moveTo = (e: React.PointerEvent) => {
    const box = pad.current?.getBoundingClientRect();
    if (!box || box.width === 0 || box.height === 0) return;
    onChange({
      cameraOffsetX: Number(clamp(((e.clientX - box.left) / box.width) * 2 - 1).toFixed(2)),
      cameraOffsetY: Number(clamp(((e.clientY - box.top) / box.height) * 2 - 1).toFixed(2)),
    });
  };
  const line = auto ? trackLine(tracks) : null;
  const centred = layout.cameraOffsetX === 0 && layout.cameraOffsetY === 0;
  const pct = (v: number) => `${v > 0 ? "+" : ""}${Math.round(v * 100)}%`;

  return (
    <div className="space-y-2">
      <div className="flex justify-between items-center">
        <span className="text-label text-studio-400">Camera framing</span>
      </div>
      <div className="flex gap-0.5 bg-studio-900 border border-studio-700 rounded-control p-0.5">
        <button
          type="button"
          aria-pressed={auto}
          onClick={() => onChange({ cameraFraming: "auto" })}
          className={segmented(auto)}
          title="Follow the speaker's face: the crop holds still while the face stays near the middle and eases over when it moves"
        >
          <ScanFace className="inline w-3 h-3 mr-1" />
          Auto
        </button>
        <button
          type="button"
          aria-pressed={!auto}
          onClick={() => onChange({ cameraFraming: "manual" })}
          className={segmented(!auto)}
          title="Put the crop where you place it below"
        >
          Manual
        </button>
      </div>
      {line && (
        <p className="text-meta text-studio-500 flex items-center gap-1.5" aria-live="polite">
          {line.busy && <Loader2 className="w-3 h-3 animate-spin shrink-0" />}
          <span>{line.text}</span>
          {line.retry && (
            <button type="button" onClick={retry} className="ml-auto shrink-0 text-studio-300 hover:text-white">
              Find face
            </button>
          )}
        </p>
      )}
      <div className="space-y-1">
        <div className="flex justify-between">
          <span className="text-label text-studio-400">{auto ? "Nudge" : "Position"}</span>
          <button
            type="button"
            onClick={() => onChange({ cameraOffsetX: 0, cameraOffsetY: 0 })}
            className="font-mono text-meta tabular-nums text-studio-200 hover:text-white"
            title={auto ? "No nudge: just follow the face" : "Back to the centre"}
          >
            {centred ? (auto ? "on the face" : "centre") : `${pct(layout.cameraOffsetX)} · ${pct(layout.cameraOffsetY)}`}
          </button>
        </div>
        <div
          ref={pad}
          role="slider"
          tabIndex={0}
          aria-label={auto ? "Camera framing nudge" : "Camera framing position"}
          aria-valuetext={`${pct(layout.cameraOffsetX)} across, ${pct(layout.cameraOffsetY)} down`}
          title="Drag to move the camera crop; double-click to centre. Arrow keys nudge it."
          onPointerDown={(e) => {
            dragging.current = true;
            e.currentTarget.setPointerCapture(e.pointerId);
            moveTo(e);
          }}
          onPointerMove={(e) => dragging.current && moveTo(e)}
          onPointerUp={() => (dragging.current = false)}
          onPointerCancel={() => (dragging.current = false)}
          onDoubleClick={() => onChange({ cameraOffsetX: 0, cameraOffsetY: 0 })}
          onKeyDown={(e) => {
            const step = e.shiftKey ? 0.2 : 0.05;
            const moves: Record<string, [number, number]> = {
              ArrowLeft: [-step, 0],
              ArrowRight: [step, 0],
              ArrowUp: [0, -step],
              ArrowDown: [0, step],
            };
            const move = moves[e.key];
            if (!move) return;
            e.preventDefault();
            onChange({
              cameraOffsetX: Number(clamp(layout.cameraOffsetX + move[0]).toFixed(2)),
              cameraOffsetY: Number(clamp(layout.cameraOffsetY + move[1]).toFixed(2)),
            });
          }}
          className="relative w-full aspect-video rounded-control border border-studio-700 bg-studio-900 cursor-crosshair touch-none focus:outline-none focus-visible:border-accent-hover"
        >
          <div className="absolute inset-x-0 top-1/2 h-px bg-studio-800" />
          <div className="absolute inset-y-0 left-1/2 w-px bg-studio-800" />
          <div
            className="absolute w-3 h-3 -ml-1.5 -mt-1.5 rounded-full bg-accent-fg shadow-[0_0_0_2px_rgb(var(--accent-hover)/0.6)] pointer-events-none"
            style={{ left: `${((layout.cameraOffsetX + 1) / 2) * 100}%`, top: `${((layout.cameraOffsetY + 1) / 2) * 100}%` }}
          />
        </div>
      </div>
    </div>
  );
};
