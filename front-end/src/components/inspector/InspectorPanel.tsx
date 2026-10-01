import React, { useEffect, useRef, useState } from "react";
import {
  Palette,
  Sliders,
  Camera,
  Monitor,
  Frame,
} from "lucide-react";
import { useSettingsStore } from "../../stores/settingsStore";
import { useProjectStore } from "../../stores/projectStore";
import { api } from "../../lib/ipc";
import { InspectorSection } from "./InspectorSection";
import { AudioSection } from "../audio/AudioSection";
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

const RangeRow: React.FC<{
  label: string;
  value: number;
  min: number;
  max: number;
  step?: number;
  unit: string;
  onChange: (value: number) => void;
}> = ({ label, value, min, max, step = 1, unit, onChange }) => (
  <div className="space-y-1.5">
    <div className="flex justify-between text-xs">
      <span className="text-studio-400">{label}</span>
      <span className="font-mono text-studio-300">
        {Math.round(value * 10) / 10}
        {unit}
      </span>
    </div>
    <input
      type="range"
      min={min}
      max={max}
      step={step}
      value={value}
      onChange={(e) => onChange(Number(e.target.value))}
      className="w-full accent-indigo-500 h-1.5 bg-studio-800 rounded-lg cursor-pointer"
    />
  </div>
);

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
                  const existing = openedProject?.layout?.wallpaperAsset;
                  if (existing && canvas.backgroundType !== "wallpaper") {
                    setCanvas({ backgroundType: "wallpaper" });
                    return;
                  }
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
                        .projectLayoutUpdate(
                          opened.projectHandle,
                          opened.revision,
                          layout,
                          source,
                        )
                        .then((updated) => {
                          applyOpenedProject(updated);
                          updateCanvas({ backgroundType: "wallpaper" });
                          setPersistError(undefined);
                        });
                    })
                    .catch((err) => setPersistError(String(err)));
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
          {canvas.backgroundType === "wallpaper" && (
            <p className="text-[10px] text-studio-500">
              Stored in the project bundle. Export never reads an external URL.
            </p>
          )}
        </div>

        <div className="grid grid-cols-3 gap-2">
          {BACKGROUND_PRESETS.map((preset) => (
            <button
              key={preset.key}
              type="button"
              onClick={() => setCanvas({ backgroundType: "preset", backgroundPreset: preset.key })}
              className={`h-12 rounded-lg border text-left p-2 flex flex-col justify-end transition-all ${
                canvas.backgroundType === "preset" && canvas.backgroundPreset === preset.key
                  ? "border-indigo-500 shadow-md shadow-indigo-500/20"
                  : "border-studio-750 hover:border-studio-600"
              }`}
              style={{ background: presetBackgroundCss(preset.key) }}
            >
              <span className="text-[10px] font-medium text-white/90 drop-shadow">{preset.label}</span>
            </button>
          ))}
        </div>

        <div className="grid grid-cols-2 gap-2">
          {gradientPresets.map((preset) => (
            <button
              key={preset.label}
              onClick={() =>
                setCanvas({
                  backgroundType: "gradient",
                  colorStart: preset.start,
                  colorEnd: preset.end,
                })
              }
              className={`h-12 rounded-lg border text-left p-2 flex flex-col justify-end transition-all ${
                canvas.backgroundType === "gradient" &&
                canvas.colorStart === preset.start &&
                canvas.colorEnd === preset.end
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
              disabled={canvas.backgroundType === "solid"}
              className="w-full h-8 bg-studio-850 border border-studio-800 rounded cursor-pointer disabled:opacity-40"
            />
          </label>
        </div>

      </InspectorSection>

      <InspectorSection id="canvas" title="Canvas" icon={Frame}>
        <div className="space-y-1.5">
          <label className="text-xs text-studio-400">Aspect Ratio</label>
          <div className="grid grid-cols-4 gap-1.5 bg-studio-850 p-1 rounded-lg border border-studio-800">
            {(["16:9", "9:16", "4:3", "1:1"] as const).map((ratio) => (
              <button
                key={ratio}
                onClick={() => setCanvas({ aspectRatio: ratio })}
                className={`py-1 text-xs font-mono rounded transition-colors ${
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

        <div className="space-y-1.5">
          <div className="flex justify-between text-xs">
            <span className="text-studio-400">Canvas Padding</span>
            <span className="font-mono text-studio-300">{canvas.paddingPx}px</span>
          </div>
          <input
            type="range"
            min={0}
            max={80}
            value={canvas.paddingPx}
            onChange={(e) => setCanvas({ paddingPx: Number(e.target.value) })}
            className="w-full accent-indigo-500 h-1.5 bg-studio-800 rounded-lg cursor-pointer"
          />
        </div>

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

        <RangeRow
          label="Screen Size"
          value={canvas.screenScalePct}
          min={40}
          max={100}
          unit="%"
          onChange={(screenScalePct) => setCanvas({ screenScalePct })}
        />

        <div className="space-y-1.5">
          <div className="flex justify-between text-xs">
            <span className="text-studio-400">Corner Radius</span>
            <span className="font-mono text-studio-300">{canvas.cornerRadiusPx}px</span>
          </div>
          <input
            type="range"
            min={0}
            max={32}
            value={canvas.cornerRadiusPx}
            onChange={(e) => setCanvas({ cornerRadiusPx: Number(e.target.value) })}
            className="w-full accent-indigo-500 h-1.5 bg-studio-800 rounded-lg cursor-pointer"
          />
        </div>

        <div className="space-y-1.5">
          <div className="flex justify-between text-xs">
            <span className="text-studio-400">Drop Shadow</span>
            <span className="font-mono text-studio-300">{canvas.shadowBlurPx}px</span>
          </div>
          <input
            type="range"
            min={0}
            max={40}
            value={canvas.shadowBlurPx}
            onChange={(e) => setCanvas({ shadowBlurPx: Number(e.target.value) })}
            className="w-full accent-indigo-500 h-1.5 bg-studio-800 rounded-lg cursor-pointer"
          />
        </div>

        <div className="space-y-2">
          <label className="text-xs text-studio-400">Crop</label>
          {(
            [
              { key: "left", label: "Left" },
              { key: "right", label: "Right" },
              { key: "top", label: "Top" },
              { key: "bottom", label: "Bottom" },
            ] as const
          ).map(({ key, label }) => (
            <RangeRow
              key={key}
              label={label}
              value={canvas.screenCrop[key]}
              min={0}
              max={45}
              step={0.5}
              unit="%"
              onChange={(value) =>
                setCanvas({ screenCrop: { ...canvas.screenCrop, [key]: value } as ScreenCrop })
              }
            />
          ))}
        </div>
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
              <div className="grid grid-cols-4 gap-1.5 bg-studio-850 p-1 rounded-lg border border-studio-800">
                {(["sm", "md", "lg", "xl"] as const).map((size) => (
                  <button
                    key={size}
                    type="button"
                    onClick={() =>
                      setCamera({ size: size as CameraBubbleSize, sizePct: WEBCAM_SIZE_PRESET_PCT[size] })
                    }
                    className={`py-1 text-xs uppercase font-mono rounded transition-colors ${
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
              <div className="space-y-2">
                <div className="flex justify-between text-xs">
                  <span className="text-studio-400">Custom X</span>
                  <span className="font-mono text-studio-300">{Math.round(cameraBubble.customX)}%</span>
                </div>
                <input
                  type="range"
                  min={0}
                  max={100}
                  value={cameraBubble.customX}
                  onChange={(e) => setCamera({ customX: Number(e.target.value) })}
                  className="w-full accent-indigo-500 h-1.5 bg-studio-800 rounded-lg cursor-pointer"
                />
                <div className="flex justify-between text-xs">
                  <span className="text-studio-400">Custom Y</span>
                  <span className="font-mono text-studio-300">{Math.round(cameraBubble.customY)}%</span>
                </div>
                <input
                  type="range"
                  min={0}
                  max={100}
                  value={cameraBubble.customY}
                  onChange={(e) => setCamera({ customY: Number(e.target.value) })}
                  className="w-full accent-indigo-500 h-1.5 bg-studio-800 rounded-lg cursor-pointer"
                />
              </div>
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

            <div className="space-y-1.5">
              <div className="flex justify-between text-xs">
                <span className="text-studio-400">Border Width</span>
                <span className="font-mono text-studio-300">{cameraBubble.borderWidth}px</span>
              </div>
              <input
                type="range"
                min={0}
                max={8}
                value={cameraBubble.borderWidth}
                onChange={(e) => setCamera({ borderWidth: Number(e.target.value) })}
                className="w-full accent-indigo-500 h-1.5 bg-studio-800 rounded-lg cursor-pointer"
              />
            </div>
          </>
        )}
      </InspectorSection>

      <AudioSection />
    </div>
  );
};
