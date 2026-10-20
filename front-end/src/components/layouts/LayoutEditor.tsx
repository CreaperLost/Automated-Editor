import React from "react";
import { RangeRow } from "../inspector/InspectorSection";
import { Field, Segmented, Switch, cn } from "../ui";
import {
  BUBBLE_SIZES,
  BUILT_IN_LAYOUTS,
  DEFAULT_BUBBLE,
  type BubbleSpot,
  type Layout,
  type ScreenFit,
} from "../../lib/types";

const GRID: BubbleSpot[] = ["topLeft", "top", "topRight", "left", "center", "right", "bottomLeft", "bottom", "bottomRight"];
const SPOT_LABEL: Record<BubbleSpot, string> = {
  topLeft: "Top left",
  top: "Top",
  topRight: "Top right",
  left: "Left",
  center: "Centre",
  right: "Right",
  bottomLeft: "Bottom left",
  bottom: "Bottom",
  bottomRight: "Bottom right",
  custom: "Anywhere",
};

/** Each kind's starting point when switching to it. */
function startOf(kind: Layout["kind"]): Layout {
  if (kind === "bubble") return DEFAULT_BUBBLE;
  return BUILT_IN_LAYOUTS.find((b) => b.layout.kind === kind)!.layout;
}

const ScreenFitField: React.FC<{ value: ScreenFit; onChange: (screen: ScreenFit) => void }> = ({ value, onChange }) => (
  <Field label="Screen" hint={value === "auto" ? "Fitted on a landscape frame, filled on a portrait one." : undefined}>
    <Segmented<ScreenFit>
      label="How the screen takes its part"
      size="sm"
      className="w-full"
      value={value}
      onChange={onChange}
      options={[
        { value: "auto", label: "Auto" },
        { value: "fit", label: "Fit", title: "The whole screen, with the background around it" },
        { value: "fill", label: "Fill", title: "Its part filled, cut to that shape and following the zooms" },
      ]}
    />
  </Field>
);

/// Edits one layout: its kind, then that kind's settings. Every change is a whole new layout.
export const LayoutEditor: React.FC<{ layout: Layout; portrait: boolean; onChange: (layout: Layout) => void }> = ({
  layout,
  portrait,
  onChange,
}) => (
  <div className="space-y-3">
    <Segmented<Layout["kind"]>
      label="Layout kind"
      size="sm"
      className="w-full"
      value={layout.kind}
      onChange={(kind) => kind !== layout.kind && onChange(startOf(kind))}
      options={[
        { value: "bubble", label: "Bubble" },
        { value: "sideBySide", label: "Side by side" },
        { value: "cameraOnly", label: "Camera" },
        { value: "screenOnly", label: "Screen" },
      ]}
    />

    {layout.kind === "bubble" && (
      <>
        <Field label="Where">
          <div className="flex items-start gap-3">
            <div className="grid grid-cols-3 gap-1" role="radiogroup" aria-label="Bubble position">
              {GRID.map((spot) => (
                <button
                  key={spot}
                  type="button"
                  role="radio"
                  aria-checked={layout.spot === spot}
                  aria-label={SPOT_LABEL[spot]}
                  title={SPOT_LABEL[spot]}
                  onClick={() => onChange({ ...layout, spot })}
                  className={cn(
                    "h-5 w-7 rounded-[4px] border transition-colors",
                    layout.spot === spot ? "bg-accent border-accent-hover" : "bg-studio-850 border-studio-700 hover:border-studio-500",
                  )}
                />
              ))}
            </div>
            <button
              type="button"
              aria-pressed={layout.spot === "custom"}
              onClick={() => onChange({ ...layout, spot: "custom" })}
              className={cn(
                "h-7 px-2 rounded-control border text-meta",
                layout.spot === "custom" ? "border-accent-hover text-accent-fg bg-accent/15" : "border-studio-700 text-studio-400 hover:text-studio-100",
              )}
            >
              Anywhere
            </button>
          </div>
        </Field>
        {layout.spot === "custom" && (
          <>
            <RangeRow label="Across" value={Math.round(layout.customX)} min={0} max={100} unit="%" onChange={(customX) => onChange({ ...layout, customX })} />
            <RangeRow label="Down" value={Math.round(layout.customY)} min={0} max={100} unit="%" onChange={(customY) => onChange({ ...layout, customY })} />
          </>
        )}
        <Field label="Size">
          <Segmented<string>
            label="Bubble size"
            size="sm"
            className="w-full"
            value={BUBBLE_SIZES.find((s) => s.pct === layout.sizePct)?.label ?? "custom"}
            onChange={(label) => {
              const size = BUBBLE_SIZES.find((s) => s.label === label);
              if (size) onChange({ ...layout, sizePct: size.pct });
            }}
            options={[
              ...BUBBLE_SIZES.map((s) => ({ value: s.label as string, label: s.label, title: `${s.pct} % of the short side` })),
              { value: "custom", label: "Own", disabled: true },
            ]}
          />
        </Field>
        <RangeRow label="Exact size" value={layout.sizePct} min={5} max={60} step={0.5} unit="%" onChange={(sizePct) => onChange({ ...layout, sizePct })} />
        <Field label="Shape">
          <Segmented
            label="Bubble shape"
            size="sm"
            className="w-full"
            value={layout.shape}
            onChange={(shape) => onChange({ ...layout, shape })}
            options={[
              { value: "rect", label: "Rect" },
              { value: "circle", label: "Circle" },
              { value: "squircle", label: "Squircle" },
            ]}
          />
        </Field>
        {layout.shape === "rect" && (
          <RangeRow label="Roundness" value={layout.roundnessPct} min={0} max={50} unit="%" onChange={(roundnessPct) => onChange({ ...layout, roundnessPct })} />
        )}
        <ScreenFitField value={layout.screen} onChange={(screen) => onChange({ ...layout, screen })} />
      </>
    )}

    {layout.kind === "sideBySide" && (
      <>
        <Field label="Camera">
          <Segmented<"first" | "second">
            label="Camera side"
            size="sm"
            className="w-full"
            value={layout.cameraFirst ? "first" : "second"}
            onChange={(side) => onChange({ ...layout, cameraFirst: side === "first" })}
            options={[
              { value: "first", label: portrait ? "On top" : "Left", title: "Left on a landscape frame, on top on a portrait one" },
              { value: "second", label: portrait ? "Below" : "Right", title: "Right on a landscape frame, below on a portrait one" },
            ]}
          />
        </Field>
        <RangeRow label="Camera share" value={layout.cameraPct} min={20} max={80} unit="%" onChange={(cameraPct) => onChange({ ...layout, cameraPct })} />
        <Field label="Style">
          <Switch
            checked={layout.overlap}
            onChange={(overlap) => onChange({ ...layout, overlap })}
            label="Camera card over the screen's edge"
            title="Off: an even split. On: the camera as a rounded card overlapping the screen."
          />
        </Field>
        <ScreenFitField value={layout.screen} onChange={(screen) => onChange({ ...layout, screen })} />
      </>
    )}

    {layout.kind === "cameraOnly" && (
      <>
        <RangeRow label="Size" value={layout.sizePct} min={40} max={100} unit="%" onChange={(sizePct) => onChange({ ...layout, sizePct })} />
        <RangeRow label="Punch-in" value={layout.punchIn} min={1} max={2} step={0.05} unit="×" onChange={(punchIn) => onChange({ ...layout, punchIn })} />
      </>
    )}

    {layout.kind === "screenOnly" && <ScreenFitField value={layout.screen} onChange={(screen) => onChange({ ...layout, screen })} />}
  </div>
);
