import React, { useEffect, useRef, useState } from "react";
import { EyeOff, Grid3x3, Sun, X, ZoomIn } from "lucide-react";
import { useProjectStore } from "../../stores/projectStore";
import { api } from "../../lib/ipc";
import type { EffectArea, EffectAt, EffectKind } from "../../lib/types";

/** What a box drawn now covers by default, from the playhead. */
const DEFAULT_LENGTH_US = 5_000_000;

const TOOLS: { kind: EffectKind; label: string; icon: typeof Sun; hint: string }[] = [
  { kind: "blur", label: "Blur", icon: Grid3x3, hint: "Blur a part of the screen (keys, emails, balances)" },
  { kind: "hide", label: "Hide", icon: EyeOff, hint: "Cover a part of the screen with a block: the safest way to keep it out" },
  { kind: "spotlight", label: "Spotlight", icon: Sun, hint: "Dim the screen around a part of it" },
  { kind: "magnify", label: "Magnify", icon: ZoomIn, hint: "Show a part of the screen larger" },
];

const KIND_LABEL: Record<EffectKind, string> = {
  blur: "Blur",
  hide: "Hide",
  spotlight: "Spotlight",
  magnify: "Magnifier",
  dim: "Dim",
  blurBehind: "Blur behind",
};

/** What dragging a box does: move it, or pull one of its corners. */
type Grip = "move" | "nw" | "ne" | "sw" | "se";
const CORNERS: Exclude<Grip, "move">[] = ["nw", "ne", "sw", "se"];
const MIN_SIDE = 0.01;

/** `from` dragged by (`dx`, `dy`) canvas fractions with `grip`, kept inside the canvas. */
const dragged = (from: EffectArea, grip: Grip, dx: number, dy: number): EffectArea => {
  if (grip === "move") {
    return {
      ...from,
      x: Math.min(Math.max(from.x + dx, 0), 1 - from.w),
      y: Math.min(Math.max(from.y + dy, 0), 1 - from.h),
    };
  }
  let [x0, y0, x1, y1] = [from.x, from.y, from.x + from.w, from.y + from.h];
  if (grip.includes("w")) x0 = Math.min(Math.max(x0 + dx, 0), x1 - MIN_SIDE);
  if (grip.includes("e")) x1 = Math.max(Math.min(x1 + dx, 1), x0 + MIN_SIDE);
  if (grip.includes("n")) y0 = Math.min(Math.max(y0 + dy, 0), y1 - MIN_SIDE);
  if (grip.includes("s")) y1 = Math.max(Math.min(y1 + dy, 1), y0 + MIN_SIDE);
  return { x: x0, y: y0, w: x1 - x0, h: y1 - y0 };
};

const clock = (us: number) => {
  const s = Math.floor(us / 1e6);
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;
};

/// Screen effects on the preview (FX-2): pick a tool, drag a box over the screen, and it covers
/// what the recording shows there for five seconds from the playhead, staying on it through
/// zooms, crops, layouts and cuts. While paused, the effects at the playhead show as boxes to
/// move, resize from a corner, end or remove.
/// `aspect` is the canvas shape the preview is fitted to.
export const EffectsOverlay: React.FC<{ aspect: number }> = ({ aspect }) => {
  const project = useProjectStore((s) => s.openedProject);
  const applyOpenedProject = useProjectStore((s) => s.applyOpenedProject);
  const currentTimeUs = useProjectStore((s) => s.currentTimeUs);
  const isPlaying = useProjectStore((s) => s.isPlaying);
  const ref = useRef<HTMLDivElement | null>(null);
  const [size, setSize] = useState({ w: 0, h: 0 });
  const [tool, setTool] = useState<EffectKind | null>(null);
  const [drag, setDrag] = useState<{ x0: number; y0: number; x1: number; y1: number } | null>(null);
  const [here, setHere] = useState<EffectAt[]>([]);
  const [error, setError] = useState<string>();
  // A box being moved (`move`) or resized from a corner, in canvas fractions.
  const [moving, setMoving] = useState<{ id: string; grip: Grip; x: number; y: number; from: EffectArea; area: EffectArea } | null>(
    null,
  );

  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const observer = new ResizeObserver(() => setSize({ w: el.clientWidth, h: el.clientHeight }));
    observer.observe(el);
    return () => observer.disconnect();
  }, []);

  // The canvas inside the box, fitted like the preview.
  const canvas = (() => {
    if (!size.w || !size.h) return { x: 0, y: 0, w: 0, h: 0 };
    const w = Math.min(size.w, size.h * aspect);
    const h = w / aspect;
    return { x: (size.w - w) / 2, y: (size.h - h) / 2, w, h };
  })();

  // Esc puts the tool down.
  useEffect(() => {
    if (!tool) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        setTool(null);
        setDrag(null);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [tool]);

  const handle = project?.projectHandle;
  const revision = project?.revision;
  useEffect(() => {
    if (!handle || isPlaying) {
      setHere([]);
      return;
    }
    const timer = window.setTimeout(() => {
      void api
        .projectEffectsAt(handle, Math.round(currentTimeUs))
        .then(setHere)
        .catch(() => setHere([]));
    }, 120);
    return () => window.clearTimeout(timer);
  }, [handle, revision, currentTimeUs, isPlaying]);

  if (!project || project.shortView) return null;

  const local = (event: React.PointerEvent) => {
    const box = ref.current!.getBoundingClientRect();
    return { x: event.clientX - box.left, y: event.clientY - box.top };
  };
  const toArea = (d: NonNullable<typeof drag>): EffectArea => {
    const x = (Math.min(d.x0, d.x1) - canvas.x) / canvas.w;
    const y = (Math.min(d.y0, d.y1) - canvas.y) / canvas.h;
    return { x, y, w: Math.abs(d.x1 - d.x0) / canvas.w, h: Math.abs(d.y1 - d.y0) / canvas.h };
  };

  const finish = async (d: NonNullable<typeof drag>) => {
    setDrag(null);
    const kind = tool;
    setTool(null);
    if (!kind || Math.abs(d.x1 - d.x0) < 6 || Math.abs(d.y1 - d.y0) < 6) return;
    const current = useProjectStore.getState().openedProject;
    if (!current) return;
    try {
      const atUs = Math.round(useProjectStore.getState().currentTimeUs);
      const added = await api.projectEffectAdd(current.projectHandle, current.revision, kind, atUs, DEFAULT_LENGTH_US, toArea(d));
      applyOpenedProject(added.project);
    } catch (err) {
      setError(String(err));
    }
  };

  const remove = async (id: string) => {
    const current = useProjectStore.getState().openedProject;
    if (!current) return;
    try {
      const next = (current.effects ?? []).filter((e) => e.id !== id);
      applyOpenedProject(await api.projectEffectsSet(current.projectHandle, current.revision, next));
    } catch (err) {
      setError(String(err));
    }
  };

  const endHere = async (id: string) => {
    const current = useProjectStore.getState().openedProject;
    if (!current) return;
    try {
      applyOpenedProject(
        await api.projectEffectEnd(current.projectHandle, current.revision, id, Math.round(useProjectStore.getState().currentTimeUs)),
      );
    } catch (err) {
      setError(String(err));
    }
  };

  const startMove = (event: React.PointerEvent, id: string, grip: Grip, from: EffectArea) => {
    event.stopPropagation();
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
    const p = local(event);
    setMoving({ id, grip, x: p.x, y: p.y, from, area: from });
  };
  const followMove = (event: React.PointerEvent) => {
    if (!moving || !canvas.w || !canvas.h) return;
    const p = local(event);
    const area = dragged(moving.from, moving.grip, (p.x - moving.x) / canvas.w, (p.y - moving.y) / canvas.h);
    setMoving({ ...moving, area });
  };
  const finishMove = async () => {
    const done = moving;
    setMoving(null);
    if (!done) return;
    const { from, area } = done;
    const still = [area.x - from.x, area.y - from.y, area.w - from.w, area.h - from.h].every((d) => Math.abs(d) < 1e-4);
    const current = useProjectStore.getState().openedProject;
    if (still || !current) return;
    // Shown where it was dropped until the effects at the playhead come back.
    setHere((list) => list.map((e) => (e.effect.id === done.id ? { ...e, canvas: area } : e)));
    try {
      applyOpenedProject(
        await api.projectEffectMove(
          current.projectHandle,
          current.revision,
          done.id,
          Math.round(useProjectStore.getState().currentTimeUs),
          area,
        ),
      );
    } catch (err) {
      setError(String(err));
    }
  };

  const shownDrag = drag && {
    left: Math.min(drag.x0, drag.x1),
    top: Math.min(drag.y0, drag.y1),
    width: Math.abs(drag.x1 - drag.x0),
    height: Math.abs(drag.y1 - drag.y0),
  };

  return (
    <div ref={ref} className="absolute inset-0">
      {tool && (
        <div
          className="absolute pointer-events-auto cursor-crosshair"
          style={{ left: canvas.x, top: canvas.y, width: canvas.w, height: canvas.h }}
          onPointerDown={(e) => {
            e.currentTarget.setPointerCapture(e.pointerId);
            const p = local(e);
            setDrag({ x0: p.x, y0: p.y, x1: p.x, y1: p.y });
          }}
          onPointerMove={(e) => {
            if (!drag) return;
            const p = local(e);
            const clampX = Math.min(Math.max(p.x, canvas.x), canvas.x + canvas.w);
            const clampY = Math.min(Math.max(p.y, canvas.y), canvas.y + canvas.h);
            setDrag({ ...drag, x1: clampX, y1: clampY });
          }}
          onPointerUp={() => drag && void finish(drag)}
        />
      )}
      {shownDrag && (
        <div className="absolute border-2 border-dashed border-accent bg-accent/10 pointer-events-none" style={shownDrag} />
      )}
      {!tool &&
        here.map(({ effect, canvas: shown, span }) => {
          const area = moving?.id === effect.id ? moving.area : shown;
          return (
          <div
            key={effect.id}
            title="Drag to move it, or a corner to resize it"
            className="absolute border border-white/70 rounded-sm pointer-events-auto cursor-move shadow-[0_0_0_1px_rgba(0,0,0,0.5)]"
            style={{
              left: canvas.x + area.x * canvas.w,
              top: canvas.y + area.y * canvas.h,
              width: area.w * canvas.w,
              height: area.h * canvas.h,
            }}
            onPointerDown={(e) => startMove(e, effect.id, "move", shown)}
            onPointerMove={followMove}
            onPointerUp={() => void finishMove()}
          >
            {CORNERS.map((grip) => (
              <div
                key={grip}
                aria-label={`Resize from the ${grip} corner`}
                className={`absolute w-2.5 h-2.5 rounded-sm bg-white border border-black/50 ${
                  grip.includes("n") ? "-top-1.5" : "-bottom-1.5"
                } ${grip.includes("w") ? "-left-1.5" : "-right-1.5"} ${
                  grip === "nw" || grip === "se" ? "cursor-nwse-resize" : "cursor-nesw-resize"
                }`}
                onPointerDown={(e) => startMove(e, effect.id, grip, shown)}
                onPointerMove={(e) => {
                  e.stopPropagation();
                  followMove(e);
                }}
                onPointerUp={(e) => {
                  // The box under it would save the same drag again.
                  e.stopPropagation();
                  void finishMove();
                }}
              />
            ))}
            <div
              className="absolute -top-6 left-0 flex items-center gap-1 pointer-events-auto cursor-default rounded bg-black/75 px-1.5 py-0.5 text-meta text-white whitespace-nowrap"
              onPointerDown={(e) => e.stopPropagation()}
            >
              <span>
                {KIND_LABEL[effect.kind]}
                {span ? ` ${clock(span[0])}–${clock(span[1])}` : ""}
              </span>
              <button
                type="button"
                className="underline decoration-white/40 hover:decoration-white"
                title="End it at the playhead"
                onClick={() => void endHere(effect.id)}
              >
                end here
              </button>
              <button type="button" aria-label={`Remove this ${KIND_LABEL[effect.kind]}`} onClick={() => void remove(effect.id)}>
                <X className="w-3 h-3" />
              </button>
            </div>
          </div>
          );
        })}
      <div className="absolute left-2 top-2 flex items-center gap-1 pointer-events-auto">
        {TOOLS.map(({ kind, label, icon: Icon, hint }) => (
          <button
            key={kind}
            type="button"
            title={tool === kind ? "Drag a box over the screen (Esc to stop)" : hint}
            aria-pressed={tool === kind}
            onClick={() => setTool((t) => (t === kind ? null : kind))}
            className={`flex items-center gap-1 h-6 px-1.5 rounded-control text-meta transition-colors ${
              tool === kind ? "bg-accent text-white" : "bg-black/55 text-white/85 hover:bg-black/75"
            }`}
          >
            <Icon className="w-3.5 h-3.5" />
            <span className="hidden xl:inline">{label}</span>
          </button>
        ))}
      </div>
      {error && (
        <button
          type="button"
          onClick={() => setError(undefined)}
          className="absolute left-2 bottom-2 max-w-[80%] pointer-events-auto rounded bg-danger/90 px-2 py-1 text-meta text-white text-left"
        >
          {error}
        </button>
      )}
    </div>
  );
};
