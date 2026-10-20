# Shorts workspace + voiceover shorts: plan v1 (2026-10-18)

Two asks, in this order:

1. **Shorts becomes a workspace tab** (beside Edit and Cleanup) that swaps the panels in the
   main window, instead of opening the separate Shorts Studio window.
2. **Voiceover shorts**: make a short from a *new* recording (webcam + mic, ~30–60 s) that
   tells one part of a long video again, and have the agent fill the picture with the matching
   footage from the long video, then dress it with motion graphics.
   Example: "GTA 6 myths" long-form → one short per myth, each a fresh 30 s camera take, with
   the gameplay for that myth pulled from the long video.

Part 2 builds on part 1 (it lives in the Shorts workspace).

---

## Part 1: Shorts as a workspace

### Where it stands (from the code)

| Piece | Today |
|---|---|
| Tab | `EditorTopBar.tsx:187` — choosing "Shorts" calls `api.openShortsWindow()` instead of `setWorkspace` |
| Window | `open_shorts_window` (`lib.rs:1109`) opens a second Tauri window; `main.tsx` renders `<ShortsStudio/>` in it via `isShortsWindow()` |
| Studio | `ShortsStudio.tsx` (1,151 lines): its own dockview (`list`, `preview`, `settings`="Look", `timeline`), own layout key `aeroedits.shortsLayout.v1`, all state in one component |
| Project state | Its own JS store copy: `openedProject` is **replaced by the short's view** (`projectShortView` → `applyOpenedProject`); the whole video is kept in a local `main` |
| Playback | Window focus decides who owns the engine: `takeShortPlayback` on focus, `releaseShortPlayback` on close; `setEditTarget(shortId)` tells MCP/edits which timeline they hit |
| Sync | `windowSync` (`listenForProjects`, `listenForCaptionChanges`, `listenForProjectSwitches`) keeps the two windows' stores in step |
| Workspaces | `WORKSPACES = { edit, cleanup }` in `dockLayout.ts`; each has its own saved layout and preset; `workspaceStore` persists the choice |

### Design

**One window, three workspaces.** `WORKSPACES` gains `shorts` with its own preset. Switching
to it swaps the dock layout like Edit ⇄ Cleanup does.

**Panels.** ShortsStudio is split into dock panels registered in `DOCK_PANELS`:

- `shorts` — the list (new, rename, delete, check for export, export progress);
- `shortLook` — today's "Look" panel (split, captions, background, scenes, graphic slots);
- reused as they are: `preview`, `timeline`, `transcript`, `agent`.

Its state (selected short, draft layout, busy, exports, checked) moves out of the component
into a small `shortsStore` (zustand), so the panels share it without a context provider.
Default Shorts preset: list left, preview centre, Look right (agent behind it), timeline below.
Panels not used in Shorts (Jump cuts, Mouth noises, Zoom, Chapters…) still exist in the layout
(the "every panel is always open somewhere" rule) but sit in background tabs.

**What the shared panels show.** The rule stays what the Studio window did: *while in Shorts,
the store's `openedProject` is the selected short's view*, so the timeline, transcript and
preview edit the short with no changes to them. The store gains `mainProject` (today's local
`main`) so the list can place new shorts on the whole video.

- Enter Shorts: `setEditTarget(selected)`, `setViewShort(selected)`, fetch the view,
  `playbackFocusShort`.
- Leave Shorts: `setEditTarget(null)`, `setViewShort(undefined)`, apply `mainProject`,
  `releaseShortPlayback`.
- No short yet: the workspace shows the list with an empty-state ("New short" / "New
  voiceover short" / "Ask the agent").

**Preview.** One preview surface now (the wgpu-under-transparent-webview path has a single
host). It must draw 9:16 while a short is the view. Check what the Studio's preview does that
`StagePanel` does not (its own canvas ref + `useEngineFrames`, PreviewBar) and fold that into
`StagePanel` keyed on `openedProject.shortView`. **Biggest risk of part 1**; verify in the
real app, not just the browser preview (see driving-the-dev-app notes).

**Removed outright** (no compatibility): `open_shorts_window` and its init script, the second
window's capability entry, `isShortsWindow` in `main.tsx`, the focus-based engine hand-off,
and the `windowSync` listeners that only existed for the second window. Old
`aeroedits.shortsLayout.v1` is ignored.

**MCP.** Unchanged (`inShort` follows the edit target). Optional nicety: when the agent
creates or edits a short and the user is in Edit, show a "Open in Shorts" chip on the agent
batch instead of switching under them.

### Steps

1. `shortsStore` + `mainProject` in `projectStore`; enter/leave hooks driven by the workspace.
2. Split `ShortsStudio.tsx` into `ShortsListPanel`, `ShortLookPanel` (+ helpers file).
3. `WORKSPACES.shorts`, preset, `DOCK_PANELS` entries, migrations in `restoreLayout` for the
   new panel ids; top bar tab just calls `setWorkspace("shorts")`.
4. Preview 9:16 in `StagePanel`; delete the second window path.
5. Tests: store enter/leave (view + edit target restored), layout restore with new ids.
   Real-app check: switch Edit → Shorts → Edit with playback running; switching shorts speed
   (Phase A target <150 ms still applies); export a short.

Size: medium. Touches `ShortsStudio.tsx`, layout, stores, top bar, `lib.rs`, `main.tsx`.

---

## Part 2: Voiceover shorts

### Built 18 Oct (v1): "shorts from a take"

Built on the existing own-timeline short (`ShortEdit`) instead of the Phase C `Spine` model,
which isn't built yet; `Short.take` marks it. Code: `src-tauri/src/take_short.rs`.

- **Take** = camera picture (optional) + voice: any imported video (its own mic), a separate mic
  file, or an AeroEdits recording's webcam + mic. A separate mic is lined up with the camera's
  own sound by loudness-envelope correlation (5 ms steps, ±10 s, first 2 min); weak match → both
  start together and the user is told to check lip sync. Roles are set in the same undo step
  (camera → webcam so face framing finds it; voice → mic).
- **Its timeline**: Footage (screen) under Camera (webcam); Voice (mic) and Footage sound
  (background, ducked). Not magnetic. Captions read the take's transcript.
- **Footage**: placed over a stretch of the short (overwrites), from any media at a source time;
  optional background sound. Where no footage plays, automatic camera-only scenes; hand-set
  scenes stay.
- **UI**: Shorts panel → "From a take…" (pick/import camera + voice, title) → transcribes the
  voice. Look → "Take and footage" lists footage with remove.
- **Agent**: `media_import`, `short_from_take`, `short_footage` (add by take words + video words,
  remove, list), template `take-shorts`.

George's decisions (18 Oct), now built:

1. Footage only from **what the video keeps**: footage is a stretch of the video's edited
   timeline (`videoUs`), so its cuts carry over; a cut word can't be a source.
2. Footage sound **ducked** under the voice, on by default.
3. Take files: **either** a camera with its own mic or a separate mic file (or a recording);
   the app tells them apart (`take_short::guess`; the form does the same): picture = camera,
   sound-only file = voice, else a recording's mic, else the camera's own sound.
4. **Split look** by default (camera-only where no footage plays).
5. The take goes through the **general process**: on its own timeline, transcribe → jump cuts
   → mouth noises (→ fillers) → AI picks footage. Cleanup passes take a short target
   (`target_document`, `*_for` commands, `shortId` from the edit target); transcript commands
   find a take's short from its voice (`shorts::take_short_of`). Auto-clean after transcribing
   (form option, on). In Shorts: Transcript, Jump cuts and Mouth noises panels (for shorts cut
   from the video they point to Edit/Cleanup). Agent: `silence` with `short`; `mouth_noises`,
   `fillers`, `transcribe` with the take's voice as `stream`; template `take-shorts`.

Not done yet: footage picker in the UI (only the agent places footage), speed/freeze fit modes,
per-short ducking amount.

### The idea in app terms

Today a short is *a stretch of the long video* (`Short.source_start_us..source_end_us`, or a
copied `ShortEdit` sequence). A voiceover short flips the spine:

- **Spine** = the new recording: its sound is the short's audio, its picture is the camera
  part. Its transcript drives timing and captions.
- **B-roll** = stretches of the long video (or any project asset) placed under spans of the
  voiceover's words, shown in the screen part (or full frame).
- **Dress** = graphics anchored to voiceover words ("MYTH #1", "BUSTED" stamp, etc.) through
  HyperFrames, in the existing graphic slots.

Pieces we already have: assets can be imported (`media_bin::import`); any sound stream can be
transcribed (`transcribe` picks by stream key); face tracks + `focus_uv` frame the new camera
take (Phase B); scenes (camera / screen / split) and graphic slots per short; `frame_render`
and `storyboard` let the agent *see* the long video; proposals let the user review the agent's
picks.

### Data model (replace, no migration)

Fold this into Phase C of `shorts-redesign.md` so there is one model, not two:

```rust
pub struct Short {
    pub id: String,
    pub title: String,
    pub idea: String,
    pub spine: Spine,
    pub broll: Vec<Broll>,       // empty for excerpt shorts unless the user adds some
    pub cuts: Vec<WordRange>,    // words removed from the spine (Tighten)
    pub layout: ShortLayout,
    pub reason: String,
}

pub enum Spine {
    /// Phase C: passages of the long video, in play order.
    Excerpts { pieces: Vec<Piece> },
    /// A recording made for this short (webcam + mic). Lives in the project's assets.
    Recording { asset: String },
}

pub struct Broll {
    pub id: String,
    pub from_word: String,       // voiceover word ids: where it starts and ends
    pub to_word: String,
    pub media: String,           // source asset (usually the long video's screen/game capture)
    pub source_start_us: u64,    // where in that asset; length follows the word span
    pub fit: Fit,                // Trim (default) | Speed (stretch ±25%) | Freeze end
    pub slot: BrollSlot,         // Screen (default in split) | Full
    pub sound: BrollSound,       // Mute | Duck { db } (default Duck -20)
}
```

**The short's timeline is built, not stored:** spine recording (minus `cuts`, minus its own
silences) → camera + audio tracks; each `Broll` → a clip on the screen track at its words'
times, its source sound on a ducked audio track. Because b-roll hangs off words, tightening
the voiceover never knocks footage out of sync. Gaps with no b-roll fall back to a camera-only
scene (or hold the last frame; setting on the short).

### User flow (Shorts workspace)

1. **New voiceover short** → pick the take (file dialog or drag from Media). Imported as an
   asset, marked as this short's camera, transcribed with the saved transcriber, face track
   computed.
2. **Tighten the take**: silences and fillers in the take are cut automatically (reuse the
   jump-cut and filler detectors on the take's stream), shown struck through in the short's
   transcript; the user can restore any.
3. **B-roll**: select words in the short's transcript → "Add footage" → a source picker:
   search the long video's transcript / chapters, scrub its frames, set the in point. The
   agent can fill all of it at once (below) and the user reviews the batch.
4. **Dress** (Phase E): graphics on words, captions, background, export.

### Agent (MCP)

The app ships no model (AI-readiness rule): matching meaning is the agent's job; the app gives
it cheap, exact tools.

- `short_create` gains `recording: <asset id or path>` (imports + transcribes if needed).
- `transcript_get` with `short` returns the take's words; `transcript_search` (Phase C) finds
  where the long video talks about the same thing ("myth", "Vice City map", "leak").
- `storyboard` / `frame_render` on the long video to check the footage actually shows it
  (talking about a myth is not the same as showing it).
- `broll_propose { short, items: [{fromWord, toWord, media, fromUs | fromWord(main), fit, slot, sound}] }`
  → a reviewable proposal batch in the Agent panel, like cuts today. `broll_list`,
  `broll_remove`.
- New template `skill/templates/voiceover-short.md`: (1) status + transcribe the take,
  (2) read the take, split it into beats (hook, claim, evidence, verdict), (3) for each beat
  find the source footage: transcript search → chapter → storyboard check, prefer action over
  menus/talking, no repeats across shorts, (4) propose b-roll, (5) after OK, graphics through
  HyperFrames on the beat words (title card on the hook, verdict stamp), (6) storyboard the
  short's 9:16 frames and fix, (7) export waits for the user.

Batch idea for later: one long video + N takes dropped in at once → N shorts, one per take.

### Phases

| Phase | What | Depends on |
|---|---|---|
| V0 | Part 1 (Shorts workspace) | — |
| V1 | `Spine::Recording`: import + transcribe + face-frame a take; short plays camera-only | V0, ideally with Phase C's model change |
| V2 | `Broll` model + timeline build + preview/export parity; manual "Add footage" picker | V1 |
| V3 | MCP `broll_*`, `short_create { recording }`, `voiceover-short.md`, proposals UI | V2 (tools), Phase C `transcript_search` |
| V4 | Take cleanup (silence/filler on the take), ducking, fit modes, graphics step | V2 |

Tests: timeline build from spine + b-roll (word anchors survive cuts, gaps fall back, ducking
gain), validate (b-roll inside the take, source range inside the asset, no overlaps in a
slot), parity probe rendering a b-roll frame in preview vs export.

### Open questions for George

1. **B-roll from raw footage or only what the long edit kept?** Recommend raw: gameplay under a
   cut silence is still good footage. (Differs from shorts decision 2, which is about speech.)
2. **Game sound under the voice:** muted or ducked? Recommend ducked to −20 dB.
3. **The take:** one file with webcam + mic, or the AeroShoot recorder's separate camera/mic
   streams? Both can work; affects import.
4. **Default look:** split (camera top, footage below) like today, or full-frame footage with
   the camera as a bubble?
5. **Auto-clean the take** (cut silences/fillers on import), or leave it raw until asked?
