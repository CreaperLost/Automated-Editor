import React, { useEffect, useRef, useState } from "react";
import { ChevronDown, ChevronRight, Copy } from "lucide-react";
import { IconButton, cn } from "../ui";
import type { ExportLogLine } from "../../lib/types";

const LEVEL_CLASS: Record<ExportLogLine["level"], string> = {
  info: "text-studio-300",
  warn: "text-suggest-fg",
  error: "text-danger-fg",
};

const stamp = (secs: number) => {
  const m = Math.floor(secs / 60);
  const s = Math.floor(secs % 60);
  return `${m}:${s.toString().padStart(2, "0")}`;
};

/** The export's log as it streams in: collapsed to a "Log" line, open on a failure. */
export const ExportLog: React.FC<{ lines: ExportLogLine[]; failed: boolean }> = ({ lines, failed }) => {
  const [open, setOpen] = useState(failed);
  const listRef = useRef<HTMLOListElement>(null);
  // Stays at the newest line unless the reader scrolled up.
  const pinned = useRef(true);

  useEffect(() => {
    if (failed) setOpen(true);
  }, [failed]);
  useEffect(() => {
    const list = listRef.current;
    if (list && pinned.current) list.scrollTop = list.scrollHeight;
  }, [lines.length, open]);

  if (lines.length === 0) return null;
  const text = lines.map((l) => `${stamp(l.secs)} ${l.level === "info" ? "" : `${l.level.toUpperCase()} `}${l.text}`).join("\n");
  return (
    <div className="space-y-1">
      <div className="flex items-center gap-1">
        <button
          type="button"
          aria-expanded={open}
          onClick={() => setOpen(!open)}
          className="inline-flex items-center gap-1 text-meta text-studio-400 hover:text-studio-100"
        >
          {open ? <ChevronDown className="w-3.5 h-3.5" /> : <ChevronRight className="w-3.5 h-3.5" />}
          Log
          <span className="text-studio-500">({lines.length})</span>
        </button>
        {open && (
          <IconButton
            icon={Copy}
            size="sm"
            label="Copy the log"
            className="ml-auto"
            onClick={() => void navigator.clipboard?.writeText(text).catch(() => undefined)}
          />
        )}
      </div>
      {open && (
        <ol
          ref={listRef}
          onScroll={(e) => {
            const el = e.currentTarget;
            pinned.current = el.scrollHeight - el.scrollTop - el.clientHeight < 8;
          }}
          className="max-h-48 overflow-y-auto rounded-control border border-studio-800 bg-studio-900 px-2 py-1.5 font-mono text-meta leading-relaxed select-text"
        >
          {lines.map((line, i) => (
            <li key={i} className={cn("flex gap-2", LEVEL_CLASS[line.level])}>
              <span className="shrink-0 tabular-nums text-studio-500">{stamp(line.secs)}</span>
              <span className="min-w-0 break-words">{line.text}</span>
            </li>
          ))}
        </ol>
      )}
    </div>
  );
};
