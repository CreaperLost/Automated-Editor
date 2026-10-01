import React, { useState } from "react";
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
      {open && <div className="space-y-4 px-3 pb-3 pt-1">{children}</div>}
    </section>
  );
};

export const RangeRow: React.FC<{
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
