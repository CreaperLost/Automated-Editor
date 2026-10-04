import React, { useEffect, useRef, useState } from "react";
import { Palette, Camera, Monitor } from "lucide-react";
import { InspectorSection, NumberGrid, RangeRow } from "./InspectorSection";
import { WebcamFocusSection } from "./WebcamFocusSection";
import { TrackClipSection } from "./TrackClipSection";
import { useSettingsStore } from "../../stores/settingsStore";
import { useProjectStore } from "../../stores/projectStore";
import { api } from "../../lib/ipc";
import { AudioSection } from "../audio/AudioSection";
import { CaptionsSection } from "../captions/CaptionsSection";
import { Button, Field, Notice, Segmented, Switch, Tabs, cn } from "../ui";
import {
  BACKGROUND_PRESETS,
  CameraBubblePosition,
  CameraBubbleSize,
  LAYOUT_UNSUPPORTED,
  ScreenCrop,
  WEBCAM_SIZE_PRESET_PCT,
  layoutFromSettings,
  presetBackgroundCss,
} from "../../lib/types";

type InspectorTab = "clip" | "video" | "audio" | "captions";
const TAB_KEY = "aeroedits.inspectorTab.v1";

function loadTab(): InspectorTab {
  try {
    const stored = window.localStorage.getItem(TAB_KEY);
    if (stored === "video" || stored === "audio" || stored === "captions") return stored;
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
  const selectedClipId = useProjectStore((s) => s.selectedOverlayClipId);
  const persistTimer = useRef<number>();
  const openedRef = useRef(openedProject);
  openedRef.current = openedProject;
  const [persistError, setPersistError] = useState<string>();

  // The Clip page exists while a clip on a track is selected; selecting one opens it.
  const clipSelected = !!selectedClipId && (openedProject?.overlayTracks ?? []).some((t) =>
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
    if (!openedRef.current) return;
    window.clearTimeout(persistTimer.current);
    persistTimer.current = window.setTimeout(() => {
      const opened = openedRef.current;
      if (!opened) return;
      const layout = layoutFromSettings(
        nextCanvas,
        nextCamera,
        opened.layout?.wallpaperAsset,
      );
      void api
        .projectLayoutUpdate(opened.projectHandle, opened.revision, layout)
        .then((updated) => {
          applyOpenedProject(updated);
          setPersistError(undefined);
        })
        .catch((err) => setPersistError(String(err)));
    }, 180);
  };

  useEffect(() => () => window.clearTimeout(persistTimer.current), []);

  const setCanvas = (patch: Parameters<typeof updateCanvas>[0]) => {
    updateCanvas(patch);
    persistLayout({ ...canvas, ...patch }, cameraBubble);
  };

  const setCamera = (patch: Parameters<typeof updateCameraBubble>[0]) => {
    updateCameraBubble(patch);
    persistLayout(canvas, { ...cameraBubble, ...patch });
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
        const layout = layoutFromSettings(canvas, cameraBubble, opened.layout?.wallpaperAsset);
        layout.backgroundType = "wallpaper";
        return api
          .projectLayoutUpdate(opened.projectHandle, opened.revision, layout, source)
          .then((updated) => {
            applyOpenedProject(updated);
            updateCanvas({ backgroundType: "wallpaper" });
            setPersistError(undefined);
          });
      })
      .catch((err) => setPersistError(String(err)));
  };

  const gradientPresets = [
    { label: "Indigo Cosmic", start: "#312e81", end: "#0f172a" },
    { label: "Electric Violet", start: "#4c1d95", end: "#1e1b4b" },
    { label: "Deep Obsidian", start: "#27272a", end: "#09090b" },
    { label: "Emerald Matrix", start: "#064e3b", end: "#022c22" },
    { label: "Sunset Velvet", start: "#881337", end: "#1e1b4b" },
    { label: "Cyber Ocean", start: "#0c4a6e", end: "#0f172a" },
  ];

  /** A background swatch: a gradient or built-in background to pick. */
  const swatch = (key: string, label: string, background: string, selected: boolean, onClick: () => void) => (
    <button
      key={key}
      type="button"
      onClick={onClick}
      aria-pressed={selected}
      className={cn(
        "h-12 rounded-control border text-left p-2 flex flex-col justify-end transition-shadow",
        selected
          ? "border-accent-fg shadow-[0_0_0_2px_rgb(var(--accent-hover)/0.7)]"
          : "border-studio-700 hover:border-studio-500",
      )}
      style={{ background }}
    >
      <span className="text-meta font-medium text-white/90 drop-shadow truncate">{label}</span>
    </button>
  );

  const colorInput = (label: string, value: string, onChange: (value: string) => void) => (
    <label className="block space-y-1">
      <span className="text-label text-studio-400">{label}</span>
      <input
        type="color"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        className="w-full h-control bg-studio-850 border border-studio-700 rounded-control cursor-pointer"
      />
    </label>
  );

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
            { value: "captions", label: "Captions", title: "Caption style and placement" },
          ]}
        />
      </div>

      {persistError && (
        <Notice tone="danger" onDismiss={() => setPersistError(undefined)}>
          {persistError}
        </Notice>
      )}

      <div className="flex-1 min-h-0 overflow-y-auto">
        {tab === "clip" && <TrackClipSection />}

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
              <RangeRow
                label="Corner radius"
                value={canvas.cornerRadiusPx}
                min={0}
                max={32}
                unit="px"
                onChange={(cornerRadiusPx) => setCanvas({ cornerRadiusPx })}
              />
              <RangeRow
                label="Drop shadow"
                value={canvas.shadowBlurPx}
                min={0}
                max={40}
                unit="px"
                onChange={(shadowBlurPx) => setCanvas({ shadowBlurPx })}
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

            <InspectorSection id="background" title="Background" icon={Palette}>
              <Segmented
                label="Background type"
                size="sm"
                className="w-full"
                value={canvas.backgroundType}
                onChange={(kind) => {
                  if (kind !== "wallpaper") {
                    setCanvas({ backgroundType: kind });
                    return;
                  }
                  if (openedProject?.layout?.wallpaperAsset) {
                    setCanvas({ backgroundType: "wallpaper" });
                    return;
                  }
                  pickWallpaper();
                }}
                options={[
                  { value: "solid", label: "Solid" },
                  { value: "gradient", label: "Gradient" },
                  { value: "preset", label: "Built-in" },
                  { value: "wallpaper", label: "Image" },
                ]}
              />

              {/* Only the selected type's controls are shown. */}
              {canvas.backgroundType === "solid" &&
                colorInput("Color", canvas.colorStart, (colorStart) => setCanvas({ colorStart }))}

              {canvas.backgroundType === "gradient" && (
                <>
                  <div className="grid grid-cols-2 gap-2">
                    {gradientPresets.map((preset) =>
                      swatch(
                        preset.label,
                        preset.label,
                        `linear-gradient(135deg, ${preset.start}, ${preset.end})`,
                        canvas.colorStart === preset.start && canvas.colorEnd === preset.end,
                        () => setCanvas({ colorStart: preset.start, colorEnd: preset.end }),
                      ),
                    )}
                  </div>
                  <div className="grid grid-cols-2 gap-2">
                    {colorInput("Start", canvas.colorStart, (colorStart) => setCanvas({ colorStart }))}
                    {colorInput("End", canvas.colorEnd, (colorEnd) => setCanvas({ colorEnd }))}
                  </div>
                </>
              )}

              {canvas.backgroundType === "preset" && (
                <div className="grid grid-cols-3 gap-2">
                  {BACKGROUND_PRESETS.map((preset) =>
                    swatch(preset.key, preset.label, presetBackgroundCss(preset.key), canvas.backgroundPreset === preset.key, () =>
                      setCanvas({ backgroundPreset: preset.key }),
                    ),
                  )}
                </div>
              )}

              {canvas.backgroundType === "wallpaper" && (
                <div className="space-y-1.5">
                  <Button variant="secondary" className="w-full" onClick={pickWallpaper}>
                    Choose image…
                  </Button>
                  <p className="text-meta text-studio-500">Copied into the project, so the export never depends on an outside file.</p>
                </div>
              )}
            </InspectorSection>

            <InspectorSection
              id="webcam"
              title="Camera"
              icon={Camera}
              extra={
                <Switch
                  checked={cameraBubble.enabled}
                  onChange={(enabled) => setCamera({ enabled })}
                  title={cameraBubble.enabled ? "Hide the camera" : "Show the camera"}
                />
              }
            >
              {!cameraBubble.enabled && <p className="text-label text-studio-500">The camera is hidden.</p>}
              {cameraBubble.enabled && (
                <>
                  <Field label="Shape" hint={cameraBubble.shape === "rect_16_9" ? LAYOUT_UNSUPPORTED.rect169 : undefined}>
                    <Segmented
                      label="Camera shape"
                      size="sm"
                      className="w-full"
                      value={cameraBubble.shape}
                      onChange={(shape) => setCamera({ shape })}
                      options={[
                        { value: "rect", label: "Rect" },
                        { value: "circle", label: "Circle" },
                        { value: "squircle", label: "Squircle" },
                        { value: "rect_16_9", label: "16:9", disabled: true, title: LAYOUT_UNSUPPORTED.rect169 },
                      ]}
                    />
                  </Field>

                  {cameraBubble.shape === "rect" && (
                    <RangeRow
                      label="Roundness"
                      value={cameraBubble.roundnessPct}
                      min={0}
                      max={50}
                      unit="%"
                      onChange={(roundnessPct) => setCamera({ roundnessPct })}
                    />
                  )}

                  <RangeRow
                    label="Size"
                    value={cameraBubble.sizePct}
                    min={5}
                    max={60}
                    step={0.5}
                    unit="%"
                    onChange={(sizePct) => setCamera({ sizePct })}
                  />
                  <Field label="">
                    <Segmented<CameraBubbleSize | "custom">
                      label="Camera size preset"
                      size="sm"
                      className="w-full"
                      value={
                        ((["sm", "md", "lg", "xl"] as const).find((size) => cameraBubble.sizePct === WEBCAM_SIZE_PRESET_PCT[size]) ??
                          "custom") as CameraBubbleSize | "custom"
                      }
                      onChange={(size) => {
                        if (size !== "custom") setCamera({ size, sizePct: WEBCAM_SIZE_PRESET_PCT[size] });
                      }}
                      options={(["sm", "md", "lg", "xl"] as const).map((size) => ({ value: size, label: size.toUpperCase() }))}
                    />
                  </Field>

                  <Field label="Position">
                    <select
                      aria-label="Camera position"
                      value={cameraBubble.position}
                      onChange={(e) => setCamera({ position: e.target.value as CameraBubblePosition })}
                      className="ui-field w-full"
                    >
                      <option value="bottom-right">Bottom right</option>
                      <option value="bottom-left">Bottom left</option>
                      <option value="top-right">Top right</option>
                      <option value="top-left">Top left</option>
                      <option value="custom">Custom</option>
                    </select>
                  </Field>

                  {cameraBubble.position === "custom" && (
                    <>
                      <RangeRow
                        label="Across"
                        value={Math.round(cameraBubble.customX)}
                        min={0}
                        max={100}
                        unit="%"
                        onChange={(customX) => setCamera({ customX })}
                      />
                      <RangeRow
                        label="Down"
                        value={Math.round(cameraBubble.customY)}
                        min={0}
                        max={100}
                        unit="%"
                        onChange={(customY) => setCamera({ customY })}
                      />
                    </>
                  )}

                  <RangeRow
                    label="Border"
                    value={cameraBubble.borderWidth}
                    min={0}
                    max={8}
                    unit="px"
                    onChange={(borderWidth) => setCamera({ borderWidth })}
                  />
                  <div className="flex flex-wrap gap-x-6 gap-y-2 pt-1">
                    <Switch checked={cameraBubble.mirror} onChange={(mirror) => setCamera({ mirror })} label="Mirror" />
                    <Switch checked={cameraBubble.shadow} onChange={(shadow) => setCamera({ shadow })} label="Shadow" />
                  </div>
                </>
              )}
            </InspectorSection>

            <WebcamFocusSection webcamShown={cameraBubble.enabled} />
          </>
        )}

        {tab === "audio" && <AudioSection />}
        {tab === "captions" && <CaptionsSection />}
      </div>
    </div>
  );
};
