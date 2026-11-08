import React, { useEffect, useState } from "react";
import { AlertTriangle, CheckCircle2, Mic } from "lucide-react";
import { Button } from "../ui";
import { api } from "../../lib/ipc";
import { listenTo } from "../../lib/windowSync";
import type { FacelessVoice } from "../../lib/types";
import { useProjectStore } from "../../stores/projectStore";

const seconds = (us: number) => `${(us / 1e6).toFixed(1)} s`;

/** The production's narration: make it (one click, the channel's voice on your OpenRouter key),
 * then what the transcription heard differently, to listen to and accept. */
export const VoiceView: React.FC<{ refreshKey: unknown; ready: boolean; onChange: () => void }> = ({
  refreshKey,
  ready,
  onChange,
}) => {
  const [voice, setVoice] = useState<FacelessVoice | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [notes, setNotes] = useState<string[]>([]);

  useEffect(() => {
    let active = true;
    void api
      .facelessVoiceGet()
      .then((v) => active && setVoice(v))
      .catch(() => active && setVoice(null));
    return () => {
      active = false;
    };
  }, [refreshKey]);

  useEffect(() => {
    if (!busy) return;
    return listenTo<{ fraction: number; message: string }>("faceless-voice-progress", (p) =>
      setBusy(`${p.message} (${Math.round(p.fraction * 100)}%)`),
    );
  }, [busy !== null]); // eslint-disable-line react-hooks/exhaustive-deps

  const seek = (us: number) => {
    const { openedProject, applyPlaybackStatus } = useProjectStore.getState();
    if (!openedProject) return;
    void api.playbackSeek(openedProject.projectHandle, us).then(applyPlaybackStatus).catch(() => undefined);
  };

  const run = async (work: () => Promise<{ voice: FacelessVoice; diagnostics: string[]; opened?: unknown }>) => {
    setError(null);
    setBusy("Starting");
    try {
      const result = await work();
      setVoice(result.voice);
      setNotes(result.diagnostics);
      const store = useProjectStore.getState();
      if (result.opened) await store.applyOpenedProject(result.opened as never);
      store.bumpCaptions();
      onChange();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(null);
    }
  };

  return (
    <div className="mt-2 space-y-2">
      <div className="flex items-center gap-2">
        <Button
          size="sm"
          variant={voice ? "secondary" : "primary"}
          icon={Mic}
          disabled={!ready || busy !== null}
          title={ready ? undefined : "Voice comes after the review"}
          onClick={() => void run(() => api.facelessVoice())}
        >
          {voice ? "Voice again" : "Voice the script"}
        </Button>
        {busy && <span className="text-meta text-studio-400 truncate">{busy}</span>}
      </div>
      {!voice && !busy && (
        <p className="text-meta text-studio-500">
          Will (Eleven v4 Turbo) through OpenRouter, sped up 1.2×. Only scenes whose words changed are paid for again.
        </p>
      )}
      {error && <p className="text-meta text-danger-fg">{error}</p>}
      {voice && (
        <div className="space-y-1.5 text-meta text-studio-400">
          <p>
            {voice.scenes.length} scenes, {seconds(voice.durationUs)} on the timeline · {voice.heardWords} words heard
            {voice.generated > 0 && ` · ${voice.generated} voiced last time`}
            {voice.costUsd != null && voice.generated > 0 && ` · about $${voice.costUsd.toFixed(2)}`}
          </p>
          {notes.map((n) => (
            <p key={n} className="text-studio-500">
              {n}
            </p>
          ))}
          {voice.lengthProblem && (
            <p className="flex gap-1 text-suggest-fg">
              <AlertTriangle className="w-3 h-3 mt-0.5 shrink-0" aria-hidden /> {voice.lengthProblem}
            </p>
          )}
          {voice.issues.length > 0 && (
            <div>
              <p className="flex items-center gap-1 text-suggest-fg">
                <AlertTriangle className="w-3 h-3" aria-hidden /> Heard differently: listen to these scenes
              </p>
              <ul className="mt-1 space-y-0.5">
                {voice.issues.map((i) => (
                  <li key={`${i.scene}-${i.expected}`} className="flex items-center gap-2">
                    <button
                      type="button"
                      className="text-accent-fg hover:underline"
                      onClick={() => seek(voice.scenes[i.scene - 1]?.startUs ?? 0)}
                    >
                      Scene {i.scene}
                    </button>
                    <span>
                      {i.kind}: <span className="text-studio-200">{i.expected}</span>
                    </span>
                  </li>
                ))}
              </ul>
            </div>
          )}
          {(voice.issues.length > 0 || voice.lengthProblem) &&
            (voice.accepted ? (
              <p className="flex items-center gap-1 text-success-fg">
                <CheckCircle2 className="w-3 h-3" aria-hidden /> You listened and accepted it
              </p>
            ) : (
              <div className="flex items-center gap-2">
                <Button size="sm" variant="secondary" disabled={busy !== null} onClick={() => void run(api.facelessVoiceAccept)}>
                  It sounds right
                </Button>
                <span className="text-studio-500">or fix the script and voice again</span>
              </div>
            ))}
        </div>
      )}
    </div>
  );
};
