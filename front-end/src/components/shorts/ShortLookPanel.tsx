import React from "react";
import {
  ArrowDownToLine,
  ArrowUpToLine,
  Camera,
  Captions,
  Clapperboard,
  Monitor,
  Palette,
  Shapes,
  Sparkles,
  Trash2,
} from "lucide-react";
import { api } from "../../lib/ipc";
import { seekPlayback } from "../../lib/playbackControl";
import { assetById, videoTracks } from "../../lib/sequence";
import type { GraphicSlot } from "../../lib/types";
import { BACKGROUND_PRESETS, presetBackgroundCss } from "../../lib/types";
import { useProjectStore } from "../../stores/projectStore";
import {
  applyLookToAll,
  changeLayout,
  fullLayout,
  removeShort,
  runShorts,
  sceneOp,
  useSelectedShort,
  useShortsStore,
} from "../../stores/shortsStore";
import { InspectorSection } from "../inspector/InspectorSection";
import { CameraFramingControl } from "./CameraFraming";
import { TakeFootage } from "./TakeFootage";
import { Button, Switch } from "../ui";

/** How long a scene added at the playhead lasts. */
const NEW_SCENE_US = 3_000_000;
/** Where a graphic can play in a short: value, label, what it does. */
const GRAPHIC_SLOTS: readonly (readonly [GraphicSlot, string, string])[] = [
  ["full", "Full", "The whole frame, over everything"],
  ["camera", "Camera", "The camera's part of the split, over everything there"],
  ["screen", "Screen", "The screen's part of the split, over everything there"],
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

/// The picked short's look: the camera and screen split, scenes, graphic places, background
/// and captions. Changes show in the preview at once.
export const ShortLookPanel: React.FC = () => {
  const project = useProjectStore((s) => s.openedProject);
  const currentTimeUs = useProjectStore((s) => s.currentTimeUs);
  const selected = useSelectedShort();
  const draft = useShortsStore((s) => s.draft);
  const busy = useShortsStore((s) => s.busy);
  const shortsCount = project?.shorts?.length ?? 0;
  if (!project) return null;

  const layout = fullLayout(draft ?? selected?.layout);
  // The store shows the picked short's own timeline once its view has arrived.
  const viewing = !!selected && project.shortView === selected.id;
  const lengthUs = viewing ? project.durationUs : 0;
  const playable = viewing && lengthUs > 0;
  // A take short always has its own timeline; it has no stretch of the video to follow.
  const ownEdit = !!selected?.edit && !selected?.take;
  const scenes = [...(layout.scenes ?? [])]
    .filter((scene) => scene.shownStartUs !== undefined && scene.shownEndUs !== undefined)
    .sort((a, b) => a.shownStartUs! - b.shownStartUs!);
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

          <InspectorSection id="short-camera" title="Camera" icon={Camera}>
            <div className="flex gap-0.5 bg-studio-900 border border-studio-700 rounded-control p-0.5">
              <button type="button" aria-pressed={layout.cameraPosition === "top"} onClick={() => changeLayout({ cameraPosition: "top" })} className={segmented(layout.cameraPosition === "top")}>
                <ArrowUpToLine className="inline w-3 h-3 mr-1" />
                On top
              </button>
              <button type="button" aria-pressed={layout.cameraPosition === "bottom"} onClick={() => changeLayout({ cameraPosition: "bottom" })} className={segmented(layout.cameraPosition === "bottom")}>
                <ArrowDownToLine className="inline w-3 h-3 mr-1" />
                Below
              </button>
            </div>
            <label className="block space-y-1">
              <div className="flex justify-between">
                <span className="text-label text-studio-400">Camera height</span>
                <span className="font-mono text-meta tabular-nums text-studio-200">{Math.round(layout.cameraPct)}%</span>
              </div>
              <input type="range" aria-label="Camera height" min={20} max={70} step={1} value={layout.cameraPct} onChange={(e) => changeLayout({ cameraPct: Number(e.target.value) })} className="w-full" />
            </label>
            <CameraFramingControl project={project} layout={layout} onChange={changeLayout} segmented={segmented} />
          </InspectorSection>

          <InspectorSection id="short-scenes" title="Scenes" icon={Clapperboard}>
            <p className="text-meta text-studio-500">
              {selected.take
                ? "Split where footage plays, the camera alone where none does. A scene shows only the camera or only the screen, easing in and out."
                : "Split by default. A scene shows only the camera or only the screen, easing in and out."}
            </p>
            {!selected.take && (
            <Button
              size="sm"
              variant="secondary"
              icon={Sparkles}
              className="w-full"
              disabled={!!busy || !playable}
              onClick={() =>
                sceneOp("Picking scenes", (p) => api.projectShortScenesPick(p.projectHandle, p.revision, selected.id))
              }
              title="From the recording, no AI: the camera for the opening line and while the mouse rests as you talk, the screen while you click a lot. Scenes you set yourself stay."
            >
              {busy === "Picking scenes" ? "Picking…" : "Pick scenes"}
            </Button>
            )}
            <div className="flex gap-1.5">
              {(
                [
                  ["camera", "Camera here", Camera],
                  ["screen", "Screen here", Monitor],
                ] as const
              ).map(([mode, label, Icon]) => (
                <Button
                  key={mode}
                  size="sm"
                  variant="ghost"
                  icon={Icon}
                  className="flex-1"
                  disabled={!!busy || !playable}
                  title={`Only the ${mode} for ${NEW_SCENE_US / 1e6} s from the playhead`}
                  onClick={() => {
                    const start = Math.min(currentTimeUs, Math.max(0, lengthUs - 500_000));
                    const end = Math.min(lengthUs, start + NEW_SCENE_US);
                    sceneOp("Adding a scene", (p) =>
                      api.projectShortSceneAdd(p.projectHandle, p.revision, selected.id, start, end, mode),
                    );
                  }}
                >
                  {label}
                </Button>
              ))}
            </div>
            {playable && scenes.length > 0 && (
              <>
                {/* The short at a glance: where each scene sits, and the playhead. */}
                <div
                  className="relative h-3 rounded-sm bg-studio-800 overflow-hidden cursor-pointer"
                  aria-hidden
                  onClick={(e) => {
                    const box = e.currentTarget.getBoundingClientRect();
                    seekPlayback(((e.clientX - box.left) / box.width) * lengthUs);
                  }}
                >
                  {scenes.map((scene) => (
                    <div
                      key={scene.id}
                      className={`absolute inset-y-0 ${scene.mode === "camera" ? "bg-accent/70" : "bg-video/70"}`}
                      style={{
                        left: `${(scene.shownStartUs! / lengthUs) * 100}%`,
                        width: `${((scene.shownEndUs! - scene.shownStartUs!) / lengthUs) * 100}%`,
                      }}
                    />
                  ))}
                  <div className="absolute inset-y-0 w-px bg-white" style={{ left: `${(currentTimeUs / lengthUs) * 100}%` }} />
                </div>
                <ul className="space-y-1">
                  {scenes.map((scene) => (
                    <li key={scene.id} className="flex items-center gap-1.5">
                      <button
                        type="button"
                        onClick={() => seekPlayback(scene.shownStartUs!)}
                        className="font-mono text-meta tabular-nums text-studio-300 hover:text-white w-[7.5rem] text-left"
                        title="Go to the scene"
                      >
                        {formatTime(scene.shownStartUs!)}–{formatTime(scene.shownEndUs!)}
                      </button>
                      <div className="flex flex-1 gap-0.5 bg-studio-900 border border-studio-700 rounded-control p-0.5">
                        {(["camera", "screen"] as const).map((mode) => (
                          <button
                            key={mode}
                            type="button"
                            aria-pressed={scene.mode === mode}
                            onClick={() =>
                              changeLayout({
                                scenes: layout.scenes.map((s) => (s.id === scene.id ? { ...s, mode, auto: false } : s)),
                              })
                            }
                            className={`${segmented(scene.mode === mode)} !h-6 !text-meta capitalize`}
                          >
                            {mode}
                          </button>
                        ))}
                      </div>
                      <button
                        type="button"
                        aria-label="Remove the scene"
                        title={scene.auto ? "Remove (picked automatically)" : "Remove"}
                        onClick={() => changeLayout({ scenes: layout.scenes.filter((s) => s.id !== scene.id) })}
                        className="h-6 w-6 shrink-0 inline-flex items-center justify-center rounded-control text-studio-500 hover:text-danger-fg hover:bg-danger/15"
                      >
                        <Trash2 className="w-3.5 h-3.5" />
                      </button>
                    </li>
                  ))}
                </ul>
                <label className="block space-y-1">
                  <div className="flex justify-between">
                    <span className="text-label text-studio-400">Change speed</span>
                    <span className="font-mono text-meta tabular-nums text-studio-200">
                      {layout.sceneTransitionMs ? `${layout.sceneTransitionMs} ms` : "cut"}
                    </span>
                  </div>
                  <input
                    type="range"
                    aria-label="Scene change speed"
                    min={0}
                    max={1000}
                    step={50}
                    value={layout.sceneTransitionMs}
                    onChange={(e) => changeLayout({ sceneTransitionMs: Number(e.target.value) })}
                    className="w-full"
                  />
                </label>
              </>
            )}
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

          <InspectorSection id="short-background" title="Background" icon={Palette}>
            <div className="grid grid-cols-5 gap-0.5 bg-studio-900 border border-studio-700 rounded-control p-0.5">
              {(
                [
                  ["project", "Video's"],
                  ["solid", "Colour"],
                  ["gradient", "Gradient"],
                  ["preset", "Built-in"],
                  ["wallpaper", "Image"],
                ] as const
              ).map(([kind, label]) => (
                <button
                  key={kind}
                  type="button"
                  aria-pressed={layout.backgroundType === kind}
                  onClick={() => changeLayout({ backgroundType: kind })}
                  disabled={kind === "wallpaper" && !project.layout?.wallpaperAsset}
                  title={kind === "wallpaper" && !project.layout?.wallpaperAsset ? "Choose an image in the editor's Background settings first" : undefined}
                  className={`${segmented(layout.backgroundType === kind)} !text-meta disabled:opacity-40`}
                >
                  {label}
                </button>
              ))}
            </div>
            {(layout.backgroundType === "solid" || layout.backgroundType === "gradient") && (
              <div className="flex gap-2">
                <input type="color" aria-label="Background colour" value={layout.backgroundColorStart} onChange={(e) => changeLayout({ backgroundColorStart: e.target.value })} className="h-control flex-1 bg-studio-850 border border-studio-700 rounded-control cursor-pointer" />
                {layout.backgroundType === "gradient" && (
                  <input type="color" aria-label="Background end colour" value={layout.backgroundColorEnd} onChange={(e) => changeLayout({ backgroundColorEnd: e.target.value })} className="h-control flex-1 bg-studio-850 border border-studio-700 rounded-control cursor-pointer" />
                )}
              </div>
            )}
            {layout.backgroundType === "preset" && (
              <div className="grid grid-cols-3 gap-1">
                {BACKGROUND_PRESETS.map((preset) => (
                  <button
                    key={preset.key}
                    type="button"
                    aria-pressed={layout.backgroundPreset === preset.key}
                    onClick={() => changeLayout({ backgroundPreset: preset.key })}
                    className={`h-10 rounded-control border text-meta text-white/90 flex items-end p-1.5 ${
                      layout.backgroundPreset === preset.key ? "border-accent-fg shadow-[0_0_0_2px_rgb(var(--accent-hover)/0.7)]" : "border-studio-700"
                    }`}
                    style={{ background: presetBackgroundCss(preset.key) }}
                  >
                    {preset.label}
                  </button>
                ))}
              </div>
            )}
          </InspectorSection>

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
            <p className="text-meta text-studio-500">Colours and font come from the editor's Captions settings; edit caption text on the short's timeline.</p>
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
