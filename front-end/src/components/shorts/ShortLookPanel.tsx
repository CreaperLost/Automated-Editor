import React from "react";
import { Camera, Captions, Monitor, Shapes, Trash2 } from "lucide-react";
import { api } from "../../lib/ipc";
import { seekPlayback } from "../../lib/playbackControl";
import { assetById, videoTracks } from "../../lib/sequence";
import type { GraphicSlot } from "../../lib/types";
import { useProjectStore } from "../../stores/projectStore";
import {
  applyLookToAll,
  changeLayout,
  fullLayout,
  removeShort,
  runShorts,
  useSelectedShort,
  dropShortCaptionMarks,
  useShortsStore,
} from "../../stores/shortsStore";
import { InspectorSection } from "../inspector/InspectorSection";
import { LayoutsSection } from "../layouts/LayoutsSection";
import { CameraFramingControl } from "./CameraFraming";
import { TakeFootage } from "./TakeFootage";
import { Switch } from "../ui";

/** Where a graphic can play in a short: value, label, what it does. */
const GRAPHIC_SLOTS: readonly (readonly [GraphicSlot, string, string])[] = [
  ["full", "Full", "The whole frame, over everything"],
  ["camera", "Camera", "The camera's part of the base layout, over everything there"],
  ["screen", "Split", "Your face above, the graphic in the screen's part (the split shows while it plays)"],
  ["overFace", "Over face", "Your face fills the frame and the graphic is drawn over it, see-through"],
  ["overScreen", "Over screen", "Over the screen's part, under the captions; its see-through parts show the screen"],
];

function formatTime(us: number): string {
  const total = Math.max(0, us) / 1_000_000;
  const minutes = Math.floor(total / 60);
  const seconds = total - minutes * 60;
  return `${minutes}:${seconds.toFixed(1).padStart(4, "0")}`;
}

const segmented = (active: boolean) =>
  `flex-1 h-7 inline-flex items-center justify-center gap-1 rounded-[5px] text-label font-medium transition-colors ${
    active
      ? "bg-accent/20 text-accent-fg shadow-[inset_0_0_0_1px_rgb(var(--accent-hover)/0.55)]"
      : "text-studio-400 hover:text-studio-100 hover:bg-studio-800"
  }`;

/// The picked short's look: its layouts, camera framing, graphic places, background and
/// captions. Changes show in the preview at once.
export const ShortLookPanel: React.FC = () => {
  const project = useProjectStore((s) => s.openedProject);
  const selected = useSelectedShort();
  const draft = useShortsStore((s) => s.draft);
  const busy = useShortsStore((s) => s.busy);
  const shortsCount = project?.shorts?.length ?? 0;
  if (!project) return null;
  const ownMarks = Object.values(selected?.captionMarks ?? {}).reduce((n, marks) => n + Object.keys(marks).length, 0);

  const layout = fullLayout(draft ?? selected?.layout);
  // The store shows the picked short's own timeline once its view has arrived.
  const viewing = !!selected && project.shortView === selected.id;
  const lengthUs = viewing ? project.durationUs : 0;
  const playable = viewing && lengthUs > 0;
  // A take short always has its own timeline; it has no stretch of the video to follow.
  const ownEdit = !!selected?.edit && !selected?.take;
  // The graphics this short plays (its own timeline is in the store while viewing it).
  const shortGraphics = viewing
    ? videoTracks(project.sequence)
        .filter((track) => !track.hidden)
        .flatMap((track) => track.clips)
        .filter((clip) => assetById(project, clip.asset)?.kind === "graphic")
        .sort((a, b) => a.startUs - b.startUs)
    : [];
  const setGraphicSlot = (clipId: string, slot: GraphicSlot) => {
    const slots = { ...(layout.graphicSlots ?? {}) };
    if (slot === "full") delete slots[clipId];
    else slots[clipId] = slot;
    changeLayout({ graphicSlots: slots });
  };

  return (
    <>
      {selected && (
        <aside className="h-full overflow-y-auto text-label" aria-label="Short settings">
          {ownEdit && (
            <div className="p-4 border-b border-studio-800">
              <div className="space-y-2 rounded-control border border-suggest/40 bg-suggest/10 p-3">
                <p className="text-meta text-suggest-fg">
                  Its own timeline: cuts, deleted words and muted noises here change only this short. Later edits of
                  the video don't reach it.
                </p>
                <button
                  type="button"
                  disabled={!!busy}
                  onClick={() =>
                    void runShorts("Re-syncing", (p) => api.projectShortResync(p.projectHandle, p.revision, selected.id))
                  }
                  className="h-7 px-2.5 rounded-control border border-suggest/50 text-suggest-fg hover:bg-suggest/15 disabled:opacity-40"
                  title="Replace this short's timeline with its stretch of the video as edited now; its muted noises stay"
                >
                  Take the video's cuts again
                </button>
              </div>
            </div>
          )}

          {selected.take && <TakeFootage project={project} short={selected} viewing={viewing} />}

          {/* The layouts of the short's own timeline (in the store while viewing it). */}
          {playable ? (
            <LayoutsSection />
          ) : (
            <p className="p-4 text-meta text-studio-500">The short's layouts show once it plays.</p>
          )}

          <InspectorSection id="short-camera" title="Camera framing" icon={Camera}>
            <CameraFramingControl project={project} layout={layout} onChange={changeLayout} segmented={segmented} />
          </InspectorSection>

          {shortGraphics.length > 0 && (
            <InspectorSection id="short-graphics" title="Graphics" icon={Shapes}>
              <p className="text-meta text-studio-500">
                Where each graphic plays: the whole frame, the camera's or the screen's part of the split, or over the
                screen under the captions. It renders for that space.
              </p>
              <ul className="space-y-2">
                {shortGraphics.map((clip) => {
                  const slot = layout.graphicSlots?.[clip.id] ?? "full";
                  return (
                    <li key={clip.id} className="space-y-1">
                      <button
                        type="button"
                        onClick={() => seekPlayback(clip.startUs)}
                        className="flex w-full items-center gap-2 text-left text-label text-studio-200 hover:text-white"
                        title="Go to the graphic"
                      >
                        <span className="font-mono text-meta tabular-nums text-studio-400">{formatTime(clip.startUs)}</span>
                        <span className="truncate">{assetById(project, clip.asset)?.name ?? "Graphic"}</span>
                      </button>
                      <div className="flex gap-0.5 bg-studio-900 border border-studio-700 rounded-control p-0.5">
                        {GRAPHIC_SLOTS.map(([value, label, title]) => (
                          <button
                            key={value}
                            type="button"
                            aria-pressed={slot === value}
                            title={title}
                            onClick={() => setGraphicSlot(clip.id, value)}
                            className={`${segmented(slot === value)} !h-6 !text-meta`}
                          >
                            {label}
                          </button>
                        ))}
                      </div>
                    </li>
                  );
                })}
              </ul>
            </InspectorSection>
          )}

          <InspectorSection id="short-screen" title="Screen" icon={Monitor}>
            <label className="block space-y-1">
              <div className="flex justify-between">
                <span className="text-label text-studio-400">Zoom</span>
                <button
                  type="button"
                  onClick={() => changeLayout({ screenZoom: 1 })}
                  className="font-mono text-meta tabular-nums text-studio-200 hover:text-white"
                  title="1×: the screen fills its part of the frame. Below 1× it zooms out and the background shows around it."
                >
                  {layout.screenZoom.toFixed(1)}×{layout.screenZoom < 1 ? " (out)" : ""}
                </button>
              </div>
              <input type="range" aria-label="Screen zoom" min={0.3} max={3} step={0.05} value={layout.screenZoom} onChange={(e) => changeLayout({ screenZoom: Number(e.target.value) })} onDoubleClick={() => changeLayout({ screenZoom: 1 })} className="w-full" />
            </label>
            {(
              [
                ["screenPanX", "Left / right"],
                ["screenPanY", "Up / down"],
              ] as const
            ).map(([key, label]) => (
              <label key={key} className="block space-y-1">
                <div className="flex justify-between">
                  <span className="text-label text-studio-400">{label}</span>
                  <button
                    type="button"
                    onClick={() => changeLayout({ [key]: 0 })}
                    className="font-mono text-meta tabular-nums text-studio-200 hover:text-white"
                    title="Back to the centre"
                  >
                    {layout[key] === 0 ? "centre" : `${layout[key] > 0 ? "+" : ""}${Math.round(layout[key] * 100)}%`}
                  </button>
                </div>
                <input
                  type="range"
                  aria-label={`Screen ${label}`}
                  min={-1}
                  max={1}
                  step={0.02}
                  value={layout[key]}
                  onChange={(e) => changeLayout({ [key]: Number(e.target.value) })}
                  onDoubleClick={() => changeLayout({ [key]: 0 })}
                  className="w-full"
                />
              </label>
            ))}
            <Switch
              checked={layout.followZooms}
              onChange={(followZooms) => changeLayout({ followZooms })}
              label="Follow the video's zooms"
              title="The screen part follows the zooms of the video (clicks and manual zooms)"
            />
          </InspectorSection>

          <p className="px-3 py-2 text-meta text-studio-500 leading-relaxed border-b border-studio-800">
            The background and how captions look at 9:16 are the project&apos;s style (the Inspector&apos;s Style page,
            &ldquo;At 9:16&rdquo;).
          </p>

          <InspectorSection
            id="short-captions"
            title="Captions"
            icon={Captions}
            extra={<Switch checked={layout.captions} onChange={(captions) => changeLayout({ captions })} title="Show captions in this short" />}
          >
            {layout.captions && (
              <div className="flex gap-0.5 bg-studio-900 border border-studio-700 rounded-control p-0.5">
                {(
                  [
                    ["seam", "Between"],
                    ["screen", "On screen"],
                    ["camera", "On camera"],
                  ] as const
                ).map(([spot, label]) => (
                  <button key={spot} type="button" aria-pressed={layout.captionSpot === spot} onClick={() => changeLayout({ captionSpot: spot })} className={segmented(layout.captionSpot === spot)}>
                    {label}
                  </button>
                ))}
              </div>
            )}
            {layout.captions && (
              <div className="space-y-2">
                <label className="block space-y-1">
                  <div className="flex justify-between">
                    <span className="text-label text-studio-400">Words at once</span>
                    <span className="font-mono text-meta tabular-nums text-studio-200">{layout.captionMaxWords || "editor's"}</span>
                  </div>
                  <input type="range" aria-label="Words per caption" min={0} max={12} step={1} value={layout.captionMaxWords} onChange={(e) => changeLayout({ captionMaxWords: Number(e.target.value) })} className="w-full" />
                </label>
                <div className="space-y-1">
                  <span className="text-label text-studio-400">Lines</span>
                  <div className="flex gap-0.5 bg-studio-900 border border-studio-700 rounded-control p-0.5">
                    {[1, 2, 3].map((lines) => (
                      <button key={lines} type="button" aria-pressed={layout.captionLines === lines} onClick={() => changeLayout({ captionLines: lines })} className={segmented(layout.captionLines === lines)}>
                        {lines}
                      </button>
                    ))}
                  </div>
                </div>
                <label className="block space-y-1">
                  <div className="flex justify-between">
                    <span className="text-label text-studio-400">Text size</span>
                    <span className="font-mono text-meta tabular-nums text-studio-200">{layout.captionSizePct ? `${layout.captionSizePct.toFixed(1)}%` : "editor's"}</span>
                  </div>
                  <input type="range" aria-label="Caption size" min={0} max={15} step={0.5} value={layout.captionSizePct} onChange={(e) => { const v = Number(e.target.value); changeLayout({ captionSizePct: v > 0 && v < 2 ? 2 : v }); }} className="w-full" />
                </label>
              </div>
            )}
            {ownMarks > 0 && selected && (
              <div className="flex items-center justify-between gap-2">
                <span className="text-meta text-studio-300">
                  {ownMarks === 1 ? "1 word is" : `${ownMarks} words are`} shown differently from the video
                </span>
                <button
                  type="button"
                  disabled={!!busy}
                  onClick={() => dropShortCaptionMarks(selected.id)}
                  title="Drop this short's own caption breaks, hidden words and emphasis"
                  className="h-7 px-2 rounded-control bg-studio-800 border border-studio-700 text-label hover:bg-studio-700 disabled:opacity-40"
                >
                  Use the video's
                </button>
              </div>
            )}
            <p className="text-meta text-studio-500">
              Colours come from the editor's Captions settings, the font and accent from the brand kit. On the short's caption track, splits, joins, hidden words and
              emphasis are this short's own; text fixes reach the video too.
            </p>
          </InspectorSection>

          <div className="flex flex-col gap-2 p-4">
            <button
              type="button"
              disabled={!!busy || shortsCount < 2}
              onClick={() => applyLookToAll(layout)}
              className="h-control px-3 rounded-control bg-studio-800 border border-studio-700 hover:bg-studio-700 disabled:opacity-40"
            >
              Use this look for every short
            </button>
            <button
              type="button"
              disabled={!!busy}
              onClick={() => removeShort(selected.id)}
              className="h-control flex items-center justify-center gap-1.5 px-3 rounded-control text-danger-fg hover:bg-danger/15 disabled:opacity-40"
            >
              <Trash2 className="w-4 h-4" /> Remove this short
            </button>
          </div>
        </aside>
      )}
      {!selected && <p className="p-4 text-studio-500">Pick or create a short.</p>}
    </>
  );
};
