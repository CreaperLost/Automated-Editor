# Shorts: what is built and what comes next

Merged on 21 Oct 2026 from the shorts redesign plan (17 Oct) and the Shorts workspace +
voiceover shorts plan (18 Oct), which described the same `Short` model twice. Roadmap items:
[SHT-1](../done.md#sht-1) to [SHT-4](../done.md#sht-4); the shared dependency is source
search, [FTG-1](../done.md#ftg-1).

## The goal

**22 Oct: two features.** **Shorts** start from a take recorded outside the app and put anything
under it: the user's camera, the long video's graphics the agent picks (re-made for 9:16), new
graphics (news, a new feature, filler), the long video's footage, outside video as B-roll. This
comes first ([SHT-5](../done.md#sht-5), [SHT-6](../done.md#sht-6)). **Clips** cut the best
moments out of the long video (the idea-first model below, [SHT-1](../done.md#sht-1)). Both use
the one `Short` model and the same panels.

Start from an **idea**, gather every part of the video that serves it, cut it down to what is
essential, then dress it with graphics. And: make a short from a **new take** (camera + mic,
30–60 s) that retells one part of a long video, with the matching footage from the long video
under it (example: a "GTA 6 myths" video → one short per myth).

## Built

| Piece | Status | Where |
|---|---|---|
| Shorts as a workspace tab in the main window (the second window is gone) | Built 18 Oct | `dockLayout.ts`, `shortsStore`, `ShortsListPanel`, `ShortLookPanel` |
| Fast switching between shorts, no flash (Phase A) | Built 17 Oct | commit `68f10ab` |
| Face framing on camera slots (Phase B); since 21 Oct on **every** camera slot, video included, lining up every 30 s; hand framing per clip | Built | `face/`, `focus_uv`, layouts decision 5 |
| Word-range shorts that follow the video's edits, or get their own timeline once edited | Built | `shorts.rs` (`Short.source_start_us..source_end_us`, `ShortEdit`) |
| Layouts per short (base + sections, own timeline) | Built 21 Oct | `layouts/`, [layouts.md](layouts.md) |
| Graphic slots per short (full / camera / screen / over screen) | Built 15 Oct | `ShortLayout.graphic_slots` |
| **Take shorts (v1):** a take (camera + voice, or a recording) becomes a short with its own timeline; a separate mic is lined up by loudness-envelope correlation; footage from the video's edited timeline placed under spans of the take, ducked; camera-only where no footage plays; the take goes through transcribe → jump cuts → mouth noises (→ fillers) | Built 18 Oct | `take_short.rs`; MCP `short_from_take`, `short_footage`; template `take-shorts` |

## As built, 31 Oct

SHT-1's first part, inside today's model rather than the `Spine` rewrite below:
- `Short` gains `idea` (the premise, shown in the Clips tab's tooltip) and `pieces` (each a
  source stretch on the clip's `media` clock, in play order).
- `clips` `op: create` takes `idea` and `pieces: [{fromWord, toWord}]` (up to 12) instead of
  one `fromWord`/`toWord`. More than one piece gives the clip its own timeline at once
  (`ShortEdit`): `shorts::join_slices` plays each piece's stretch of the edited timeline, every
  track, one after another; a clip sliced twice gets a new id and link for the second.
  Overlapping pieces are refused. `source_start_us..source_end_us` spans the pieces (for
  sorting); `op: range` turns a gathered clip back into one stretch.
- Layouts are picked inside each piece only (`layouts_find_impl` takes the pieces), so a
  gathered clip's sections cover what it plays.
- The Clips method (`preset/methods/clips.md`) starts from ideas: one premise, the words
  that serve it from anywhere, hook first, shown with each piece.
- The Clips tab shows "N pieces · length" and the idea.
- Tested in the dev app: a 5:10–5:29 stretch then a 2:05–2:11 one, 24.9 s, storyboard in that
  order, its graphics at 9:16, sections 11.8 s inside it.

**Gather, later on 31 Oct:** words go into a clip from the video's transcript. Select them,
then "Add to clip…" in the selection bar: "New clip" makes one of them (titled with its first
words), a clip's name plays them after what it plays now (`project_clip_add_words`). Agents
do the same with `clips` `op: add` (`clip` and `pieces`). Both use `shorts::add_piece`: the
clip's own timeline (or its stretch) stays as it is, its own cuts too, and the new stretch is
appended as the video has it (`append_slices`, tracks matched by id); words it already has
are refused, as is going past 3 min. Tested in the dev app through DevTools: a new clip of
41 selected words, then 21 words added to "Fit test" (3 pieces, 29.8 s), the Clips tab
showing "3 pieces".

**Arranging pieces, later still:** a selected gathered clip lists its pieces in the Clips tab
("1. from 5:10.3 of the video, 21.9s"), each with earlier, later and remove; agents use
`clips` `op: arrange` with `order` (piece numbers from 1; one left out goes), and `clips`
`list` shows each piece's place in the video. `shorts::piece_spans` finds what each piece
holds on the clip's own timeline: from where its first kept moment plays to where the next
piece's does, so its own cuts, graphics and every track go with it;
`shorts::arrange_pieces` joins those stretches in the new order. A clip keeps at least one
piece and 3 s. Pieces carry their place in the video for the UI (`editedStartUs`, filled in
like a short's and cleared before storing). Tested in the dev app: piece 3 moved up, piece 2
removed (2 pieces, 21.9 s).

**The video's later edits, and the cloud find, last on 31 Oct:** every clip keeps its own
timeline, so the video's later edits reach a clip only through "Take the video's cuts again"
(`resync_short`). For a gathered clip that played everything from its first piece to its last;
now `shorts::video_sequence` rejoins each piece as the video has it, in the clip's order (a
piece the video trimmed plays what is kept, one it cut whole is left out). Agents have it as
`clips` `op: resync`. The cloud pass (`ai/shorts.rs`) asks for ideas with up to four pieces in
play order (one `from`/`to` still works); `shorts::with_found_clips` makes each with
`new_clip` + `add_piece`, and a pick that takes any piece of a clip already made is left out.
Not tried against a live model: the parser is covered by tests.

Not yet: no Tighten step of its own (the clip's transcript already strikes words); a graphic
cut by a piece's edge plays only its part, as at a single clip's ends.

## Not built

| Phase | What | Roadmap |
|---|---|---|
| D | Caption editing inside a short: breaks, emphasis, hidden words | SHT-2 |
| E | Graphics as the last ("Dress") step, proposed as a review batch | SHT-3 |
| V2–V4 | Footage picker in the UI, fit modes (speed ±25 %, freeze end), per-short ducking amount; the take model folded into the same `Short` | SHT-4, SHT-1 |

## The model (replace outright, no migration)

One `Short` for both kinds, so excerpt shorts and take shorts share Tighten, captions and Dress:

```rust
pub struct Short {
    pub id: String,
    pub title: String,
    pub idea: String,            // the premise / hook, also shown to the agent
    pub spine: Spine,
    pub broll: Vec<Broll>,       // footage under the spine's words (take shorts; optional for excerpts)
    pub cuts: Vec<WordRange>,    // words removed from the spine (Tighten)
    pub captions: CaptionShaping,// breaks, emphasis, hidden words (Phase D)
    pub layout: ShortLayout,     // background, captions style, graphic slots, framing
    pub layouts: LayoutTrack,    // base + sections, as today
    pub reason: String,
}

pub enum Spine {
    /// Passages of the long video, in play order.
    Excerpts { pieces: Vec<Piece> },          // Piece { id, media, source_start_us, source_end_us } snapped to words
    /// A take recorded for this short (camera + voice), an asset in the project.
    Recording { asset: String },
}

pub struct Broll {
    pub id: String,
    pub from_word: String, pub to_word: String,   // spine words: where it starts and ends
    pub media: String, pub source_start_us: u64,  // where in the source
    pub fit: Fit,                                  // Trim (default) | Speed (±25 %) | FreezeEnd
    pub slot: BrollSlot,                           // Screen (default in split) | Full
    pub sound: BrollSound,                         // Mute | Duck { db } (default Duck −20)
}
```

**The short's timeline is built, not stored.** Excerpts: for each piece in order, the parts the
main edit keeps (so the video's cleanup carries over), minus `cuts`, joined end to end. Recording:
the take minus `cuts` and its own silences on camera + voice tracks; each `Broll` a clip on the
screen track at its words' times, its sound ducked. Zooms, layout sections and graphics are
anchored in source time or to words, so they land right whatever the order, and tightening never
knocks footage out of sync. `ShortEdit` (the copied sequence) goes away; extra tracks a short
needs live in a small per-short overlay sequence anchored to words.

## The flow (Shorts workspace)

1. **Idea:** title and a one-line premise.
2. **Gather:** search the whole video's transcript ([FTG-1](../done.md#ftg-1)), add passages as
   pieces in any order; cut passages are greyed out and can't be added. For a take: pick or
   import the take; it is transcribed, framed and cleaned.
3. **Tighten:** the short's own transcript: strike words, drop sentences, a running length against
   the target (30–60 s), layout sections under it. Caption text fixes edit the shared transcript
   word; breaks, emphasis and hidden words are stored on the short.
4. **Dress:** graphics into the short's slots, caption style, background, export.

## Agent (MCP)

Meaning-based work (which passages make the idea, which footage shows the claim) is the agent's
job; the app gives cheap, exact tools.

- `clips` `op: create` (built 24 Oct with one stretch; pieces later) takes `title`, `idea` and `pieces: [{fromWord, toWord}]`, or `recording` for a
  take. `short_pieces` (`add | remove | move | set`) and `short_cut` (word ranges).
- `source_search` (FTG-1) to find passages and footage, including what the edit cut.
- `short_footage` grows into reviewable B-roll batches (`broll` items with fit, slot, sound) shown
  in the Agent panel like cuts.
- `short_captions` for breaks, emphasis and hidden words.
- Templates: `shorts.md` becomes (1) read the transcript, (2) propose 3–5 **ideas**, each with
  passages from anywhere and the hook line first, (3) user picks, (4) assemble, (5) tighten,
  (6) graphics into slots, (7) export waits for the user. `take-shorts.md` keeps its flow: read
  the take, split it into beats (hook, claim, evidence, verdict), find footage per beat (search →
  storyboard check, prefer action over menus, no repeats across shorts), propose, then graphics.

## Checks

`shorts::validate` gains: pieces non-empty, word-aligned, not overlapping within a short, total
length within limits, cuts inside the spine; B-roll inside the take, source range inside its
asset, no overlaps in a slot. Tests: building the timeline from pieces through a main edit (kept
parts, cuts, reordering), sections and zooms mapping onto reordered pieces, word anchors surviving
cuts, a preview-vs-export parity probe of a B-roll frame.

## Decisions

- **17 Oct:** the slow switching was between shorts (fixed); shorts use **only what the main edit
  keeps** (no raw pieces; restore a passage in the video first); caption text fixes are **shared**;
  one default length target, 30–60 s; gaming footage without a camera recording gets no face
  framing for now. ~~Face framing for shorts only~~, superseded 21 Oct: every camera slot.
- **18 Oct (take shorts):** footage only from what the video keeps (a stretch of the edited
  timeline, so its cuts carry over); footage sound ducked by default; the take is a camera with
  its own mic, a separate mic file, or a recording (the app tells them apart); split look by
  default, camera-only where no footage plays; the take goes through the normal cleanup, with
  auto-clean after transcribing on by default.

## Open

- Batch: one long video + N takes dropped in at once → N shorts.
- Does the old `ai/shorts.rs` cloud pass learn to return ideas with pieces, or does that become
  agent-only? (See [Q-AI-OWNER](../decisions.md#q-ai-owner).)
