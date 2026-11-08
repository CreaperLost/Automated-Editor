import React, { useEffect, useState } from "react";
import { AlertTriangle, MoveHorizontal, ScanSearch } from "lucide-react";
import { Button } from "../ui";
import { api } from "../../lib/ipc";
import type { FacelessVisuals } from "../../lib/types";
import { useProjectStore } from "../../stores/projectStore";

const clock = (us: number) => {
  const s = Math.floor(us / 1e6);
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;
};

/** The production's graphics as the visuals check sees them: what blocks, what to look at,
 * and each graphic with the words under it. */
export const VisualsView: React.FC<{ refreshKey: unknown; ready: boolean; onChange: () => void }> = ({
  refreshKey,
  ready,
  onChange,
}) => {
  const [visuals, setVisuals] = useState<FacelessVisuals | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [open, setOpen] = useState(false);

  useEffect(() => {
    let active = true;
    void api
      .facelessVisualsGet()
      .then((v) => active && setVisuals(v))
      .catch(() => active && setVisuals(null));
    return () => {
      active = false;
    };
  }, [refreshKey]);

  const run = async (fill: boolean) => {
    setBusy(true);
    setError(null);
    try {
      const result = await api.facelessVisuals(fill);
      setVisuals(result.visuals);
      if (result.opened) await useProjectStore.getState().applyOpenedProject(result.opened);
      onChange();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const seek = (us: number) => {
    const { openedProject, applyPlaybackStatus } = useProjectStore.getState();
    if (!openedProject) return;
    void api.playbackSeek(openedProject.projectHandle, us).then(applyPlaybackStatus).catch(() => undefined);
  };

  return (
    <div className="mt-2 space-y-2">
      <div className="flex flex-wrap items-center gap-2">
        <Button size="sm" variant="secondary" icon={ScanSearch} disabled={!ready || busy} onClick={() => void run(false)}>
          Check the graphics
        </Button>
        <Button
          size="sm"
          variant="secondary"
          icon={MoveHorizontal}
          disabled={!ready || busy}
          title="Stretch each graphic to meet the next, so nothing drops to the background (one undo step)"
          onClick={() => void run(true)}
        >
          Close the gaps
        </Button>
      </div>
      {!ready && <p className="text-meta text-studio-500">Graphics come after the voice: they land on its words.</p>}
      {error && <p className="text-meta text-danger-fg">{error}</p>}
      {visuals && (
        <div className="space-y-1.5 text-meta text-studio-400">
          <p>
            {visuals.beats.length} graphics over {clock(visuals.durationUs)}
          </p>
          {visuals.errors.map((e) => (
            <p key={e} className="flex gap-1 text-danger-fg">
              <AlertTriangle className="w-3 h-3 mt-0.5 shrink-0" aria-hidden /> {e}
            </p>
          ))}
          {visuals.notes.map((n) => (
            <p key={n} className="flex gap-1 text-suggest-fg">
              <AlertTriangle className="w-3 h-3 mt-0.5 shrink-0" aria-hidden /> {n}
            </p>
          ))}
          {visuals.beats.length > 0 && (
            <button type="button" className="text-accent-fg hover:underline" onClick={() => setOpen((o) => !o)}>
              {open ? "Hide the graphics" : "Show the graphics"}
            </button>
          )}
          {open && (
            <ol className="space-y-0.5">
              {visuals.beats.map((b) => (
                <li key={b.clipId} className="flex gap-2">
                  <button type="button" className="tabular-nums text-accent-fg hover:underline" onClick={() => seek(b.startUs)}>
                    {clock(b.startUs)}
                  </button>
                  <span className="text-studio-200 truncate">{b.name}</span>
                  <span className="text-studio-500 truncate">{b.said}</span>
                </li>
              ))}
            </ol>
          )}
        </div>
      )}
    </div>
  );
};
