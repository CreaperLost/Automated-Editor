import React, { useEffect, useRef, useState } from "react";
import { LayoutGrid } from "lucide-react";
import { LAYOUT_PRESETS, applyPresetToWorkspace, type LayoutPreset } from "./dockLayout";

/// Top-bar menu that switches the workspace to a built-in panel layout.
export const LayoutMenu: React.FC = () => {
  const [open, setOpen] = useState(false);
  const rootRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    const onPointerDown = (event: PointerEvent) => {
      if (!rootRef.current?.contains(event.target as Node)) setOpen(false);
    };
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") setOpen(false);
    };
    window.addEventListener("pointerdown", onPointerDown);
    window.addEventListener("keydown", onKeyDown);
    return () => {
      window.removeEventListener("pointerdown", onPointerDown);
      window.removeEventListener("keydown", onKeyDown);
    };
  }, [open]);

  const choose = (preset: LayoutPreset) => {
    applyPresetToWorkspace(preset);
    setOpen(false);
  };

  return (
    <div ref={rootRef} className="relative">
      <button
        type="button"
        aria-haspopup="menu"
        aria-expanded={open}
        onClick={() => setOpen((value) => !value)}
        className="flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg bg-studio-850 hover:bg-studio-800 border border-studio-700 text-studio-300 text-xs font-medium transition-colors"
        title="Panel layout. Drag a panel's tab to dock it somewhere else."
      >
        <LayoutGrid className="w-3.5 h-3.5" />
        <span>Layout</span>
      </button>
      {open && (
        <div
          role="menu"
          className="absolute right-0 top-full mt-1.5 w-72 z-50 rounded-xl border border-studio-700 bg-studio-900 shadow-2xl p-1.5 text-xs"
        >
          {(Object.keys(LAYOUT_PRESETS) as LayoutPreset[]).map((preset) => (
            <button
              key={preset}
              type="button"
              role="menuitem"
              onClick={() => choose(preset)}
              className="w-full text-left px-2.5 py-2 rounded-lg hover:bg-studio-800"
            >
              <div className="font-medium text-studio-100">{LAYOUT_PRESETS[preset].label}</div>
              <div className="text-studio-500">{LAYOUT_PRESETS[preset].hint}</div>
            </button>
          ))}
          <div className="my-1 h-px bg-studio-800" />
          <button
            type="button"
            role="menuitem"
            onClick={() => choose("editing")}
            className="w-full text-left px-2.5 py-2 rounded-lg text-studio-300 hover:bg-studio-800"
          >
            Reset to default layout
          </button>
          <p className="px-2.5 pt-1 pb-1.5 text-[11px] text-studio-500">
            Drag a panel by its tab onto another panel&apos;s edge to dock it there, or onto its middle to tab them
            together. Drag the gaps between panels to resize.
          </p>
        </div>
      )}
    </div>
  );
};
