import React, { useEffect, useRef, useState } from "react";
import { Palette, Sliders, Camera, Monitor } from "lucide-react";
import { InspectorSection, NumberGrid, RangeRow } from "./InspectorSection";
import { WebcamFocusSection } from "./WebcamFocusSection";
import { TrackClipSection } from "./TrackClipSection";
import { useSettingsStore } from "../../stores/settingsStore";
import { useProjectStore } from "../../stores/projectStore";
import { api } from "../../lib/ipc";
import { AudioSection } from "../audio/AudioSection";
import { CaptionsSection } from "../captions/CaptionsSection";
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

export const InspectorPanel: React.FC = () => {
  const {
    canvas,
    cameraBubble,
    updateCanvas,
    updateCameraBubble,
  } = useSettingsStore();
  const openedProject = useProjectStore((s) => s.openedProject);
  const applyOpenedProject = useProjectStore((s) => s.applyOpenedProject);
  const persistTimer = useRef<number>();
  const openedRef = useRef(openedProject);
  openedRef.current = openedProject;
  const [persistError, setPersistError] = useState<string>();

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

  return (
    <div className="studio-inspector min-w-0 h-full border-l border-studio-800 bg-studio-900/95 flex flex-col overflow-y-auto select-none p-5 space-y-3">
      <div className="flex items-center justify-between pb-3 border-b border-studio-800">
        <div className="flex items-center space-x-2 text-white font-semibold text-sm">
          <Sliders className="w-4 h-4 text-indigo-400" />
          <span>Studio Inspector</span>
        </div>
        <span className="text-[11px] px-2 py-0.5 rounded bg-studio-800 text-studio-400 font-mono">
          {openedProject ? "Revisioned" : "Customizer"}
        </span>
      </div>

      {persistError && (
        <p role="alert" className="text-[11px] text-rose-300 bg-rose-950/40 border border-rose-900/40 rounded p-2">
          {persistError}
        </p>
      )}

      <TrackClipSection />

      <InspectorSection id="background" title="Background" icon={Palette}>

        <div className="space-y-1.5">
          <label className="text-xs text-studio-400">Type</label>
          <div className="grid grid-cols-4 gap-1.5 bg-studio-850 p-1 rounded-lg border border-studio-800">
            {(["solid", "gradient", "preset", "wallpaper"] as const).map((kind) => (
              <button
                key={kind}
                type="button"
                onClick={() => {
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
                className={`py-1 text-xs capitalize rounded transition-colors ${
                  canvas.backgroundType === kind
                    ? "bg-indigo-600 text-white font-medium"
                    : "text-studio-400 hover:text-studio-200"
                }`}
              >
                {kind === "wallpaper" ? "Image" : kind === "preset" ? "Built-in" : kind}
              </button>
            ))}
          </div>
        </div>

        {/* Only the selected type's controls are shown. */}
        {canvas.backgroundType === "solid" && (
          <label className="block text-xs text-studio-400 space-y-1">
            <span>Color</span>
            <input
              type="color"
              value={canvas.colorStart}
              onChange={(e) => setCanvas({ colorStart: e.target.value })}
              className="w-full h-8 bg-studio-850 border border-studio-800 rounded cursor-pointer"
            />
          </label>
        )}

        {canvas.backgroundType === "gradient" && (
          <>
            <div className="grid grid-cols-2 gap-2">
              {gradientPresets.map((preset) => (
                <button
                  key={preset.label}
                  type="button"
                  onClick={() => setCanvas({ colorStart: preset.start, colorEnd: preset.end })}
                  className={`h-12 rounded-lg border text-left p-2 flex flex-col justify-end transition-all ${
                    canvas.colorStart === preset.start && canvas.colorEnd === preset.end
                      ? "border-indigo-500 shadow-md shadow-indigo-500/20"
                      : "border-studio-750 hover:border-studio-600"
                  }`}
                  style={{
                    background: `linear-gradient(135deg, ${preset.start}, ${preset.end})`,
                  }}
                >
                  <span className="text-[10px] font-medium text-white/90 drop-shadow">
                    {preset.label}
                  </span>
                </button>
              ))}
            </div>
            <div className="grid grid-cols-2 gap-2">
              <label className="text-xs text-studio-400 space-y-1">
                <span>Start</span>
                <input
                  type="color"
                  value={canvas.colorStart}
                  onChange={(e) => setCanvas({ colorStart: e.target.value })}
                  className="w-full h-8 bg-studio-850 border border-studio-800 rounded cursor-pointer"
                />
              </label>
              <label className="text-xs text-studio-400 space-y-1">
                <span>End</span>
                <input
                  type="color"
                  value={canvas.colorEnd}
                  onChange={(e) => setCanvas({ colorEnd: e.target.value })}
                  className="w-full h-8 bg-studio-850 border border-studio-800 rounded cursor-pointer"
                />
              </label>
            </div>
          </>
        )}

        {canvas.backgroundType === "preset" && (
          <div className="grid grid-cols-3 gap-2">
            {BACKGROUND_PRESETS.map((preset) => (
              <button
                key={preset.key}
                type="button"
                onClick={() => setCanvas({ backgroundPreset: preset.key })}
                className={`h-12 rounded-lg border text-left p-2 flex flex-col justify-end transition-all ${
                  canvas.backgroundPreset === preset.key
                    ? "border-indigo-500 shadow-md shadow-indigo-500/20"
                    : "border-studio-750 hover:border-studio-600"
                }`}
                style={{ background: presetBackgroundCss(preset.key) }}
              >
                <span className="text-[10px] font-medium text-white/90 drop-shadow">{preset.label}</span>
              </button>
            ))}
          </div>
        )}

        {canvas.backgroundType === "wallpaper" && (
          <div className="space-y-1.5">
            <button
              type="button"
              onClick={pickWallpaper}
              className="w-full py-1.5 text-xs rounded border border-studio-750 text-studio-300 hover:border-studio-600 hover:text-studio-100"
            >
              Choose image…
            </button>
            <p className="text-[10px] text-studio-500">
              Stored in the project bundle. Export never reads an external URL.
            </p>
          </div>
        )}

      </InspectorSection>

      <InspectorSection
        id="screen"
        title="Screen"
        icon={Monitor}
        extra={
          <button
            type="button"
            onClick={() =>
              setCanvas({ screenScalePct: 100, screenCrop: { left: 0, top: 0, right: 0, bottom: 0 } })
            }
            className="text-[11px] text-studio-400 hover:text-studio-200"
          >
            Reset
          </button>
        }
      >
        <div className="grid grid-cols-[6.5rem_minmax(0,1fr)] items-center gap-2">
          <span className="text-xs text-studio-400">Canvas</span>
          <div
            role="radiogroup"
            aria-label="Canvas aspect ratio"
            className="grid grid-cols-4 gap-1 bg-studio-850 p-0.5 rounded-md border border-studio-800"
          >
            {(["16:9", "9:16", "4:3", "1:1"] as const).map((ratio) => (
              <button
                key={ratio}
                type="button"
                role="radio"
                aria-checked={canvas.aspectRatio === ratio}
                onClick={() => setCanvas({ aspectRatio: ratio })}
                className={`py-0.5 text-[11px] font-mono rounded transition-colors ${
                  canvas.aspectRatio === ratio
                    ? "bg-indigo-600 text-white font-semibold"
                    : "text-studio-400 hover:text-studio-200"
                }`}
              >
                {ratio}
              </button>
            ))}
          </div>
        </div>

        <RangeRow
          label="Screen Size"
          value={canvas.screenScalePct}
          min={40}
          max={100}
          unit="%"
          onChange={(screenScalePct) => setCanvas({ screenScalePct })}
        />
        <RangeRow
          label="Corner Radius"
          value={canvas.cornerRadiusPx}
          min={0}
          max={32}
          unit="px"
          onChange={(cornerRadiusPx) => setCanvas({ cornerRadiusPx })}
        />
        <RangeRow
          label="Drop Shadow"
          value={canvas.shadowBlurPx}
          min={0}
          max={40}
          unit="px"
          onChange={(shadowBlurPx) => setCanvas({ shadowBlurPx })}
        />

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

      <InspectorSection
        id="webcam"
        title="Webcam"
        icon={Camera}
        extra={
          <input
            type="checkbox"
            aria-label="Show webcam"
            title="Show webcam"
            checked={cameraBubble.enabled}
            onChange={(e) => setCamera({ enabled: e.target.checked })}
            className="rounded bg-studio-800 border-studio-700 text-indigo-600 focus:ring-0 cursor-pointer"
          />
        }
      >

        {cameraBubble.enabled && (
          <>
            <div className="space-y-1.5">
              <label className="text-xs text-studio-400">Shape</label>
              <div className="grid grid-cols-4 gap-1.5 bg-studio-850 p-1 rounded-lg border border-studio-800">
                {(
                  [
                    { key: "rect", label: "Rect" },
                    { key: "circle", label: "Circle" },
                    { key: "squircle", label: "Squircle" },
                  ] as const
                ).map(({ key, label }) => (
                  <button
                    key={key}
                    type="button"
                    onClick={() => setCamera({ shape: key })}
                    className={`py-1 text-xs rounded transition-colors ${
                      cameraBubble.shape === key
                        ? "bg-indigo-600 text-white font-medium"
                        : "text-studio-400 hover:text-studio-200"
                    }`}
                  >
                    {label}
                  </button>
                ))}
                <button
                  type="button"
                  disabled
                  title={LAYOUT_UNSUPPORTED.rect169}
                  className="py-1 text-xs rounded text-studio-600 cursor-not-allowed"
                >
                  16:9
                </button>
              </div>
              {cameraBubble.shape === "rect_16_9" && (
                <p className="text-[10px] text-studio-500">{LAYOUT_UNSUPPORTED.rect169}</p>
              )}
            </div>

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

            <div className="space-y-1.5">
              <RangeRow
                label="Size"
                value={cameraBubble.sizePct}
                min={5}
                max={60}
                step={0.5}
                unit="%"
                onChange={(sizePct) => setCamera({ sizePct })}
              />
              <div className="ml-[7rem] grid grid-cols-4 gap-1 bg-studio-850 p-0.5 rounded-md border border-studio-800">
                {(["sm", "md", "lg", "xl"] as const).map((size) => (
                  <button
                    key={size}
                    type="button"
                    onClick={() =>
                      setCamera({ size: size as CameraBubbleSize, sizePct: WEBCAM_SIZE_PRESET_PCT[size] })
                    }
                    className={`py-0.5 text-[11px] uppercase font-mono rounded transition-colors ${
                      cameraBubble.sizePct === WEBCAM_SIZE_PRESET_PCT[size]
                        ? "bg-indigo-600 text-white font-medium"
                        : "text-studio-400 hover:text-studio-200"
                    }`}
                  >
                    {size}
                  </button>
                ))}
              </div>
            </div>

            <div className="space-y-1.5">
              <label className="text-xs text-studio-400">Position</label>
              <div className="grid grid-cols-2 gap-1.5">
                {(
                  [
                    { key: "bottom-right", label: "Bottom Right" },
                    { key: "bottom-left", label: "Bottom Left" },
                    { key: "top-right", label: "Top Right" },
                    { key: "top-left", label: "Top Left" },
                    { key: "custom", label: "Custom" },
                  ] as const
                ).map(({ key, label }) => (
                  <button
                    key={key}
                    onClick={() => setCamera({ position: key as CameraBubblePosition })}
                    className={`py-1.5 px-2 text-xs rounded-md border text-left transition-colors ${
                      cameraBubble.position === key
                        ? "bg-indigo-600/20 border-indigo-500/40 text-indigo-200 font-medium"
                        : "bg-studio-850 border-studio-800 text-studio-400 hover:text-studio-200"
                    }`}
                  >
                    {label}
                  </button>
                ))}
              </div>
            </div>

            {cameraBubble.position === "custom" && (
              <>
                <RangeRow
                  label="Custom X"
                  value={Math.round(cameraBubble.customX)}
                  min={0}
                  max={100}
                  unit="%"
                  onChange={(customX) => setCamera({ customX })}
                />
                <RangeRow
                  label="Custom Y"
                  value={Math.round(cameraBubble.customY)}
                  min={0}
                  max={100}
                  unit="%"
                  onChange={(customY) => setCamera({ customY })}
                />
              </>
            )}

            <label className="flex items-center justify-between text-xs text-studio-400">
              <span>Mirror webcam</span>
              <input
                type="checkbox"
                checked={cameraBubble.mirror}
                onChange={(e) => setCamera({ mirror: e.target.checked })}
                className="rounded bg-studio-800 border-studio-700 text-indigo-600 focus:ring-0 cursor-pointer"
              />
            </label>

            <label className="flex items-center justify-between text-xs text-studio-400">
              <span>Webcam shadow</span>
              <input
                type="checkbox"
                checked={cameraBubble.shadow}
                onChange={(e) => setCamera({ shadow: e.target.checked })}
                className="rounded bg-studio-800 border-studio-700 text-indigo-600 focus:ring-0 cursor-pointer"
              />
            </label>

            <RangeRow
              label="Border Width"
              value={cameraBubble.borderWidth}
              min={0}
              max={8}
              unit="px"
              onChange={(borderWidth) => setCamera({ borderWidth })}
            />
          </>
        )}
      </InspectorSection>

      <WebcamFocusSection webcamShown={cameraBubble.enabled} />
      <AudioSection />
      <CaptionsSection />
    </div>
  );
};
