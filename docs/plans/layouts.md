# Layouts: plan v1 (2026-10-21)

Task 1 in [future-work.md](../future-work.md#1-more-layouts). Status: **plan, waiting on the
open decisions at the end.** No code yet.

Goal: a small set of named layouts (camera bubble, side by side, camera only, screen only) that
work in 16:9 and 9:16, changed over time by **layout sections** that stay on their content
through cuts, with a **base layout** that fills every gap. One system replaces today's global
layout, webcam focus and the shorts' scenes. The presenter cut-out is task 9 (the backlog says
"needs task 10"; it needs task 9's matte).

## What exists today

| System | Where | What it does | Anchoring |
|---|---|---|---|
| Global layout | `project/layout.rs` `EditLayout` | One look for the whole video: canvas, background, screen crop/scale/corners, the bubble's shape, size, position, border, mirror, plus telemetry polish (cursor, badges, auto-hide, window crop) | none: global |
| Webcam focus | `webcam_focus.rs` | Segments where the camera grows from the bubble to a centred rect (`focus_size_pct`, 40-100 %). Auto from mouse rest + speech, manual add/remove, `normal_view` ranges that pin the bubble. Eases in and out *inside* each segment | source time of the recording's **camera** clips (`focus_clock` = `role_clock(Webcam)`) |
| Shorts scenes | `short_scenes.rs`, `shorts.rs` `ShortLayout` | A short is a split (camera top/bottom at `camera_pct`); scenes switch to camera only or screen only, easing the seam. `suggest` picks them from telemetry; take shorts fill camera scenes where no footage plays (`take_short::camera_scenes`) | source time of the recording or take voice (`asset_clock`) on the short's own timeline |

How they are drawn (`export/mod.rs` `SceneEvaluator::scene_at`, shared by preview and export):

- Video: `Scene::from_layout_scaled` lays out background, screen and bubble, then
  `apply_webcam_focus` lerps the bubble's rect, radius, UV and border toward the focus rect by
  `focus_weight`, then `bubble_hide` fades it.
- Short (`document.short_layout` set): a separate `split_scene` computes `camera_share` from
  the scenes, `split_rects_at`, `screen_placement` (cut to the slot, following zooms) and
  `camera_framing` (face focus or hand framing), then places captions between split and
  full-frame spots.
- Ranges are mapped once per evaluator (`OnceCell`); each frame is a binary search and a lerp.

Other readers of focus and scenes, which must move too: `graphics::full_camera_ranges`
(graphics ease in over a full camera), `sound_effects` (whooshes on changes), `bubble_hide`,
`check.rs` (frames at layout switches), `mcp/storyboard.rs`, `templates.rs`, MCP
`webcam_focus` / `short_scenes` / `settings`, the Inspector's `WebcamFocusSection`, the
timeline's focus lane, and `ShortLookPanel`'s scenes block.

What already works and should stay: anchoring in an asset's time through
`sequence::clock` (survives ripple trims, reorders and undo); auto vs manual with
`replace_auto` (re-picking never touches the user's own); everything derived at render time,
never written back.

## Lessons taken from Tella and Borumi

- Tella's kinds and placement rules shape the kinds below and the task 11 hooks. Its
  transitions are `hardCut` or `spring` per layout; ours are per section, with an automatic
  default.
- Borumi's pitfalls, and how this design avoids each:
  - **Drift after ripple trims:** sections are stored in source time and mapped through the
    clock on every read. A trim changes the mapping, not the section.
  - **Gaps flash a default layout:** there is no default. A gap shows the *base* layout,
    which the user chose. Slivers too short to read are absorbed when resolving (below).
  - **Deleting one layout rebuilds others:** sections are independent records. Removing one
    removes that id. The derived timeline is recomputed but never stored, so nothing else
    changes on disk.

## Model

### Style vs layout

`EditLayout` today mixes two things. Split them:

- **Style** (stays per project, one per timeline): canvas aspect, padding, background, screen
  crop, screen scale, screen corner radius and shadow, camera border colour/width, camera
  mirror and shadow, cursor, shortcut badges, bubble auto-hide, window crop. Shorts keep their
  own background and caption settings as today.
- **Layout** (what sections switch): how screen and camera are arranged. The bubble-only
  fields (`webcam_shape`, `webcam_size*`, `webcam_position`, `webcam_custom_*`,
  `webcam_roundness_pct`) move into `Layout::Bubble`. `webcam_enabled` goes away: "no camera"
  is a `ScreenOnly` base.

Old project files are not read (no migration); fields are simply removed.

### Layout kinds

```rust
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Layout {
    /// The screen in its styled frame, the camera in a bubble over it.
    Bubble { position: BubblePosition, size_pct: f32, shape: CameraShape, roundness_pct: f32 },
    /// Camera and screen in two slots: left/right in landscape, top/bottom in portrait.
    SideBySide { camera_first: bool, camera_pct: f32, style: SideStyle /* Even | Overlap */ },
    /// Only the camera: full bleed at size 100, or centred over the background below 100
    /// (today's focus size); `punch_in` > 1 zooms in on the face.
    CameraOnly { size_pct: f32, punch_in: f32 },
    /// Only the screen.
    ScreenOnly,
}
```

The same layout resolves for either canvas, so a favourite works in the video and in shorts:

| Kind | 16:9 | 9:16 |
|---|---|---|
| Bubble | Screen framed by the style (scale, padding, corners); bubble at a corner or custom spot | Screen fills the frame, cut to its shape and following zooms (today's screen scene); bubble on top |
| Side by side | Camera left (`camera_first`) or right, `camera_pct` of the width. Even: slots side by side. Overlap: the camera panel overlaps the screen's edge | Camera on top (`camera_first`) or bottom, `camera_pct` of the height: today's short split |
| Camera only | Cover, face-framed; centred rect below 100 % | Cover, face-framed |
| Screen only | Screen framed by the style | Screen fills the frame, following zooms |

Rule for the screen slot: **fit** (letterboxed, background around) where the slot has
the recording's shape, **fill** (cut to the slot, following zooms, `shorts::screen_window`)
where it doesn't. Portrait slots always fill; open decision 9 is about 16:9 side by side.

Missing streams resolve down: no camera at that moment gives screen only, no screen gives
camera only. Gaming footage without a camera needs nothing special.

### Layout tracks and sections

```rust
pub struct LayoutTrack {
    pub base: Layout,
    pub sections: Vec<LayoutSection>,
    /// Default move length for `Transition::Auto` / `Move` without its own.
    pub transition_ms: u32,
}

pub struct LayoutSection {
    pub id: String,
    /// The asset whose time anchors it; `None` is the first recording.
    pub media: Option<String>,
    pub source_start_us: u64,
    pub source_end_us: u64,
    /// `Base` pins the base layout here (replaces webcam focus `normal_view`; auto picks give
    /// way to it).
    pub layout: SectionLayout, // Base | Layout(Layout)
    pub transition: Transition, // Auto | Cut | Move { ms: Option<u32> }
    /// Picked by a rule; picking again replaces these and keeps manual ones.
    pub auto: bool,
    /// Which rule picked it (task 11 explains its plan with this).
    pub rule: Option<String>,
    /// Which camera fills the camera slot; `None` is the one on the timeline (task 10 hook).
    pub camera: Option<String>,
    /// Filled in for the UI, never stored.
    pub edited_ranges: Vec<EditedRange>,
}
```

- `EditDocument.layouts: LayoutTrack` replaces `webcam_focus`. Each `Short` gets its own
  `layouts: LayoutTrack` on its own timeline (base: side by side, camera on top at 35 %).
- **Anchoring:** the section's `media` asset clock, `asset_clock` (any clip of the asset), as
  chapters and shorts use. When added from the UI, `media` is the asset of the screen clip
  under the selection, not blindly the first recording. A selection over a cut is stored as
  one section when its source hull maps back to the same edited range (the cut material maps
  nowhere). Otherwise it is stored as one piece per run, sharing a group id.
- **Validation:** ids unique, half-open ranges, at most 1,000 sections, no overlap within one
  `media`.
- **Known limit:** where one moment of an asset plays twice, the earlier clip owns it (clock
  rule). The repeat shows the base layout.

### Resolving: sections to a layout timeline

`layouts::resolve(track, clock) -> Vec<Shown { start_us, end_us, layout, transition }>`, pure
and computed once per evaluator, does the following:

1. Map each section through its clock. Pieces of one section that touch (a cut inside it) merge.
2. Sections on different `media` that overlap on the timeline: manual beats auto, then the
   later start wins.
3. **Sliver rules**, which are derived and never stored:
   - a base gap shorter than `MIN_BASE_GAP_US` (1 s) between two sections is absorbed by the
     earlier one, or merges them when their layouts are equal;
   - a section piece shorter than `MIN_SHOWN_US` (1 s) after cuts is not shown. It stays in the
     data and comes back if the cut is undone. The lane draws it hatched ("too short after
     cuts").
4. Everything not covered is the base.

Every consumer reads this one list: renderer, graphics `full_camera`, sound effects,
bubble-hide, check, storyboard, MCP `list`.

### Transitions

- `Cut`: switch on the frame.
- `Move`: interpolate the two resolved arrangements (below) over `ms`, eased with
  `zoom::cubic_bezier_unit`.
- `Auto` (default): a cut when both sides are full-frame shots (camera only ↔ screen only), or
  when the edge lands on a clip boundary (a jump cut), since a move across a jump cut looks
  broken. Otherwise a move. This is Tella's rule.
- **Where the move plays:** inside the section, as webcam focus does today. A section eases in
  over its first `ms` and back to the base over its last. Between two sections that meet, the
  move plays over the start of the later one. Pieces shorter than two moves shorten them
  (`min(rise, fall)`, as now). (Open decision 4.)
- The shorts' "captions move between split spot and full-frame spot" becomes: captions follow
  the current arrangement's seam or full-frame spot, eased with the same weight.

### Favourites and built-ins

- **Built-ins** (always there, not editable): Bubble M bottom left, Side by side, Camera only,
  Punch-in, Screen only. Each is resolved per canvas, so the same five work in shorts.
- **Favourites:** `{ id, name, layout: Layout, slot: Option<u8 /* 1-9 */> }` in the app's config
  folder (`layouts.json`, next to `templates.json`, with the same atomic write and
  `AEROEDITS_TEMPLATES_DIR` override). They are shared by every project.
- Applying a favourite **copies** its `Layout` into the section and remembers its name for
  display. Editing or deleting a favourite later never changes a project. (Open decision 2.)
- Templates (`templates.rs`, task 6) hold the style and the **base** layout. Favourites hold
  arrangements only.

## Rendering: one path

Replace `from_layout_scaled`'s webcam geometry, `apply_webcam_focus` and `split_scene` with:

```rust
/// Pure geometry: where each slot goes for `layout` on this canvas.
fn arrange(layout: &Layout, style: &Style, canvas: (u32, u32), has: Streams) -> Arrangement;
struct Arrangement { screen: Option<Slot>, camera: Option<Slot>, caption_spot: CaptionAnchor }
struct Slot { rect: [f32; 4], radius: f32, clip: ClipMode, fit: Fit /* Fit | Fill */,
              border: bool, shadow: f32, opacity: f32 }
fn lerp(a: &Arrangement, b: &Arrangement, t: f32) -> Arrangement;
```

- Per frame: find the shown layout(s) and weight with a `partition_point`, then `arrange` both
  ends and `lerp`. Then the evaluator fills in per-frame UVs for the lerped rects: screen
  crop, zoom follow and pan for a fill slot (`screen_window`), `focus_uv` with
  `camera_framing` for a camera slot. This generalises both of today's eases.
  `apply_webcam_focus` is a bubble→camera-only lerp; `camera_share` is a side-by-side→one-slot
  lerp.
- A slot that exists on only one side: the camera fades and scales from 0.85 in place. The
  screen stays under a growing camera until it is covered, then drops.
- Layer order, roles, borders and shadows stay the existing `Layer` path, so GPU/CPU parity
  holds. Add a parity test for a mid-move side by side.
- Shorts stop being a separate path: `short_layout` keeps only short-only things (background,
  captions, screen zoom/pan, hand-framed clips, graphic slots). `graphic_slots` refer to the
  **base** arrangement's slots, as they now use the split at the short's own share.

## Webcam focus and shorts scenes fold in

No old-file migration; the code moves:

- `webcam_focus.rs`: `WebcamFocus`, its segments, `normal_view` and `focus_weight` go. The
  detector, `detect_focus_ranges` (mouse rest + speech), stays as the **rest rule** of `pick`.
  It makes auto `CameraOnly { size_pct: focus_size_pct }` sections anchored on the recording.
  Its settings (`idle_ms`, `require_speech`, ...) become the rule's settings. `normal_view`
  becomes manual `Base` sections. The master on/off switch goes: clearing auto sections does
  the same, and the base is always there.
- `short_scenes.rs`: `Scene`/`SceneMode` go. `suggest` becomes the **busy rule** (screen only
  while clicks, scrolls and typing come fast), plus the **hook rule** (camera only for the
  opening line, shorts only), plus the rest rule with shorter thresholds, as now.
  `replace_auto`, `merge` and `subtract` move to `layouts`.
- `take_short::camera_scenes` becomes the **no-footage rule** for take shorts: camera only
  wherever no screen footage plays.
- `ShortLayout` loses `camera_position`, `camera_pct`, `scenes`, `scene_transition_ms`.
- Readers (`graphics::full_camera_ranges`, `sound_effects`, `bubble_hide`, `check`,
  `storyboard`) switch to `resolve`. `bubble_hide` applies only while the shown layout is a
  bubble.

## MCP

There is one new action, `layouts`, so `mcp/actions.rs` gets one `ToolDef` and the code lives
in a new `mcp/layouts.rs`. `webcam_focus` and `short_scenes` are removed. `settings area:
layout` keeps the style fields; `area: webcam_focus` goes. Times are seconds on the target's
own timeline, as `short_scenes set` uses now.

| `op` | Does |
|---|---|
| `list` (default) | Base; each section with id, from–to, layout summary, auto/manual and rule, transition, "not shown: too short after cuts"; built-ins and favourites with their numbers |
| `add` | `sections: [{fromS, toS, layout, transition?, camera?}]`, all in one undo step. `layout` is a built-in name, a favourite name or id, `"base"`, or an inline spec. New sections trim overlapping manual ones (new wins), auto ones give way |
| `update` | `id` plus any of `layout`, `fromS`/`toS`, `transition` |
| `remove` | `ids`, or `fromS`/`toS` to cut a stretch out of whatever covers it |
| `base` | Set the base layout |
| `pick` | Auto sections from the rules (rest, busy, hook, no-footage) with optional rule settings; replaces auto sections and keeps manual ones; `apply: false` only reports |
| `clear` | `auto` (default) or `all` |
| `save` / `forget` | Save a favourite from a section `id`, `base` or a spec, with `name` and optional `slot` 1-9 / delete one |

- `target`: `video` (default), or a short's id or title.
- The storyboard tile gains the layout name, so an agent can check its plan in one image.
- Skill: `tutorial.md`, `shorts.md` and `take-shorts.md` swap `webcam_focus` and
  `short_scenes` for `layouts pick`.
- Merge note: the 20 Oct uncommitted work in the main checkout edits `mcp/actions.rs` and
  `mcp/mod.rs`. This plan touches them only to swap two ToolDefs for one, delete the two old
  functions, and drop one `settings` branch. Land it after that work is committed, then
  rebase.

## Hooks for task 10 and task 11

- **Task 10 (number keys):**
  - Built-ins and favourites have a `slot` 1-9, which makes the numbered palette.
  - A core `switch_at(target, at_us, layout)` gives key semantics as in multicam editing: from
    `at_us` to the next section edge, the layout becomes N, splitting the stretch it lands in.
    MCP `layouts op: switch` is the same call.
  - `LayoutSection.camera` chooses a camera when there are several.
  - While playing, switches show at once through the look-only swap (performance, item 1)
    and commit as one undo step on stop.
- **Task 11 (auto layouts):**
  - `pick` is a list of rules over telemetry, speech and face tracks. Sections carry `auto`
    and `rule`.
  - Room is left for Tella's limits: at most N changes a minute, and length windows per kind
    (screen 2-8 s, camera 2-5 s, punch-in 1.5-4 s).
  - `camera_first` can come from the face track's gaze side.
  - The app proposes and the agent reviews with `list` and the storyboard.

## UI

Mockups in `output/ui-review` give the style only.

- **Timeline Layout lane** under Zooms, replacing the webcam focus lane:
  - one block per shown section with a kind icon; base gaps stay empty;
  - auto sections lighter than manual; pieces too short to show are hatched;
  - click to select, drag edges to trim (live preview, one edit on release), Delete removes;
  - a timeline selection plus a tile click adds a section.
  - Built as memoized blocks like `ClipBlock`, drawn only near the view, and never subscribed
    to `currentTimeUs`.
- **Inspector "Layout" section:**
  - Style controls as today.
  - Below them, a strip of tiles (built-ins, then favourites, numbered 1-9). The tiles are
    static SVG diagrams, not rendered frames.
  - With a section selected: its kind's parameters and its transition, plus **Save as
    favourite**.
  - Without one: **Set as base**.
  - A **Pick layouts** button opens a popover with the rule settings, replacing
    `WebcamFocusSection`.
- **Shorts tab:** the same lane on the short's timeline and the same strip. `ShortLookPanel`'s
  Scenes block goes.
- Favourites are managed (rename, renumber, delete) from the strip's menu.

## Performance risks

Follows the preview-performance rules.

1. **Look edits rebuild playback.** Any edit bumps `content_generation`. `playback/engine.rs`
   then builds a new `SceneEvaluator` *and* a new `AudioMixer` and drops the last frame. That
   is fine for occasional edits, but wrong for dragging a section edge and for task 10's live
   keys.
   - **Mitigation:** a separate look generation. Edits that change only layouts or style swap
     a new `Arc<ResolvedLayouts>` and style into the running evaluator, which resets only its
     layout caches. The mixer, decoders and last frame stay.
   - Dragging previews on the front end and commits on release.
2. **Per frame:** one `partition_point`, two `arrange` calls and a lerp of at most two slots,
   which is negligible. The resolved list is built once per evaluator, never per frame.
3. **Decoder restarts at section edges:** hidden slots keep decoding, as `split_scene` does
   today, so a screen-only → bubble edge never starts a decoder. Skipping the screen during
   long camera-only stretches waits until it is measured, and would then lean on `prefetch`.
4. **Face framing per frame:** `camera_framing` must hit a per-evaluator cache keyed by clip,
   not re-read the face track. It now runs for the video too if decision 5 says so.
5. **Camera at full frame in preview** decodes at the preview limit and looks soft. This is
   unchanged from today's focus. Export decodes full size.
6. **Measure:** `perf.rs` `perf_preview` and `perf_steps_back` before and after on a real
   `.aero` with about 30 sections. No regression is expected in p50/p95 frame time; add a
   case that toggles layouts during playback (look swap, no rebuild).

## Phases

1. Model, `resolve`, `arrange`/`lerp`, video renderer, focus rules folded in; unit and parity
   tests.
2. Shorts on the same path (scenes, take shorts), the other readers, templates.
3. MCP `layouts`, storyboard, skill docs.
4. UI: lane, strip, inspector, favourites, Shorts tab.
5. Look-only swap in playback, then perf measurements.

Each phase builds and passes `cargo test` and the front-end type check. George tries it in the
app after phases 1, 4 and 5.

## Open decisions for George

1. **Style shared, layouts only arrange?** One style per timeline (background, borders,
   corners, mirror), and layouts and favourites hold only the arrangement. *Recommended.* The
   alternative, a style per layout, is Borumi's model and is heavier to edit.
2. **Favourites copied or linked?** Copied into the section when applied, so editing a
   favourite later doesn't touch old projects. *Recommended.*
3. **Default transition:** `Auto` (cuts between full-frame shots and at jump cuts, a 350 ms
   move otherwise), or today's default, a hard cut everywhere?
4. **Where a move plays:** inside the section (today's focus behaviour; the section's look is
   complete only after the move), or centred on its edge?
5. **Face framing in the main video:** side by side in 16:9 cuts a 16:9 camera to about 8:9,
   so centre crops cut people off. Turn on face framing for the video's camera slots too? It
   is shorts-only today by your 17 Oct call. *Recommended: yes for side by side and camera
   only; the bubble keeps the centre crop.*
6. **Shorts keep their own base and sections** (*recommended*, since shorts are mostly split
   and the video mostly bubble), or inherit the video's?
7. **Remove outright:** `webcam_enabled`, the webcam focus on/off switch, and the MCP
   `webcam_focus` and `short_scenes` actions, all replaced by `layouts`. OK?
8. **Bubble placement:** today's 4 corners plus custom and a size slider, or Tella's 9
   positions and S/M/L?
9. **Side by side in 16:9:** screen *fit* (whole screen letterboxed in its half, background
   around), or *fill* (cut to the slot and following zooms, as in shorts)? *Recommended: fit
   by default, fill as an option.*
10. **Number slots (for task 10):** built-ins fixed on 1-5 and favourites on 6-9, or you
    number everything?
11. **Merge order:** land this after the 20 Oct work in the main checkout is committed, since
    both touch `mcp/actions.rs`?
