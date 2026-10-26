# 11 — The gate already reads pixels

- **Source:** our own source — `src-tauri/src/graphics/gate.rs` (`GATE_VERSION = 2`), read 25 Oct 2026
- **Evidence grade:** **verified in this repo.** Every claim below is from the source, not from a
  capture. This file corrects [02](02-model-free-graphics-checks.md) and
  [07](07-motion-taste-checklist.md), which both under-describe the gate.
- **Why it is a finding and not a note:** the gate is the thing the whole field is converging on —
  every pack in [10](10-the-rest-of-the-field.md) ships a "deliver-and-verify loop". Knowing
  precisely which half of that loop we already own changes what we should build next.

## The correction

Findings 02 and 07 both say the graphics gate is **file-level**: lint, runtime errors, layout
overflow, contrast. That is true of the first half of the gate and false of the second.

`gate.rs` is **two gates in one**. The first reads the graphic's files. The second renders staged
copies of the graphic through the pinned HyperFrames and **measures the resulting pixels**. The
module says so in its own first line: *"The gate looks at the graphic's files, then at staged
copies of it rendered through the pinned HyperFrames."*

The rendered half measures **frame coverage** (how much of the frame the card covers, at two alpha
thresholds), **timing** (whether it is see-through at both ends and shown in between), **cut
behaviour** (whether the first and last frames show the card whole), **whether the graphic's own
`data-overlay` mark matches how much of the frame it actually covers**, and **whether each string
variable really changes the picture** (a pixel difference between two renders).

Finding 02's central claim survives intact — *"It checks nothing about whether the graphic is well
placed in the edit"* — and finding 07's claim that the gate cannot judge motion quality also
survives. But the reason given in both was wrong, and the correction changes what we should build.

## What the gate actually checks

**The file half** (`static_checks`):

| id | Check | Fails when |
|---|---|---|
| `network` | No network | any `http(s)` or protocol-relative reference (XML namespaces and the CDN GSAP link are exempt) |
| `brandColours` | Brand kit colours | `background`, `text` or `accent` is not declared as a colour variable |
| `portrait` | Portrait layout | *warns* if there is no `@media (orientation: portrait)` block |
| `look` | The video's look | a **colour of its own** is written in the file; a font, radius or CSS timing is a **warning** |
| `fixedText` | No fixed text | *warns* on visible text that no variable fills |

**The pixel half** (rendered through HyperFrames, then measured):

| id | Check | What it measures |
|---|---|---|
| `hyperframes` | HyperFrames check | lint, runtime errors, contrast — at 16:9 and 9:16, with the graphic's own values **and with long test values for every string variable** |
| `fitsWide` / `fitsTall` / `fitsSplit` | Fits 16:9 / 9:16 / a short's split part (1080×864) | nothing leaves its box or the frame |
| `lengthShort` / `lengthLong` | Follows a 3 s / 8 s length | see-through at the start, shown in between, see-through at the end (unless it holds to the end) |
| `cut` | Cuts in and out | the first and last frames show the card whole when `data-entrance="cut"` |
| `overlay` | Full screen or overlay | the measured coverage matches the `data-overlay` mark |
| `variables` | Text variables change the picture | two renders with different values differ by more than 100 pixels |

The thresholds are explicit constants, not magic: `SEE_THROUGH_MAX = 0.005` (a frame is see-through
at ≤0.5 % covered), `SHOWN_MIN = 0.02`, `FILLS_MIN = 0.9` (fills the frame at ≥90 % covered),
`OVERLAY_MAX = 0.6` (below 60 % it is an overlay), `CUT_SHOWN_SHARE = 0.5`, `CHANGED_PIXELS = 100`.

It also returns a **contact sheet** of both shapes, keeps results in the render cache **keyed by
the source hash** so an unchanged graphic is not re-checked, and bumps `GATE_VERSION` when the
checks change so stale results are not reused.

## The sharper framing: there are three gates, and we have two

This is the part worth carrying forward.

| | what it asks | status |
|---|---|---|
| **Gate 1 — the files** | is this graphic written as it should be? | **have it** |
| **Gate 2 — the graphic alone** | does it render the way it claims to? | **have it** — and it is more than the field's best reference |
| **Gate 3 — the edit** | does it sit correctly in *this* video? | **missing** |

Gate 3 is where every check in [01](01-treatment-model-and-visual-lead.md) and
[02](02-model-free-graphics-checks.md) lives: the overlay must not reach into the face box, the
anchors must land in gaps between words, nothing may be drawn outside a planned scene, and the
rhythm warnings (8 s of only the speaker, a full-frame graphic shorter than 3 s or longer than 8 s,
two in a row, never opening or closing on the speaker).

`gate.rs` is **completely isolated** from the face and transcript modules — the only occurrences of
"face", "yunet", "transcript" or "word" in the file are comments about how to build a long test
string. It has no idea what video it is being put into.

**So the gap is narrower than finding 02 says, and the work is smaller.** We are not missing the
ability to read pixels; we are missing a gate that knows about the edit. And the pieces are already
present, exactly as finding 02's table says — YuNet face tracks, transcript word ids, graphics
anchored to words, the timeline. What is missing is the **comparison**, not the machinery.

The same correction applies to finding 07. The gate cannot judge motion quality — not because it
is file-level, but because **it has no opinion about curves**. It can prove a graphic is see-through
at its first frame and shown at its middle one. It cannot say the entrance *eased* rather than
cut, because easing is not visible in a three-still sample. That is a sampling limit, not a
philosophy, and it changes what the fix looks like: more frames, or a rule about the timing tokens
themselves — not a new kind of checker.

## What this changes in the plan

- **[10](10-the-rest-of-the-field.md)'s benchmark reads differently.** In the field's own test,
  every mechanical check was free — all seventeen runs passed, including no skill at all. Only the
  look separated them. Our position is the same shape but further along: we are strong on
  mechanical checks (gates 1 and 2) and we have no taste check at all. **The gate is not our
  differentiator; the style is.** That is an argument for putting effort into the look, not the
  linter.
- **GFX-5** (styles proposed by the agent, Next) depends on the `look` check to be meaningful: it
  is the thing that decides whether a proposed style actually reaches every colour, font and
  timing in a graphic. The check is already there; the agent-proposed styles are what would feed
  it.
- **[07](07-motion-taste-checklist.md)'s severity-tiered checklist** is still the right shape for
  the missing taste checks — but it is a *third* gate's worth of rules, not an extension of a
  file-level one.
- The two accessibility rules in finding 07 — a `prefers-reduced-motion` alternative, and nothing
  over 5 s that cannot be paused — remain unconsidered anywhere in the project. They are still
  cheap and still get expensive to retrofit.

## Caveats

- This is a reading of the source, **not a run of the gate**. No graphic was rendered through it
  during this research, and the thresholds were not tuned or validated against a real video.
- `GATE_VERSION = 2` and the source is moving; the check list here is accurate as of 25 Oct 2026
  and should be re-read before it is used to plan work.
- The pixel half needs Chrome and the pinned HyperFrames, so it is slower and more fragile than the
  file half. The gate renders up to 3 Chrome instances at once with a 180 s timeout per run.
- "Measures coverage" is not "measures taste". Every check here is mechanical by construction,
  which is the point — but it means gate 2 can certify that a graphic is correct and say nothing
  about whether it is good.
