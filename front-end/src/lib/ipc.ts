import {
  OpenedProject,
  SegmentPage,
  WaveformPage,
  PlaybackStatus,
  EditCut,
  PreviewStatus,
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
  TranscriptCutSuggestion,
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
  projectSplit: (projectHandle: string, expectedRevision: number, editedUs: number) =>
    invokeTauri<OpenedProject>("project_split", {
      projectHandle,
      expectedRevision,
      editedUs: Math.max(0, Math.round(editedUs)),
    }),
  projectRestoreCuts: (projectHandle: string, expectedRevision: number, ranges: EditCut[]) =>
    invokeTauri<OpenedProject>("project_restore_cuts", {
      projectHandle,
      expectedRevision,
      ranges,
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
  /** Latest webview preview frame after `after`: 8-byte LE sequence number + JPEG, or empty. */
  previewFrame: (after: number) => invokeTauri<ArrayBuffer>("preview_frame", { after }),
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
  transcriptDownloadModel: () =>
    invokeTauri<TranscriptSettingsView>("transcript_download_model"),
  getDefaultProjectsDir: () => invokeTauri<string>("get_default_projects_dir"),
  pickProjectFolder: () => invokeTauri<string | null>("pick_project_folder"),
  pickExportDestination: (projectHandle?: string) =>
    invokeTauri<string | null>("pick_export_destination", { projectHandle: projectHandle ?? null }),
  pickWallpaperSource: () => invokeTauri<string | null>("pick_wallpaper_source"),
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
