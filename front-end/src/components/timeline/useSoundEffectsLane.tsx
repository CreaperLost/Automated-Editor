import { useEffect, useState } from "react";
import { Sparkles, Volume2, VolumeX } from "lucide-react";
import { api } from "../../lib/ipc";
import { DEFAULT_AUDIO_SETTINGS, DEFAULT_SOUND_EFFECTS, type SoundEffect } from "../../lib/types";
import { useProjectStore } from "../../stores/projectStore";
import { HDR_BUTTON, HeaderRow, MIN_LANE_PX, type TimelineView } from "./timelineShared";

/**
 * The sound effects track: where the soft clicks and the whooshes play, and its mute. The
 * volume and which sounds play are on the Audio page.
 */
export function useSoundEffectsLane({ view, onError }: { view: TimelineView; onError: (message?: string) => void }) {
  const openedProject = useProjectStore((s) => s.openedProject);
  const applyOpenedProject = useProjectStore((s) => s.applyOpenedProject);
  const [effects, setEffects] = useState<SoundEffect[]>([]);
  const settings = openedProject?.audio?.soundEffects ?? DEFAULT_SOUND_EFFECTS;
  const hasRecording = !!openedProject?.assets.some((a) => a.kind === "recording");

  useEffect(() => {
    if (!openedProject || !hasRecording) return;
    let active = true;
    void api
      .projectSoundEffects(openedProject.projectHandle)
      .then((next) => active && setEffects(next))
      .catch(() => active && setEffects([]));
    return () => {
      active = false;
    };
    // The sounds follow the cuts, the zooms, the layout and the lane's own switches.
  }, [
    openedProject?.projectHandle,
    openedProject?.sequence,
    openedProject?.zooms,
    JSON.stringify(openedProject?.layouts ?? null),
    JSON.stringify(settings),
    hasRecording,
  ]);

  if (!openedProject || !hasRecording) return { header: null, lane: null };

  const toggleMute = () => {
    void api
      .projectAudioUpdate(openedProject.projectHandle, openedProject.revision, {
        ...DEFAULT_AUDIO_SETTINGS,
        ...openedProject.audio,
        soundEffects: { ...settings, muted: !settings.muted },
      })
      .then((updated) => {
        applyOpenedProject(updated);
        onError(undefined);
      })
      .catch((err) => onError(String(err)));
  };
  const clicks = effects.filter((e) => e.kind === "click").length;
  const whooshes = effects.length - clicks;
  const silent = settings.muted || (!settings.clicks && !settings.whooshes);
  const height = MIN_LANE_PX;

  const header = (
    <HeaderRow
      key="sound-effects"
      height={height}
      tone="audio"
      icon={Sparkles}
      name="Sound effects"
      dim={silent}
      meta={silent ? "Muted" : `${clicks} click${clicks === 1 ? "" : "s"} · ${whooshes} whoosh${whooshes === 1 ? "" : "es"}`}
      title="Soft clicks on your mouse clicks and a light whoosh on zooms and layout switches, lowered under speech. Volume and which sounds play are on the Audio page."
      actions={
        <button
          type="button"
          className={HDR_BUTTON}
          onClick={toggleMute}
          aria-pressed={settings.muted}
          aria-label={settings.muted ? "Unmute sound effects" : "Mute sound effects"}
          title={settings.muted ? "Unmute sound effects" : "Mute sound effects"}
        >
          {settings.muted ? <VolumeX className="w-4 h-4 text-danger-fg" /> : <Volume2 className="w-4 h-4" />}
        </button>
      }
    />
  );

  const lane = (
    <div
      key="sound-effects"
      data-track-row="sound-effects"
      className={`relative rounded-md bg-studio-850/30 ${silent ? "opacity-40" : ""}`}
      style={{ height }}
    >
      {view.durationUs > 0 &&
        !silent &&
        effects.map((effect, index) =>
          effect.kind === "click" ? (
            <div
              key={index}
              className="absolute top-2 bottom-2 w-px bg-audio/80 pointer-events-none"
              style={{ left: view.pct(effect.atUs) }}
            />
          ) : (
            <div
              key={index}
              className="absolute top-1.5 bottom-1.5 rounded-full bg-audio/30 border border-audio/50 pointer-events-none"
              style={{ left: view.pct(effect.atUs), width: view.pct(480_000) }}
            />
          ),
        )}
    </div>
  );

  return { header, lane };
}
