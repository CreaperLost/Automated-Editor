import {
  OpenedProject,
  SegmentPage,
  WaveformPage,
  PlaybackStatus,
  EditCut,
  PreviewQuality,
  PreviewStatus,
  TrackEdit,
  PreviewViewport,
  PreviewHitMode,
  MediaInteropStatus,
  MediaParityReport,
  ExportSettings,
  ExportStatus,
  SilenceConfig,
  SilenceDetectionResult,
  ZoomGeneration,
  ZoomConfig,
  ProjectZoom,
  ManualZoomInput,
  WebcamFocus,
  WebcamFocusDetection,
  WebcamFocusSettings,
  EditLayout,
  AudioSettings,
  CaptionSettings,
  TranscriptCutSuggestion,
  AiProvider,
  Chapter,
  Short,
  ShortLayout,
  AiSettings,
  AiSettingsView,
  TranscriptRunResult,
  TranscriptSettings,
  TranscriptSettingsView,
  TranscriptView,
} from "./types";

declare global {
  interface Window {
    __TAURI_INTERNALS__?: unknown;
    __TAURI__?: unknown;
  }
}

export const isTauriEnvironment = (): boolean => {
  return typeof window !== "undefined" && (Boolean(window.__TAURI_INTERNALS__) || Boolean(window.__TAURI__));
};

async function invokeTauri<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (!isTauriEnvironment()) {
    throw new Error(`${cmd} requires the desktop app.`);
  }
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    return await invoke<T>(cmd, args);
  } catch (err) {
    console.warn(`[Tauri IPC] Failed invoking ${cmd}:`, err);
    throw err;
  }
}

// One wrapper per command registered in src-tauri/src/lib.rs.
export const api = {
  openProject: (path: string) => invokeTauri<OpenedProject>("open_project", { path }),
  closeProject: (projectHandle: string) => invokeTauri<void>("close_project", { projectHandle }),
  projectSegments: (projectHandle: string, trackId: string, offset = 0, limit = 100) =>
    invokeTauri<SegmentPage>("project_segments", { projectHandle, trackId, offset, limit }),
  projectWaveform: (
    projectHandle: string,
    trackId: string,
    startUs: number,
    endUs: number,
    bucketCount = 256,
  ) =>
    invokeTauri<WaveformPage>("project_waveform", {
      projectHandle,
      trackId,
      startUs,
      endUs,
      bucketCount,
    }),
  projectZoomSuggestions: (projectHandle: string, config?: ZoomConfig) =>
    invokeTauri<ZoomGeneration>(
      "project_zoom_suggestions",
      config ? { projectHandle, config } : { projectHandle },
    ),
  projectZoomAccept: (projectHandle: string, expectedRevision: number, ids: string[]) =>
    invokeTauri<OpenedProject>("project_zoom_accept", {
      projectHandle,
      expectedRevision,
      ids,
    }),
  projectZoomDismiss: (projectHandle: string, expectedRevision: number, ids: string[]) =>
    invokeTauri<OpenedProject>("project_zoom_dismiss", {
      projectHandle,
      expectedRevision,
      ids,
    }),
  projectZoomUpdate: (projectHandle: string, expectedRevision: number, zoom: ProjectZoom) =>
    invokeTauri<OpenedProject>("project_zoom_update", {
      projectHandle,
      expectedRevision,
      zoom,
    }),
  projectZoomAdd: (projectHandle: string, expectedRevision: number, input: ManualZoomInput) =>
    invokeTauri<OpenedProject>("project_zoom_add", {
      projectHandle,
      expectedRevision,
      input,
    }),
  projectZoomDelete: (projectHandle: string, expectedRevision: number, id: string) =>
    invokeTauri<OpenedProject>("project_zoom_delete", {
      projectHandle,
      expectedRevision,
      id,
    }),
  projectWebcamFocusDetect: (
    projectHandle: string,
    expectedRevision: number,
    settings: WebcamFocusSettings,
  ) =>
    invokeTauri<WebcamFocusDetection>("project_webcam_focus_detect", {
      projectHandle,
      expectedRevision,
      settings,
    }),
  projectWebcamFocusUpdate: (projectHandle: string, expectedRevision: number, focus: WebcamFocus) =>
    invokeTauri<OpenedProject>("project_webcam_focus_update", {
      projectHandle,
      expectedRevision,
      focus,
    }),
  projectWebcamFocusAdd: (
    projectHandle: string,
    expectedRevision: number,
    editedStartUs: number,
    editedEndUs: number,
  ) =>
    invokeTauri<OpenedProject>("project_webcam_focus_add", {
      projectHandle,
      expectedRevision,
      editedStartUs,
      editedEndUs,
    }),
  projectWebcamFocusRemove: (
    projectHandle: string,
    expectedRevision: number,
    editedStartUs: number,
    editedEndUs: number,
  ) =>
    invokeTauri<OpenedProject>("project_webcam_focus_remove", {
      projectHandle,
      expectedRevision,
      editedStartUs,
      editedEndUs,
    }),
  projectLayoutUpdate: (
    projectHandle: string,
    expectedRevision: number,
    layout: EditLayout,
    wallpaperSource?: string,
  ) =>
    invokeTauri<OpenedProject>(
      "project_layout_update",
      wallpaperSource
        ? { projectHandle, expectedRevision, layout, wallpaperSource }
        : { projectHandle, expectedRevision, layout },
    ),
  projectAudioUpdate: (projectHandle: string, expectedRevision: number, audio: AudioSettings) =>
    invokeTauri<OpenedProject>("project_audio_update", {
      projectHandle,
      expectedRevision,
      audio,
    }),
  projectCaptionsUpdate: (projectHandle: string, expectedRevision: number, captions: CaptionSettings) =>
    invokeTauri<OpenedProject>("project_captions_update", {
      projectHandle,
      expectedRevision,
      captions,
    }),
  projectRippleCuts: (
    projectHandle: string,
    expectedRevision: number,
    cuts: EditCut[],
  ) =>
    invokeTauri<OpenedProject>("project_ripple_cuts", {
      projectHandle,
      expectedRevision,
      cuts,
    }),
  /** Premiere-style Q ("previous") and E ("next") ripple trim at the playhead. */
  projectRippleTrim: (
    projectHandle: string,
    expectedRevision: number,
    playheadUs: number,
    side: "previous" | "next",
  ) =>
    invokeTauri<OpenedProject>("project_ripple_trim", {
      projectHandle,
      expectedRevision,
      playheadUs: Math.max(0, Math.round(playheadUs)),
      side,
    }),
  /** Moves the edited range [startUs, endUs) (a clip) to edited position targetUs. */
  projectMoveRange: (
    projectHandle: string,
    expectedRevision: number,
    startUs: number,
    endUs: number,
    targetUs: number,
  ) =>
    invokeTauri<OpenedProject>("project_move_range", {
      projectHandle,
      expectedRevision,
      startUs: Math.max(0, Math.round(startUs)),
      endUs: Math.max(0, Math.round(endUs)),
      targetUs: Math.max(0, Math.round(targetUs)),
    }),
  projectSplit: (projectHandle: string, expectedRevision: number, editedUs: number) =>
    invokeTauri<OpenedProject>("project_split", {
      projectHandle,
      expectedRevision,
      editedUs: Math.max(0, Math.round(editedUs)),
    }),
  /**
   * Puts removed media back. `grow` picks the clip that grows when the media touches two
   * clips that are no longer neighbours: "end" (default) the one ending where it starts,
   * "start" the one starting where it ends.
   */
  projectRestoreCuts: (
    projectHandle: string,
    expectedRevision: number,
    ranges: EditCut[],
    grow: "end" | "start" = "end",
  ) =>
    invokeTauri<OpenedProject>("project_restore_cuts", {
      projectHandle,
      expectedRevision,
      ranges,
      grow,
    }),
  projectUndo: (projectHandle: string, expectedRevision: number) =>
    invokeTauri<OpenedProject>("project_undo", { projectHandle, expectedRevision }),
  projectRedo: (projectHandle: string, expectedRevision: number) =>
    invokeTauri<OpenedProject>("project_redo", { projectHandle, expectedRevision }),
  projectRename: (projectHandle: string, newName: string) =>
    invokeTauri<OpenedProject>("project_rename", { projectHandle, newName }),
  playbackStatus: (projectHandle: string) =>
    invokeTauri<PlaybackStatus>("playback_status", { projectHandle }),
  playbackPlay: (projectHandle: string) =>
    invokeTauri<PlaybackStatus>("playback_play", { projectHandle }),
  playbackPause: (projectHandle: string) =>
    invokeTauri<PlaybackStatus>("playback_pause", { projectHandle }),
  playbackSeek: (projectHandle: string, editedUs: number) =>
    invokeTauri<PlaybackStatus>("playback_seek", { projectHandle, editedUs: Math.max(0, Math.round(editedUs)) }),
  previewAttach: (windowLabel: string, hitMode: PreviewHitMode = "consume") =>
    invokeTauri<PreviewStatus>("preview_attach", { windowLabel, hitMode }),
  previewLayout: (viewport: PreviewViewport) =>
    invokeTauri<PreviewStatus>("preview_layout", { viewport }),
  previewPresentFixed: (r: number, g: number, b: number, generation = 0) =>
    invokeTauri<PreviewStatus>("preview_present_fixed", { r, g, b, generation }),
  previewPresentFixture: (path: string, generation = 0) =>
    invokeTauri<PreviewStatus>("preview_present_fixture", { path, generation }),
  previewStatus: () => invokeTauri<PreviewStatus>("preview_status"),
  /**
   * Latest webview preview frame after `after`: 8-byte LE sequence number + JPEG. Waits up to
   * `waitMs` for one to arrive; empty when none did.
   */
  previewFrame: (after: number, waitMs = 0) =>
    invokeTauri<ArrayBuffer>("preview_frame", { after, waitMs }),
  previewQuality: () => invokeTauri<PreviewQuality>("preview_quality"),
  previewQualitySet: (quality: PreviewQuality) =>
    invokeTauri<PreviewQuality>("preview_quality_set", { quality }),
  projectTracksEdit: (projectHandle: string, expectedRevision: number, edit: TrackEdit) =>
    invokeTauri<OpenedProject>("project_tracks_edit", { projectHandle, expectedRevision, edit }),
  previewHitTest: (x: number, y: number) => invokeTauri<boolean>("preview_hit_test", { x, y }),
  previewDetach: (windowLabel: string, generation?: number) =>
    invokeTauri<PreviewStatus>("preview_detach", { windowLabel, generation }),
  mediaInteropStatus: () => invokeTauri<MediaInteropStatus>("media_interop_status"),
  mediaRunParity: () => invokeTauri<MediaParityReport>("media_run_parity"),
  exportStart: (projectHandle: string, settings: ExportSettings) =>
    invokeTauri<ExportStatus>("export_start", { projectHandle, settings }),
  exportStatus: (jobId?: string) =>
    invokeTauri<ExportStatus>("export_status", { jobId: jobId ?? null }),
  exportCancel: (jobId: string) => invokeTauri<ExportStatus>("export_cancel", { jobId }),
  detectSilence: (projectHandle: string, trackId: string, config: SilenceConfig) =>
    invokeTauri<SilenceDetectionResult>("detect_silence", { projectHandle, trackId, config }),
  transcriptSettingsGet: () => invokeTauri<TranscriptSettingsView>("transcript_settings_get"),
  transcriptSettingsSet: (settings: TranscriptSettings) =>
    invokeTauri<TranscriptSettingsView>("transcript_settings_set", { settings }),
  transcriptSetApiKey: (key: string) =>
    invokeTauri<TranscriptSettingsView>("transcript_set_api_key", { key }),
  transcriptGet: (projectHandle: string, trackId: string) =>
    invokeTauri<TranscriptView | null>("transcript_get", { projectHandle, trackId }),
  transcriptRun: (projectHandle: string, trackId: string) =>
    invokeTauri<TranscriptRunResult>("transcript_run", { projectHandle, trackId }),
  transcriptCancel: () => invokeTauri<void>("transcript_cancel"),
  transcriptDelete: (projectHandle: string, trackId: string) =>
    invokeTauri<void>("transcript_delete", { projectHandle, trackId }),
  transcriptSuggestions: (projectHandle: string, trackId: string) =>
    invokeTauri<TranscriptCutSuggestion[]>("transcript_suggestions", { projectHandle, trackId }),
  /** Sends the kept words to the chosen AI provider; progress arrives as `transcript-progress`. */
  transcriptAiSuggest: (projectHandle: string, trackId: string) =>
    invokeTauri<TranscriptCutSuggestion[]>("transcript_ai_suggest", { projectHandle, trackId }),
  aiSettingsGet: () => invokeTauri<AiSettingsView>("ai_settings_get"),
  aiSettingsSet: (settings: AiSettings) => invokeTauri<AiSettingsView>("ai_settings_set", { settings }),
  aiSetApiKey: (provider: AiProvider, key: string) =>
    invokeTauri<AiSettingsView>("ai_set_api_key", { provider, key }),
  transcriptCutWords: (
    projectHandle: string,
    expectedRevision: number,
    trackId: string,
    wordIds: string[],
  ) =>
    invokeTauri<OpenedProject>("transcript_cut_words", {
      projectHandle,
      expectedRevision,
      trackId,
      wordIds,
    }),
  transcriptSetWordText: (projectHandle: string, trackId: string, wordId: string, text: string) =>
    invokeTauri<TranscriptView>("transcript_set_word_text", { projectHandle, trackId, wordId, text }),
  transcriptDismissSuggestions: (projectHandle: string, trackId: string, ids: string[], dismissed: boolean) =>
    invokeTauri<TranscriptCutSuggestion[]>("transcript_dismiss_suggestions", {
      projectHandle,
      trackId,
      ids,
      dismissed,
    }),
  transcriptDownloadModel: () =>
    invokeTauri<TranscriptSettingsView>("transcript_download_model"),
  getDefaultProjectsDir: () => invokeTauri<string>("get_default_projects_dir"),
  pickProjectFolder: () => invokeTauri<string | null>("pick_project_folder"),
  pickRecordingFolder: () => invokeTauri<string | null>("pick_recording_folder"),
  pickProjectLocation: () => invokeTauri<string | null>("pick_project_location"),
  /** Makes a project folder in `location` (default: the projects folder) and opens it. */
  projectCreate: (name: string, location?: string, recording?: string) =>
    invokeTauri<OpenedProject>("project_create", {
      name,
      location: location || null,
      recording: recording || null,
    }),
  pickExportDestination: (projectHandle?: string) =>
    invokeTauri<string | null>("pick_export_destination", { projectHandle: projectHandle ?? null }),
  pickWallpaperSource: () => invokeTauri<string | null>("pick_wallpaper_source"),
  pickMediaFiles: () => invokeTauri<string[]>("pick_media_files"),
  projectMediaImport: (projectHandle: string, expectedRevision: number, paths: string[]) =>
    invokeTauri<OpenedProject>("project_media_import", { projectHandle, expectedRevision, paths }),
  projectMediaRemove: (projectHandle: string, expectedRevision: number, assetId: string) =>
    invokeTauri<OpenedProject>("project_media_remove", { projectHandle, expectedRevision, assetId }),
  /** Places the asset's default clip (whole video/audio, 5 s for images) at targetUs. */
  projectMediaInsert: (projectHandle: string, expectedRevision: number, assetId: string, targetUs: number) =>
    invokeTauri<OpenedProject>("project_media_insert", {
      projectHandle,
      expectedRevision,
      assetId,
      targetUs: Math.max(0, Math.round(targetUs)),
    }),
  projectChaptersSet: (projectHandle: string, expectedRevision: number, chapters: Chapter[]) =>
    invokeTauri<OpenedProject>("project_chapters_set", {
      projectHandle,
      expectedRevision,
      chapters: chapters.map(({ id, sourceUs, title }) => ({ id, sourceUs, title })),
    }),
  /** Asks the AI provider for chapters from a track's transcript. */
  projectChaptersGenerate: (projectHandle: string, trackId: string) =>
    invokeTauri<OpenedProject>("project_chapters_generate", { projectHandle, trackId }),
  projectCurrent: () => invokeTauri<OpenedProject | null>("project_current"),
  openShortsWindow: () => invokeTauri<void>("open_shorts_window"),
  projectShortsSet: (projectHandle: string, expectedRevision: number, shorts: Short[]) =>
    invokeTauri<OpenedProject>("project_shorts_set", {
      projectHandle,
      expectedRevision,
      shorts: shorts.map(({ editedStartUs: _s, editedEndUs: _e, ...short }) => short),
    }),
  /** Asks the AI provider for moments that work as shorts; replaces the shorts list. */
  projectShortsGenerate: (projectHandle: string, trackId: string) =>
    invokeTauri<OpenedProject>("project_shorts_generate", { projectHandle, trackId }),
  projectShortExport: (projectHandle: string, shortId: string, settings: ExportSettings) =>
    invokeTauri<ExportStatus>("project_short_export", { projectHandle, shortId, settings }),
  /** A JPEG of the short `offsetUs` into it, drawn with `layout`. */
  shortPreviewFrame: (projectHandle: string, shortId: string, layout: ShortLayout, offsetUs: number) =>
    invokeTauri<ArrayBuffer>("short_preview_frame", {
      projectHandle,
      shortId,
      layout,
      offsetUs: Math.max(0, Math.round(offsetUs)),
    }),
  showInFinder: (path: string): Promise<void> =>
    invokeTauri<void>("show_in_finder", { path }),
  setWindowTitle: async (title: string): Promise<void> => {
    if (typeof document !== "undefined") {
      document.title = title;
    }
    if (isTauriEnvironment()) {
      try {
        await invokeTauri<void>("set_window_title", { title });
      } catch {
        try {
          const { getCurrentWindow } = await import("@tauri-apps/api/window");
          await getCurrentWindow().setTitle(title);
        } catch (err) {
          console.warn("[Tauri] Failed to set window title:", err);
        }
      }
    }
  },
};
