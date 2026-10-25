# What creators do with AI editing: the video research, condensed

One digest of the 20–21 Oct 2026 video research: the 19 AI-editing and motion-graphics videos in
the [dossier](evidence/video-dossier-2026-10-21.md), plus the short-form tutorials (6), Dan
Kieft's production workflow, Roboverse's prompt-to-motion demo, Nate Herk's two guides, two
ChatCut case studies and five X posts on tutorial motion graphics. It replaces eleven overlapping
review docs, the second-opinion critique and the per-video deep dives.

**Trust.** Captions are speech-recognition text (brand names mangled: "Appus", "Davinchi"); they
show what creators *say*, never visual quality. Creator times, costs and "one prompt" claims are
claims. Seven of the 19 videos still have no full caption track (YouTube IP-blocked the fetch;
[re-run the script](evidence/scripts/README.md) with a long `--sleep`). Only corrected facts are
used below; see [Corrections](#corrections-that-change-conclusions).

## The ten things the research actually established

1. **The model directs, something else renders.** In every motion-graphics video the LLM plans,
   writes code or drives a tool, and something deterministic draws the pixels (HyperFrames,
   Remotion, After Effects, Blender, plain JS). That is our shape: we render, the agent chooses.
2. **The artefact is the product, the model is a commodity.** Every creator sells or publishes
   *something*: a skill folder or zip (Tao), a 1,400-line skill file (Christian Peverelli), a
   prompt Google Doc (Zane Hoyer, "refined for $700"), a Claude Artifact (Jason Cooperson), a PDF
   (RoboNuggets, Rourke), an editing skill **sold together with an SFX library** (Higgsfield).
3. **The method lives in a file the user owns and the agent improves.** Christian: "save the
   correction in the skill file". Jason and Zane: the prompt is a versioned, compiled artefact.
   Our `SKILL.md` is compiled into the app and reinstalled; the user can't own it. This is the
   sharpest structural difference between our loop and the strong examples → [AGT-3](../done.md#agt-3).
4. **Feedback is anchored to the timeline, not phrased in chat.** Mike Cann (Tandem) leaves a
   comment on a time range that Claude reads; Tao labels clipped words and a repeated zoom at
   49–59 s, rejects "boring" as feedback, and fixed a HUD with a Paint sketch after words failed
   → considered as NOTE-1, dropped 24 Oct: one user feeding back live in chat doesn't need it
   ([decisions](../decisions.md#not-doing)).
5. **A reference is mandatory, and it should be small and real.** Screen-record only the section
   you want to mimic (Science Based AI); record your own product rather than invent it (Matt
   Loui); give the agent concrete asset files, not an inspiration board it can't resolve (Ben
   Stevenson, Jay E). For tutorials, our own frames and telemetry are the first evidence.
6. **Ask before the long job.** Matt Loui spends two minutes answering the agent's questions on
   style, pace, colour and length; Ben puts "ask questions if needed" in the prompt because a
   render takes ~20 min. Our skill already asks with numbers; the lesson is to ask about *style*
   up front too.
7. **Sound is placement and art direction, not library breadth.** Zane maps *his own* SFX library
   onto the timeline by keyword (claims ~40 min saved per video, placement only); Ben places
   ElevenLabs SFX at the exact frame an element pops, because the animation is code, and states
   the style in words: "keep it pretty minimal and subtle … no heavy risers or drops". Joseph's
   starter kit: five whooshes; two or three each of mechanical, sci-fi and UI; two risers; two
   hits → [SND-1](../done.md#snd-1), [SND-2](../roadmap.md#snd-2).
8. **The output must stay editable in the host app, and the agent stops at a reviewable state.**
   Every AI-editing creator ends in an editable timeline or project; the failure they advertise
   against is "AI made it and I can't touch it". The human keeps style, grade, final trim and
   what to keep. Our proposal/approval model is this, built in.
9. **Everyone duplicates the project before an agent touches it** (every Resolve creator) →
   [SAFE-1](../done.md#safe-1), [SAFE-2](../done.md#safe-2).
10. **Review by watching, with a human in the loop.** "Codex checks against the storyboard,
    renders, checks the render" (Chase AI), but the same creator warns against autonomous loops
    and their time cost; Jason's agent loops "until the cut is clean". Stills can't judge motion
    and the agent can't hear, so the user listens to the joins the agent picks
    → [REV-1](../done.md#rev-1), [Q-VERIFY](../decisions.md#q-verify).

What the research did **not** establish: that one model has better taste (no experiment held
subject, assets, skill and budget constant), that deterministic rendering gives better *visual*
quality (it gives control), or that viral-reel polish is the right yardstick for tutorials.

## Six ways the creators work

| Archetype | Who | What it means for us |
|---|---|---|
| **A. The agent drives a host app's UI** (computer use) | Rourke (AE), Higgsfield, Greg, Pat Simmons (Premiere) | We are the opposite: a typed API. Their trick survives as "a screenshot is a layout spec": `frame_render` / a state screenshot |
| **B. The agent edits a real NLE through its API** | Joris (Resolve MCP vs invideo: "the prompt didn't matter, the host did"), Zane, Jason | Prompts as compiled artefacts; the host's surface quality decides the result |
| **C. The agent writes the motion graphics** | Tao, Science Based AI, Chase AI, Marcin, Matt Loui, RoboNuggets, kaye | Style → topic → script → iterate → render → sound. Generate shots, not whole videos |
| **D. The session becomes a skill** | Christian, Jason, Mike Cann (Tandem), RoboNuggets | User-owned, agent-improved method file; comments as the review unit |
| **E. The agent does sound** | Zane (own library), Science Based AI (ElevenLabs at the visual peak), Christian (Epidemic Sound MCP) | Placement from a library the user owns; style in words; a loudness target (Zane balances dialogue to ~−16.5 LUFS) |
| **F. The vendor ships the whole loop** | Higgsfield (grade in Resolve, graphics in AE, sound, long → vertical → trailer), invideo | Brand assets and templates uploaded per session: style travels with the job |

## Tricks worth stealing, ranked by cost to us

| # | Trick | Source | Where it lands |
|---|---|---|---|
| 1 | Anchor feedback to a time range, not a chat message | Tandem; Tao | Not doing (24 Oct): chat is enough for one user |
| 2 | Feed the agent a small, exact reference (a frame, a 5 s clip), never a whole file | Science Based AI | Skill guidance; `frame_render` |
| 3 | Ask questions before the long job | Matt Loui; Ben Stevenson | [AGT-3](../done.md#agt-3) / templates |
| 4 | Art-direct sound in words; place it at the visual peak | Ben Stevenson; Zane | [SND-2](../roadmap.md#snd-2), [STY-1](../done.md#sty-1) |
| 5 | Keep the prompt/method as a versioned file the agent edits | Christian, Jason, Zane | [AGT-3](../done.md#agt-3) |
| 6 | Filter style references by what the model can actually execute ("Opus isn't good at mimicking Kurzgesagt") | Science Based AI | Gate the style trial with `graphic_check` |
| 7 | Approve a 5–10 s sample before scaling a look | ChatCut / Marcin | [PLAN-1](../roadmap.md#plan-1) |
| 8 | Keep the result composable: green-screen one element, or two transparent PNG layers you can recolour | Science Based AI; Higgsfield | Expose layers, not the flat frame ([DLV-3](../roadmap.md#dlv-3)) |
| 9 | Let the model animate; render hard images once as an asset sheet | Tao ("great at animating, not at rendering realistic images") | Supplied assets in graphics |
| 10 | Separate the brief into **requirements** (format, deliverables), **preferences** (taste) and **analysis** (transcript, telemetry) | Short-form review part 2 | [STY-1](../done.md#sty-1) |
| 11 | Keep spoken editor directions ("cut this", "add a graphic here") as notes that survive cleanup | Dan Kieft | Not now (24 Oct, [decisions](../decisions.md#not-doing)) |
| 12 | Revise on a copy; say what to change *and what to keep* ("reduce the bounce, keep the glow") | Roboverse | [GFX-4](../roadmap.md#gfx-4) |

## Tutorial graphics: three workflows and a production contract

From the tutorial motion research (five X posts, their linked repos, and our target: AI
tutorials in English and Greek). Use these when building [GFX-3](../roadmap.md#gfx-3) and
[PLAN-1](../roadmap.md#plan-1).

1. **Direct the viewer through a real screen** (find a control, choose a setting, notice an
   output): the smallest intervention (zoom alone, a highlight, a short label, a detail inset);
   keep the target visible while the action happens; keep English UI words exact inside Greek
   narration. Never an invented screenshot.
2. **Explain a concept with a small animated sequence** (an agent loop, retrieval, tool calls):
   write the one sentence the viewer should understand; build a literal state sequence first;
   reveal each state at its spoken phrase; hold the finished diagram long enough to read.
3. **Show a result or comparison** (before/after, two outputs): real inputs, equal framing,
   measured values only; in portrait, stack or sequence rather than shrink.

**Per-graphic contract:** its teaching purpose; transcript word ids for entry, emphasis, hold and
exit; approved copy, real assets and the source of any factual claim; layout mode (replace,
overlay, inset, short slot) and target region; the style reference and exact traits to borrow;
Greek/English font requirements; separate landscape and portrait decisions; what a later
revision must preserve; evidence from checks, stills and moving playback.

**Greek checks:** real copy with tonos, diaeresis, final sigma, punctuation and mixed English
product names; the intended font bundled (never judge a fallback); review any case conversion by
eye; long headings and real UI/code strings in both ratios.

**Shorts are their own composition:** fewer simultaneous elements, explicit camera/caption
clearance; don't force a visual change every two seconds when the viewer needs to read or act.

Useful references with real source: [mg-styles-15](https://github.com/Vincentwei1021/mg-styles-15)
(15 styles with prompts, source and a `renderAt(t)` renderer),
[lemo-opuscar](https://github.com/lemomo-ai/lemo-opuscar) (51 film styles),
[claude-animation-skill](https://github.com/buildwithhanif/claude-animation-skill) (contact
sheets, transition strips, out-of-order determinism checks, staged rendering),
[LottieFiles motion-design-skill](https://github.com/LottieFiles/motion-design-skill) (timing and
choreography guidance), [pdoom-video](https://github.com/mexicat/pdoom-video) (word-synchronised
typography).

## Short-form specifics

From six short-form tutorials (Joseph's Opus reel, two Higgsfield-era reel guides, an Opus
talking-head edit, an Astra/Borumi comparison, a Claude + HyperFrames short):

- One complete, consistent style beats a big catalogue. Build one, measure it, then broaden.
- Joseph's caption spec (a medium sans plus a script font for key words, white plus one accent,
  word-by-word pop-in) is **one creator's taste**; the transferable rule is *"highlight the one
  word that makes you keep watching"*: emphasis chosen by meaning, the agent's job →
  [CAP-1](../done.md#cap-1).
- A visual needs an explanatory job; presenter-only moments are a deliberate choice.
- Brief compliance ≠ taste ≠ execution quality: an agent can follow a restrained brief and still
  disappoint a creator who wanted energy. Judge them separately.
- Some looks need compositor work over real footage (dim/blur behind an overlay, a presenter
  cut-out, gaze-aware cutaways); a graphic can't blur what is composited under it later
  → [FX-2](../done.md#fx-2), [FX-1](../roadmap.md#fx-1).

## Production workflows (Dan Kieft, Roboverse)

- **Kieft** (*Master AI Video Editing in 25 Minutes*): keep spoken editor directions as notes,
  plan visuals per beat with a target slot, prepare an asset request (selected words, interval,
  references, duration, what must be preserved), attach the result, review in context, place as
  one undoable action. First release: a manual export/attach round trip, providers later.
  Anchors must use clip occurrences plus word ids; never reuse an old absolute timestamp after
  the edit changed.
- **Roboverse** (*Claude Code Can Now Automate Your Videos*): brief by purpose and second-by-second
  behaviour → editable composition → preview → revision on a copy → reuse in a longer piece →
  render queue of named deliverables. Remotion in the video; nothing in it requires Remotion.

## Numbers worth remembering (all creator-reported)

| Claim | Source | Caveat |
|---|---|---|
| 12 min → 2:34; first draft in 49m30s, **3–4 h** with feedback rounds | Tao | The 49 min is not total time |
| 76 s → 50 s, 12 animations, 25 min of sound design, ~110 min elapsed, 20 min of his own work, $1–5 | Joseph | Subscription-allocation estimate, not an invoice |
| 15 s of AE motion in 22m56s | Chase AI | — |
| Same prompt, opposite verdicts on Astra vs Fable | Chase AI vs Marcin | Different subjects, assets and settings: not a controlled test |
| Fable finished for £5.90 vs Astra $15.05 | one A/B | Currencies differ; not "double" |
| A motion-graphics run ≈ 2 % of a $100/month ChatGPT plan, ~10 min | Rourke | — |
| "What used to take me 16 hours now takes 2 to 3"; first pass 22 min; one-shot long form 2 h 42 min; 3 months, 86k lines, 335 commits | Jason Cooperson | From his description |
| Reformat with linked labels and cursor targets: 50 min vs 2–3 h by hand | Higgsfield commercial | Dependency-preserving reformat, not a canvas resize |
| $6.40 host usage so far for an intro | Marcin (ChatCut) | Excludes ChatCut charges |

Measure our own instead: cost per accepted video, by stage ([EVAL-1](../done.md#eval-1)).

## Artefact index

The reusable things the creators point at (from the dossier; free unless noted).

| Artefact | Link | Note |
|---|---|---|
| HyperFrames | github.com/heygen-com/hyperframes | Our runtime's upstream |
| Tandem | github.com/mikecann/tandem | MIT editor with MCP and timeline comments |
| Claude Motion Graphics Skill (Tao) | drive.google.com/drive/folders/17OlOeC9SEPTezAqzlJnIuqAdQL2DFTKE | A skill zip |
| Editing Skill (Christian Peverelli) | skool.com/wearenocode | 1,400 lines, self-rewriting; closest to our `SKILL.md` |
| Blueprint artifact (Jason Cooperson) | claude.ai/artifact/BYH6Mcma5to3kt2KogkEzB | His project as an artifact |
| Master prompt for a DaVinci edit (Zane) | docs.google.com/document/d/1gPOosPSFHd3p_WuC83j7rW24QbErRQpv5IKNKs1GuAw | A Google Doc |
| Prompt kit (Matt Loui) | docs.google.com/document/d/1cnVw2cUQXnNARu9ta0-wepbVZyFUi3g7lPBnxPtDa2k | A Google Doc |
| Higgsfield Editing Skill + SFX Library | higgsfield.ai/s/astra-for-editing-higgsfieldai-XqLQyi | Skill and sounds as one product |
| Blender MCP server | blender.org/lab/mcp-server | The honest answer to "can we do 3D?" |
| Herk Edit Kit | github.com/nateherkai/herk-edit-kit | See [competitors](competitors.md#herk-edit-kit) |

## Corrections that change conclusions

Earlier review docs got these wrong; everything above uses the corrected version.

- **Two taxonomies were conflated.** Joseph's *animation subjects* are text / person / object /
  graphic; his *scene choices* per line are captions, captions with UI, half-screen, full-screen.
  "Two of four scene archetypes have no template" was built on that mix-up and is withdrawn.
  Replace "archetype coverage" with **observed tutorial jobs**.
- **"106 sound effects"** and Joseph's **"12 hours / 4.5B tokens"** are not in his captions.
- **Video 18 is Mike Cann's Tandem**, not a Convex product; its creator isn't "a Convex dev".
- **Zane's sound video maps his own SFX library**; the ElevenLabs/Epidemic mechanism came from
  other uploads.
- **CapCut has no MCP** in Jason's setup ("Claude clicks through the UI"); three MCP hosts, one UI host.
- **`H6lBZ7AfDDE` (Science Based AI) and `AsvN_feasyY` (kaye.creatives)** are two different videos
  with near-identical titles; the first was missed by the first pass.
- **Our own code was misdescribed** in the ten-video review: there is no `project_view`,
  `project_save_as`, `proposals_undo` (it's `undo_batch`), `project_diagnose` or `aeroedits://`
  handler; `transcript_cut_words*` and `transcript_caption_edit` are UI commands, not agent tools;
  alignment is wav2vec2, not Whisper; `propose_edit` *does* place, move and trim clips.
- **Borrowed style bans are wrong for tutorials:** "no holds over 1 s" fights reading time; Ben
  Stevenson *recommends* blur-ins.

## Evidence and how to re-fetch it

Raw material lives in [evidence/](evidence/): `transcript.md` (captions of the original eleven),
`transcripts-wave2-2026-10-21.md` (three tracks captured later), `video-dossier-2026-10-21.md`
(all 19: verbatim descriptions, every link, chapters, metadata) and
`nine-ai-editor-videos-transcripts-2026-10-21.md` (third-party timestamped extracts for videos
whose captions are still blocked). Regenerate with the scripts in
[evidence/scripts/](evidence/scripts/README.md). Quote descriptions and links, never caption spellings.
