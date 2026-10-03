import React, { useRef, useState } from "react";
import { ChevronDown, LucideIcon } from "lucide-react";

const SECTION_STATE_KEY = "aeroedits.inspector.sections";

function readOpenSections(): Record<string, boolean> {
  try {
    return JSON.parse(window.localStorage.getItem(SECTION_STATE_KEY) ?? "{}") ?? {};
  } catch {
    return {};
  }
}

/// A collapsible inspector group. Open/closed state is remembered per section.
export const InspectorSection: React.FC<{
  id: string;
  title: string;
  icon: LucideIcon;
  extra?: React.ReactNode;
  children: React.ReactNode;
}> = ({ id, title, icon: Icon, extra, children }) => {
  const [open, setOpen] = useState(() => readOpenSections()[id] ?? true);
  const toggle = () => {
    const next = !open;
    setOpen(next);
    try {
      window.localStorage.setItem(
        SECTION_STATE_KEY,
        JSON.stringify({ ...readOpenSections(), [id]: next }),
      );
    } catch {
      // Storage unavailable: the section still toggles for this session.
    }
  };
  return (
    <section className="rounded-lg border border-studio-800 bg-studio-900">
      <div className="flex items-center justify-between px-3 py-2">
        <button
          type="button"
          onClick={toggle}
          aria-expanded={open}
          className="flex flex-1 items-center space-x-2 text-xs font-semibold uppercase tracking-wider text-studio-400 hover:text-studio-200"
        >
          <ChevronDown
            className={`w-3.5 h-3.5 transition-transform ${open ? "" : "-rotate-90"}`}
          />
          <Icon className="w-3.5 h-3.5 text-indigo-400" />
          <span>{title}</span>
        </button>
        {extra}
      </div>
      {open && <div className="space-y-3 px-3 pb-3 pt-1">{children}</div>}
    </section>
  );
};

const clampStep = (value: number, min: number, max: number, step: number) => {
  const snapped = Math.round((value - min) / step) * step + min;
  // Trim float noise from the step arithmetic (0.1 + 0.2 and friends).
  return Math.min(max, Math.max(min, Number(snapped.toFixed(6))));
};

const formatNumber = (value: number) => String(Math.round(value * 10) / 10);

/// Pixels of horizontal drag that sweep a scrub field across its whole range.
const SCRUB_RANGE_PX = 240;
/// Movement before a press on the field becomes a drag instead of a click to type.
const SCRUB_THRESHOLD_PX = 3;

/// A compact numeric field: click to type an exact value, or drag sideways to scrub it
/// (Shift for fine steps). Arrow keys step it while typing.
export const ScrubNumber: React.FC<{
  label: string;
  value: number;
  min: number;
  max: number;
  step?: number;
  unit: string;
  onChange: (value: number) => void;
  className?: string;
}> = ({ label, value, min, max, step = 1, unit, onChange, className = "" }) => {
  const [draft, setDraft] = useState<string | null>(null);
  const [scrubbing, setScrubbing] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);
  const drag = useRef<{ x: number; origin: number; moved: boolean } | null>(null);

  const commit = (text: string) => {
    const parsed = Number.parseFloat(text.replace(",", "."));
    if (Number.isFinite(parsed)) {
      const next = clampStep(parsed, min, max, step);
      if (next !== value) onChange(next);
    }
    setDraft(null);
  };

  const onPointerDown = (event: React.PointerEvent<HTMLInputElement>) => {
    if (draft !== null || event.button !== 0) return; // already typing: let the caret move
    event.preventDefault();
    event.currentTarget.setPointerCapture(event.pointerId);
    drag.current = { x: event.clientX, origin: value, moved: false };
  };
  const onPointerMove = (event: React.PointerEvent<HTMLInputElement>) => {
    const current = drag.current;
    if (!current) return;
    const dx = event.clientX - current.x;
    if (!current.moved && Math.abs(dx) < SCRUB_THRESHOLD_PX) return;
    if (!current.moved) {
      current.moved = true;
      setScrubbing(true);
    }
    const perPx = Math.max(step, (max - min) / SCRUB_RANGE_PX) * (event.shiftKey ? 0.1 : 1);
    const next = clampStep(current.origin + dx * perPx, min, max, step);
    if (next !== value) onChange(next);
  };
  const onPointerUp = (event: React.PointerEvent<HTMLInputElement>) => {
    const current = drag.current;
    drag.current = null;
    setScrubbing(false);
    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }
    if (current && !current.moved) {
      // A plain click: type an exact value.
      setDraft(formatNumber(value));
      requestAnimationFrame(() => {
        inputRef.current?.focus();
        inputRef.current?.select();
      });
    }
  };

  return (
    <input
      ref={inputRef}
      type="text"
      inputMode="decimal"
      aria-label={label}
      title={`${label}: click to type, drag sideways to adjust (Shift for fine steps)`}
      value={draft ?? `${formatNumber(value)}${unit}`}
      readOnly={draft === null}
      onChange={(event) => setDraft(event.target.value)}
      onFocus={() => {
        // Tabbing in starts typing too.
        if (draft === null && !drag.current) {
          setDraft(formatNumber(value));
          requestAnimationFrame(() => inputRef.current?.select());
        }
      }}
      onBlur={() => draft !== null && commit(draft)}
      onKeyDown={(event) => {
        if (event.key === "Enter") {
          commit(draft ?? String(value));
          event.currentTarget.blur();
        } else if (event.key === "Escape") {
          setDraft(null);
          event.currentTarget.blur();
        } else if (event.key === "ArrowUp" || event.key === "ArrowDown") {
          event.preventDefault();
          const base = Number.parseFloat(draft ?? String(value));
          const delta = (event.key === "ArrowUp" ? step : -step) * (event.shiftKey ? 10 : 1);
          const next = clampStep((Number.isFinite(base) ? base : value) + delta, min, max, step);
          setDraft(formatNumber(next));
          onChange(next);
        }
      }}
      onPointerDown={onPointerDown}
      onPointerMove={onPointerMove}
      onPointerUp={onPointerUp}
      onPointerCancel={() => {
        drag.current = null;
        setScrubbing(false);
      }}
      className={`h-6 w-16 shrink-0 rounded border bg-studio-850 px-1.5 text-right font-mono text-[11px] tabular-nums outline-none transition-colors ${
        draft !== null
          ? "border-indigo-500 text-white cursor-text"
          : scrubbing
            ? "border-indigo-500/70 text-white cursor-ew-resize"
            : "border-studio-800 text-studio-300 hover:border-studio-600 cursor-ew-resize"
      } ${className}`}
    />
  );
};

/// One line: label, slider, exact value. Label and value columns have fixed widths so
/// every slider in the inspector lines up.
export const RangeRow: React.FC<{
  label: string;
  value: number;
  min: number;
  max: number;
  step?: number;
  unit: string;
  onChange: (value: number) => void;
}> = ({ label, value, min, max, step = 1, unit, onChange }) => (
  <div className="grid grid-cols-[6.5rem_minmax(0,1fr)_auto] items-center gap-2">
    <span className="text-xs leading-tight text-studio-400 line-clamp-2" title={label}>
      {label}
    </span>
    <input
      type="range"
      aria-label={label}
      min={min}
      max={max}
      step={step}
      value={value}
      onChange={(e) => onChange(Number(e.target.value))}
      className="w-full min-w-0 accent-indigo-500 h-1.5 bg-studio-800 rounded-lg cursor-pointer"
    />
    <ScrubNumber label={label} value={value} min={min} max={max} step={step} unit={unit} onChange={onChange} />
  </div>
);

/// Several values of one property as a compact two-column grid of scrub fields, no sliders.
export const NumberGrid: React.FC<{
  title: string;
  fields: { key: string; label: string; value: number }[];
  min: number;
  max: number;
  step?: number;
  unit: string;
  onChange: (key: string, value: number) => void;
}> = ({ title, fields, min, max, step, unit, onChange }) => (
  <fieldset className="space-y-1.5">
    <legend className="mb-1.5 text-[10px] font-semibold uppercase tracking-wider text-studio-500">
      {title}
    </legend>
    <div className="grid grid-cols-2 gap-x-3 gap-y-1.5">
      {fields.map((field) => (
        <label key={field.key} className="flex items-center justify-between gap-2 text-xs text-studio-400">
          <span>{field.label}</span>
          <ScrubNumber
            label={`${title} ${field.label}`}
            value={field.value}
            min={min}
            max={max}
            step={step}
            unit={unit}
            onChange={(value) => onChange(field.key, value)}
          />
        </label>
      ))}
    </div>
  </fieldset>
);
