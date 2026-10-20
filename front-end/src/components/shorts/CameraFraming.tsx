import React, { useEffect, useRef, useState } from "react";
import { Loader2, ScanFace } from "lucide-react";
import { api } from "../../lib/ipc";
import { useProjectStore } from "../../stores/projectStore";
import { clipAt, clipRole, videoTracks } from "../../lib/sequence";
import type { Clip, FaceTrackStatus, OpenedProject, ShortLayout } from "../../lib/types";

type FramingPatch = Partial<Pick<ShortLayout, "cameraClips">>;

/** The camera clip showing at timeline time `us`: the topmost one. */
function cameraClipAt(project: OpenedProject, us: number): Clip | undefined {
  for (const track of videoTracks(project.sequence).reverse()) {
    if (track.hidden) continue;
    const clip = clipAt(track, us);
    if (clip && clipRole(project, track, clip) === "webcam") return clip;
  }
  return undefined;
}

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
      ? { text: `Each clip lines up with the face (seen in ${share}% of the camera).` }
      : { text: "No face found in the camera; it stays centred.", retry: true };
  }
  return { text: "The face has not been looked for yet.", retry: true };
}

/**
 * A short's camera framing, clip by clip. Every camera clip is lined up with the speaker's face
 * as it starts (found once, in the background) and holds for the whole clip. The clip under the
 * playhead can be framed by hand instead: drag the dot to place its crop; double-click to centre.
 */
export const CameraFramingControl: React.FC<{
  project: OpenedProject;
  layout: Pick<ShortLayout, "cameraClips">;
  onChange: (patch: FramingPatch) => void;
  segmented: (active: boolean) => string;
}> = ({ project, layout, onChange, segmented }) => {
  const projectHandle = project.projectHandle;
  const [tracks, setTracks] = useState<FaceTrackStatus[] | null>(null);
  const currentUs = useProjectStore((s) => s.currentTimeUs);
  const clip = cameraClipAt(project, currentUs);
  const byHand = clip ? layout.cameraClips?.[clip.id] : undefined;
  const handCount = Object.keys(layout.cameraClips ?? {}).length;

  // Look for the face when the short opens, then follow the detection until it is done.
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
  useEffect(() => look(true), [projectHandle]); // eslint-disable-line react-hooks/exhaustive-deps

  const retry = () => {
    setTracks(null);
    look(true);
  };

  const setClip = (framing: { offsetX: number; offsetY: number } | null) => {
    if (!clip) return;
    const next = { ...(layout.cameraClips ?? {}) };
    if (framing) next[clip.id] = framing;
    else delete next[clip.id];
    onChange({ cameraClips: next });
  };

  const pad = useRef<HTMLDivElement | null>(null);
  const dragging = useRef(false);
  const moveTo = (e: React.PointerEvent) => {
    const box = pad.current?.getBoundingClientRect();
    if (!box || box.width === 0 || box.height === 0) return;
    setClip({
      offsetX: Number(clamp(((e.clientX - box.left) / box.width) * 2 - 1).toFixed(2)),
      offsetY: Number(clamp(((e.clientY - box.top) / box.height) * 2 - 1).toFixed(2)),
    });
  };
  const line = trackLine(tracks);
  const pct = (v: number) => `${v > 0 ? "+" : ""}${Math.round(v * 100)}%`;

  return (
    <div className="space-y-2">
      <span className="block text-label text-studio-400">Camera framing</span>
      <p className="text-meta text-studio-500 flex items-center gap-1.5" aria-live="polite">
        {line.busy && <Loader2 className="w-3 h-3 animate-spin shrink-0" />}
        <span>{line.text}</span>
        {line.retry && (
          <button type="button" onClick={retry} className="ml-auto shrink-0 text-studio-300 hover:text-white">
            Find face
          </button>
        )}
      </p>
      {!clip ? (
        <p className="text-meta text-studio-500">No camera clip at the playhead.</p>
      ) : (
        <>
          <span className="block text-meta text-studio-400">Clip at the playhead</span>
          <div className="flex gap-0.5 bg-studio-900 border border-studio-700 rounded-control p-0.5">
            <button
              type="button"
              aria-pressed={!byHand}
              onClick={() => setClip(null)}
              className={segmented(!byHand)}
              title="Line this clip up with the speaker's face as it starts, zooming in a little when the face is near an edge"
            >
              <ScanFace className="inline w-3 h-3 mr-1" />
              Centre on face
            </button>
            <button
              type="button"
              aria-pressed={!!byHand}
              onClick={() => !byHand && setClip({ offsetX: 0, offsetY: 0 })}
              className={segmented(!!byHand)}
              title="Place this clip's crop yourself"
            >
              By hand
            </button>
          </div>
          {byHand && (
            <div className="space-y-1">
              <div className="flex justify-between">
                <span className="text-label text-studio-400">Position</span>
                <button
                  type="button"
                  onClick={() => setClip({ offsetX: 0, offsetY: 0 })}
                  className="font-mono text-meta tabular-nums text-studio-200 hover:text-white"
                  title="Back to the centre"
                >
                  {byHand.offsetX === 0 && byHand.offsetY === 0 ? "centre" : `${pct(byHand.offsetX)} · ${pct(byHand.offsetY)}`}
                </button>
              </div>
              <div
                ref={pad}
                role="slider"
                tabIndex={0}
                aria-label="Camera clip position"
                aria-valuetext={`${pct(byHand.offsetX)} across, ${pct(byHand.offsetY)} down`}
                title="Drag to place this clip's camera crop; double-click to centre. Arrow keys nudge it."
                onPointerDown={(e) => {
                  dragging.current = true;
                  e.currentTarget.setPointerCapture(e.pointerId);
                  moveTo(e);
                }}
                onPointerMove={(e) => dragging.current && moveTo(e)}
                onPointerUp={() => (dragging.current = false)}
                onPointerCancel={() => (dragging.current = false)}
                onDoubleClick={() => setClip({ offsetX: 0, offsetY: 0 })}
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
                  setClip({
                    offsetX: Number(clamp(byHand.offsetX + move[0]).toFixed(2)),
                    offsetY: Number(clamp(byHand.offsetY + move[1]).toFixed(2)),
                  });
                }}
                className="relative w-full aspect-video rounded-control border border-studio-700 bg-studio-900 cursor-crosshair touch-none focus:outline-none focus-visible:border-accent-hover"
              >
                <div className="absolute inset-x-0 top-1/2 h-px bg-studio-800" />
                <div className="absolute inset-y-0 left-1/2 w-px bg-studio-800" />
                <div
                  className="absolute w-3 h-3 -ml-1.5 -mt-1.5 rounded-full bg-accent-fg shadow-[0_0_0_2px_rgb(var(--accent-hover)/0.6)] pointer-events-none"
                  style={{ left: `${((byHand.offsetX + 1) / 2) * 100}%`, top: `${((byHand.offsetY + 1) / 2) * 100}%` }}
                />
              </div>
            </div>
          )}
        </>
      )}
      {handCount > 0 && (
        <div className="flex items-center text-meta text-studio-500">
          <span>
            {handCount} {handCount === 1 ? "clip" : "clips"} framed by hand
          </span>
          <button type="button" onClick={() => onChange({ cameraClips: {} })} className="ml-auto text-studio-300 hover:text-white">
            Centre all on face
          </button>
        </div>
      )}
    </div>
  );
};
