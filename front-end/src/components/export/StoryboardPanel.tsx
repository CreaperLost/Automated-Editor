import React, { useEffect, useState } from "react";
import { LayoutGrid, Loader2 } from "lucide-react";
import { api } from "../../lib/ipc";
import { useProjectStore } from "../../stores/projectStore";
import { Button } from "../ui";
import type { Storyboard } from "../../lib/types";

interface StoryboardPanelProps {
  /** Shows a moment on the timeline (the dialog closes so it can be seen). */
  onShow: (timeUs: number) => void;
  disabled?: boolean;
}

/**
 * The storyboard (`mcp/storyboard.rs`): one picture of the whole edit, a tile per scene, the
 * same sheet agents get from the `storyboard` action. Clicking a tile shows its moment.
 */
export const StoryboardPanel: React.FC<StoryboardPanelProps> = ({ onShow, disabled }) => {
  const handle = useProjectStore((s) => s.openedProject?.projectHandle);
  const revision = useProjectStore((s) => s.openedProject?.revision);
  const [shown, setShown] = useState(false);
  const [board, setBoard] = useState<Storyboard>();
  const [error, setError] = useState<string>();

  // Drawn on demand (it renders a frame per tile), then again whenever the edit changes.
  useEffect(() => {
    if (!shown || !handle) return;
    let current = true;
    setError(undefined);
    api
      .storyboardRender(handle)
      .then((next) => current && setBoard(next))
      .catch((err) => current && setError(String(err)));
    return () => {
      current = false;
    };
  }, [shown, handle, revision]);

  const loading = shown && !error && (!board || board.revision !== revision);

  return (
    <section aria-labelledby="export-storyboard-title" className="space-y-2">
      <div className="flex items-center gap-2">
        <h3 id="export-storyboard-title" className="text-label font-medium text-studio-200">
          Storyboard
        </h3>
        <span className="text-label text-studio-400 flex items-center gap-1.5">
          {loading && <Loader2 className="w-3.5 h-3.5 animate-spin" aria-hidden />}
          {board && !loading && `${board.tiles.length} scenes`}
        </span>
        <span className="flex-1" />
        {!shown && (
          <Button size="sm" variant="secondary" icon={LayoutGrid} disabled={disabled} onClick={() => setShown(true)}>
            Show storyboard
          </Button>
        )}
      </div>
      {error && (
        <p role="alert" className="text-label text-danger-fg">
          {error}
        </p>
      )}
      {shown && board && (
        <div className="relative w-full rounded-control overflow-hidden border border-studio-800 bg-studio-950">
          <img src={board.image} alt="A tile per scene of the edit" className="block w-full h-auto" draggable={false} />
          {board.tiles.map((tile, i) => {
            const col = i % board.cols;
            const row = Math.floor(i / board.cols);
            const pct = (v: number, of: number) => `${(v / of) * 100}%`;
            return (
              <button
                key={tile.n}
                type="button"
                className="absolute rounded-sm hover:ring-2 hover:ring-accent-hover focus-visible:ring-2 focus-visible:ring-accent-hover outline-none"
                style={{
                  left: pct(board.gap + col * (board.tileWidth + board.gap), board.width),
                  top: pct(board.gap + row * (board.tileHeight + board.gap), board.height),
                  width: pct(board.tileWidth, board.width),
                  height: pct(board.tileHeight, board.height),
                }}
                title={`${tile.from}–${tile.to} · ${tile.kind}${tile.say ? ` · “${tile.say}”` : ""}`}
                aria-label={`Show scene ${tile.n} at ${tile.from}`}
                disabled={disabled}
                onClick={() => onShow(tile.fromUs)}
              />
            );
          })}
        </div>
      )}
    </section>
  );
};
