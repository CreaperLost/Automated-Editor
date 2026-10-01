import React, { useEffect, useRef, useState } from "react";
import {
  Play,
  Pause,
  SkipBack,
  ZoomIn,
  ZoomOut,
  Scissors,
  Eye,
  Volume2,
  VolumeX,
  Plus,
  Check,
  X,
  Trash2,
  RotateCcw,
  Video,
} from "lucide-react";
import { useProjectStore } from "../../stores/projectStore";
import { useTimeline } from "../../hooks/useTimeline";
import { WaveformRenderer } from "../waveform/WaveformRenderer";
import { api } from "../../lib/ipc";
import { buildClips, buildCutMarkers, editedToSourceUs } from "../../lib/projectUtils";
import type { OpenedProject, ProjectZoom, ZoomKeyframe } from "../../lib/types";

type DragMode = "move" | "start" | "end";

export const TimelineStudio: React.FC = () => {
  const {
    openedProject,
    tracks,
    zoomKeyframes,
    pendingZoomSuggestions,
    zoomDiagnostics,
    currentTimeUs,
    durationUs,
    setIsSilenceModalOpen,
    setTrackWaveform,
    applyOpenedProject,
    applyZoomGeneration,
  } = useProjectStore();

  const { isPlaying, togglePlayPause, seekToUs, formattedTime, formattedDuration } = useTimeline();

  const [rangeStart, setRangeStart] = useState("0");
  const [rangeEnd, setRangeEnd] = useState("0");
  const [editError, setEditError] = useState<string>();
  const [editing, setEditing] = useState(false);
  const [selectedZoomId, setSelectedZoomId] = useState<string>();
  const [zoomBusy, setZoomBusy] = useState(false);
  const dragging = useRef<{
    mode: DragMode;
    zoomId: string;
    startX: number;
    originStart: number;
    originEnd: number;
  } | null>(null);
  const suppressSeek = useRef(false);
  useEffect(() => { setRangeStart("0"); setRangeEnd(String(durationUs / 1e6)); setEditError(undefined); }, [openedProject?.projectHandle, durationUs]);
  const runEdit = async (work: (project: OpenedProject) => Promise<OpenedProject>) => {
    if (!openedProject || editing) return;
    setEditing(true); setEditError(undefined);
    try { applyOpenedProject(await work(openedProject)); }
    catch (err) { setEditError(String(err)); }
    finally { setEditing(false); }
  };
  const editRange = async (trim: boolean) => {
    if (!openedProject || editing) return;
    const startUs = Math.round(Number(rangeStart) * 1e6);
    const endUs = Math.round(Number(rangeEnd) * 1e6);
    if (!Number.isSafeInteger(startUs) || !Number.isSafeInteger(endUs) || startUs < 0 || startUs >= endUs || endUs > durationUs) {
      setEditError("Choose a start and end within the timeline, with start before end."); return;
    }
    const cuts = trim ? [
      ...(startUs > 0 ? [{ startUs: 0, endUs: startUs }] : []),
      ...(endUs < durationUs ? [{ startUs: endUs, endUs: durationUs }] : []),
    ] : [{startUs, endUs}];
    if (!cuts.length) return;
    await runEdit((project) => api.projectRippleCuts(project.projectHandle, project.revision, cuts));
  };
  const splitAtPlayhead = () =>
    runEdit((project) => api.projectSplit(project.projectHandle, project.revision, currentTimeUs));
  const restoreCut = (startUs: number, endUs: number) =>
    runEdit((project) => api.projectRestoreCuts(project.projectHandle, project.revision, [{ startUs, endUs }]));

  // The range fields double as the timeline selection; the full range means nothing is selected.
  const selectedStartUs = Math.round(Number(rangeStart) * 1e6);
  const selectedEndUs = Math.round(Number(rangeEnd) * 1e6);
  const selection =
    Number.isSafeInteger(selectedStartUs) &&
    Number.isSafeInteger(selectedEndUs) &&
    selectedStartUs >= 0 &&
    selectedStartUs < selectedEndUs &&
    selectedEndUs <= durationUs &&
    !(selectedStartUs === 0 && selectedEndUs === durationUs)
      ? { startUs: selectedStartUs, endUs: selectedEndUs }
      : null;
  const selectRange = (startUs: number, endUs: number) => {
    setRangeStart(String(startUs / 1e6));
    setRangeEnd(String(endUs / 1e6));
  };
  const clearSelection = () => selectRange(0, durationUs);

  const retained = openedProject?.retainedIntervals ?? [];
  const clips = buildClips(retained, openedProject?.splitPointsUs);
  const cutMarkers = buildCutMarkers(retained, openedProject?.removedIntervals);

  const shortcuts = useRef({ splitAtPlayhead, deleteSelection: () => {}, clearSelection });
  shortcuts.current = {
    splitAtPlayhead,
    deleteSelection: () => { if (selection) void editRange(false); },
    clearSelection,
  };
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement | null;
      if (
        event.ctrlKey || event.metaKey || event.altKey ||
        target?.closest("input, textarea, select, [contenteditable='true']")
      ) return;
      if (event.key === "s" || event.key === "S") {
        event.preventDefault();
        void shortcuts.current.splitAtPlayhead();
      } else if (event.key === "Delete" || event.key === "Backspace") {
        event.preventDefault();
        shortcuts.current.deleteSelection();
      } else if (event.key === "Escape") {
        shortcuts.current.clearSelection();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);

  const timelineTrackRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    if (!openedProject || durationUs <= 0) return;
    let active = true;
    const audioTracks = openedProject.tracks.filter(
      (track) =>
        track.descriptor.trackType === "mic_audio" || track.descriptor.trackType === "system_audio",
    );
    for (const track of audioTracks) {
      void api
        .projectWaveform(openedProject.projectHandle, track.descriptor.id, 0, durationUs, 256)
        .then((page) => {
          if (active && !page.cancelled) {
            setTrackWaveform(track.descriptor.id, page);
          }
        })
        .catch((err) => {
          console.warn("[Timeline] Waveform query failed:", err);
          if (active) {
            setTrackWaveform(track.descriptor.id, {
              trackId: track.descriptor.id,
              startUs: 0,
              endUs: durationUs,
              sampleRate: 0,
              channels: 0,
              channelPolicy: "max_energy",
              buckets: [],
              diagnostics: [String(err)],
              cancelled: false,
            });
          }
        });
    }
    return () => {
      active = false;
    };
  }, [openedProject?.projectHandle, openedProject?.revision, durationUs, setTrackWaveform]);

  useEffect(() => {
    if (!openedProject) return;
    let active = true;
    void api
      .projectZoomSuggestions(openedProject.projectHandle)
      .then((generation) => {
        if (active) applyZoomGeneration(generation);
      })
      .catch((err) => {
        console.warn("[Timeline] Zoom suggestions failed:", err);
        if (active) applyZoomGeneration({ suggestions: [], diagnostics: [String(err)] });
      });
    return () => {
      active = false;
    };
  }, [openedProject?.projectHandle, openedProject?.revision, applyZoomGeneration]);

  useEffect(() => {
    if (selectedZoomId && !zoomKeyframes.some((bar) => bar.zoomId === selectedZoomId)) {
      setSelectedZoomId(undefined);
    }
  }, [zoomKeyframes, selectedZoomId]);

  const persistZoom = async (work: () => Promise<OpenedProject>) => {
    if (!openedProject || zoomBusy) return;
    setZoomBusy(true);
    setEditError(undefined);
    try {
      applyOpenedProject(await work());
    } catch (err) {
      setEditError(String(err));
    } finally {
      setZoomBusy(false);
    }
  };

  const selectedBar = zoomKeyframes.find((bar) => bar.zoomId === selectedZoomId);
  const persistedSelected = openedProject?.zooms?.find((zoom) => zoom.id === selectedZoomId);

  const patchPersisted = (zoom: ProjectZoom, sourceStartUs: number, sourceEndUs: number) => {
    if (!openedProject) return;
    const duration = sourceEndUs - sourceStartUs;
    if (duration < 3) {
      setEditError("Zoom range is too short.");
      return;
    }
    const transitionUs = Math.min(Math.max(1, Math.floor(duration / 5)), 400_000, duration - 1);
    void persistZoom(() =>
      api.projectZoomUpdate(openedProject.projectHandle, openedProject.revision, {
        ...zoom,
        sourceStartUs,
        sourceEndUs,
        transitionUs,
      }),
    );
  };

  const rangeDrag = useRef<{ startX: number; active: boolean } | null>(null);
  const clientXToUs = (clientX: number) => {
    const rect = timelineTrackRef.current?.getBoundingClientRect();
    if (!rect || rect.width <= 0) return 0;
    return Math.round(Math.max(0, Math.min(1, (clientX - rect.left) / rect.width)) * durationUs);
  };
  const onTrackPointerDown = (event: React.PointerEvent<HTMLDivElement>) => {
    if (event.button !== 0 || !openedProject || durationUs <= 0) return;
    rangeDrag.current = { startX: event.clientX, active: false };
  };
  const onTrackPointerMove = (event: React.PointerEvent<HTMLDivElement>) => {
    const drag = rangeDrag.current;
    if (!drag) return;
    if (!drag.active) {
      if (Math.abs(event.clientX - drag.startX) < 4) return;
      // Capture only once it is a drag, so a plain click still reaches the clip under it.
      drag.active = true;
      event.currentTarget.setPointerCapture(event.pointerId);
    }
    const a = clientXToUs(drag.startX);
    const b = clientXToUs(event.clientX);
    if (a !== b) selectRange(Math.min(a, b), Math.max(a, b));
  };
  const onTrackPointerUp = () => {
    if (rangeDrag.current?.active) suppressSeek.current = true;
    rangeDrag.current = null;
  };

  const handleTimelineClick = (e: React.MouseEvent<HTMLDivElement>) => {
    if (suppressSeek.current) {
      suppressSeek.current = false;
      return;
    }
    if (!timelineTrackRef.current) return;
    const rect = timelineTrackRef.current.getBoundingClientRect();
    const clickX = e.clientX - rect.left;
    const progress = Math.max(0, Math.min(1, clickX / rect.width));
    seekToUs(progress * durationUs);
  };

  const beginDrag = (event: React.PointerEvent, bar: ZoomKeyframe, mode: DragMode) => {
    if (bar.pending || !openedProject || zoomBusy) return;
    event.preventDefault();
    event.stopPropagation();
    setSelectedZoomId(bar.zoomId);
    dragging.current = {
      mode,
      zoomId: bar.zoomId,
      startX: event.clientX,
      originStart: bar.sourceStartUs,
      originEnd: bar.sourceEndUs,
    };
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
  };

  const onBarPointerMove = (event: React.PointerEvent) => {
    const drag = dragging.current;
    if (!drag || !timelineTrackRef.current || durationUs <= 0) return;
    event.stopPropagation();
    suppressSeek.current = true;
  };

  const endDrag = (event: React.PointerEvent, bar: ZoomKeyframe) => {
    const drag = dragging.current;
    dragging.current = null;
    if (!drag || drag.zoomId !== bar.zoomId || !openedProject || !timelineTrackRef.current) return;
    event.stopPropagation();
    const rect = timelineTrackRef.current.getBoundingClientRect();
    if (rect.width <= 0) return;
    const deltaUs = ((event.clientX - drag.startX) / rect.width) * durationUs;
    if (Math.abs(deltaUs) < 1_000) return;
    const retained = openedProject.retainedIntervals;
    let sourceStart = drag.originStart;
    let sourceEnd = drag.originEnd;
    if (drag.mode === "move") {
      const editedStart = bar.tUs + deltaUs;
      const mapped = editedToSourceUs(retained, Math.max(0, Math.min(durationUs - 1, editedStart)));
      if (mapped == null) return;
      const duration = drag.originEnd - drag.originStart;
      sourceStart = mapped;
      sourceEnd = mapped + duration;
    } else if (drag.mode === "start") {
      const mapped = editedToSourceUs(retained, Math.max(0, Math.min(bar.endUs - 1, bar.tUs + deltaUs)));
      if (mapped == null) return;
      sourceStart = mapped;
    } else {
      const mapped = editedToSourceUs(
        retained,
        Math.max(bar.tUs + 1, Math.min(durationUs, bar.endUs + deltaUs)),
      );
      if (mapped == null) return;
      sourceEnd = mapped + 1;
    }
    if (sourceEnd <= sourceStart + 2) return;
    const zoom = openedProject.zooms?.find((item) => item.id === bar.zoomId);
    if (!zoom) return;
    patchPersisted(zoom, sourceStart, sourceEnd);
  };

  const progress = durationUs > 0 ? currentTimeUs / durationUs : 0;
  const pendingCount = pendingZoomSuggestions.length;
  const persistedCount = openedProject?.zooms?.length ?? 0;

  return (
    <div className="flex flex-col h-full bg-studio-900 border-t border-studio-800 select-none">
      {/* Timeline Toolbar */}
      <div className="h-12 px-6 flex items-center justify-between border-b border-studio-800 bg-studio-850">
        {/* Playback Controls & Timecode */}
        <div className="flex items-center space-x-4">
          <button
            disabled={!openedProject}
            onClick={() => seekToUs(0)}
            className="p-1.5 rounded-md hover:bg-studio-700 text-studio-300 transition-colors"
            title="Jump to Start"
          >
            <SkipBack className="w-4 h-4" />
          </button>

          <button
            disabled={!openedProject}
            onClick={togglePlayPause}
            className="p-2 rounded-lg bg-indigo-600 hover:bg-indigo-500 text-white transition-colors"
            title={isPlaying ? "Pause (Space)" : "Play (Space)"}
          >
            {isPlaying ? (
              <Pause className="w-4 h-4 fill-white" />
            ) : (
              <Play className="w-4 h-4 fill-white ml-0.5" />
            )}
          </button>

          <div className="font-mono text-sm tracking-wider text-studio-200">
            <span className="text-white font-semibold">{formattedTime}</span>
            <span className="text-studio-500 mx-1.5">/</span>
            <span className="text-studio-400">{formattedDuration}</span>
          </div>
        </div>

        {/* Action Tools: Silence Detection & Zoom Keyframe */}
        <div className="flex items-center space-x-3">
          <button
            disabled={!openedProject || editing || durationUs === 0}
            onClick={() => void splitAtPlayhead()}
            className="flex items-center space-x-1 px-2 py-1.5 rounded-md text-xs text-studio-200 hover:bg-studio-700 disabled:opacity-40"
            title="Split the clip at the playhead (S)"
          >
            <Scissors className="w-3.5 h-3.5" />
            <span>Split</span>
          </button>
          <button
            disabled={!openedProject?.undoAvailable}
            onClick={() => {
              if (!openedProject) return;
              void api
                .projectUndo(openedProject.projectHandle, openedProject.revision)
                .then(applyOpenedProject)
                .catch((err) => console.warn("[Timeline] undo failed:", err));
            }}
            className="px-2 py-1.5 rounded-md text-xs text-studio-300 hover:bg-studio-700 disabled:opacity-40"
            title="Undo edit"
          >
            Undo
          </button>
          <button
            disabled={!openedProject?.redoAvailable}
            onClick={() => {
              if (!openedProject) return;
              void api
                .projectRedo(openedProject.projectHandle, openedProject.revision)
                .then(applyOpenedProject)
                .catch((err) => console.warn("[Timeline] redo failed:", err));
            }}
            className="px-2 py-1.5 rounded-md text-xs text-studio-300 hover:bg-studio-700 disabled:opacity-40"
            title="Redo edit"
          >
            Redo
          </button>

          <button
            disabled={!openedProject}
            onClick={() => setIsSilenceModalOpen(true)}
            className="flex items-center space-x-1.5 px-3 py-1.5 rounded-md bg-emerald-600/20 hover:bg-emerald-600/30 border border-emerald-500/30 text-emerald-300 text-xs font-medium transition-colors"
            title="Detect and ripple-delete silence"
          >
            <Scissors className="w-3.5 h-3.5" />
            <span>AI Jump Cuts</span>
          </button>

          <button
            disabled={!openedProject || zoomBusy || durationUs < 3}
            onClick={() => {
              if (!openedProject) return;
              const selectedStart = Math.round(Number(rangeStart) * 1e6);
              const selectedEnd = Math.round(Number(rangeEnd) * 1e6);
              const useSelection =
                Number.isSafeInteger(selectedStart) &&
                Number.isSafeInteger(selectedEnd) &&
                selectedEnd - selectedStart >= 3 &&
                selectedEnd <= durationUs;
              const startUs = useSelection
                ? selectedStart
                : Math.max(0, currentTimeUs - 600_000);
              const endUs = useSelection
                ? selectedEnd
                : Math.min(durationUs, Math.max(startUs + 2_000_000, currentTimeUs + 1_400_000));
              void persistZoom(() =>
                api.projectZoomAdd(openedProject.projectHandle, openedProject.revision, {
                  editedStartUs: startUs,
                  editedEndUs: endUs,
                  centerX: 0.5,
                  centerY: 0.5,
                  scale: 1.8,
                }),
              );
            }}
            className="flex items-center space-x-1.5 px-3 py-1.5 rounded-md bg-indigo-600/20 hover:bg-indigo-600/30 border border-indigo-500/30 text-indigo-300 text-xs font-medium transition-colors disabled:opacity-40"
            title="Add a manual zoom on the selection, or around the playhead"
          >
            <Plus className="w-3.5 h-3.5" />
            <span>Add Zoom</span>
          </button>

          <button
            disabled={!openedProject || editing || !selection}
            onClick={() => {
              if (!selection) return;
              void runEdit((project) =>
                api.projectWebcamFocusAdd(project.projectHandle, project.revision, selection.startUs, selection.endUs),
              );
            }}
            className="flex items-center space-x-1.5 px-3 py-1.5 rounded-md bg-amber-500/15 hover:bg-amber-500/25 border border-amber-400/30 text-amber-200 text-xs font-medium transition-colors disabled:opacity-40"
            title="Make the webcam fill the frame over the selection (turns Auto Webcam on)"
          >
            <Video className="w-3.5 h-3.5" />
            <span>Cam Focus</span>
          </button>
          {pendingCount > 0 && (
            <>
              <button
                disabled={zoomBusy}
                onClick={() => {
                  if (!openedProject) return;
                  void persistZoom(() =>
                    api.projectZoomAccept(
                      openedProject.projectHandle,
                      openedProject.revision,
                      pendingZoomSuggestions.map((item) => item.id),
                    ),
                  );
                }}
                className="flex items-center space-x-1 px-2 py-1.5 rounded-md text-xs text-indigo-200 hover:bg-indigo-600/20 disabled:opacity-40"
                title="Accept all pending auto-zooms"
              >
                <Check className="w-3.5 h-3.5" />
                <span>Accept {pendingCount}</span>
              </button>
              {selectedBar?.pending && (
                <>
                  <button
                    disabled={zoomBusy}
                    onClick={() => {
                      if (!openedProject || !selectedZoomId) return;
                      void persistZoom(() =>
                        api.projectZoomAccept(openedProject.projectHandle, openedProject.revision, [
                          selectedZoomId,
                        ]),
                      );
                    }}
                    className="px-2 py-1.5 rounded-md text-xs text-indigo-200 hover:bg-indigo-600/20 disabled:opacity-40"
                  >
                    Accept selected
                  </button>
                  <button
                    disabled={zoomBusy}
                    onClick={() => {
                      if (!openedProject || !selectedZoomId) return;
                      void persistZoom(() =>
                        api.projectZoomDismiss(openedProject.projectHandle, openedProject.revision, [
                          selectedZoomId,
                        ]),
                      );
                    }}
                    className="flex items-center space-x-1 px-2 py-1.5 rounded-md text-xs text-studio-300 hover:bg-studio-700 disabled:opacity-40"
                  >
                    <X className="w-3.5 h-3.5" />
                    <span>Dismiss</span>
                  </button>
                </>
              )}
            </>
          )}
          {persistedSelected && (
            <>
              <label className="flex items-center space-x-1 text-[11px] text-studio-300">
                <span>Scale</span>
                <input
                  aria-label="Zoom scale"
                  type="number"
                  min={1}
                  max={8}
                  step={0.1}
                  value={persistedSelected.scale}
                  disabled={zoomBusy}
                  onChange={(event) => {
                    const scale = Number(event.target.value);
                    if (!Number.isFinite(scale) || scale < 1 || scale > 8 || !openedProject) return;
                    void persistZoom(() =>
                      api.projectZoomUpdate(openedProject.projectHandle, openedProject.revision, {
                        ...persistedSelected,
                        scale,
                      }),
                    );
                  }}
                  className="w-14 bg-studio-950 px-1 py-0.5 rounded"
                />
              </label>
              <button
                disabled={zoomBusy}
                onClick={() => {
                  if (!openedProject || !selectedZoomId) return;
                  void persistZoom(() =>
                    api.projectZoomDelete(openedProject.projectHandle, openedProject.revision, selectedZoomId),
                  );
                }}
                className="flex items-center space-x-1 px-2 py-1.5 rounded-md text-xs text-rose-300 hover:bg-rose-950/40 disabled:opacity-40"
                title="Delete this zoom"
              >
                <Trash2 className="w-3.5 h-3.5" />
                <span>Delete</span>
              </button>
            </>
          )}
          {pendingCount > 0 && (
            <span className="text-[11px] text-indigo-300/80">
              {pendingCount} pending auto-zoom{pendingCount === 1 ? "" : "s"}
            </span>
          )}
          {persistedCount > 0 && (
            <span className="text-[11px] text-studio-400">
              {persistedCount} saved
            </span>
          )}
          {zoomDiagnostics.length > 0 && pendingCount === 0 && persistedCount === 0 && (
            <span className="text-[11px] text-studio-400 truncate max-w-[220px]" title={zoomDiagnostics.join(" · ")}>
              No auto-zoom
            </span>
          )}

          <div className="h-4 w-px bg-studio-700 mx-1" />

          <button disabled className="p-1.5 rounded hover:bg-studio-700 text-studio-400">
            <ZoomOut className="w-4 h-4" />
          </button>
          <button disabled className="p-1.5 rounded hover:bg-studio-700 text-studio-400">
            <ZoomIn className="w-4 h-4" />
          </button>
        </div>
      </div>

      {openedProject && <div className="flex flex-wrap items-center gap-2 px-4 py-2 text-xs border-b border-studio-800">
        <label>Start (s) <input aria-label="Selection start in seconds" type="number" min="0" step="0.001" value={rangeStart} onChange={e => setRangeStart(e.target.value)} className="w-24 bg-studio-950 px-2 py-1 rounded" /></label>
        <button onClick={() => setRangeStart(String(currentTimeUs / 1e6))}>Set start here</button>
        <label>End (s) <input aria-label="Selection end in seconds" type="number" min="0" step="0.001" value={rangeEnd} onChange={e => setRangeEnd(e.target.value)} className="w-24 bg-studio-950 px-2 py-1 rounded" /></label>
        <button onClick={() => setRangeEnd(String(currentTimeUs / 1e6))}>Set end here</button>
        <button disabled={editing || durationUs === 0} onClick={() => void editRange(false)} className="text-rose-300 disabled:opacity-40" title="Cut the selection and close the gap (Delete)">Delete range</button>
        <button disabled={editing || durationUs === 0} onClick={() => void editRange(true)} className="text-teal-300 disabled:opacity-40">Keep range</button>
        {cutMarkers.length > 0 && (
          <button
            disabled={editing}
            onClick={() => void restoreCut(0, openedProject.sourceDurationUs)}
            className="flex items-center gap-1 text-amber-300 disabled:opacity-40"
            title="Put every cut back on the timeline"
          >
            <RotateCcw className="w-3 h-3" />
            Restore all {cutMarkers.length} cut{cutMarkers.length === 1 ? "" : "s"}
          </button>
        )}
        <span className="text-studio-500">Drag on the timeline to select. S splits at the playhead, Delete removes the selection.</span>
        {editError && <span role="alert" className="text-rose-300">{editError}</span>}
      </div>}

      {/* Multi-Track Workspace */}
      <div className="flex-1 flex overflow-hidden">
        {/* Left Track Headers */}
        <div className="w-56 border-r border-studio-800 bg-studio-900 shrink-0 flex flex-col">
          {/* Header spacer aligned with time ruler */}
          <div className="h-7 border-b border-studio-800 px-3 flex items-center text-[10px] font-semibold tracking-wider uppercase text-studio-400">
            Tracks
          </div>

          <div className="flex-1 space-y-2 py-2">
            <div className="h-8 px-3 flex items-end text-[10px] font-semibold tracking-wider uppercase text-studio-400">
              Clips
            </div>
            {tracks.map((track) => (
              <div
                key={track.id}
                className="h-14 px-3 flex items-center justify-between border-b border-studio-800/40 hover:bg-studio-850/50"
              >
                <div className="truncate">
                  <div className="text-xs font-medium text-studio-200 truncate">{track.name}</div>
                  <div className="text-[10px] uppercase font-mono text-studio-400">
                    {track.trackType}
                  </div>
                </div>

                <div className="flex items-center space-x-1">
                  <button disabled className="p-1 rounded text-studio-400 hover:text-white">
                    {track.muted ? <VolumeX className="w-3.5 h-3.5" /> : <Volume2 className="w-3.5 h-3.5" />}
                  </button>
                  <button disabled className="p-1 rounded text-studio-400 hover:text-white">
                    <Eye className="w-3.5 h-3.5" />
                  </button>
                </div>
              </div>
            ))}
          </div>
        </div>

        {/* Right Track Lanes & Playhead */}
        <div className="flex-1 flex flex-col overflow-x-auto relative">
          {/* Time Ruler */}
          <div className="h-7 border-b border-studio-800 bg-studio-850/70 relative">
            <div className="flex items-center h-full px-2 text-[10px] font-mono text-studio-500 justify-between">
              {Array.from({ length: 7 }, (_, i) => <span key={i}>{(durationUs * i / 6 / 1_000_000).toFixed(1)}s</span>)}
            </div>
          </div>

          {/* Interactive Track Area */}
          <div
            ref={timelineTrackRef}
            onClick={handleTimelineClick}
            onPointerDown={onTrackPointerDown}
            onPointerMove={onTrackPointerMove}
            onPointerUp={onTrackPointerUp}
            onPointerCancel={onTrackPointerUp}
            className="flex-1 relative cursor-pointer py-2 space-y-2"
          >
            {/* Playhead Vertical Line */}
            <div
              className="absolute top-0 bottom-0 w-0.5 bg-indigo-500 z-30 pointer-events-none transition-all duration-75"
              style={{ left: `${progress * 100}%` }}
            >
              {/* Playhead Top Scrubber Cap */}
              <div className="w-3 h-3 bg-indigo-500 rounded-sm transform -translate-x-1/2 -top-1 absolute rotate-45 shadow-md shadow-indigo-500/50" />
            </div>

            {/* Zoom Keyframe Track overlay */}
            <div className="h-4 absolute top-0 left-0 right-0 z-20">
              {zoomKeyframes.map((k) => {
                const startProg = durationUs > 0 ? k.tUs / durationUs : 0;
                const widthProg = durationUs > 0 ? Math.max(0, (k.endUs - k.tUs) / durationUs) : 0;
                const selected = k.zoomId === selectedZoomId;
                const label = k.pending
                  ? `Pending auto-zoom ${k.scale}x (${k.origin ?? "click"})`
                  : `${k.source === "manual" ? "Manual" : "Saved"} zoom ${k.scale}x`;
                return (
                  <div
                    key={k.id}
                    className="absolute top-0.5 h-3 rounded-sm"
                    style={{
                      left: `${startProg * 100}%`,
                      width: `${Math.max(widthProg * 100, 0.4)}%`,
                    }}
                    title={label}
                    onClick={(event) => {
                      event.stopPropagation();
                      setSelectedZoomId(k.zoomId);
                    }}
                    onPointerDown={(event) => beginDrag(event, k, "move")}
                    onPointerMove={onBarPointerMove}
                    onPointerUp={(event) => endDrag(event, k)}
                  >
                    <div
                      className={`h-full rounded-sm border ${
                        k.pending
                          ? "bg-indigo-400/20 border-dashed border-indigo-300/80"
                          : "bg-indigo-400/60 border-indigo-200/90"
                      } ${selected ? "ring-1 ring-white/80" : ""}`}
                    />
                    {!k.pending && (
                      <>
                        <button
                          aria-label="Resize zoom start"
                          className="absolute inset-y-0 left-0 w-1.5 cursor-ew-resize"
                          onPointerDown={(event) => beginDrag(event, k, "start")}
                        />
                        <button
                          aria-label="Resize zoom end"
                          className="absolute inset-y-0 right-0 w-1.5 cursor-ew-resize"
                          onPointerDown={(event) => beginDrag(event, k, "end")}
                        />
                      </>
                    )}
                  </div>
                );
              })}
            </div>

            {/* Drag selection */}
            {selection && durationUs > 0 && (
              <div
                className="absolute top-0 bottom-0 bg-white/10 border-x border-white/60 z-10 pointer-events-none"
                style={{
                  left: `${(selection.startUs / durationUs) * 100}%`,
                  width: `${((selection.endUs - selection.startUs) / durationUs) * 100}%`,
                }}
              />
            )}

            {/* Clip lane: edges come from cuts and splits; markers restore cuts */}
            <div className="h-8 relative">
              {durationUs > 0 && clips.map((clip, index) => {
                const selected = selection?.startUs === clip.startUs && selection?.endUs === clip.endUs;
                return (
                  <button
                    key={`${clip.sourceStartUs}-${index}`}
                    className={`absolute top-2.5 bottom-0 rounded border text-[9px] font-mono text-left px-1 truncate ${
                      selected
                        ? "bg-teal-500/40 border-teal-200 text-white"
                        : "bg-teal-500/15 border-teal-400/40 text-teal-200 hover:bg-teal-500/25"
                    }`}
                    style={{
                      left: `${(clip.startUs / durationUs) * 100}%`,
                      width: `${((clip.endUs - clip.startUs) / durationUs) * 100}%`,
                    }}
                    title={`Clip ${index + 1}: ${((clip.endUs - clip.startUs) / 1e6).toFixed(2)}s. Click to select.`}
                    onClick={(event) => {
                      event.stopPropagation();
                      selectRange(clip.startUs, clip.endUs);
                    }}
                  >
                    {index + 1}
                  </button>
                );
              })}
              {durationUs > 0 && cutMarkers.map((marker) => (
                <button
                  key={marker.sourceStartUs}
                  disabled={editing}
                  aria-label="Restore cut"
                  className="absolute top-1 bottom-0 w-3 -translate-x-1/2 z-20 flex justify-center group disabled:opacity-40"
                  style={{ left: `${(marker.editedUs / durationUs) * 100}%` }}
                  title={`Restore ${((marker.sourceEndUs - marker.sourceStartUs) / 1e6).toFixed(2)}s cut`}
                  onPointerDown={(event) => event.stopPropagation()}
                  onClick={(event) => {
                    event.stopPropagation();
                    void restoreCut(marker.sourceStartUs, marker.sourceEndUs);
                  }}
                >
                  <span className="w-0.5 h-full bg-rose-400 group-hover:bg-rose-200" />
                </button>
              ))}
            </div>

            {/* Individual Lanes */}
            {tracks.map((track) => (
              <div
                key={track.id}
                className="h-14 mx-2 rounded-lg bg-studio-800/60 border border-studio-750 relative overflow-hidden flex items-center"
              >
                {/* Waveform for audio tracks */}
                {track.waveform && track.waveform.buckets.length > 0 && (
                  <div className="w-full h-full px-2 py-1">
                    <WaveformRenderer
                      buckets={track.waveform.buckets}
                      currentTimeProgress={progress}
                      activeBarColor={track.trackType === "mic" ? "#10b981" : "#6366f1"}
                    />
                  </div>
                )}

                {track.waveform && track.waveform.buckets.length === 0 && (
                  <span className="px-4 text-xs text-studio-400">Waveform unavailable</span>
                )}

                {!track.waveform && (track.trackType === "mic" || track.trackType === "system") && (
                  <span className="px-4 text-xs text-studio-400">Loading waveform…</span>
                )}

                {!track.waveform && track.trackType !== "mic" && track.trackType !== "system" && (
                  <div className="mx-2 h-8 flex-1 rounded bg-indigo-500/15 border border-indigo-400/20" />
                )}

                {/* Auto webcam layout: where the webcam fills the frame */}
                {track.trackType === "webcam" && durationUs > 0 &&
                  (openedProject?.webcamFocus?.segments ?? []).flatMap((segment) =>
                    (segment.editedRanges ?? []).map((range, index) => (
                      <div
                        key={`${segment.id}-${index}`}
                        className={`absolute top-1 bottom-1 rounded-sm border z-10 pointer-events-none ${
                          segment.enabled && openedProject?.webcamFocus?.enabled
                            ? "bg-amber-400/30 border-amber-300/70"
                            : "border-dashed border-studio-500/60"
                        }`}
                        style={{
                          left: `${(range.startUs / durationUs) * 100}%`,
                          width: `${((range.endUs - range.startUs) / durationUs) * 100}%`,
                        }}
                        title={`Webcam ${segment.enabled ? "fills the frame" : "focus off"} (${segment.source})`}
                      />
                    )),
                  )}

                {/* Excluded intervals / Silence cuts overlay */}
                {track.intervals
                  .filter((int) => int.excluded)
                  .map((cut) => {
                    const cutStartProg = cut.startUs / durationUs;
                    const cutWidthProg = (cut.endUs - cut.startUs) / durationUs;
                    return (
                      <div
                        key={cut.id}
                        className="absolute top-0 bottom-0 bg-rose-950/70 border-x border-rose-500/50 backdrop-blur-[1px] flex items-center justify-center z-10 pointer-events-none"
                        style={{
                          left: `${cutStartProg * 100}%`,
                          width: `${cutWidthProg * 100}%`,
                        }}
                      >
                        <span className="text-[9px] font-mono text-rose-300 font-semibold uppercase tracking-wider">
                          CUT
                        </span>
                      </div>
                    );
                  })}
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
};
