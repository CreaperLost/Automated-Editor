import React, { useEffect, useRef, useState } from "react";
import { Trash2, Video } from "lucide-react";
import { InspectorSection, RangeRow } from "./InspectorSection";
import { useProjectStore } from "../../stores/projectStore";
import { api } from "../../lib/ipc";
import { formatTimeUs } from "../../hooks/useTimeline";
import {
  DEFAULT_WEBCAM_FOCUS,
  WebcamFocus,
  WebcamFocusSegment,
  WebcamFocusSettings,
} from "../../lib/types";

/** Settings that change which segments are found, so they need a re-detect. */
const DETECTION_KEYS: (keyof WebcamFocusSettings)[] = [
  "speechThresholdDb",
  "pauseToleranceMs",
  "idleMs",
  "requireSpeech",
  "minFocusMs",
  "cursorMovesAreActivity",
];

function segmentEditedSpan(segment: WebcamFocusSegment) {
  const ranges = segment.editedRanges ?? [];
  if (ranges.length === 0) return null;
  return {
    startUs: ranges[0].startUs,
    durationUs: ranges.reduce((sum, r) => sum + (r.endUs - r.startUs), 0),
  };
}

/// Auto webcam layout: the webcam fills the frame once the mouse has rested for a while.
export const WebcamFocusSection: React.FC<{ webcamShown: boolean }> = ({ webcamShown }) => {
  const openedProject = useProjectStore((s) => s.openedProject);
  const applyOpenedProject = useProjectStore((s) => s.applyOpenedProject);
  const applyPlaybackStatus = useProjectStore((s) => s.applyPlaybackStatus);
  const openedRef = useRef(openedProject);
  openedRef.current = openedProject;
  const saved = openedProject?.webcamFocus ?? DEFAULT_WEBCAM_FOCUS;
  const [draft, setDraft] = useState<WebcamFocusSettings>(saved.settings);
  const [stale, setStale] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string>();
  const [notes, setNotes] = useState<string[]>([]);
  const persistTimer = useRef<number>();

  // Follow the document (undo, redo, another edit) unless a slider change is still pending.
  const savedKey = JSON.stringify(saved.settings);
  useEffect(() => {
    if (persistTimer.current === undefined) setDraft(saved.settings);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [savedKey]);
  useEffect(() => {
    setStale(false);
    setNotes([]);
    setError(undefined);
  }, [openedProject?.projectHandle]);
  useEffect(() => () => window.clearTimeout(persistTimer.current), []);

  const update = (next: (current: WebcamFocus) => WebcamFocus) => {
    const opened = openedRef.current;
    if (!opened) return Promise.resolve();
    const current = opened.webcamFocus ?? DEFAULT_WEBCAM_FOCUS;
    return api
      .projectWebcamFocusUpdate(opened.projectHandle, opened.revision, next(current))
      .then((updated) => {
        applyOpenedProject(updated);
        setError(undefined);
      })
      .catch((err) => setError(String(err)));
  };

  const detect = (settings: WebcamFocusSettings) => {
    const opened = openedRef.current;
    if (!opened || busy) return;
    window.clearTimeout(persistTimer.current);
    persistTimer.current = undefined;
    setBusy(true);
    setError(undefined);
    void api
      .projectWebcamFocusDetect(opened.projectHandle, opened.revision, settings)
      .then((result) => {
        applyOpenedProject(result.project);
        setStale(false);
        setNotes([
          result.detected === 0
            ? "No long enough idle-mouse stretches were found."
            : `Found ${result.detected} idle-mouse stretch${result.detected === 1 ? "" : "es"}.`,
          ...result.diagnostics,
        ]);
      })
      .catch((err) => setError(String(err)))
      .finally(() => setBusy(false));
  };

  const setSetting = (patch: Partial<WebcamFocusSettings>) => {
    const next = { ...draft, ...patch };
    setDraft(next);
    if (DETECTION_KEYS.some((key) => key in patch)) setStale(true);
    window.clearTimeout(persistTimer.current);
    persistTimer.current = window.setTimeout(() => {
      persistTimer.current = undefined;
      void update((current) => ({ ...current, settings: next }));
    }, 250);
  };

  const toggleEnabled = (enabled: boolean) => {
    if (enabled && saved.segments.length === 0) {
      detect(draft);
      return;
    }
    void update((current) => ({ ...current, enabled }));
  };

  const patchSegment = (id: string, patch: Partial<WebcamFocusSegment> | null) =>
    update((current) => ({
      ...current,
      segments: current.segments.flatMap((segment) =>
        segment.id !== id ? [segment] : patch === null ? [] : [{ ...segment, ...patch }],
      ),
    }));

  const seek = (timeUs: number) => {
    const opened = openedRef.current;
    if (!opened) return;
    void api
      .playbackSeek(opened.projectHandle, timeUs)
      .then(applyPlaybackStatus)
      .catch(() => {});
  };

  const visible = saved.segments
    .map((segment) => ({ segment, span: segmentEditedSpan(segment) }))
    .filter((item) => item.span !== null);
  const onCount = visible.filter((item) => item.segment.enabled).length;
  const onSeconds = visible
    .filter((item) => item.segment.enabled)
    .reduce((sum, item) => sum + (item.span?.durationUs ?? 0), 0) / 1e6;

  return (
    <InspectorSection
      id="webcam-focus"
      title="Auto Webcam"
      icon={Video}
      extra={
        <input
          type="checkbox"
          aria-label="Auto webcam layout"
          title="Auto webcam layout"
          disabled={!openedProject || busy}
          checked={saved.enabled}
          onChange={(e) => toggleEnabled(e.target.checked)}
          className="rounded bg-studio-800 border-studio-700 text-indigo-600 focus:ring-0 cursor-pointer disabled:opacity-40"
        />
      }
    >
      <p className="text-[11px] text-studio-400">
        The webcam fills the frame once the mouse has rested for the time below, and goes back
        to its bubble just before you click, scroll or move the mouse. Select clips on the
        timeline and use Normal view to keep the bubble there.
      </p>
      {!openedProject && <p className="text-[11px] text-studio-500">Open a project to use this.</p>}
      {openedProject && !webcamShown && (
        <p className="text-[11px] text-amber-300">Turn on the webcam above to see this layout.</p>
      )}
      {error && (
        <p role="alert" className="text-[11px] text-rose-300 bg-rose-950/40 border border-rose-900/40 rounded p-2">
          {error}
        </p>
      )}

      <div className="flex items-center justify-between gap-2">
        <span className="text-[11px] text-studio-300">
          {saved.segments.length === 0
            ? "Not detected yet"
            : `${onCount} of ${visible.length} on · ${onSeconds.toFixed(1)}s`}
        </span>
        <button
          type="button"
          disabled={!openedProject || busy}
          onClick={() => detect(draft)}
          className={`px-2.5 py-1 rounded-md text-xs font-medium border transition-colors disabled:opacity-40 ${
            stale
              ? "bg-amber-500/20 border-amber-400/50 text-amber-200 hover:bg-amber-500/30"
              : "bg-indigo-600/20 border-indigo-500/30 text-indigo-300 hover:bg-indigo-600/30"
          }`}
          title="Find stretches where the mouse rests. Segments you added or switched off are kept."
        >
          {busy ? "Detecting…" : saved.segments.length === 0 ? "Detect" : "Re-detect"}
        </button>
      </div>
      {stale && !busy && (
        <p className="text-[10px] text-amber-300">Detection settings changed. Re-detect to apply them.</p>
      )}
      {notes.map((note) => (
        <p key={note} className="text-[10px] text-studio-500">
          {note}
        </p>
      ))}

      <div className="space-y-3">
        <label className="text-xs text-studio-400">Look</label>
        <RangeRow
          label="Full-frame webcam size"
          value={draft.focusSizePct}
          min={40}
          max={100}
          unit="%"
          onChange={(focusSizePct) => setSetting({ focusSizePct })}
        />
        <RangeRow
          label={draft.transitionMs === 0 ? "Transition (instant)" : "Transition"}
          value={draft.transitionMs}
          min={0}
          max={2000}
          step={50}
          unit="ms"
          onChange={(transitionMs) => setSetting({ transitionMs })}
        />
      </div>

      <div className="space-y-3">
        <label className="text-xs text-studio-400">Detection</label>
        <RangeRow
          label="Mouse idle before full frame"
          value={draft.idleMs / 1000}
          min={0}
          max={60}
          step={0.5}
          unit="s"
          onChange={(seconds) => setSetting({ idleMs: Math.round(seconds * 1000) })}
        />
        <label className="flex items-center justify-between text-xs text-studio-400">
          <span>Only while I'm talking</span>
          <input
            type="checkbox"
            checked={draft.requireSpeech}
            onChange={(e) => setSetting({ requireSpeech: e.target.checked })}
            className="rounded bg-studio-800 border-studio-700 text-indigo-600 focus:ring-0 cursor-pointer"
          />
        </label>
        {draft.requireSpeech && (
          <>
            <RangeRow
              label="Speech level"
              value={draft.speechThresholdDb}
              min={-70}
              max={-10}
              unit=" dB"
              onChange={(speechThresholdDb) => setSetting({ speechThresholdDb })}
            />
            <RangeRow
              label="Allowed pause in speech"
              value={draft.pauseToleranceMs}
              min={100}
              max={3000}
              step={50}
              unit="ms"
              onChange={(pauseToleranceMs) => setSetting({ pauseToleranceMs })}
            />
          </>
        )}
        <RangeRow
          label="Shortest full-frame stretch"
          value={draft.minFocusMs}
          min={500}
          max={15000}
          step={250}
          unit="ms"
          onChange={(minFocusMs) => setSetting({ minFocusMs })}
        />
        <label className="flex items-center justify-between text-xs text-studio-400">
          <span>Mouse movement counts as activity</span>
          <input
            type="checkbox"
            checked={draft.cursorMovesAreActivity}
            onChange={(e) => setSetting({ cursorMovesAreActivity: e.target.checked })}
            className="rounded bg-studio-800 border-studio-700 text-indigo-600 focus:ring-0 cursor-pointer"
          />
        </label>
      </div>

      {visible.length > 0 && (
        <div className="space-y-1.5">
          <label className="text-xs text-studio-400">Segments</label>
          <p className="text-[10px] text-studio-500">
            Untick a segment to keep the bubble there. Select a range on the timeline and use
            Cam Focus to add your own.
          </p>
          <div className="max-h-48 overflow-y-auto space-y-1 pr-1">
            {visible.map(({ segment, span }) => (
              <div
                key={segment.id}
                className={`flex items-center gap-2 rounded px-2 py-1 text-[11px] border ${
                  segment.enabled
                    ? "bg-amber-500/10 border-amber-400/30 text-studio-200"
                    : "bg-studio-850 border-studio-800 text-studio-500"
                }`}
              >
                <input
                  type="checkbox"
                  aria-label="Use this segment"
                  checked={segment.enabled}
                  onChange={(e) => void patchSegment(segment.id, { enabled: e.target.checked })}
                  className="rounded bg-studio-800 border-studio-700 text-indigo-600 focus:ring-0 cursor-pointer"
                />
                <button
                  type="button"
                  className="flex-1 text-left font-mono hover:text-white"
                  onClick={() => span && seek(span.startUs)}
                  title="Jump to this segment"
                >
                  {formatTimeUs(span!.startUs)} · {(span!.durationUs / 1e6).toFixed(1)}s
                </button>
                <span className="text-[9px] uppercase text-studio-500">{segment.source}</span>
                <button
                  type="button"
                  aria-label="Delete segment"
                  onClick={() => void patchSegment(segment.id, null)}
                  className="text-studio-500 hover:text-rose-300"
                >
                  <Trash2 className="w-3 h-3" />
                </button>
              </div>
            ))}
          </div>
        </div>
      )}
    </InspectorSection>
  );
};
