# Plan: Premiere-style tracks (no V1)

Status: **implemented on branch `claude/premiere-tracks` (2026-10-04)**: phases 0-6 done,
Phase 7 (George's check in the app) pending. Decisions in section 3 are settled.

How it was built, where it differs from the text below:
- ClockMap is a `TimelineMapper` built from the clips that play a stream or asset
  (`sequence/clock.rs`); where one moment plays twice, the earlier clip counts.
- Audio track mix (volume, noise reduction, ducking) stays in `audio.tracks`, keyed by the
  audio track's id; mute and role live on the track.
- Webcam focus belongs to one recording (`webcamFocus.media`), mapped through its camera clips.
- Stream keys are `<asset>.<stream>` (transcripts, waveforms, pauses, captions).
- Recordings (the project's and imported ones) play from their own folder, mounted in the
  project at `recordings/<asset id>/`.
- Real-data probes (`src-tauri/src/parity.rs`, `AERO_PROBE=<dir>`) give the same pauses,
  transcript positions and zooms as before the change.

**Top priority (George):** importing media, transcripts and jump cuts must keep working at
every step. Section 4 lists exactly what that means and how each phase proves it.

Ground rule (from George): **no migration.** No old projects exist and none ever will need
converting. Old fields, serde fallbacks and "older projects" code paths are deleted, not
kept alive.

---

## 1. Goal

Make the timeline work like Premiere: **just tracks of clips.**

- Video tracks V1, V2, … and audio tracks A1, A2, …, all the same kind of thing.
- A recording arrives as **linked clips**: Screen on a video track, Camera on another video
  track (camera role → drawn as the bubble), Microphone and System audio on audio tracks.
- Imported media works the same way: a video becomes a picture clip plus linked sound clips.
- Selecting a clip selects its linked partners; **U** unlinks/relinks. Unlinked clips move
  and trim on their own.
- **Magnetic** stays as a toggle and becomes ripple editing: cuts close the gap on every
  track, moves insert. Off, cuts leave gaps and moves overwrite.
- Every feature that works in "recording time" today keeps working, by mapping through the
  clips that play that recording.

What disappears: `retained_intervals`, `removed_intervals`, `split_points_us`, `main_track`,
the V1 strip, `main-sound-*` / `{track}-sound-*` linked sound lanes, `msound-*` ids, the
"lift from main / drop to main" edits, and the split between "the project's recording" and
"imported recordings".

## 2. The new model

### 2.1 Assets and streams (sources)

Everything that can be played is an **asset** with one or more **streams**. All streams of
one asset share one clock (the asset's own time, from 0).

```text
Asset {
  id, name,
  kind: Recording { folder } | Video | Image | Audio,
  streams: [Stream],          // what can be put on a track
  duration_us, width, height,
  missing,                     // worked out on open
}
Stream {
  id: StreamId,                // "screen" | "camera" | "audio:0" | "audio:1" | "picture"
  kind: Picture | Sound,
  role: Screen | Camera | Overlay | Speech | Background,   // the default role
  name,                        // "Microphone", "System audio", "Audio 2" …
  // where its pixels / samples come from:
  //   Recording: the recorder's segment files (with gaps for pauses)
  //   Video/Audio: the file, plus an extracted 48 kHz WAV per sound stream
}
StreamRef = (asset_id, StreamId)
```

- **The project's own recording is just the first Recording asset.** The editor no longer
  treats it differently from an imported recording. (Big simplification: one code path for
  recordings, their zooms, cursor, camera and transcripts.)
- A Recording's streams map onto the recorder's segment lists (`SegmentSummary`, with gaps
  where the recorder paused). The reader keeps doing what it does today, per stream.

### 2.2 Tracks and clips (the sequence)

```text
Sequence {
  tracks: [Track],             // video tracks bottom→top, then audio tracks top→bottom
  magnetic: bool,              // ripple editing (default on)
}
Track {
  id, kind: Video | Audio, name ("V1", "A2" or the user's),
  hidden, muted, locked,
  role: Option<Role>,          // video: Screen | Camera | Overlay; audio: Speech | Background
                               // unset → each clip's stream role
  // audio mix (today's TrackMix): volume_db, denoise_db, duck_db
  clips: [Clip],               // sorted by start, never overlapping on one track
}
Clip {
  id,
  source: StreamRef,
  start_us,                    // on the timeline
  in_us,                       // on the asset's clock
  duration_us,
  link: Option<LinkId>,        // clips cut from one source moment move/trim together
  fit: Contain | Cover,        // overlay pictures only
}
```

- **Video clips never carry sound.** Sound is always an audio clip on an audio track
  (linked to its picture). This is the Premiere model and it removes "linked sound lanes",
  `audio_unlinked`, and the mixer's special cases.
- One audio track = one mixer lane. Track mix settings live on the track (no more lane-id
  map in `audio.tracks`).
- Linking is by `link` id (we already use it for unlinked sound). Linked clips share the
  same source moment: `in_us` equal and `start_us` equal when created; edits keep them in
  step unless unlinked.

### 2.3 The project document

```text
EditDocument {
  revision,
  assets: [Asset],
  sequence: Sequence,
  layout,                      // canvas, background, screen styling, camera bubble
  zooms: [Zoom],               // each on an asset clock (as `media` is today)
  zoom_settings,
  webcam_focus: per asset,     // segments + normal view in that asset's time
  captions,                    // settings + which stream to caption (StreamRef)
  chapters: [Chapter],         // anchored to (asset, source time) as today
  shorts: [Short],
  audio,                       // project-wide: normalize, target LUFS
}
```

Transcripts stay in their own files, keyed by **StreamRef** (`transcripts/<asset>.<stream>.json`)
instead of by recording track id or `msound-…` id.

### 2.4 One mapping primitive: clocks

Today there are five mappers (`mapper`, `mapper_for_media`, `mapper_for_sound`,
`sound_retained`, `mapper_for_transcript`). They all become one:

```text
ClockMap::for_stream(sequence, StreamRef) -> ClockMap
ClockMap::for_asset(sequence, asset, preferring: StreamId) -> ClockMap
  // built from the clips that play that stream, in timeline order
  source_to_timeline(source_us) -> Vec<timeline_us>     (a moment can appear twice)
  source_range_to_timeline(a, b) -> Vec<(start, end)>
  timeline_to_source(timeline_us) -> Option<source_us>
```

Who uses which clock:

| Feature | Clock | Which clips |
|---|---|---|
| Zooms, cursor, webcam focus | the recording / media asset | its Screen (or picture) clips |
| Transcript words, captions, cut-by-words, fillers | the transcribed stream | its sound clips |
| Jump cuts (silence) | the scanned stream | its sound clips |
| Chapters | the asset they were made on | its picture clips |
| "Restore cut" markers | per clip | neighbouring clips of the same stream |

### 2.5 What the old concepts become

| Today | New model |
|---|---|
| V1 + Screen/Camera/Mic/System rows | Linked clips on V1, V2 (camera role), A1, A2 |
| `retained_intervals` | The clips themselves |
| `removed_intervals`, cut markers, Restore cuts | Gaps between adjacent clips of the same source; restoring = trimming the clip edge back out |
| `split_points_us` | Nothing: a split makes two clips |
| Magnetic V1 / `main_track` | `sequence.magnetic` (ripple across tracks) |
| Gaps on V1 (`@gap`) | Empty timeline space (nothing on any video track = black) |
| Lift from main / drop to main | Ordinary moves between video tracks |
| Overlay tracks V2+ | Just video tracks |
| `main-sound-k`, `{track}-sound-k`, `msound-*` | Audio clips on audio tracks; streams named by StreamRef |
| Project recording vs imported recording | Both are Recording assets |
| Short's own `ShortEdit { retained, split_points, overlay_tracks }` | A short's own `Sequence` |

## 3. Decisions (settled)

George accepted every recommendation on 2026-10-04:

1. **Empty time on the timeline is black** (nothing on any video track = black frame).
2. **Shorts follow the video until edited on their own.** A short is a window anchored to
   (asset, source time) at its start and end; its timeline is the main sequence sliced between
   where those anchors land. Editing it stores its own Sequence (as `ShortEdit` does today).
3. **Magnetic ripples every track except locked ones.**
4. **Camera-role clips draw as the bubble**; with two at once, the top one wins.
5. **Screen styling** (size, corners, shadow, crop, zooms, cursor) applies to screen-role clips;
   other video clips draw full-frame (contain/cover).
6. **Tracks are numbered by position** (V1, V2, A1…) with an optional custom name.
7. **Imported video is always split** into a picture clip and linked audio clips on import.

## 4. Must keep working: media, transcripts, jump cuts

These are the workflows George relies on. Each has **automated tests from Phase 0 on**, and
every later phase must keep them green before it is called done. "Real data" means the
probe approach used before: copies of George's real projects (`Untitled 3 Oct 2026`, an
imported-media project; `Untitled 4 Oct 2026.aero`, a recording) in the scratchpad, never
the originals, with ignored tests run on demand.

### 4.1 Importing media

| Must work | Proven by |
|---|---|
| Import a video with sound: an asset with a picture stream and one sound stream per audio track in the file (two-stream files keep both) | unit test on import; real-data probe on the imported mp4 |
| Import audio only, image only, a whole folder, a recorder folder (as a Recording asset with screen, camera, mic, system streams) | unit tests per kind |
| Place on the timeline: picture clip on a video track + linked sound clips on audio tracks; magnetic inserts, otherwise overwrites; tracks added when needed | `PlaceAsset` edit tests |
| Media referenced in place; missing files flagged, their clips black/silent, "Clean up" removes them | reader + edit tests |
| Removing an asset removes its clips (undoable) | edit test |
| Waveforms for every sound clip (imported and recorded) | waveform test per stream (the size-0 segment bug from 2026-10-03 must not come back: segments carry the file's real size) |
| Imported media plays and exports with its sound | Phase-0 golden audio test on a media-only project |

### 4.2 Transcripts and captions

| Must work | Proven by |
|---|---|
| Transcribe **any** sound stream: the recording's microphone or system audio, any imported file's stream | fake-transcriber test per source kind (the probe from 2026-10-03, kept as a real test) |
| Words map to the timeline through that stream's clips, after cuts, moves, splits and an unlink | mapping tests before/after each edit |
| A transcribed stream not on the timeline yet shows its words as "not on the timeline" | view test |
| Cut by words cuts every track (ripple when magnetic); undo restores | edit test |
| Fix a word, hide words from captions, new caption / merge, remove punctuation (separate punctuation words too) | transcript store tests (exist; re-pointed at StreamRef keys) |
| Fillers, retakes and AI review list and cut correctly | suggestion tests |
| Captions read the chosen stream, appear on the caption track at the right times, and render in export and shorts | caption cue tests + Phase-0 golden frame with captions |
| The Transcript panel picks a stream automatically (including sound imported after opening) | front-end check |

### 4.3 Jump cuts

| Must work | Proven by |
|---|---|
| Find pauses on any sound stream (recording mic, system audio, imported file stream) | silence-detection test per source kind; real-data probe on the imported mp4 (it found 2 pauses before) |
| Suggestions are in timeline time through that stream's clips, including clips moved or split | mapping tests |
| Apply cuts every track at once (ripple), in one undo step | edit test |
| Stale suggestions after another edit are refused with "scan again" | front-end check (exists) |
| Gentle / Balanced / Tight and the play-before-pause button | front-end check |

### 4.4 Gate for every phase

A phase is done only when: `cargo test` passes including the 4.1–4.3 tests, the real-data
probes give the same results as before (pause count and positions, transcript word
positions, imported streams), and `npm run build` passes. From Phase 4 on, George also checks
import → transcribe → jump cuts → export in the app.

## 5. Phases

Each phase ends with the section 4.4 gate passed. Phases 1–4 are backend; 5 is the timeline
UI; 6 removes the old code.

### Phase 0: parity tests first (no behaviour change)

Before touching the model, pin down what the editor produces today so the rewrite can be
measured against it.

- Golden **render tests** (CPU compositor, small canvas) on a fixture recording: plain frame,
  frame after a cut, mid-zoom frame, camera bubble, Cam Focus, an overlay clip, captions on.
- Golden **audio tests**: a mixed chunk with mute, volume, ducking and denoise on.
- Mapping tests: transcript word → timeline time, zoom edited ranges, silence suggestions,
  short window, before and after a cut and a move.
- **The section 4 workflow tests** (import every kind, transcribe every source kind with a
  fake transcriber, jump cuts on every source kind, cut by words, captions), written against
  today's model first so they record today's results.
- **Real-data probes** turned into ignored tests: the imported-mp4 project (pauses found,
  streams, transcript mapping) and the `.aero` recording (zooms, cursor, mic pauses).
- These tests are rewritten against the new model in later phases; the expected numbers stay.

### Phase 1: the new model and its edits (Rust, alongside the old)

New module `sequence` (assets, streams, tracks, clips, ClockMap) plus `sequence::edit`:

| Edit | Meaning |
|---|---|
| `AddTrack { kind }`, `RemoveTrack`, `SetTrack { hidden, muted, locked, role, name, mix }`, `MoveTrack { up }` | track management |
| `PlaceAsset { asset, at, tracks? }` | creates linked clips, one per stream, on suitable tracks (adding tracks as needed); magnetic inserts, otherwise overwrites |
| `MoveClips { ids, delta, to_track? }` | linked partners move too unless unlinked; insert or overwrite by magnetic |
| `TrimClip { id, edge, at }` | trims a clip (and its linked partners); extending out reveals source; magnetic ripples |
| `Split { at, ids? }` | none given: every track at the playhead |
| `Delete { ids }`, `DeleteRange { ranges, tracks }` | ripple or lift by magnetic |
| `Link { ids }`, `Unlink { ids }` | |
| `SetMagnetic(bool)` | |
| `SetClip { id, fit }` | overlay fit |

- Validation: clips don't overlap on a track, sound clips only on audio tracks, sources exist
  and stay inside the asset's length (images stretch), ids unique.
- New project from a recording builds the starting sequence: V1 Screen, V2 Camera (camera
  role), A1 Microphone, A2 System audio, with one linked clip set **per recorded span between
  pauses**.
- Thorough unit tests for every edit with magnetic on and off, linked and unlinked.

### Phase 2: playing and exporting the new model

- **Stream decoding.** One `StreamSource` trait for "frame at source time" / "samples":
  recording segments and imported files behind the same interface (today these are
  `decode_layer` + segments vs `media_frame`).
- **SceneEvaluator.** Per timeline time: background; video tracks bottom→top, each active
  clip by role (screen styling + zooms + cursor; camera bubble + focus; overlay
  contain/cover); captions on top. Replaces `main_scene_at` / `media_scene` / `push_overlays`.
- **AudioMixer + polish.** Sum the active clips of each unmuted audio track; per-track
  volume, denoise, ducking (ducking keys on speech-role tracks); normalize. Replaces span
  building over `retained_intervals` and lane ids.
- **Playback owner and engine.** Plans and open files from the clips near the playhead;
  short focus uses a short's Sequence.
- Phase-0 golden tests pass against the new evaluator.

### Phase 3: features on clocks

Rewrite each on ClockMap; each has its Phase-0 test. **The priority workflows go first:**

1. Waveforms per (asset, audio stream), drawn per clip (import's visible result).
2. Transcripts: run, store and view keyed by StreamRef; cut words → `DeleteRange` on all
   tracks; fixes, caption marks, punctuation; fillers, retakes, AI review; captions and the
   caption track.
3. Jump cuts on any stream.

Then:

4. Zooms (generation per Recording asset, edited ranges, the zoom lane), cursor drawing.
5. Webcam focus (Auto Webcam, Cam Focus, Normal view), per Recording asset.
6. Chapters, and AI chapters / shorts generation.
7. Shorts: window anchors, slicing, own Sequence, short view for the Shorts Studio, export.

### Phase 4: commands and the opened project

- `OpenedProject` sends `assets`, `sequence` (with track numbers and each clip's mapped
  info the UI needs), zooms/chapters/shorts with their timeline positions; drops
  `retainedIntervals`, `removedIntervals`, `splitPointsUs`, `tracks`, `overlayTracks`,
  `mainTrack`.
- One `project_sequence_edit(edit)` command replaces `project_tracks_edit`, ripple cuts, media
  insert, restore. Undo/redo, the save indicator and short views keep working the same way.
- Media import creates assets only; placing on the timeline is `PlaceAsset`.

### Phase 5: the timeline UI, rebuilt on tracks

`TimelineStudio.tsx` (~2,950 lines, much of it V1-specific) is replaced by smaller parts:

- `TrackHeaders`: name, role, hide/mute/lock, volume, order, resize.
- `TrackLane` + `ClipView`: clips with thumbnails (picture) or waveforms (sound), trim
  handles, linked-clip highlighting, restore-cut markers at same-source joins.
- `useTimelineInteractions`: select / multi-select / range, move with snapping between
  compatible tracks, trim, split, delete, link/unlink, drop from the Media panel,
  magnetic insert/overwrite previews.
- Zooms and Captions lanes on top (kept from today, mapped through ClockMap).
- Toolbar unchanged in look (Split, ripple trims, Unlink, Magnetic, Cam Focus, Normal view,
  zoom/Fit).
- Inspector: the Clip page works for **any** clip (screen, camera, overlay, sound).
- Audio section: one row per audio track.
- Shorts Studio timeline: the same component on the short's Sequence.

### Phase 6: delete the old model

Remove `retained_intervals` and everything built on it: `TimelineMapper`'s V1 paths,
`revision.rs` helpers (cut/move/restore/lift/place main, `canonical_retained`, gaps),
`tracks.rs` V1 edits, `MainTrack`, linked sound lanes, `msound` ids, lane-id track mixes,
`projectUtils.buildClips/buildCutMarkers/clipTrimLimits`, `trackUtils.soundLanes` and
friends, the recording-vs-imported-recording branches in export and audio, and the old
`ShortEdit`.

### Phase 7: check in the app

George builds and checks, **in this order** (the priority workflows first):
1. Import media: a video with two sound tracks, an audio file, an image, a recorder folder;
   drag onto the timeline; play with sound; missing-file clean-up.
2. Transcripts: transcribe the recording's mic and an imported file; cut by words; fix words;
   captions on the caption track and in the preview; remove punctuation.
3. Jump cuts: on the mic and on an imported file; listen to a pause; apply; undo.
4. Then: cutting, moving, magnetic on/off, unlink the camera and move it, zooms, cursor,
   shorts (follow and own edit), export.

## 6. Order of work and size

| Phase | Size | Can George see it? |
|---|---|---|
| 0 Parity tests | S | no (tests) |
| 1 Model + edits | L | no (tests) |
| 2 Play/export | L | yes, once 4 lands |
| 3 Features on clocks | L | yes, once 4 lands |
| 4 Commands | M | yes: backend done, old UI breaks |
| 5 Timeline UI | XL | yes |
| 6 Delete old | M | no change |
| 7 App check | — | yes |

Phases 1–3 can be built beside the old model (new modules, old ones untouched) so the app
keeps working until Phase 4 switches over. Phases 4 and 5 should land together on one branch;
in between, the app's timeline is not usable.

## 7. Risks and how to handle them

- **A/V sync with independent clips.** Picture and sound are separate clips; linked edits
  must move both by exactly the same amount. Covered by edit tests and the audio/render
  goldens.
- **Recorder pauses.** Recordings have gaps between segments; the starting sequence must
  create one clip set per continuous span, and trimming must not extend a clip into a pause.
- **The same moment twice.** With free clips, one source moment can play twice (duplicated
  clip). ClockMap returns lists; zooms, captions and focus draw at every occurrence; cut by
  words cuts every occurrence the selection covers.
- **Playback performance.** More tracks and clips per frame; decoding stays per stream with
  the existing caches; measure with the release probe used for shorts (target ≤ 30 ms/frame
  at preview quality).
- **Timeline UI size.** The rewrite is the largest piece; it is split into the parts in
  Phase 5 so each can be checked on its own in the browser preview with stand-in data.
- **Regressions in the priority workflows.** Media import, transcripts and jump cuts each
  touch several layers (import, stream ids, waveforms, mapping, edits). Their tests come
  first (Phase 0) and gate every phase (section 4.4); the stream-id change (`msound-…` and
  recording track ids → StreamRef) is done in one place and tested on every source kind.

## 8. Files touched (current tree)

Backend: `project/revision.rs`, `project/reader.rs`, `project/folder.rs`, `tracks.rs`
(replaced), `timeline/mapper.rs` (replaced by ClockMap), `export/mod.rs`, `render/mod.rs`
(small), `media/audio.rs`, `media/polish.rs`, `media_bin.rs`, `playback/mod.rs`,
`playback/engine.rs`, `zoom/mod.rs`, `webcam_focus.rs`, `captions.rs`, `chapters.rs`,
`shorts.rs`, `transcript/edit.rs`, `transcript/store.rs`, `commands/mod.rs`,
`commands/transcript.rs`, `project/silence.rs`, `project/waveform.rs`, `lib.rs`.

Front end: `lib/types.ts`, `lib/ipc.ts`, `lib/projectUtils.ts`, `lib/trackUtils.ts`,
`stores/projectStore.ts`, `components/timeline/*` (rebuilt), `components/inspector/*`,
`components/audio/*`, `components/media/MediaPanel.tsx`, `components/transcript/*`,
`components/silence-modal/SilenceModal.tsx`, `components/captions/*`,
`components/chapters/*`, `components/shorts/ShortsStudio.tsx`,
`components/zoom/ZoomPanel.tsx`.
