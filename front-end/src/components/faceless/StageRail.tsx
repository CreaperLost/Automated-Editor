import React from "react";
import { Check, Circle, RefreshCw } from "lucide-react";
import { cn } from "../ui";
import type { FacelessStage } from "../../lib/types";

const STATE_TEXT = {
  todo: "Not done yet",
  done: "Done",
  stale: "Out of date: something before it changed, so do it again",
} as const;

/** A production's stages in order: done, out of date, or still to do; the next one stands out. */
export const StageRail: React.FC<{ stages: FacelessStage[]; next?: string; compact?: boolean }> = ({
  stages,
  next,
  compact,
}) => (
  <ol className="flex items-center gap-1 min-w-0 flex-wrap" aria-label="Stages">
    {stages.map((stage, index) => {
      const Icon = stage.state === "done" ? Check : stage.state === "stale" ? RefreshCw : Circle;
      const isNext = stage.id === next;
      return (
        <li key={stage.id} className="flex items-center gap-1">
          {index > 0 && <span className="w-2 h-px bg-studio-700" aria-hidden />}
          <span
            title={`${stage.label}: ${STATE_TEXT[stage.state]}${isNext ? " (next)" : ""}`}
            className={cn(
              "inline-flex items-center gap-1 rounded-full border px-2 whitespace-nowrap",
              compact ? "h-5 text-meta" : "h-6 text-label",
              stage.state === "done" && "border-success/40 bg-success/10 text-success-fg",
              stage.state === "stale" && "border-suggest/40 bg-suggest/10 text-suggest-fg",
              stage.state === "todo" && !isNext && "border-studio-700 text-studio-500",
              stage.state === "todo" && isNext && "border-accent/60 bg-accent/10 text-accent-fg",
            )}
          >
            <Icon className="w-3 h-3" aria-hidden />
            {stage.label}
          </span>
        </li>
      );
    })}
  </ol>
);
