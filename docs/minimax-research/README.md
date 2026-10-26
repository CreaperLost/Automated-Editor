# Research findings — AI motion graphics & explainers

A research wave into how other people make agents produce motion graphics and explainer video,
so AeroEdits can take the best ideas without adopting their mistakes.

**Scope.** References captured 25 Oct 2026. Every claim about another product is a *vendor or
creator claim* unless it says "verified in this repo" — nothing here was installed or benchmarked
against AeroEdits. Where a claim was checked against our own source, it says so.

**This folder.** One file per finding. The raw captures these were read from are in
[evidence/](evidence/) — metadata, the README verbatim, the licence file and the file tree for each
reference. The capture table with a verdict on each is [evidence/INDEX.md](evidence/INDEX.md).

**Re-capturing.** `scripts/intake.py` fetches a reference the same way every time:

```bash
python3 docs/minimax-research/scripts/intake.py <url> [<url> ...]
```

GitHub's unauthenticated API allows 60 requests an hour and will start refusing mid-run — set
`GITHUB_TOKEN` in the environment to avoid that. YouTube rate-limits hard and the block lasts for
a whole session; `--meta-only` still gets the description and the links when captions are refused.

## The findings

| # | Finding | Source | Licence |
|---|---|---|---|
| 01 | [Three treatments per line, and visual lead frames](01-treatment-model-and-visual-lead.md) | iart-ai/b-roll-skills | MIT |
| 02 | [Model-free graphics checks and rhythm thresholds](02-model-free-graphics-checks.md) | iart-ai/b-roll-skills | MIT |
| 03 | [The shot recipe-card format](03-shot-recipe-card-format.md) | Vincentwei1021/video-shotcraft | Apache-2.0 |
| 04 | [The script as truth, and the anti-slideshow position](04-script-as-truth-and-anti-slideshow.md) | Vincentwei1021/video-talkcraft | PolyForm Noncommercial — study only |
| 05 | [A style is data plus a guide](05-style-as-data-with-a-guide.md) | moritzkremb/framejam | MIT |
| 06 | [Licence landscape: what we may actually reuse](06-licence-landscape.md) | all captures | — |
| 07 | [Motion taste as a severity-tiered checklist](07-motion-taste-checklist.md) | LottieFiles/motion-design-skill | MIT |
| 08 | [Taste-neutral skills, and video over MCP](08-taste-neutral-skills-and-video-mcp.md) | Remotion / Motion / motionvid | vendor claims |
| 09 | [A new competitor: OpenChatCut](09-competitor-openchatcut.md) | 0xsline/OpenChatCut | AGPL-3.0 |
| 10 | [The rest of the field, and what is left to read](10-the-rest-of-the-field.md) | awesome lists, iart-ai packs | CC0 / MIT |
| 11 | [The gate already reads pixels](11-the-gate-already-reads-pixels.md) | verified in this repo | — |
| 12 | [HyperFrames is our runtime, and it now ships agent guidance](12-hyperframes-is-our-runtime.md) | heygen-com/hyperframes | Apache-2.0 |
| 13 | [The LLM reads the video, it never watches it](13-the-llm-reads-the-video.md) | browser-use/video-use | MIT |
| 14 | [The schema is derived from the code, and the evals publish their failures](14-ffmpeg-skill-contract-and-evals.md) | kajisho5/ffmpeg-skill | MIT |
| 15 | [The provenance record is written before the clip](15-broll-provenance-before-the-clip.md) | edigeh/broll-mcp | MIT |
| 16 | [Every choice priced before the render, every render checked after](16-kitcut-priced-choices-checked-renders.md) | kitcut-hq/kitcut | **none — refused for reuse** |
| 17 | [What the seven agree on, and the one thing they say we have not built](17-what-the-seven-agree-on.md) | this batch | — |
| 18 | [Three more director layers on top of our own runtime](18-three-more-director-layers.md) | motion-director · hyperdirector · xilo-opus-video | AGPL / Apache-2.0 / MIT |
| 19 | [The folder is the prompt](19-the-folder-is-the-prompt.md) | Zane Hoyer, 24 Oct 2026 — a creator workflow | — |
| 20 | [The runtime's author, on the record](20-the-runtimes-author-on-the-record.md) | Jake Moran (HyperFrames) on Peter Yang's channel, 25 Oct 2026 | — |
| 21 | [The agent writes the host's project files, then writes itself a skill](21-agent-writes-the-project-files.md) | Matt Loui, 23 Oct 2026 — CapCut + Claude | — |
| 22 | [The closest thing to AeroEdits that is open source (and MIT)](22-the-open-source-mirror.md) | sirioberati/YouTube-AI-Video-Editing-System | MIT |
| 23 | [The style is derived by measuring the frames, and then it gets a name](23-the-style-is-derived-measured-and-named.md) | Jad M.H, 21 Oct 2026 — Editor V1, on our own runtime | bespoke licence — free incl. commercial, no resale |
| 24 | [The vendor's own numbers, and three defect classes only a human caught](24-astra-the-vendors-own-numbers.md) | Higgsfield AI / Adil, 2 Oct 2026 — Astra + Resolve | vendor claims |
| 25 | [The kit behind the video: a derived style, written down as a spec](25-the-kit-behind-the-video.md) | NextPhase Editor V1 skill kit, 79 files, 21 Oct 2026 | bespoke licence — usable outright |
| 26 | [The style is a program: tokens, seeds and a frame grid](26-the-style-is-a-program.md) | The same kit, read as code | — |
| 27 | [The skill Higgsfield publishes is one editor's personal process, in Russian](27-the-personal-skill-higgsfield-publishes.md) | YT_video_editor_skill, 11 files, the artefact behind the Astra video | no licence — study the method |
| 28 | [21 named styles, two pixel checks, and notes written against frame numbers](28-21-styles-and-two-pixel-checks.md) | Claude Video 2.0 + Motion Lab, on our runtime | **MIT** |

## The short version

The closest thing to AeroEdits' thesis we found is **`iart-ai/b-roll-skills`** (MIT): it decides
per line whether the viewer sees the speaker, a graphic beside them, or a full-frame graphic, and
it draws every graphic from what the speaker actually said. Two ideas in it are worth more than the
rest of this folder put together — a **visual lead rule** (structure lands a few frames before the
word, numbers land on the word, conclusions land on the reveal) and a **self-check that reads the
pixels** (does the overlay touch the face box? is the rhythm right?) rather than the plan. Both map
onto machinery AeroEdits already has: word ids, YuNet face boxes, and a gate that already reads the
rendered pixels — but only about the graphic on its own, never about the edit it goes into
([11](11-the-gate-already-reads-pixels.md)). The richest *format* we found is the recipe card
in **`video-shotcraft`** (Apache-2.0) — it documents what **wrong** looks like, which is the one
thing you cannot prompt into a model. And the richest *method* we found, **`video-talkcraft`**,
cannot be reused at all: it is PolyForm Noncommercial.

The field's own benchmark says why this matters. Sixteen video skills and a no-skill baseline all
made the same 15-second film, and **all seventeen passed every mechanical check** — the only things
that separated them were time, sound and *the look*, and six of the sixteen looked like no skill at
all ([10](10-the-rest-of-the-field.md)). Correctness is free. Taste is the whole difference, and a
gate that only lints will never catch it.

The second wave says the same thing from six more directions, and names the gap precisely. The
rule every one of them keeps is **the check must read the output, not the plan** — video-use at
every cut boundary, kitcut on the finished render, framejam on the render, ffmpeg-skill on every
write, broll-mcp on the decoded clip. We have that for the graphic; we do not have it for the cut
([17](17-what-the-seven-agree-on.md)). But the artefact is nearly there: our `storyboard` tool
already renders the finished edit as one PNG — a tile per scene, with what is said — and it simply
does not mark where a cut starts.

And the one thing nobody else does is ours. Framejam keeps the look in a preset file, HyperFrames
in a design file, the iart-ai packs in the prompt, video-use in the agent — **we keep it in the
app**, which is the only way every graphic in one video matches every other. Two of our own
dependencies now argue with us about it: our runtime ships its own look reference and a
`DESIGN.md` format ([12](12-hyperframes-is-our-runtime.md)), and three more director layers have
grown on top of that runtime ([18](18-three-more-director-layers.md)). That collision is new, and
it is worth having.

And the one workflow in this folder with **no check at all** is the one that makes money
([19](19-the-folder-is-the-prompt.md)). Zane Hoyer's setup — a folder per client, style
references, a brand kit, and one short prompt — produces the quality, and nothing verifies the
result. That is the argument for the gate we have not finished: **a check is what makes a setup
amortisable.** It is what lets a client folder be built once and then edited for months without
anyone re-reading the brief.

And the last one is the runtime's own author. **Jake Moran is on the HyperFrames team** (a HeyGen
employee), and he answers the question GFX-5 was blocked on — *"research how they do it before
building this"* — with the three inputs, confirmed one by one: a board of reference images, their
brand, a tuned prompt as a skill ([20](20-the-runtimes-author-on-the-record.md)). Two of those we
already have as planned items. The third does not: **his brand comes from the website, and ours
is assembled by hand.**

And the fourth wave makes the sharpest statement of a rule we already decided on. Matt Loui has
Claude editing **a real CapCut project by writing its files** — *"Do not use computer use or open
CapCut"* ([21](21-agent-writes-the-project-files.md)). No MCP, no UI, no clicking. **What makes a
host editable by an agent is not whether it has an MCP; it is whether its project is a writable
file.** That is our "no computer use" decision, arrived at from the other side. And the same video
answers the repo's sharpest structural gap: the agent diffs your manual tweaks and **writes itself
a skill** — which for us is the difference between two checkpoints.

And the fifth wave is the most useful one in the folder, because it is **MIT**. Sirio's *Eyeline*
is a structural mirror of AeroEdits — seven per-stage skills, JSON schemas, a
`config/workflow.json` of workflow stages, `qa_cut.py` and `qa_edges.py`, 267 HTML reference
boards, a teleprompter that records the *data* an editor needs ([22](22-the-open-source-mirror.md)).
It also has the one number nobody else here has: **a 3 % clipping error rate against 20–30 %** for
"ask Claude to transcribe and start cutting" — because the cut comes from **predetermined data,
not an AI watching an hour of footage**. Its boards expose `window.seek(seconds)`, which is the
clean answer to the sampling limit in our own gate, and its second transcription pass over the
assembled cut is gate 3, done with a transcript instead of pixels.

And the sixth wave found the closest thing to AeroEdits in the field — **and it runs on our own
runtime.** Editor V1 is a Claude skill for HyperFrames, the engine we already ship at 0.8.162
([23](23-the-style-is-derived-measured-and-named.md)). What it does is derive a style the way our
roadmap's GFX-5 asks: **measure two or three references frame by frame, turn the measurements into
thresholds, and give the result a name** — 175 picture changes a minute, at least half the reel
moving rather than still, most shots pushing in, every new shot *"starts blurry and goes sharp"*,
and a picture held for four seconds *"counts actually as a mistake"*. Those are not adjectives.
They are guardrails. Two of them we do not have at all: **we have no maximum hold anywhere** — our
gate reads a graphic's pixels and never how long it is held — and **no focus pull**, because the
only blur in the app is a drop shadow. We also record no source and no licence for any asset we
put on screen, and our default transcriber is a paid cloud API although we ship the better local
model.

And the seventh wave is a known reference — Higgsfield's Astra is in the 21 Oct transcript — but
the presenter reports his own numbers, and the numbers are the point
([24](24-astra-the-vendors-own-numbers.md)). **He tracks two clocks**: how long the process takes,
and how much of *his* attention it costs. We measure neither. His second clock has a magnitude:
a 49-second commercial came back with **12 seconds of air** in it, and a professional with a
frontier agent in DaVinci Resolve **still trimmed the pauses by hand** — a quarter of the runtime,
which is our own `gaps.md` naming the pause-and-filler pass as the fragile one, now with a number
attached. And the three defects he lists are *all* cross-boundary: a moment that *"doesn't carry
through the cut"*, shots that *"pass way too quickly"*, and *"Hear that pop? Terrible."* **We
synthesise the audio of an export and never listen to it.** The same wave also contains the other
side of the computer-use question, from the vendor itself: *"Back up your projects, review
permissions, and keep sensitive apps closed."*

And then the kit came out of the Drive folder, and it is the best artefact in the folder
([25](25-the-kit-behind-the-video.md)). **Eighty files, on our own runtime, with a derivation
record** — the method, the raw numbers, and a section headed *"What is not measured (decisions)"*
that says out loud which parts are the author's taste rather than the references' facts. That
separation is the most transferable idea in the whole folder: **a derived style has to record what
was measured and what was chosen.** It ships the plan and the build code for the first 45 seconds
of the very video in finding 23 — so the kit *is* that edit, published. It carries a bespoke
licence: free including commercial, **but not to be resold or repackaged as a paid product**,
which is aimed squarely at us. And it has three checks, the last of which is the cheapest gate 3
in the folder — **re-transcribe the final and diff it against the take**, which caught a real
defect in a real build (*"Claude did."* came back as *"Flaw dead"*, because a drop sat on the
consonant). We already call the same command the kit's agent looks through —
`hyperframes snapshot --at <a,b,c>` — but we point it at a *graphic* rather than at the *edit*,
and we have `FaceBox` with an `eye_y` that nothing checks against.

And then the kit read as code, which is where the engineering is
([26](26-the-style-is-a-program.md)). **The style is a program**: ~30 tokens in a CSS file, each
one annotated with the measurement or the decision behind it; a 188-line treatment engine where
every transform is seeded and deterministic; a device library where **each device writes its own
HTML, its own GSAP and its own sound cues**, so a picture and its click cannot drift apart. The
biggest gap it exposes is one of verbs: **we check pixels, they transform them.** We measure a
graphic to decide whether it is acceptable; we never grade a still, a clip or the footage, where
their rule is *"Every found image is treated."* Three more things worth having: the ASR reports
**the first word after a pause 60–220 ms early**, and they snap it to the silence we can already
detect; **the bed cuts where we duck**, because a duck leaves the consonant audible over a moving
bed; and their install **proves itself** — a 7-second render from pictures it draws in code, plus
a sheet of one frame per beat. Two renderer rules to check on our side: durations must be
**floored**, not rounded, because the renderer ceils; and imported footage wants **30 fps CFR with
`-g 30`**, because long-GOP and VFR sources render black or frozen.

And the same URL turned out to have an artefact of its own
([27](27-the-personal-skill-higgsfield-publishes.md)). The description promises an *Editing Skill
+ SFX Library*; what ships is **`YT_video_editor_skill`** — eleven files, **in Russian**, no sounds
in it, and **no licence**. It is *"Max's personal editing process"*, written for Higgsfield's own
tutorial episodes, with **the author's own final cut as its only reference**. Two corrections
follow: the earlier wave's note that taste and assets are *sold as one download* does not hold for
this artefact — the download is the skill alone; and the irony is exact, **the skill Higgsfield
wrote for the public carries a licence and the one that is actually their working method does
not.** The useful parts are not the code — there is none — but three rules. It **names the defect
class we cannot see**: *"blinking"*, the speaker opening for a few frames between two inserts,
which a duration list cannot catch because *"a long lower clip masked the problem in the list of
lengths"* — the fix is to measure the **visible frame**, not the clip. It **arrives at four seconds
independently**, on a different corpus, in a different language, which makes ~4 s the field's
number rather than one person's taste. And it **records its own verification gap** — the initial
analysis *"did not include continuous listening of all 15 minutes, so the next final check must
explicitly account for that gap rather than inherit the claim 'everything was watched'"** — while
keeping the user's words, an observation and its own hypothesis apart, and refusing to turn its
medians into a norm for the next episode.

And then the best artefact in the folder turns up
([28](28-21-styles-and-two-pixel-checks.md)): **Claude Video 2.0**, **MIT**, 21 named styles, and
**built on HyperFrames — our runtime**. It is the first fully permissive licence in the field, and
the first complete implementation of **gate 3** anyone here has published. Its two checks read the
*pixels of the rendered video*: a **pacing check** (*no dead gap over 1 s, nothing on screen under
0.3 s, nothing that pops in and out*) and a **loop check** that renders the frame at 0 s and the
frame at the end and fails if they differ, with the seam measured against the median frame step —
**both built on `hyperframes snapshot --at 0,D`, the exact command our gate already calls, pointed
at a whole project instead of a graphic.** Each style is a **`STYLE.md` spec** — palette in hex,
type, easing as names and seconds, 4–8 motion rules, 2–4 signature moves, a composition and a
**ban list** — and the person picks one from a **gallery of real videos**. And its review notes
have the shape our learning loop is missing: **`frame 84: <what is wrong>: <the number that
changes>`**, one or two per round, re-render only the frames named, *"two to four rounds is
normal"*. Its origin, Motion Lab, states the whole thesis in one line: **"Unspecified choices
fall back to one look"** — the model renders perfectly; the problem is that every open decision
collapses into the same style. One thing to check on our side: the skill documents that
**HyperFrames rewrites system font names**, and our templates name *"SF Pro Display"*.
