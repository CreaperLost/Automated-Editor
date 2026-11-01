# AI video editing: what the research established

Consolidated 30 Oct 2026 from every research source in `docs/`. This is the evidence base for
[requirements.md](../requirements.md). It decides nothing; decisions stay in
[decisions.md](../decisions.md). Per-product detail stays in [competitors.md](competitors.md);
raw captions and ingests in [evidence/](evidence/).

**Folded in and deleted on 30 Oct:** the MiniMax and Gemini per-video write-ups (the same twelve
videos, twice, plus four Gemini ecosystem deep-dives), `creator-workflows.md` (the earlier
19-video digest) and `ai-styles.md` (the style-intake research). They are kept in git as
`refs/backup/research-before-synthesis`:
`git show refs/backup/research-before-synthesis:docs/<path>` reads one. Our own measurements come
from [EVAL-1](../plans/evaluation.md) and the 25 Oct graphics test in
[plans/graphics-quality.md](../plans/graphics-quality.md).

---

## 1. Sources and how far to trust them

| Grade | Meaning |
|---|---|
| **Caption** | The creator said it; caption track, timestamped `[m:ss]`. A claim, never a measurement |
| **Doc** | A vendor or maintainer's published page, repo or course. A claim |
| **Their test** | A third party's published, repeatable measurement (the 16-skill test) |
| **Ours** | Measured in this repo |
| **Unsupported** | Appears only in a write-up, not in any caption, page or measurement (§7) |

**The twelve videos** (`V01`–`V12`). Two independent write-ups exist. The MiniMax set quotes the
caption track at every step and corrects the earlier digest; it is the primary reading. The
Gemini set covers the same ground more expansively and adds a few correct caption details (V09's
rules), but also adds specifics that no source contains (§7). Where the two disagree, the
caption wins.

| # | Video | Creator | Agent → renderer | Family (§2) |
|---|---|---|---|---|
| V01 | This is how I make videos now | Mike Cann (Convex channel) | Claude Code → Tandem, his own MIT Mac editor with CLI/MCP | A |
| V02 | I Fully Automated My Video Editing | Christian Peverelli | Claude Code → Parakeet, HyperFrames, Tella MCP, Epidemic Sound, FFmpeg | A/B |
| V03 | Opus 5.5 Changed Motion Graphics Forever | Tao Prompts | Claude → whisper.cpp, OpenArt, Blender MCP | B |
| V04 | The NEW Way to Edit Videos (GPT-6 Astra) | Tao Prompts | Codex/Astra → HyperFrames, on his own tutorial footage | A |
| V05 | Viral animated reels with HyperFrames | Science Based AI (Ben) | Claude Code/Codex → HyperFrames, 4–10 s shots | B |
| V06 | Is Astra 6 better than Fable 5.1? | Marcin AI | Astra / Fable → Remotion + Higgsfield MCP | C |
| V07 | GPT-6 Astra is INSANE for motion design | Chase AI | Astra → Higgsfield MCP → Seedance 2.5 (generative) | C |
| V08 | Astra + After Effects in minutes | Adil, Higgsfield | Astra → Higgsfield AE plugin → editable AE project | C |
| V09 | Astra one-shots motion graphics (`/animate`) | Jay E, RoboNuggets | Codex/Astra → AssemblyAI, HyperFrames | B |
| V10 | My exact Opus 5.5 motion workflow | Science Based AI (Ben) | Claude chat → JavaScript, ElevenLabs SFX | B |
| V11 | Opus 5.5 animated every graphic | Matt Loui | Claude Code → Remotion → CapCut by hand | B |
| V12 | Astra unlocked After Effects | Chase AI | Astra → Higgsfield plugin, scripts + computer use → AE | C |

**Beyond the twelve** (the 20–21 Oct dossier, captions in [evidence/](evidence/)): Joseph (an
Opus reel, a starter SFX kit), Zane (a DaVinci master prompt, his own SFX library mapped by
keyword), Jason Cooperson (a Claude Artifact as his method; CapCut driven through the UI), Joris
(Resolve MCP against invideo: "the prompt didn't matter, the host did"), Rourke and Pat Simmons
(After Effects and Premiere by computer use), kaye.creatives (a second "Claude Motion" video),
Dan Kieft and Roboverse (production workflows), Nate Herk (the Herk Edit Kit), Brendan Jowett and
Marcin (ChatCut, sponsored). Their findings are merged into F1–F14 below.

**The ecosystem sources** (all **Doc**, none run by us):

| Source | What it is | Raw |
|---|---|---|
| awesome-claude-video-skills | 362 agent video repos (generated, not curated) and a published test of 16 of them against one brief with a no-skill control | [evidence](evidence/claude-video-skills-2026-10-27.md) |
| beamnxw/my_works | A 14-page production protocol (Opus 5.5 + Remotion launch videos), its 8-file harness, two 600-entry catalogues | [evidence](evidence/beamnxw-my-works-2026-10-27.md) |
| motionvideo.xyz | 500 AI motion videos with the prompt behind each; tools and studio directories; a $29/$49 skill pack with HyperFrames guides | [evidence](evidence/motionvideo-2026-10-27.md) |
| HyperFrames Pit Crew Academy | HeyGen's free 12-lesson course on directing a video agent; sells HyperFrames Studio / the free Framey app | [evidence](evidence/hyperframes-pit-crew-academy-2026-10-27.md) |

---

## 2. The shape every working pipeline shares

**The model directs; something else draws the pixels.** In every video where the mechanism is
visible, the model writes a plan, a prompt or code, and HyperFrames, Remotion, Blender, After
Effects, FFmpeg or a generative model renders. The creators drive a renderer they chose. They
don't ask a model to "make the video".

The twelve differ in where the pixels come from:

| Family | Mechanism | Editable afterwards | Who |
|---|---|---|---|
| **A. The agent edits a real project through a typed surface** | Code or tool calls against an editor's CLI/MCP; the editor's timeline is the deliverable | Yes, it *is* the project | V01, V04, V02 (partly) |
| **B. The agent writes the motion graphics; a deterministic renderer draws** | HTML/JS/React → HyperFrames, Remotion, Blender | If layers are kept apart | V03, V05, V09, V10, V11 |
| **C. The agent drives a vendor's generative pipeline or a host app's UI** | Expanded prompt → Seedance; or scripts + computer use → After Effects | Rarely (AE is the exception) | V06, V07, V08, V12 |

AeroEdits is family A for the edit and family B for graphics. Family C costs per generation and
ends in a flat clip or a host we don't own.

**Stages, and how many of the twelve show each:**

| Stage | n | Note |
|---|---|---|
| The method lives in a file (skill, plugin, template) | 11 | All but V11, which publishes a prompt doc |
| Reference + brief | 8 | Small and real (F6) |
| Transcribe for timing | 4 | Graphics must land on words, not seconds |
| Cut dead air and bad takes from the transcript | 6 | "Claude can't actually hear my videos" (V02 [1:27]) |
| Plan before the expensive render | 7 | Storyboard, shot list, build spec, pre-flight |
| Generate (code or expanded prompt) | 11 | |
| Render deterministically | 8 | |
| **Sound** | **12** | The only universal stage (F8) |
| Assemble in a host NLE or FFmpeg | 4 | |
| Review and iterate | 11 | Timestamped notes, a gate, a self-check |

Counts are the videos the MiniMax README lists per stage; three of its own counts (plan 12,
reference 9, render 9) don't match its lists and are corrected here.

---

## 3. Findings

Each finding is stated once, with its strongest evidence and its limits.

### F1. The model directs; a deterministic engine renders

All family A and B videos (§2); even family C hands rendering to a tool (Seedance, After
Effects). **Limit:** deterministic rendering buys control and
repeatability, not better *visual* quality; nothing measured the latter.

### F2. The method lives in a file the agent re-reads and rewrites on correction

- V02: a ~1,400-line `YouTube-Edit` skill, 36 rules, "a file of instructions that Claude reads
  before it does the job" [5:36]; after feedback he has Claude save the lesson into it [10:36].
- V09: "calibrate my /animate skill with this feedback" [13:10]; the skill is a folder with rules,
  design files and asset paths [14:04].
- V01: "put it into a skill, your own personal skill for video editing" [24:28]. V04: save the
  learned style as a skill [14:03]. V07, V08, V12 ship the method as a Higgsfield skill/plugin; V03
  as a zip; V04 and V11 as prompt documents.

**Limits:** no creator measured that the file improves later videos. Every published method is
gated (Skool, Drive, Higgsfield link) or unpublished. The rules are free text, learned in chat.

### F3. A skill doesn't buy correctness; it buys a look, a workflow, or footage handling

**Their test** (16 skills, one 15 s promo brief, a no-skill control, scored from the file):
every run, *including the control*, passed all four checks (renders, spec matches, every copy line
legible at 2 frames/s, the picture moves). The control took 4.4 min; skills ranged 2.0 to
23.5 min (HyperFrames the slowest, with 38% held frames). Stars predicted neither speed nor
motion. **Limit:** one brief, one machine, 16 of 362 hand-picked, look not scored. The
maintainer's own line: a skill earns its place with a look, a workflow, or footage to edit.

### F4. Plan and ask before the expensive render, then approve on frames, not words

- **Ask first:** "Ask me as many questions as you need" is "arguably the entire video" (V11
  [3:56]); the answers (style, pacing, colour, length) take about 2 minutes, and without them it
  "essentially just guesses" [4:03]. V10 puts "ask questions if needed" in the prompt [12:35].
  Fable asked length and scope before a long job; Astra didn't (V06 [5:17]).
- **A short brief:** without one there are "infinite possibilities" (V04 [4:40]). The academy's
  brief is "the goal, who it is for, two to four specifics and the exact words in quotes", only
  "the few things you would be upset to see wrong". Quoted words are copied exactly; described
  words get paraphrased. motionvideo's prompts converge on `<inputs>` (what to ask, with defaults
  if skipped), `<direction>`, `<structure>`, `<build>`, `<gotchas>`, `<start>` (29 of 494 prompts
  use four or more).
- **Plan, then gate:** V12 storyboards first [1:54] and cut 18 s to 10 s before any build; V10
  gets a shot plan and a build spec before code [7:57]–[8:24]; V04 pre-flights the brief before a
  49-minute pass [6:09]. Reply to a plan with specifics ("yes, but merge scenes four and five and
  keep it under 20 seconds"), never a bare "ok" (academy).
- **Approve on frames:** beamnxw builds the opening first, renders stills at named frames
  (0/18/42/60/119) plus a moving preview for the human, gets direction approved, then builds the
  rest. The academy's storyboard is stills at named times. **Ours (25 Oct):** three agents' plans
  were approved as words, and George approved descriptions more expressive than what got built.

### F5. Feedback is a spec: where, what is wrong, what it becomes, what stays

- "The text is boring" is complaint, not direction; give the replacement (V04 [10:00]); his
  feedback document labels each clipped word by timestamp [9:26].
- beamnxw's four fields: **location** (global frame 210), **observed defect** (cursor overlaps
  result label), **change** (departure 204–210, 24 px clearance), **preserve** (press 195, reveal
  210, card, copy, sound). Repair order: missing information and readability, then timing, then
  visual detail; update only the implicated values and check the neighbouring boundary.
- Academy: point at the element, say exactly what it becomes, with target numbers ("overshoot to
  1.08x", not "less bouncy", which came back at 1.0x); say the new words *replace* the old ones,
  or both appear; "keep scenes 1 and 3 as they are"; "three precise fixes in one reply land
  better than three rounds of vague ones".
- One change per iteration (V05 [12:01]). Rules as bullets, like briefing an editor (V09 [10:04]).
  When words fail, a sketch: V03 fixed a HUD's perspective with a Paint drawing [9:35].
- Revise on a copy and say what to change *and what to keep* ("reduce the bounce, keep the
  glow", Roboverse).
- **Anchors must survive edits:** refer to a clip occurrence plus word ids, never an absolute
  timestamp from before the edit changed (Dan Kieft).
- **Anchored to the timeline in products:** Tandem's comment on a clip that the agent reads (V01
  [11:45]); HyperFrames' *Select* (a clicked element becomes a chip in the message), *Draw* and
  *Pending edits* (Doc).

### F6. A small, real reference beats a mood board; concrete files beat adjectives

- Screen-record only the section to mimic (V10 [4:05]). Record your own product, "not AI slop"
  (V11 description); screenshots of a signed-in tool the agent can't log into (V11 [6:28]).
- Design systems as Markdown files and exact asset paths (V09 [8:43], [12:30]). Without
  references you get "that kind of AI look" (V05 [10:54]).
- Filter styles by what the model can execute: "Kurzgesagt … Opus actually isn't good at
  mimicking that, I tried" (V10 [3:20]); some styles HyperFrames "usually won't understand" (V05
  [12:58]).
- Never present an invented screenshot, feature, metric or customer result as real (beamnxw
  harness).
- **2 frames/s** is the convergent rate for reading a reference: beamnxw's
  `ffmpeg -vf fps=2`, the skills test's legibility sampler, and an OCR paper all use it.
- **Style intake** (24 Oct web research, mostly community skills and product docs):
  [motion-graphics-skills](https://github.com/charlie947/motion-graphics-skills) starts with a
  brand intake that writes `brand.md` and `MOTION.md`, which every later skill reads first;
  [brand-motion-design-skill](https://github.com/ouerf-man/brand-motion-design-skill) asks a few
  questions or reads a website URL for colour and type tokens, and shows still style frames before
  anything animates; references are decomposed into moves, not copied. Products converge on the
  same three inputs, kit + references + brief: HeyGen
  [brand kits](https://developers.heygen.com/docs/brand-kits), OpenArt's
  [brand kit](https://openart.ai/features/ai-brand-kit/) (which says what it dropped), Typeface,
  Lovart.
- **One complete style beats a catalogue:** build one, measure it, then broaden (short-form
  tutorials in the dossier).
- **Split the brief:** requirements (format, deliverables), preferences (taste), analysis
  (transcript, telemetry).

### F7. Generate hard images once; let the model animate them

"Claude is great at animating the motion graphics. It's not really that great at rendering
realistic images" (V03 [4:39]); he generates an asset sheet with an image model and has Claude
animate it. V09 keeps an element library added one item at a time so paths reach the prompt
[12:30]. V05 places background-free PNGs [6:28].

### F8. Sound is universal, placed on the visual event, art-directed in words, ducked under voice

- **12 of 12** videos involve music or SFX. Against that, only 9 of 16 skills added sound when
  the brief made it optional (their test).
- Placement: because the animation is code, the model can "go frame by frame through it" and place
  effects where things pop (V10 [10:30]). beamnxw finds each clip's transient by waveform and
  offsets the clip so the audible event lands on the visible press or reveal frame; music and
  effects stay on separate, editable tracks; the ending is checked on headphones and a phone
  speaker.
- Direction in words: "keep it pretty minimal and subtle, tech launch vibes, no heavy risers or
  drops" (V10 [13:17]); unspecified SFX come out "really generic" (V05 [3:10]); "you always need
  a pop" (V11 [11:50]).
- Mixing: describe defects as heard ("muffled, like behind a curtain"), have the music "make room
  for the voice" rather than lower the master, start music at about 1 s (academy). Zane balances
  dialogue to about −16.5 LUFS (dossier).
- The library is the asset: Epidemic Sound (V02); Higgsfield sells its editing skill *with* an
  SFX library; Joseph's starter kit is five whooshes, two or three each of mechanical, sci-fi and
  UI, two risers, two hits (dossier).

### F9. The output must stay editable; a flat file is the failure mode

V08 sells "an editable project I can keep refining" [2:31]; V12 hands over the project and assets
[6:00]; V05 green-screens one element for overlay [8:43]; V01's deliverable is the timeline. V11
shows the cost of the opposite: a flat 4K file dragged into CapCut [8:42], then a manual
finishing pass. The pitch creators advertise against is "AI made it and I can't touch it".

### F10. The model directs but doesn't judge; checks come from the file and the human

- "The model can't really tell if stuff is super good or not" (V10 [15:32]); "you definitely want
  … a human in the loop" (V12 [7:34]).
- Bounded self-checks exist: V07 compares the render to the central prompt and regenerates only
  the weak section [4:21]; V12 checks the build against the storyboard, then the render [6:00].
- Unbounded runs are expensive and unaccountable: V06 reports 32M, then 36M, then 35M tokens for
  one round (they don't reconcile), $32 and $38 API-equivalent [7:45]–[8:37].
- Measure from the file, not a model: VLMs misjudge duration by about 6 s and classify motion
  type below 13% accuracy; a static video scores well on consistency alone, so motion must be
  measured beside it (sources cited by their test).
- "Watch the exported file once, from start to end, before you send it" (academy).

### F11. Motion craft is a vocabulary plus a build order

From the academy (Doc), the clearest published statement:
- Enter with ease-out, exit with ease-in, never linear; overshoot as a number; leave a hold.
- Reading time is the anchor: "about three words a second, so five words need nearly two
  seconds". beamnxw counts the hold from the first frame where the words can be read (eight
  frames at 30 fps is too short) and checks labels at phone size; its example holds a result
  186 frames (6.2 s).
- Name the cut: a plain "cut" may come back as a dissolve; say hard cut, match cut or zoom cut.
- Build order, one pass each: **stills** (every element at its final size, each with an id, none
  created by script) → **timing** (one-frame pops at named times) → **camera** → **motion**
  (easing, finish, and a "Never: …" list of banned clichés). Tweens are plain calls with plain
  numbers so a checker can read them.
- Pacing left to the model drags: it defaults to 2.5–3 s scenes (V12 [7:04]). Build shots of
  4–10 s, not whole videos (V05 [11:16]).
- Motion tokens (easings, springs, timings in one place) are what the motionvideo pack and the
  charlie947 pack (`MOTION.md`) ship so every scene moves with one rhythm.

### F12. What code animation can and can't do

- **Can:** kinetic type, charts and data, UI mock-ups, diagrams, data teasers (V05, V10, V11). Of
  motionvideo's 500 entries, 228 are tagged kinetic type and 191 product UI; 468 name Opus 5.5
  (a self-selected catalogue).
- **Can't, or not well:** realistic images (V03), Kurzgesagt-style illustration (V10), 3D without
  Blender (V03 rendered it in Blender overnight), heavy particle work (V10).
- **One-shot is the exception it is sold as:** of the 117 motionvideo entries that record
  iterations, 102 are one-shot and 15 needed rounds; V09's "one-shot" title sits on a skill
  refined over many rounds.

### F13. Tutorial and shorts specifics

- A spoken number becomes a hero count-up (V09 [10:25]); record web UI in a mobile browser for
  9:16 (V09 [10:46]); shorten URLs in mock-ups and drop "eyebrow" text (V09 [9:44]–[10:25]).
- Keep content inside short-form safe zones; fix it in place, don't regenerate (V05 [4:16]).
- Script in, slots out: ask the model *where* to animate, keep the good slots (V05 [5:39]–[7:06]);
  fan out standalone prompts to parallel sessions (V05 [7:18]).
- A title over the speaker's face: move it above the head (academy).
- **Ours (25 Oct):** three agents made generic text cards; where speech describes an action, the
  graphic should *show* it (a timeline collapsing, a prompt typing, a before/after).
- **Three tutorial-graphic jobs** (five X posts on tutorial motion and their repos):
  1. *Direct the viewer through a real screen:* the smallest intervention (a zoom, a highlight, a
     short label, an inset); keep the target visible while the action happens; English UI words
     stay exact inside Greek narration; never an invented screenshot.
  2. *Explain a concept:* write the one sentence the viewer should understand; build a literal
     state sequence; reveal each state at its spoken phrase; hold the finished diagram long
     enough to read.
  3. *Show a result or comparison:* real inputs, equal framing, measured values only; in 9:16,
     stack or sequence rather than shrink.
- **A per-graphic contract** from the same research: its teaching purpose; word ids for entry,
  emphasis, hold and exit; approved copy, real assets and the source of any factual claim; layout
  mode and target region; the style traits it borrows; the fonts its English and Greek copy need;
  separate landscape and portrait decisions; what a later revision must preserve.
- **Greek:** real copy with tonos, diaeresis, final sigma and mixed English product names; the
  intended font bundled (never judge a fallback); case conversion checked by eye.
- **Shorts are their own composition:** fewer simultaneous elements, explicit face and caption
  clearance; a visual needs an explanatory job, and a presenter-only moment is a valid choice;
  don't force a change every two seconds when the viewer needs to read. Highlight the one word
  that makes you keep watching: emphasis is chosen by meaning.
- **Judge separately:** brief compliance, taste and execution. An agent can follow a restrained
  brief and still disappoint someone who wanted energy.
- Some looks need compositor work over the footage (dim or blur behind an overlay, a presenter
  cut-out): a graphic can't blur what is composited under it later.
- Layout limits (Tella, Doc): a few changes a minute at most; screen 2–8 s, camera 2–5 s,
  punch-in 1.5–4 s; camera on the side the speaker faces away from.
- Everyone duplicates the project before an agent touches it (every Resolve creator).

### F14. The number to track is cost per accepted video

| Claim | Source | What checking gave |
|---|---|---|
| $27 per video | V02 [15:57] | A $200/month plan amortised; excludes weeks training the skill |
| 12 min → 2:34; first pass 49m30s | V04 | Whole process with feedback 3–4 h [11:24] |
| 32M / 36M / 35M tokens; $32 / $38 API-equivalent | V06 | Don't reconcile |
| 15 s of AE motion in 22m56s | V12 [5:43] | In V12, not V07 |
| 16:9 → 9:16 re-layout ~50 min vs 2–3 h by hand | V08 [10:29] | One re-layout |
| ~30 min per animation, render 10–15 min | V10 | Estimate; the demo skips the wait |
| First render ~45 min; $19/month covers ~13 graphics | V11 | Claims |
| 76 s → 50 s, 12 animations, 25 min of sound design, ~110 min elapsed, $1–5 | Joseph | Subscription share, not an invoice |
| "16 hours now takes 2 to 3"; one-shot long form 2 h 42 min | Jason Cooperson | From his description |
| A motion-graphics run ≈ 2% of a $100/month plan, ~10 min | Rourke | — |
| Fable £5.90 vs Astra $15.05 | one A/B | Different currencies; not "double" |
| $6.40 of host usage for an intro | Marcin (ChatCut) | Excludes ChatCut's own charges |
| First pass ~18 min, second ~10 min | Nate Herk | Lots of human art direction on screen |
| A whole tutorial edit with graphics, $0.27 of agent time | **Ours**, EVAL-1 run 4 | Measured; 36.8k tokens for the edit, 44.6k with graphics |

Six of the twelve give no cost at all. Where one is given it is a subscription amortised per
video, not an invoice.

---

## 4. What the research does not support

- **That one model has better taste.** V06 held the prompt but not the subject, assets, harness
  or answers constant; V12's description claims both models while its captions cover one. No
  controlled comparison exists.
- **One-shot motion graphics** as the normal case (F12).
- **That a skill makes output correct** (F3).
- **That deterministic rendering looks better.** It gives control (F1).
- **Viral-reel polish as the yardstick for tutorials,** or borrowed bans such as "no holds over
  1 s", which fight reading time (F11).
- **Autonomous fix-and-render loops.** No source shows one paying off; V06 shows its cost.

---

## 5. The landscape in brief

Detail and links per product: [competitors.md](competitors.md).

| Who | Why it matters to us |
|---|---|
| **HyperFrames Studio / Framey** (HeyGen) | Owner of the runtime we pin, selling an agent-first editor and a free desktop app to our buyer, with a free course that teaches the method (Select, Draw, Pending edits) |
| **DaVinci Resolve 21.1** | Native MCP (Studio): the most direct threat for agent editing of a real timeline; arbitrary scripts as its surface |
| **OpenChatCut** | Open-source, local-first conversational editor with skills and MCP, rendering in Remotion: our shape without telemetry, approve-before-apply or HyperFrames |
| **Tandem** | Our thesis in miniature: transcript as spine, agent comments on the timeline, review gate |
| **Tella, Borumi** | Recorders with MCP: auto-layout rules, all-or-nothing batches, timeline-hash transactions |
| **ChatCut** | Style trial on a 5–10 s sample, curated graphics, named versions, export queue |
| **Higgsfield** | Skill + SFX library sold together; AE handoff is its real edge |
| **motionvideo pack, Herk Edit Kit** | Skills and motion tokens for *our* renderer, sold or MIT; card libraries to curate, not copy |
| **The 362-repo catalogue** | DaVinci Resolve appears once: the skills wave grew on script-drivable editors |

**Our position:** in the creator videos every agent drives an editor the user already has, or
builds from scratch. The directories and packs make motion graphics *from scratch*: no footage,
no transcript, no cut. An editor with a typed API, tutorial telemetry, approve-before-apply and
deterministic graphics on the user's own footage is not what any of them ship.

---

## 6. Corrections register

Earlier summaries attributed findings to the wrong video or stated things the sources don't
contain. Everything above uses the corrected version.

| Earlier claim | Correct |
|---|---|
| Tao's repeated zoom at 49–59 s, clipped words, "boring is not feedback" in V03 | V04 |
| ElevenLabs SFX at the peak, "Opus isn't good at Kurzgesagt", "screen-record only the section" in V05 | V10 |
| "Green-screen one element / transparent PNG layers" in V10 | V05 |
| "Approve a 5–10 s sample before scaling a look" from Marcin (V06) | Not in V06; it is ChatCut's documented workflow |
| Chase AI's storyboard self-check and the autonomous-loop warning in V07 | V12 |
| "15 s of AE motion in 22m56s" in V07 | V12 |
| `transcript.md` §9 is Science Based AI's motion workflow | It is kaye.creatives' `AsvN_feasyY`; V10 is `H6lBZ7AfDDE` |
| Old extract timestamps for V01 and V02 | Wrong; use the full tracks in [evidence/raw/](evidence/raw/) |
| "Two of four scene archetypes have no template" | Withdrawn: it mixed Joseph's animation subjects with his scene choices |
| "Video 18 is a Convex product by a Convex dev" | Tandem is Mike Cann's own MIT editor |
| CapCut driven over MCP in Jason Cooperson's setup | No MCP; "Claude clicks through the UI" |
| lemo-opuscar has 51 film styles | 59, per its own description |
| "106 sound effects", "12 hours / 4.5B tokens" (Joseph) | Not in his captions |
| motionvideo: "13 of 117 needed correction" | 15 (13 few rounds, 2 many) |

---

## 7. Gemini-only claims not found in any source

The Gemini write-ups (now only in the backup ref) were useful for structure, but these specifics
appear in no caption, page or measurement we hold. Don't build on them.

| Claim | Where | What the source says |
|---|---|---|
| Ducking music by −18 to −24 dB | gemini 02 | No level in the captions |
| A 390×844 mobile viewport rule | gemini README, 09 | V09 says "screen recordings in a mobile browser" [10:46], no size |
| Epidemic ≈ $1/video; "15–30 min render"; "5 min review" | gemini 02 | $9.99/month; 10–20 min for a 31 s hook; no review time |
| The no-skill control "defaulted to deep teal, floating waves" | gemini skills | Not in the test data |
| Silent skills "severely hurting perceived quality" | gemini skills | Look and sound were not scored |
| HyperFrames "73k ★" in the test | gemini skills | The test row says 60,291; 73,765 is the Studio page's figure |
| `SOUND.md` ducks −14 to −18 dB; "transition owner objects" | gemini beamnxw | Not in our ingest of the harness |
| A minimum 0.5 s hold added to "three words a second" | gemini lessons | The academy gives only "about three words a second" |
| "3Blue1Brown feasible and high-fidelity" | gemini README | V10 names it as a style; only Kurzgesagt was tried and failed |
| Skill bloat and over-fitting as failure modes | gemini 09 | Plausible, but the video says neither |

---

## 8. References worth reading

Repos with real source, for building graphics ([GFX-3](../done.md#gfx-3), GFX-8):
[mg-styles-15](https://github.com/Vincentwei1021/mg-styles-15) (15 styles with prompts, source
and a `renderAt(t)` renderer), [lemo-opuscar](https://github.com/lemomo-ai/lemo-opuscar) (59 film
styles), [claude-animation-skill](https://github.com/buildwithhanif/claude-animation-skill)
(contact sheets, transition strips, out-of-order determinism checks, staged rendering),
[LottieFiles motion-design-skill](https://github.com/LottieFiles/motion-design-skill) (timing and
choreography), [pdoom-video](https://github.com/mexicat/pdoom-video) (word-synchronised type; a
copy is `JohnHeibel/PDoomVideo`), [herk-edit-kit](https://github.com/nateherkai/herk-edit-kit)
(plan schema, validators, 406 HyperFrames cards), [Tandem](https://github.com/mikecann/tandem)
(MIT editor with MCP and timeline comments), [HyperFrames](https://github.com/heygen-com/hyperframes)
and its [Pit Crew Academy](https://hyperframes.studio/lessons).

## 9. Where the evidence lives

- [evidence/](evidence/): `transcript.md` and `transcripts-wave2-2026-10-21.md` (caption tracks),
  `video-dossier-2026-10-21.md` (all 19 videos: descriptions, links, chapters, metadata),
  `nine-ai-editor-videos-transcripts-2026-10-21.md` (third-party extracts; its V01/V02 timestamps
  are wrong), [raw/](evidence/raw/) (the full V01 and V02 tracks), and the four
  `*-2026-10-27.md` ecosystem ingests. Re-fetch captions with
  [evidence/scripts/](evidence/scripts/README.md).
- V10 is `H6lBZ7AfDDE` in `transcripts-wave2-2026-10-21.md`, not `transcript.md` §9 (§6).
- Captions are speech recognition: brand names are mangled ("Appus" = Opus, "Hicksfield" =
  Higgsfield, "Claw" = Claude). Quote descriptions and links, not caption spellings.
