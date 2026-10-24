import React, { useEffect, useRef, useState } from "react";
import { Image as ImageIcon, Camera, Monitor } from "lucide-react";
import { InspectorSection, NumberGrid, RangeRow } from "./InspectorSection";
import { LayoutsSection } from "../layouts/LayoutsSection";
import { CameraFramingControl } from "../shorts/CameraFraming";
import { TrackClipSection } from "./TrackClipSection";
import { GraphicSection } from "./GraphicSection";
import { useSettingsStore } from "../../stores/settingsStore";
import { useProjectStore } from "../../stores/projectStore";
import { api } from "../../lib/ipc";
import { useDebouncedSave } from "../../hooks/useDebouncedSave";
import { AudioSection } from "../audio/AudioSection";
import { CaptionsSection } from "../captions/CaptionsSection";
import { StylePanel } from "../style/StylePanel";
import { PresetMarks } from "../presets/PresetMarks";
import { Button, Field, Notice, Segmented, Switch, Tabs, cn } from "../ui";
import { ScreenCrop, layoutFromSettings } from "../../lib/types";

type InspectorTab = "clip" | "video" | "audio" | "captions" | "style";
const TAB_KEY = "aeroedits.inspectorTab.v1";

function loadTab(): InspectorTab {
  try {
    const stored = window.localStorage.getItem(TAB_KEY);
    if (stored === "video" || stored === "audio" || stored === "captions" || stored === "style") return stored;
  } catch {
    // Storage unavailable: start on Video.
  }
  return "video";
}

/// The inspector: what the selected clip does (when a track clip is selected), and how the
/// whole video looks, sounds and is captioned, one page each.
export const InspectorPanel: React.FC = () => {
  const {
    canvas,
    cameraBubble,
    updateCanvas,
    updateCameraBubble,
  } = useSettingsStore();
  const openedProject = useProjectStore((s) => s.openedProject);
  const applyOpenedProject = useProjectStore((s) => s.applyOpenedProject);
  const selectedClipId = useProjectStore((s) => s.selectedClipIds[0]);
  const layoutSave = useDebouncedSave(180);
  const openedRef = useRef(openedProject);
  openedRef.current = openedProject;
  const [persistError, setPersistError] = useState<string>();

  // The Clip page exists while a clip is selected; selecting one opens it.
  const clipSelected = !!selectedClipId && (openedProject?.sequence.tracks ?? []).some((t) =>
    t.clips.some((c) => c.id === selectedClipId),
  );
  const [chosenTab, setChosenTab] = useState<InspectorTab>(loadTab);
  useEffect(() => {
    if (clipSelected) setChosenTab("clip");
  }, [selectedClipId, clipSelected]);
  const tab: InspectorTab = chosenTab === "clip" && !clipSelected ? loadTab() : chosenTab;
  const chooseTab = (next: InspectorTab) => {
    setChosenTab(next);
    if (next === "clip") return;
    try {
      window.localStorage.setItem(TAB_KEY, next);
    } catch {
      // Not remembering the page is harmless.
    }
  };

  const persistLayout = (nextCanvas = canvas, nextCamera = cameraBubble) => {
    layoutSave.schedule((opened) => {
      const layout = layoutFromSettings(nextCanvas, nextCamera, opened.layout);
      void api
        .projectLayoutUpdate(opened.projectHandle, opened.revision, layout)
        .then((updated) => {
          applyOpenedProject(updated);
          setPersistError(undefined);
        })
        .catch((err) => setPersistError(String(err)));
    });
  };

  const setCanvas = (patch: Parameters<typeof updateCanvas>[0]) => {
    updateCanvas(patch);
    persistLayout({ ...canvas, ...patch }, cameraBubble);
  };

  const setCamera = (patch: Parameters<typeof updateCameraBubble>[0]) => {
    updateCameraBubble(patch);
    persistLayout(canvas, { ...cameraBubble, ...patch });
  };

  /** The camera clips framed by hand. */
  const setCameraClips = (cameraClips: Record<string, { offsetX: number; offsetY: number }> | undefined) => {
    const opened = openedRef.current;
    if (!opened) return;
    const layout = layoutFromSettings(canvas, cameraBubble, { ...opened.layout, aspectRatio: canvas.aspectRatio, paddingPx: 0, cameraClips });
    void api
      .projectLayoutUpdate(opened.projectHandle, opened.revision, layout)
      .then((updated) => {
        applyOpenedProject(updated);
        setPersistError(undefined);
      })
      .catch((err) => setPersistError(String(err)));
  };

  /** Copies a chosen image into the project's assets/ and switches the background to it. */
  const pickWallpaper = () => {
    const opened = openedRef.current;
    if (!opened) {
      setPersistError("Open a project to ingest wallpaper into assets/");
      return;
    }
    void api
      .pickWallpaperSource()
      .then((source) => {
        if (!source) return;
        const layout = layoutFromSettings(canvas, cameraBubble, opened.layout);
        return api
          .projectLayoutUpdate(opened.projectHandle, opened.revision, layout, source)
          .then((updated) => {
            applyOpenedProject(updated);
            setPersistError(undefined);
          });
      })
      .catch((err) => setPersistError(String(err)));
  };

  /** Back to the style's background: the project's image is forgotten. */
  const removeWallpaper = () => {
    const opened = openedRef.current;
    if (!opened) return;
    const layout = layoutFromSettings(canvas, cameraBubble, { ...opened.layout, wallpaperAsset: null } as typeof opened.layout);
    layout.wallpaperAsset = null;
    void api
      .projectLayoutUpdate(opened.projectHandle, opened.revision, layout)
      .then((updated) => {
        applyOpenedProject(updated);
        setPersistError(undefined);
      })
      .catch((err) => setPersistError(String(err)));
  };

  return (
    <div className="studio-inspector min-w-0 h-full flex flex-col bg-studio-900 select-none">
      <div className="shrink-0 h-10 px-1 flex items-stretch border-b border-studio-800">
        <Tabs<InspectorTab>
          label="Inspector page"
          size="sm"
          value={tab}
          onChange={chooseTab}
          items={[
            ...(clipSelected ? [{ value: "clip" as const, label: "Clip", title: "The selected clip on a track" }] : []),
            { value: "video", label: "Video", title: "Screen, background and camera" },
            { value: "audio", label: "Audio", title: "Volume, noise reduction and ducking" },
            { value: "captions", label: "Captions", title: "Where captions sit and how many words they hold" },
            { value: "style", label: "Style", title: "How graphics, captions and the frame look" },
          ]}
        />
      </div>

      {persistError && (
        <Notice tone="danger" onDismiss={() => setPersistError(undefined)}>
          {persistError}
        </Notice>
      )}

      <div className="flex-1 min-h-0 overflow-y-auto">
        {tab === "clip" && (
          <>
            <GraphicSection />
            <TrackClipSection />
          </>
        )}

        {tab === "video" && (
          <>
            <InspectorSection
              id="screen"
              title="Screen"
              icon={Monitor}
              extra={
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => setCanvas({ screenScalePct: 100, screenCrop: { left: 0, top: 0, right: 0, bottom: 0 } })}
                  title="Full size, no crop"
                >
                  Reset
                </Button>
              }
            >
              <PresetMarks part="layout" />
              <Field label="Canvas">
                <Segmented
                  label="Canvas aspect ratio"
                  size="sm"
                  className="w-full"
                  value={canvas.aspectRatio}
                  onChange={(aspectRatio) => setCanvas({ aspectRatio })}
                  options={(["16:9", "9:16", "4:3", "1:1"] as const).map((ratio) => ({ value: ratio, label: ratio }))}
                />
              </Field>

              <RangeRow
                label="Screen size"
                value={canvas.screenScalePct}
                min={40}
                max={100}
                unit="%"
                onChange={(screenScalePct) => setCanvas({ screenScalePct })}
              />

              <Field label="Cursor">
                <Switch
                  checked={canvas.cursorVisible}
                  onChange={(cursorVisible) => setCanvas({ cursorVisible })}
                  label="Show the pointer"
                  title="Draw the mouse pointer the recorder tracked (it follows zooms and moves smoothly). Recordings with the pointer already in the video are left as they are."
                />
              </Field>
              {canvas.cursorVisible && (
                <RangeRow
                  label="Cursor size"
                  value={canvas.cursorSizePct}
                  min={25}
                  max={400}
                  step={5}
                  unit="%"
                  onChange={(cursorSizePct) => setCanvas({ cursorSizePct })}
                />
              )}

              <Field label="Shortcuts">
                <Switch
                  checked={canvas.shortcutBadges}
                  onChange={(shortcutBadges) => setCanvas({ shortcutBadges })}
                  label="Show shortcut badges"
                  title="Show keyboard shortcuts pressed while recording (Ctrl + S) in a corner of the screen. Needs a recording made with keyboard recording on; typed text is never recorded."
                />
              </Field>

              <Field label="Window crop">
                <Switch
                  checked={canvas.windowCrop}
                  onChange={(windowCrop) => setCanvas({ windowCrop })}
                  label="Crop to the active window"
                  title="When the window you work in is clearly smaller than the recorded display, show just that window and glide to the next one when you switch. Quick switches, menus and popups never move it. Needs a recording made with window recording on."
                />
              </Field>
              {canvas.windowCrop && (
                <>
                  <RangeRow
                    label="Room around it"
                    value={canvas.windowCropPaddingPct}
                    min={0}
                    max={15}
                    step={0.5}
                    unit="%"
                    onChange={(windowCropPaddingPct) => setCanvas({ windowCropPaddingPct })}
                  />
                  {canvas.windowCropOff.length > 0 && (
                    <div className="flex items-center justify-between gap-2 text-meta text-studio-400">
                      <span>
                        Off for {canvas.windowCropOff.length} stretch{canvas.windowCropOff.length === 1 ? "" : "es"}
                      </span>
                      <Button size="sm" variant="ghost" onClick={() => setCanvas({ windowCropOff: [] })}>
                        Crop everywhere
                      </Button>
                    </div>
                  )}
                  <p className="text-meta text-studio-500">Turn it off for one clip from the Clip page.</p>
                </>
              )}

              <NumberGrid
                title="Crop"
                min={0}
                max={45}
                step={0.5}
                unit="%"
                fields={[
                  { key: "left", label: "Left", value: canvas.screenCrop.left },
                  { key: "right", label: "Right", value: canvas.screenCrop.right },
                  { key: "top", label: "Top", value: canvas.screenCrop.top },
                  { key: "bottom", label: "Bottom", value: canvas.screenCrop.bottom },
                ]}
                onChange={(key, value) =>
                  setCanvas({ screenCrop: { ...canvas.screenCrop, [key]: value } as ScreenCrop })
                }
              />
            </InspectorSection>

            <InspectorSection id="wallpaper" title="Background image" icon={ImageIcon}>
              <p className="text-meta text-studio-500">
                The background is the style&apos;s (Style page). An image of your own shows instead, in this project only.
              </p>
              <div className="flex gap-2">
                <Button variant="secondary" className="flex-1" onClick={pickWallpaper}>
                  {openedProject?.layout?.wallpaperAsset ? "Another image…" : "Choose image…"}
                </Button>
                {openedProject?.layout?.wallpaperAsset && (
                  <Button variant="ghost" onClick={removeWallpaper}>
                    Use the style&apos;s
                  </Button>
                )}
              </div>
            </InspectorSection>

            <LayoutsSection />

            <InspectorSection id="webcam" title="Camera" icon={Camera}>
              <p className="text-meta text-studio-500">Its border and shadow are the style&apos;s (Style page).</p>
              <Field label="Framing">
                <Switch
                  checked={cameraBubble.faceFraming}
                  onChange={(faceFraming) => setCamera({ faceFraming })}
                  label="Centre on my face"
                  title="Line every camera clip up with the face in it (found on this computer, once per recording). Off keeps the middle of the picture."
                />
              </Field>
              {openedProject && !openedProject.shortView && (
                <CameraFramingControl
                  project={openedProject}
                  layout={{ cameraClips: openedProject.layout?.cameraClips ?? {} }}
                  onChange={(patch) => setCameraClips(patch.cameraClips)}
                  segmented={(active) =>
                    cn(
                      "flex-1 h-7 inline-flex items-center justify-center gap-1 rounded-[5px] text-label font-medium transition-colors",
                      active
                        ? "bg-accent/20 text-accent-fg shadow-[inset_0_0_0_1px_rgb(var(--accent-hover)/0.55)]"
                        : "text-studio-400 hover:text-studio-100 hover:bg-studio-800",
                    )
                  }
                />
              )}
              <>
                  <div className="flex flex-wrap gap-x-6 gap-y-2 pt-1">
                    <Switch checked={cameraBubble.mirror} onChange={(mirror) => setCamera({ mirror })} label="Mirror" />
                  </div>
                  <Field label="Out of the way">
                    <Switch
                      checked={cameraBubble.autoHide}
                      onChange={(autoHide) => setCamera({ autoHide })}
                      label="Hide while you work under it"
                      title="Fade the camera bubble out while you click, drag, scroll, rest the pointer or type where it sits, and back in afterwards. It never moves to another corner."
                    />
                  </Field>
              </>
            </InspectorSection>
          </>
        )}

        {tab === "audio" && <AudioSection />}
        {tab === "captions" && <CaptionsSection />}
        {tab === "style" && <StylePanel />}
      </div>
    </div>
  );
};
