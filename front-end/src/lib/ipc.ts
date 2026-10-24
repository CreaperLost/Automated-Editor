import { useSaveStatusStore } from "../stores/saveStatusStore";
import {
  CheckReport,
  OpenedProject,
  SegmentPage,
  WaveformOverview,
  FaceTrackStatus,
  PlaybackStatus,
  JoinMark,
  JoinsView,
  Licence,
  SearchResults,
  EditCut,
  PreviewQuality,
  PreviewStatus,
  SequenceEdit,
  PreviewViewport,
  PreviewHitMode,
  MediaInteropStatus,
  MediaParityReport,
  ExportFileKind,
  ExportSettings,
  ExportStatus,
  SilenceConfig,
  SilenceDetectionResult,
  NoiseScan,
  SourceSpan,
  TranscriptGapConfig,
  TranscriptDependency,
  ZoomGeneration,
  ZoomConfig,
  ZoomSettings,
  ProjectZoom,
  ManualZoomInput,
  FavouriteLayout,
  Layout,
  LayoutChange,
  LayoutPick,
  PickSettings,
  EditLayout,
  AudioSettings,
  CaptionSettings,
  TranscriptCutSuggestion,
  AiProvider,
  Chapter,
  Short,
  ShortLayout,
  AiSettings,
  FillerEngine,
  AiSettingsView,
  McpApp,
  McpSkillHome,
  McpStatus,
  ProposalOutcome,
  ProposalsView,
  HistoryView,
  CheckpointRestoreOutcome,
  TranscriptRunResult,
  TranscriptSettings,
  TranscriptSettingsView,
  TranscriptView,
  Role,
  CaptionEdit,
  CaptionTrackView,
  SoundEffect,
  GraphicCreated,
  GraphicListed,
  GraphicSlot,
  GraphicsRuntimeStatus,
  GraphicTemplate,
  GraphicUpdated,
  GraphicView,
  CleanupSettings,
  ExportPreferences,
  LookStill,
  PresetInput,
  PresetPart,
  PresetsView,
  PresetView,
  Style,
  StyleInput,
  StylesView,
  StyleView,
  InLinePlan,
  GraphicLibraryItem,
  GraphicLibrarySave,
  GraphicLibrarySaved,
  GateReport,
  TakeShortMade,
  FootageChange,
  ShortGraphicFrom,
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
  // Edits carry the revision they expect; their answer means the project was written.
  const edit = !!args && "expectedRevision" in args;
  const status = useSaveStatusStore.getState();
  if (edit) status.begin();
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    const result = await invoke<T>(cmd, args);
    if (edit) useSaveStatusStore.getState().succeed();
    return result;
  } catch (err) {
    console.warn(`[Tauri IPC] Failed invoking ${cmd}:`, err);
    if (edit) useSaveStatusStore.getState().fail(String(err));
    throw err;
  }
}

/** The short whose own timeline is edited (the Shorts workspace), or none: the video. */
let editTarget: string | null = null;
export const setEditTarget = (shortId: string | null) => {
  editTarget = shortId;
};
export const currentEditTarget = () => editTarget;

/** Bytes before the JPEG in a `previewFrame`: its sequence number and playback generation. */
export const PREVIEW_FRAME_HEADER = 16;

// One wrapper per command registered in src-tauri/src/lib.rs.
export const api = {
  openProject: (path: string) => invokeTauri<OpenedProject>("open_project", { path }),
  closeProject: (projectHandle: string) => invokeTauri<void>("close_project", { projectHandle }),
  projectSegments: (projectHandle: string, trackId: string, offset = 0, limit = 100) =>
    invokeTauri<SegmentPage>("project_segments", { projectHandle, trackId, offset, limit }),
  /** A sound's whole waveform (`<asset>.<stream>`), a bar every 20 ms or so, on its own time;
   *  each clip draws its part. */
  projectWaveformOverview: (projectHandle: string, trackId: string) =>
    invokeTauri<WaveformOverview>("project_waveform_overview", { projectHandle, trackId }),
  /** Each camera's face track for framing shorts; `start` begins finding missing ones. */
  projectFaceTracks: (projectHandle: string, start: boolean) =>
    invokeTauri<FaceTrackStatus[]>("project_face_tracks", { projectHandle, start }),
  projectZoomSuggestions: (projectHandle: string, config?: ZoomConfig) =>
    invokeTauri<ZoomGeneration>(
      "project_zoom_suggestions",
      config ? { projectHandle, config } : { projectHandle },
    ),
  /** `config`: the auto-zoom settings the suggestions were found with. */
  projectZoomAccept: (projectHandle: string, expectedRevision: number, ids: string[], config?: ZoomConfig) =>
    invokeTauri<OpenedProject>("project_zoom_accept", {
      projectHandle,
      expectedRevision,
      ids,
      config: config ?? null,
    }),
  /** Saves the project's auto-zoom settings; automatic zooms take the new amounts. */
  projectZoomSettingsSet: (projectHandle: string, expectedRevision: number, settings: ZoomSettings) =>
    invokeTauri<OpenedProject>("project_zoom_settings_set", { projectHandle, expectedRevision, settings }),
  /** Puts the recording's zooms back with these settings; yours stay, none overlap. */
  projectZoomReload: (projectHandle: string, expectedRevision: number) =>
    invokeTauri<OpenedProject>("project_zoom_reload", { projectHandle, expectedRevision, config: null }),
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
  /** Changes the layouts of the video, or of short `shortId` (its view comes back). */
  projectLayoutsChange: (
    projectHandle: string,
    expectedRevision: number,
    shortId: string | null,
    change: LayoutChange,
    /** Changes of one group in a row are one undo step (layout keys while playing). */
    group: string | null = null,
  ) =>
    invokeTauri<OpenedProject>("project_layouts_change", {
      projectHandle,
      expectedRevision,
      shortId,
      change,
      group,
    }),
  /** Picks layout sections from the recording (no AI); sections made by hand stay. */
  projectLayoutsPick: (
    projectHandle: string,
    expectedRevision: number,
    shortId: string | null,
    settings?: PickSettings,
  ) =>
    invokeTauri<LayoutPick>("project_layouts_pick", {
      projectHandle,
      expectedRevision,
      shortId,
      settings: settings ?? null,
    }),
  layoutFavouritesList: () => invokeTauri<FavouriteLayout[]>("layout_favourites_list"),
  layoutFavouriteSave: (name: string, layout: Layout) =>
    invokeTauri<FavouriteLayout[]>("layout_favourite_save", { name, layout }),
  layoutFavouriteRemove: (id: string) => invokeTauri<FavouriteLayout[]>("layout_favourite_remove", { id }),
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
  /** One timeline edit, in this window's timeline (the video's, or a short's own). */
  projectSequenceEdit: (projectHandle: string, expectedRevision: number, edit: SequenceEdit) =>
    invokeTauri<OpenedProject>("project_sequence_edit", { projectHandle, expectedRevision, edit, shortId: editTarget }),
  projectUndo: (projectHandle: string, expectedRevision: number) =>
    invokeTauri<OpenedProject>("project_undo", { projectHandle, expectedRevision, shortId: editTarget }),
  projectRedo: (projectHandle: string, expectedRevision: number) =>
    invokeTauri<OpenedProject>("project_redo", { projectHandle, expectedRevision, shortId: editTarget }),
  projectShortView: (projectHandle: string, shortId: string) =>
    invokeTauri<OpenedProject>("project_short_view", { projectHandle, shortId }),
  projectShortResync: (projectHandle: string, expectedRevision: number, shortId: string) =>
    invokeTauri<OpenedProject>("project_short_resync", { projectHandle, expectedRevision, shortId }),
  /** Plays `shortId` instead of the video from `startUs` (or gives the video back with null). */
  playbackFocusShort: (projectHandle: string, shortId: string | null, startUs = 0, play = false) =>
    invokeTauri<PlaybackStatus>("playback_focus_short", {
      projectHandle,
      shortId,
      startUs: Math.max(0, Math.round(startUs)),
      play,
    }),
  projectRename: (projectHandle: string, newName: string) =>
    invokeTauri<OpenedProject>("project_rename", { projectHandle, newName }),
  playbackStatus: (projectHandle: string) =>
    invokeTauri<PlaybackStatus>("playback_status", { projectHandle }),
  playbackPlay: (projectHandle: string) =>
    invokeTauri<PlaybackStatus>("playback_play", { projectHandle }),
  playbackPause: (projectHandle: string) =>
    invokeTauri<PlaybackStatus>("playback_pause", { projectHandle }),
  /** Plays at `speed` (1, 1.25, 1.5 or 2), carrying on if it is playing. */
  playbackSetSpeed: (projectHandle: string, speed: number) =>
    invokeTauri<PlaybackStatus>("playback_set_speed", { projectHandle, speed }),
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
   * Latest webview preview frame after `after`: 8-byte LE sequence number, 8-byte LE playback
   * generation it was drawn for (0 for none), then the JPEG (see `PREVIEW_FRAME_HEADER`). Waits
   * up to `waitMs` for one to arrive; empty when none did.
   */
  previewFrame: (after: number, waitMs = 0) =>
    invokeTauri<ArrayBuffer>("preview_frame", { after, waitMs }),
  previewQuality: () => invokeTauri<PreviewQuality>("preview_quality"),
  previewQualitySet: (quality: PreviewQuality) =>
    invokeTauri<PreviewQuality>("preview_quality_set", { quality }),
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
  detectSilence: (projectHandle: string, trackId: string, config: SilenceConfig, trackIds?: string[], transcriptAssisted = false) =>
    invokeTauri<SilenceDetectionResult>("detect_silence", { projectHandle, trackId, config, trackIds, transcriptAssisted, shortId: editTarget }),
  /** `shortId`: a short's own timeline instead of the edit target's. */
  detectNonSpeechGaps: (projectHandle: string, trackId: string, config: TranscriptGapConfig, trackIds?: string[], shortId: string | null = editTarget) =>
    invokeTauri<SilenceDetectionResult>("detect_non_speech_gaps", { projectHandle, trackId, config, trackIds, shortId }),
  /**
   * Clicks, smacks, knocks and breaths in sound `key`, found in its audio; `sensitivity` 0 to 1,
   * `paddingMs` the room muted either side of each (never into speech).
   */
  detectMouthNoises: (projectHandle: string, key: string, sensitivity: number, paddingMs: number, shortId: string | null = editTarget) =>
    invokeTauri<NoiseScan>("detect_mouth_noises", { projectHandle, key, config: { sensitivity, paddingMs }, shortId }),
  /** The last scan of sound `key` (kept with the project) and its muted moments, without listening again. */
  noiseReview: (projectHandle: string, key: string, shortId: string | null = editTarget) =>
    invokeTauri<NoiseScan>("noise_review", { projectHandle, key, shortId }),
  /** Mutes (or unmutes) moments of sound `key`'s own time: one undoable edit. In a short (the
   *  edit target, or `shortId`) the mutes are the short's own. */
  projectNoiseMutes: (projectHandle: string, expectedRevision: number, key: string, ranges: SourceSpan[], unmute = false, shortId: string | null = editTarget) =>
    invokeTauri<OpenedProject>("project_noise_mutes", { projectHandle, expectedRevision, key, ranges, unmute, shortId }),
  applyJumpCuts: (projectHandle: string, expectedRevision: number, ranges: {startUs: number; endUs: number}[], transcriptDependencies: TranscriptDependency[], shortId: string | null = editTarget) =>
    invokeTauri<OpenedProject>("apply_jump_cuts", { projectHandle, expectedRevision, ranges, transcriptDependencies, shortId }),
  /** A word or phrase in every transcript, cut or not (Search panel). */
  sourceSearch: (projectHandle: string, query: string) =>
    invokeTauri<SearchResults>("source_search", { projectHandle, query }),
  /** The joins to listen to (Joins panel); none of these change the edit. */
  joinsList: (projectHandle: string) => invokeTauri<JoinsView>("joins_list", { projectHandle }),
  joinsMark: (projectHandle: string, id: string, mark: JoinMark | null, note?: string) =>
    invokeTauri<JoinsView>("joins_mark", { projectHandle, id, mark, note }),
  joinsFind: (projectHandle: string) => invokeTauri<JoinsView>("joins_find", { projectHandle }),
  joinsClear: (projectHandle: string) => invokeTauri<JoinsView>("joins_clear", { projectHandle }),
  transcriptSettingsGet: () => invokeTauri<TranscriptSettingsView>("transcript_settings_get"),
  transcriptSettingsSet: (settings: TranscriptSettings) =>
    invokeTauri<TranscriptSettingsView>("transcript_settings_set", { settings }),
  transcriptSetApiKey: (key: string) =>
    invokeTauri<TranscriptSettingsView>("transcript_set_api_key", { key }),
  transcriptGet: (projectHandle: string, trackId: string) =>
    invokeTauri<TranscriptView | null>("transcript_get", { projectHandle, trackId, shortId: editTarget }),
  /** The video's own transcript (edited times on the video), whatever this window shows. */
  transcriptGetVideo: (projectHandle: string, trackId: string) =>
    invokeTauri<TranscriptView | null>("transcript_get", { projectHandle, trackId, shortId: null }),
  transcriptRun: (projectHandle: string, trackId: string) =>
    invokeTauri<TranscriptRunResult>("transcript_run", { projectHandle, trackId }),
  transcriptCancel: () => invokeTauri<void>("transcript_cancel"),
  transcriptDelete: (projectHandle: string, trackId: string) =>
    invokeTauri<void>("transcript_delete", { projectHandle, trackId }),
  transcriptSuggestions: (projectHandle: string, trackId: string) =>
    invokeTauri<TranscriptCutSuggestion[]>("transcript_suggestions", { projectHandle, trackId, shortId: editTarget }),
  /** Sends the kept words to the chosen AI provider; progress arrives as `transcript-progress`. */
  /** Reviews fillers and retakes with `engine`, or the settings' choice when left out. */
  transcriptAiSuggest: (projectHandle: string, trackId: string, engine?: FillerEngine) =>
    invokeTauri<TranscriptCutSuggestion[]>("transcript_ai_suggest", { projectHandle, trackId, engine }),
  mcpStatus: () => invokeTauri<McpStatus>("mcp_status"),
  mcpSetEnabled: (enabled: boolean) => invokeTauri<McpStatus>("mcp_set_enabled", { enabled }),
  /** Adds AeroEdits to (or removes it from) the Claude app's or Codex's MCP settings. */
  mcpAppConnect: (app: McpApp, connected: boolean) =>
    invokeTauri<McpStatus>("mcp_app_connect", { app, connected }),
  /** Installs the `aeroedits` editing skill for Claude Code or Codex, or takes it out. */
  mcpSkillInstall: (home: McpSkillHome, installed: boolean) =>
    invokeTauri<McpStatus>("mcp_skill_install", { home, installed }),
  mcpSetAutopilot: (autopilot: boolean) => invokeTauri<McpStatus>("mcp_set_autopilot", { autopilot }),
  /** Stops a running agent call: it answers its agent that it was stopped, at its next step. */
  mcpStopCall: (id: number) => invokeTauri<McpStatus>("mcp_stop_call", { id }),
  /** Agents' proposal batches, newest first, as they would land in the edit now. */
  proposalsList: (projectHandle: string) => invokeTauri<ProposalsView>("proposals_list", { projectHandle }),
  /** Accepts a batch (or only `itemIds` of a cut batch) as one undo step. */
  proposalAccept: (
    projectHandle: string,
    expectedRevision: number,
    batchId: string,
    itemIds: string[] | null,
    rejectRest: boolean,
  ) =>
    invokeTauri<ProposalOutcome>("proposal_accept", {
      projectHandle,
      expectedRevision,
      batchId,
      itemIds,
      rejectRest,
    }),
  /** Rejects a batch (or some of its cuts); `rejected: false` puts rejected cuts back. */
  proposalReject: (projectHandle: string, batchId: string, itemIds: string[] | null, rejected: boolean) =>
    invokeTauri<ProposalsView>("proposal_reject", { projectHandle, batchId, itemIds, rejected }),
  /** Undoes a batch's accept while it is still the latest change. */
  proposalUndo: (projectHandle: string, expectedRevision: number, batchId: string) =>
    invokeTauri<ProposalOutcome>("proposal_undo", { projectHandle, expectedRevision, batchId }),
  proposalUndoRun: (projectHandle: string, expectedRevision: number) =>
    invokeTauri<ProposalOutcome>("proposal_undo_run", { projectHandle, expectedRevision }),
  projectHistory: (projectHandle: string) => invokeTauri<HistoryView>("project_history", { projectHandle }),
  checkpointSave: (projectHandle: string, name: string) =>
    invokeTauri<HistoryView>("checkpoint_save", { projectHandle, name }),
  checkpointRename: (projectHandle: string, id: string, name: string) =>
    invokeTauri<HistoryView>("checkpoint_rename", { projectHandle, id, name }),
  checkpointDelete: (projectHandle: string, id: string) =>
    invokeTauri<HistoryView>("checkpoint_delete", { projectHandle, id }),
  checkpointRestore: (projectHandle: string, expectedRevision: number, id: string) =>
    invokeTauri<CheckpointRestoreOutcome>("checkpoint_restore", { projectHandle, expectedRevision, id }),
  aiSettingsGet: () => invokeTauri<AiSettingsView>("ai_settings_get"),
  aiSettingsSet: (settings: AiSettings) => invokeTauri<AiSettingsView>("ai_settings_set", { settings }),
  aiSetApiKey: (provider: AiProvider, key: string) =>
    invokeTauri<AiSettingsView>("ai_set_api_key", { provider, key }),
  transcriptCutWords: (
    projectHandle: string,
    expectedRevision: number,
    trackId: string,
    wordIds: string[],
    expectedWordStamp?: string,
  ) =>
    invokeTauri<OpenedProject>("transcript_cut_words", {
      projectHandle,
      expectedRevision,
      trackId,
      wordIds,
      expectedWordStamp,
      shortId: editTarget,
    }),
  projectCaptionCues: (projectHandle: string) =>
    invokeTauri<CaptionTrackView>("project_caption_cues", { projectHandle, shortId: editTarget }),
  /** The sound effects lane's clicks and whooshes in edited time. */
  projectSoundEffects: (projectHandle: string) =>
    invokeTauri<SoundEffect[]>("project_sound_effects", { projectHandle, shortId: editTarget }),
  transcriptCaptionEdit: (projectHandle: string, trackId: string, edit: CaptionEdit) =>
    invokeTauri<CaptionTrackView>("transcript_caption_edit", { projectHandle, trackId, edit, shortId: editTarget }),
  transcriptSetWordText: (projectHandle: string, trackId: string, wordId: string, text: string) =>
    invokeTauri<TranscriptView>("transcript_set_word_text", { projectHandle, trackId, wordId, text, shortId: editTarget }),
  /** Takes . , ! ? and the like off every word (apostrophes, hyphens and numbers stay). */
  transcriptStripPunctuation: (projectHandle: string, trackId: string) =>
    invokeTauri<TranscriptView>("transcript_strip_punctuation", { projectHandle, trackId, shortId: editTarget }),
  transcriptDismissSuggestions: (projectHandle: string, trackId: string, ids: string[], dismissed: boolean) =>
    invokeTauri<TranscriptCutSuggestion[]>("transcript_dismiss_suggestions", {
      projectHandle,
      trackId,
      ids,
      dismissed,
      shortId: editTarget,
    }),
  /** Downloads the Parakeet model, or the NVIDIA GPU pack (`gpu`) or the local review model (`llm`). */
  transcriptDownloadModel: (model: "parakeet" | "gpu" | "llm" = "parakeet") =>
    invokeTauri<TranscriptSettingsView>("transcript_download_model", { model }),
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
  pickMediaFolder: () => invokeTauri<string | null>("pick_media_folder"),
  /** Sets (or with null clears) an imported file's licence. */
  projectMediaLicence: (projectHandle: string, expectedRevision: number, assetId: string, licence: Licence | null) =>
    invokeTauri<OpenedProject>("project_media_licence", { projectHandle, expectedRevision, assetId, licence }),
  /** Sets what an asset's streams stand for. */
  projectMediaRoles: (
    projectHandle: string,
    expectedRevision: number,
    assetId: string,
    roles: { stream: string; role: Role }[],
  ) =>
    invokeTauri<OpenedProject>("project_media_roles", {
      projectHandle,
      expectedRevision,
      assetId,
      roles,
    }),
  graphicsRuntimeStatus: () => invokeTauri<GraphicsRuntimeStatus>("graphics_runtime_status"),
  /** Downloads the renderer; progress comes as "graphics-runtime-progress" events. */
  graphicsRuntimeInstall: () => invokeTauri<GraphicsRuntimeStatus>("graphics_runtime_install"),
  graphicsRuntimeCancel: () => invokeTauri<void>("graphics_runtime_cancel"),
  graphicsRuntimeRemove: () => invokeTauri<GraphicsRuntimeStatus>("graphics_runtime_remove"),
  graphicsTemplates: () => invokeTauri<GraphicTemplate[]>("graphics_templates"),
  /** Looks at the graphics' files again and retries failed renders. */
  graphicsRenderAgain: () => invokeTauri<void>("graphics_render_again"),
  /** Makes a graphic from a template and places it at `atUs`, one undo step. */
  projectGraphicCreate: (
    projectHandle: string,
    expectedRevision: number,
    template: string,
    atUs: number,
    options: { name?: string; variables?: Record<string, unknown>; lengthUs?: number } = {},
  ) =>
    invokeTauri<GraphicCreated>("project_graphic_create", {
      projectHandle,
      expectedRevision,
      template,
      name: options.name ?? null,
      variables: options.variables ?? {},
      atUs: Math.max(0, Math.round(atUs)),
      lengthUs: options.lengthUs ?? null,
    }),
  projectGraphicGet: (projectHandle: string, assetId: string) =>
    invokeTauri<GraphicView>("project_graphic_get", { projectHandle, assetId }),
  projectGraphicsList: (projectHandle: string) =>
    invokeTauri<GraphicListed[]>("project_graphics_list", { projectHandle }),
  /** The timeline at `atUs` as it plays, as a small PNG data URL (a graphic's thumbnail). */
  projectFrameThumbnail: (projectHandle: string, atUs: number, width: number) =>
    invokeTauri<string>("project_frame_thumbnail", { projectHandle, atUs, width }),
  /** Small JPEG data URLs of a clip's source at `timesUs` (its own clock); null where it has no picture. */
  projectClipThumbnails: (projectHandle: string, assetId: string, stream: string, timesUs: number[], height: number) =>
    invokeTauri<(string | null)[]>("project_clip_thumbnails", { projectHandle, assetId, stream, timesUs, height }),
  /**
   * Sets variables (`null` puts one back to its default) and, optionally, the name and the
   * style ("project" follows the project's).
   */
  projectGraphicSet: (
    projectHandle: string,
    expectedRevision: number,
    assetId: string,
    variables: Record<string, unknown>,
    name?: string,
    style?: string,
  ) =>
    invokeTauri<GraphicUpdated>("project_graphic_set", {
      projectHandle,
      expectedRevision,
      assetId,
      variables,
      name: name ?? null,
      style: style ?? null,
    }),
  /** Every preset and, with a project, its preset and what it changed from it. */
  presets: (projectHandle?: string) => invokeTauri<PresetsView>("presets", { projectHandle: projectHandle ?? null }),
  /** The project follows preset `name`, or its new shorts or clips start from it (`target`); one undo step. */
  projectPresetSet: (projectHandle: string, expectedRevision: number, name: string, target: "video" | "shorts" | "clips" = "video") =>
    invokeTauri<OpenedProject>("project_preset_set", { projectHandle, expectedRevision, name, target }),
  /** Takes the project's changes back to its preset's values: all, a part's, or one field; one undo step. */
  projectPresetReset: (projectHandle: string, expectedRevision: number, part?: PresetPart, field?: string) =>
    invokeTauri<OpenedProject>("project_preset_reset", {
      projectHandle,
      expectedRevision,
      part: part ?? null,
      field: field ?? null,
    }),
  /** The project's settings into its preset, or a new one called `name` it then follows. */
  presetSaveFromProject: (projectHandle: string, expectedRevision: number, name?: string) =>
    invokeTauri<OpenedProject>("preset_save_from_project", { projectHandle, expectedRevision, name: name ?? null }),
  presetSave: (input: PresetInput) => invokeTauri<PresetView>("preset_save", { input }),
  presetRemove: (name: string) => invokeTauri<void>("preset_remove", { name }),
  /** Renames one of the user's presets; projects following it keep following it. */
  presetRename: (name: string, to: string) => invokeTauri<PresetView>("preset_rename", { name, to }),
  /** What the project's cleanup passes start from; one undo step. */
  projectCleanupSet: (projectHandle: string, expectedRevision: number, cleanup: CleanupSettings) =>
    invokeTauri<OpenedProject>("project_cleanup_set", { projectHandle, expectedRevision, cleanup }),
  /** What the project's Export dialog starts with; one undo step. */
  projectExportSet: (projectHandle: string, expectedRevision: number, exportPreferences: ExportPreferences) =>
    invokeTauri<OpenedProject>("project_export_set", { projectHandle, expectedRevision, export: exportPreferences }),
  /** Every style and, with a project, its style. */
  styles: (projectHandle?: string) => invokeTauri<StylesView>("styles", { projectHandle: projectHandle ?? null }),
  /** Saves a style app-wide (new, changed, renamed, or a copy of a built-in). */
  styleSave: (input: StyleInput) => invokeTauri<StyleView>("style_save", { input }),
  styleRemove: (name: string) => invokeTauri<void>("style_remove", { name }),
  /** Picks a logo or font file for a style; null when cancelled. */
  pickBrandFile: (kind: "logo" | "font") => invokeTauri<string | null>("pick_brand_file", { kind }),
  /** Dresses the project in style `name`, one undo step. */
  projectStyleSet: (projectHandle: string, expectedRevision: number, name: string) =>
    invokeTauri<OpenedProject>("project_style_set", { projectHandle, expectedRevision, name }),
  /** Stills of up to three of the video's graphics in `style`, changing nothing (a few seconds). */
  stylePreview: (projectHandle: string, style: Style) =>
    invokeTauri<LookStill[]>("style_preview", { projectHandle, style }),
  /** Look tokens of a graphic's own over its style's (null takes one back). */
  projectGraphicLookSet: (projectHandle: string, assetId: string, tokens: Record<string, string | null>) =>
    invokeTauri<GraphicView>("project_graphic_look_set", { projectHandle, assetId, tokens }),
  /** What bringing the video's graphics in line with its look would change, changing nothing. */
  graphicsInLinePlan: (projectHandle: string) => invokeTauri<InLinePlan[]>("graphics_in_line_plan", { projectHandle }),
  /** Brings those graphics in line, one undo step. */
  projectGraphicsInLine: (projectHandle: string, expectedRevision: number, plans: InLinePlan[]) =>
    invokeTauri<OpenedProject>("project_graphics_in_line", { projectHandle, expectedRevision, plans }),
  graphicsLibrary: () => invokeTauri<GraphicLibraryItem[]>("graphics_library"),
  graphicsLibraryRemove: (id: string) => invokeTauri<void>("graphics_library_remove", { id }),
  /** A library item's contact sheet from the gate, as a data URL. */
  graphicsLibrarySheet: (id: string) => invokeTauri<string | null>("graphics_library_sheet", { id }),
  /** Runs the library's gate on a project graphic; progress comes as "graphics-check-progress". */
  projectGraphicCheck: (projectHandle: string, assetId: string) =>
    invokeTauri<GateReport>("project_graphic_check", { projectHandle, assetId }),
  /** Runs the gate and, if it passes (or `asIs`), saves the graphic to the app's library. */
  projectGraphicLibrarySave: (projectHandle: string, request: GraphicLibrarySave) =>
    invokeTauri<GraphicLibrarySaved>("project_graphic_library_save", { projectHandle, request }),
  /** Copies a library graphic into the project and places it at `atUs`, one undo step. */
  projectGraphicFromLibrary: (projectHandle: string, expectedRevision: number, libraryId: string, atUs: number) =>
    invokeTauri<GraphicCreated>("project_graphic_from_library", {
      projectHandle,
      expectedRevision,
      libraryId,
      atUs: Math.max(0, Math.round(atUs)),
      lengthUs: null,
    }),
  /** A short from a take: `camera` and `voice` are media ids or stream keys. */
  projectShortFromTake: (
    projectHandle: string,
    expectedRevision: number,
    title: string,
    camera: string | null,
    voice: string | null,
  ) =>
    invokeTauri<TakeShortMade>("project_short_from_take", {
      projectHandle,
      expectedRevision,
      title,
      camera,
      voice,
      media: null,
      reason: null,
    }),
  projectShortFootage: (projectHandle: string, expectedRevision: number, shortId: string, change: FootageChange) =>
    invokeTauri<OpenedProject>("project_short_footage", { projectHandle, expectedRevision, shortId, change }),
  /** Makes a graphic and places it on take short `shortId`'s own timeline, one undo step. */
  projectShortGraphicAdd: (
    projectHandle: string,
    expectedRevision: number,
    shortId: string,
    from: ShortGraphicFrom,
    atUs: number,
    lengthUs: number | null,
    mode: GraphicSlot | null = null,
  ) =>
    invokeTauri<GraphicCreated>("project_short_graphic_add", {
      projectHandle,
      expectedRevision,
      shortId,
      from,
      atUs: Math.max(0, Math.round(atUs)),
      lengthUs: lengthUs === null ? null : Math.round(lengthUs),
      mode,
    }),
  projectMediaImport: (projectHandle: string, expectedRevision: number, paths: string[]) =>
    invokeTauri<OpenedProject>("project_media_import", { projectHandle, expectedRevision, paths }),
  projectMediaRemove: (projectHandle: string, expectedRevision: number, assetId: string) =>
    invokeTauri<OpenedProject>("project_media_remove", { projectHandle, expectedRevision, assetId }),
  projectChaptersSet: (projectHandle: string, expectedRevision: number, chapters: Chapter[]) =>
    invokeTauri<OpenedProject>("project_chapters_set", {
      projectHandle,
      expectedRevision,
      chapters: chapters.map(({ id, sourceUs, title, media }) => ({ id, sourceUs, title, media })),
    }),
  /** Asks the AI provider for chapters from a track's transcript. */
  projectChaptersGenerate: (projectHandle: string, trackId: string) =>
    invokeTauri<OpenedProject>("project_chapters_generate", { projectHandle, trackId }),
  /** The check before export, of the edit as it is now. */
  projectCheck: (projectHandle: string) => invokeTauri<CheckReport>("project_check", { projectHandle }),
  /** Applies the fixes of these findings (or `["all"]`), each as its own undo step. */
  projectCheckFix: (projectHandle: string, expectedRevision: number, ids: string[]) =>
    invokeTauri<OpenedProject>("project_check_fix", { projectHandle, expectedRevision, ids }),
  projectCurrent: () => invokeTauri<OpenedProject | null>("project_current"),
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
  /** Saves subtitles, the transcript or the sound of the video (or a short); answers with the
   *  path. Next to the project unless `destination`. */
  exportFile: (projectHandle: string, kind: ExportFileKind, shortId: string | null, destination: string | null) =>
    invokeTauri<string>("export_file", { projectHandle, kind, shortId, destination }),
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
