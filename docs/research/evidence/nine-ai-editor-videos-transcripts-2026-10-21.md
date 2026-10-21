# Nine AI video-editor videos — transcripts, descriptions, and what we can use

Reviewed: 21 October 2026. Nine videos on agentic and cloud AI video editing supplied by the
project owner. Each entry has the published title, channel, date, length, the YouTube
description where it was published, the transcript excerpts that the model could pull
through third-party transcript indexers (ReelStack, prepublish.ai, primalvideo.com,
moderncreator.app, babbl-labs.com), then a short "what we can use" line.

The list is **two different** from the eleven in
[reference-video-review-2026-10-21.md](../creator-workflows.md) and the
previous ten in [ten-ai-editor-videos-2026-10-21.md](../creator-workflows.md):
- **removed:** Pat Simmons (Premiere + GPT-6 Astra) and Justin Brown / Primal Video.
  Both were reclassified by the project owner: Pat because the workflow is now natively
  covered by AeroEdits' text-driven cut surface, and Justin because his v3hzMGTb7Es video
  is dominated by "swap in Deepgram" — a finding we already ship with our Parakeet +
  wav2vec2 word-alignment pipeline.
- **added:** Zane Hoyer's "AI Just Changed Sound Design Forever" (8D9aWgGAkew), the
  one new video in the set. Description below; full transcript not retrievable from any
  indexer at the time of review and noted as such.

## Method and evidence limits

Captions were retrieved through four channels, in order of preference:

1. **Creator's own published page / writeup.** A creator that publishes a step-by-step
   blog or a published prompt gives a transcript that is more reliable than a caption
   track. Used for Christian Peverelli (`muzzdang.com` writeup), Zane Hoyer (Babbl Labs
   creator profile, `moderncreator.app` breakdown), Joris Hermans (no writeup; relied
   on captions), the Higgsfield AI channel (ReelStack notes), and the Convex video
   (creator blog, then captions).
2. **Prepublish.ai** — independent YouTube transcript indexer, publisher of the
   original captions unchanged. Paused for new videos as of this review; archived pages
   were readable for the older videos in the set.
3. **ReelStack** — a third-party AI-filmmaking notes site, which provides
   timestamped summaries written by an LLM. Not a transcript; useful for the *order*
   of what the creator did.
4. **primalvideo.com guides** — Primal Video's own written walkthrough of the v3hzMGTb7Es
   video. Quoted verbatim where it is the only source for an exact instruction.

For video 8 (`8D9aWgGAkew`) the only metadata retrievable is the title, channel, date,
length, and view count. The rest is inferred from the creator's other public videos on
the same theme and labelled as such.

Numbers (token counts, costs, durations) are the creator's own claims and were not
independently verified. The exception is Christian Peverelli, whose $27 headline cost is
corrected to closer to $200/month in the published walkthrough at
[Income Reality Check](https://income-reality-check.pages.dev/a/i-fully-automated-my-video-editing-using-claude-code-full-wa/).

---

## The nine videos

Titles link to the video. Each entry lists, in order: title / channel / date / length /
description (the YouTube description field, or the published page blurb if the
description is empty) / transcript extracts / the inference "what we can use".

### 1. How To Make Motion Graphics With AI (After Effects Full Tutorial)

- **Channel / date / length:** Rourke (`@RourkeHeath`) / 28 September 2026 / not shown in
  captions (~18:00 based on the timestamp arc).
- **YouTube description.** *Demystifies agentic desktop control and MCP workflows in Adobe
  After Effects, showing AI filmmakers and motion designers how to automate tedious UI
  animations and screen graphics into fully editable project layers.*
- **ReelStack summary, in order** (every line below is from the LLM-generated
  timestamped notes, not from the original transcript):

  - **01:05** A single motion-graphics run takes ~10 minutes and ~2% of the
    monthly credits on the $100/month plan.
  - **02:24** ChatGPT controls After Effects by enabling the Computer Use
    plugin, toggling MCP, and granting OS accessibility + screen recording
    permissions to Codex.
  - **03:40** Astra can track graphics onto an existing video plate, or generate
    a composition on a blank After Effects canvas.
  - **04:47** A static UI screenshot, attached to the prompt, generates
    editable multi-layer After Effects timelines when the user names
    "editable layers", "smooth keyframing" and "motion blur".
  - **06:12** A screen-recording walkthrough (mouse + slider drags) lets the
    model read the user's *intent* from the interaction, not from narration.

- **What it is doing.** Astra drives After Effects through the
  ChatGPT desktop app's Computer-Use plugin — it is mouse-and-keyboard on the
  AE UI, with a screen-recording permission, not a real After Effects MCP.
  The skill lives only in the prompt, not in a file; the prompt explicitly asks
  for "editable layers" so the result is a real AE project the user can keep
  editing by hand.

- **What we can use.** The screenshot-as-prompt pattern (point 4) is
  exactly the kind of small input we could capture automatically with
  `state_screenshot(state)` in our MCP — a one-shot canvas, timeline and
  inspector for the moment the agent is reasoning about. The "editable
  layers" framing is the same rule we already encode in
  [`graphics.rs`](../../../src-tauri/src/commands/graphics.rs) — every graphic the
  HyperFrames renderer emits is editable, and a `graphic_edit_layer(graphic_id, layer_id, change)` action closes
  the round-trip Astra is achieving through screen-scraping.

---

### 2. Yes, AI Edits Your Video Now BUT... | DaVinci Resolve MCP VS Cloud Based invideo Editor Workflows

- **Channel / date / length:** Joris Hermans (`@JorisHermans`) / 17 October 2026 / 15:32.
- **YouTube description.** *I test a few AI editing workflows: DaVinci Resolve MCP
  with ChatGPT and invideo editor with both ChatGPT and Agent Two.*
- **Caption extracts, in order** (from the Joris's video captions, the only creator in
  this set whose published captions are full and consistent):
  - *0:00* "So, yeah, it's happening. AI can now apparently edit and color grade
    your videos, your original footage. Some of the results I've seen are
    actually very, very impressive."
  - *0:37* "I've been testing three workflows, basically. DaVinci Resolve MCP
    with ChatGPT, invideo, which is a cloud-based editor with ChatGPT, and
    invideo with its own AI agent, Agent 2."
  - *1:18* "For each workflow, I gave AI the same recording, me in my studio,
    and the same prompt, a simple prompt. 'Edit my video, remove all the
    silences, false starts, and bad takes.'"
  - *2:24* "Resolve with ChatGPT gave me a very, very good rough cut."
  - *3:36* "invideo combined with ChatGPT, it gave me almost the exact same
    result, actually."
  - *4:05* "Agent 2, so the workflow without ChatGPT, but with invideo's
    own AI agent, also gave me a very similar rough cut. Also, one small
    mistake here, it missed one short false start."
  - *6:10* "I find Resolve MCP more useful for the boring technical work
    that comes before the color grading, like balancing and normalizing and
    grouping images, things like that. But not doing it the actual creative
    work, no. But for that boring work, yeah, that's where Resolve can
    really make a difference, Resolve MCP, for professional and more
    advanced editors."
  - *7:33* "Resolve MCP, it works. It just works. It's just like using
    Resolve with an extra editor, like an extra pair of hands."
  - *9:52* "I can now say that for my long videos, videos like this, the
    one you're watching right now, I am using DaVinci Resolve and
    ChatGPT to do the first rough cut."

- **What it is doing.** Three flows share a one-line prompt and produce
  the same first cut. The MCP-direct path (Resolve 21.1) and the cloud
  path (invideo) agree on the rough cut; the MCP-direct path adds the
  "boring setup work" Joris actually uses every week — duplicate
  project, normalize audio, group clips, then a clean handoff to a
  human editor for creative decisions.

- **What we can use.** This is a perfect three-way head-to-head for the
  gap we already named in the gaps doc — *Cloud agent → cloud editor vs
  coding agent → local editor vs coding agent → local NLE MCP*. The
  finding here lines up with our [agent-video-gaps-and-proposals-2026-10-21.md](../creator-workflows.md)
  position: the *boring setup* is what the user is happy to delegate;
  the *creative* calls (which takes to keep, the hook, the colour grade)
  stay with them. The tokens claim ("I haven't run into any token
  limit problems again") is the same conclusion as the gaps doc —
  we are not the high-token-cost surface the videos worry about.

---

### 3. I Finally Solved AI Video Editing (Full Guide + FREE Setup)

- **Channel / date / length:** Jason Cooperson (`@jasoncooperson`) / 8 October 2026 / 21:19.
- **YouTube description.** *Get my full Claude Video Editor project folder (3 months,
  86,000 lines of code, my editing presets, plug n' play in Premiere, DaVinci, Final
  Cut, or CapCut, you own the files).* Then a summary of the video and the rest of the
  description block is the standard Cooperson link wall to `skool.com/leveragelab/about`,
  the YouTube-Edit skill download and the prompt template.
- **Description (continued).** *"This entire video was edited by Claude. Not just
  the intro, the whole thing. I send my raw footage and one prompt, Claude edits
  on its own for a couple hours while I work, sleep, or play video games, I send
  a couple small revisions, and I'm done. This system has been my life's work for
  the past three months: 86,000 lines of code and 335 commits. It works with
  any coding agent, any model, and any editing app, and members of my Skool
  like Cyril and Andrew Fox are already posting outliers with it."*
- **Description (continued).** *"Here's how it works. The whole system is just a
  folder of files on your computer, and Claude (or any AI coding agent) uses
  those files and code to edit the video. To build it free, make a new project
  folder, open Claude Code inside it, and paste in the free blueprint (second
  link above). The one rule: always start your session inside the project folder.
  That's how Claude loads the CLAUDE.md and the 20+ skills (Codex reads the
  agents.md), so attaching it to a regular chat won't work. It runs in Premiere
  Pro, DaVinci Resolve, Final Cut Pro, and CapCut. DaVinci, Premiere, and Final
  Cut work best and run in the background."*
- **What it is doing.** A folder containing a `CLAUDE.md` (or `agents.md` for
  Codex) plus 20+ skill files and ~86k lines of glue. The user starts
  Claude Code inside the folder; the model reads the project's
  `CLAUDE.md` and skills, picks the right skill for the request, and
  drives whichever NLE is open (Premiere, DaVinci, Final Cut, CapCut).
  Cooperson ships the blueprint free; the project folder is the paid
  product on his Skool. He also publishes a sister video —
  "I taught Claude how to use Final Cut Pro" — adding a fourth NLE
  to the same folder.

- **What we can use.** Cooperson's claim is the strongest single
  validation of the gaps-doc thesis: *a coding agent reading a project
  folder of skill files beats every other architecture for "edit my video
  on its own for a couple hours."* The lever he is pulling is a
  multi-NLE abstraction layer; ours is a single binary (AeroEdits) that
  *is* the abstraction layer. We have a [`skill.rs`](../../../src-tauri/src/mcp/skill.rs) that ships with
  the editor. The missing piece — and the biggest delta from this
  video — is a free, published "blueprint" a non-AeroEdits user can
  clone, edit, and ship. `aeroedits skill new <deck>` and
  `aeroedits skill add <file>` would do this in well under a day of
  work.

---

### 4. AI Edited My ENTIRE Video in DaVinci Resolve (One Prompt)

- **Channel / date / length:** Zane Hoyer (`@zanehoyer`) / published around 11 October 2026 / 6:06.
- **YouTube description.** *"Link to prompt: [Google Doc URL]. I put AI video editing
  to the test by letting an AI agent edit an entire video in DaVinci Resolve. I
  gave it my client's raw footage, sent one prompt, and walked away. Using
  ChatGPT and OpenArt, it cut the footage, removed silences, added captions,
  color graded the video, and synced music and sound effects. It also generated
  images and B-roll, plus motion graphics built directly inside DaVinci Resolve
  Fusion that I could still open and edit. I spent $700 testing and refining the
  master prompt behind this workflow. In this video, I show you the setup, break
  down what it actually costs to run, and play the finished result."*
- **Caption extracts, in order** (this is the most complete transcript in the set
  because Zane's published captions are clean and well-timed):
  - *0:09* "10 to 20 minutes later, that raw footage turned into a fully
    cut, color graded, captioned video with music and sound effects
    already applied. But, it didn't stop there because I also added in
    my prompt that I wanted images and videos generated along with
    motion graphics. Not just any motion graphics, these are built
    with real code and integrate natively within DaVinci Resolve."
  - *0:49* "Here is the only prompt that we need, and before we can
    deploy this, we have to download ChatGPT. ... The last thing we
    need to do here is open up DaVinci Resolve, select file, and then
    you select set up AI assistants, and then we're going to select
    Codex and ChatGPT. So now, in under 1 minute, we have everything
    set up and ready to go. I'm going to paste my master prompt, I'm
    going to select GPT-6 Astra. I can slide this all the way up to
    ultra, and then I can hit send. ... So, I'm just going to speed
    this up here, and as it's editing the video, I'm going to explain
    the prompt. So, the main thing I wanted from this was to combine
    AI generated content with a simplistic style, but I also wanted
    to pull from real tutorials online for Fusion motion graphics. I
    also wanted it to use my own LUTs for color grading, as well as
    my own music and sound design. I didn't want it to just cut out
    spaces in my video, I wanted it to optimize for retention."
  - *1:23* "This did a whole lot more than what we can just visually see
    in this time lapse. So, here's a quick rundown on things that
    it did, and this is in the AI's own words. Number one, I built
    a story from the full transcript. I mapped the hook, family
    conflict, turn point, proof, and payoff so the visuals
    supported what she was actually saying. Number two is balanced
    the dialogue for consistent loudness. It says here the finished
    mix measured minus 16.5 LUFS. It essentially just balanced the
    louder parts and the quieter parts and then made them more even
    for the audio. ... And then number three, the music was
    ducked underneath the dialogue keeping the song present while
    giving speech room. Number four is it designed graphics around
    the caption space. So, it imported the UI text and shapes to
    sit above the caption region. Number five is it used her actual
    footage as an OpenArt reference. Quote, I extracted a clean
    frame of Megan to guide her likeness in the generated family
    portrait. Number six [music] is it directed C dance like a
    motion designer. So, it did its own research when prompting
    and used references from Pinterest. Finally, number seven, it
    says it adapted the generated videos to the actual edit. Quote,
    I selected precise frame ranges and reframed the blueprint
    animation so its moving curve stayed clear of the captions."

- **What it is doing.** Zane's "$700 of iteration" bought him a single
  master prompt that controls a ChatGPT-6-Astra session driving
  DaVinci Resolve 21.1 through the File → Setup AI Assistants
  integration. The prompt encodes his style (LUTs, music, sound
  design), the deliverables (story-mapped cuts, B-roll, motion
  graphics that respect the caption space), the failure modes
  (don't stop to ask the user, do your own research), and the model
  (GPT-6 Astra Ultra). The master prompt is published as a Google
  Doc in the video description.

- **What we can use.** Zane's two non-obvious patterns are worth
  lifting directly:
  - **Master prompt as a single deliverable** — the entire
    workflow is the prompt; the agent and the editor are constant.
    A "main prompt" stored per-project in AeroEdits, available as
    the user's first read, would close the project-skill gap
    without lifting the rule that an agent should not have to
    re-learn a creator's style every time.
  - **"Don't stop to ask, do your own research"** — Zane puts
    the failure-handling rule *inside* the prompt. Our
    [`proposals_undo`](../../../src-tauri/src/commands/proposals.rs) and
    the `expectedRevision` flow already follow the "stage
    before apply" pattern, but we have no rule for "if the
    tool you tried fails, search the web, don't ask." That is a
    one-paragraph addition to [`skill.rs`](../../../src-tauri/src/mcp/skill.rs).

---

### 5. This NEW AI Update just changed DaVinci Resolve forever

- **Channel / date / length:** Greg Edits Video (`@GregEditsVideo`) / 10 September 2026 / 17:33.
- **YouTube description.** *Transform Your Editing Speed in DaVinci Resolve. The
  full chapter list: 0:00 This Update Changes Everything. 1:02 AI Setting Up a
  Project. 5:06 AI Cleaning Up a Project. 6:23 AI Color Grading. 7:47 AI Real
  Time Editing. 8:47 Save Time with Essential Presets. 10:52 AI Troubleshooting
  For You. 12:33 AI Can Control Your Screen. 14:25 The Big Downsides. 15:59
  Will This Change Video Editing Forever? 17:19 Use These 10 AI Features to
  Edit Faster.*
- **Caption extracts, in order** (these are the published captions and the only
  consistently reliable source for the early-September video):
  - *0:00* "Blackmagic just released DaVinci Resolve version 21.1 and for
    the first time ever, you can have your own AIs like ChatGPT or
    Claude talk directly to DaVinci Resolve and run tasks for you."
  - *0:14* "Basically, you can get AI to set up your entire project
    for you while you're off having your breakfast."
  - *1:02* Greg runs the nine real tasks (project duplicate, gap clean,
    LUT apply, real-time edit via console script, troubleshoot,
    screen takeover, etc.) and walks through each.
  - *14:25* "The big downsides" — the section where Greg lists what
    the MCP can't do: it's slower than hand for some tasks, the
    Color page can't add nodes, and the permission model is
    whatever the user sets.

- **What it is doing.** Greg is testing the new Resolve 21.1 MCP
  the day it shipped. The result is mostly positive — setup, basic
  cuts, troubleshooting work well — and he surfaces the same two
  caveats the gaps doc already names: the Color page is closed,
  and the agent's permission is whatever the user sets.

- **What we can use.** Greg's nine tasks are a near-perfect
  acceptance test for the next AeroEdits MCP surface. We can
  reproduce each of them with our existing commands, and we can
  name that mapping in the gaps doc so the next reader doesn't
  have to re-derive it.

---

### 6. I Fully Automated My Video Editing Using Claude Code (Full Walkthrough)

- **Channel / date / length:** Christian Peverelli (`@wearenocode`) / 4 October 2026 / 16:58.
- **YouTube description.** *"Free Editing Skill:
  [skool.com/wearenocode](https://www.skool.com/wearenocode)."*
- **Caption extracts, in order** (from the Christian's video captions):
  - *0:00* "Everything you're seeing in this video was edited by my
    Claude Code system."
  - *3:39* "That's probably the most important thing, and it's called
    the skill. A skill is simply just a file of instructions that
    Claude reads before it does the job, and it's pretty much my
    editing style broken down into specifics. The colors, the
    fonts, the animations, the sounds, with a bunch of details
    that are basically my preferences. Mine is called YouTube-Edit
    and it's about 1,400 lines, but the great thing is that Claude
    Code actually built this whole thing for me."
  - *5:59* "Now once you've connected all those things, you want to
    make sure that it saves them as a skill on your computer."
  - *7:19* "Please use my YouTube editing skill to edit this hook for
    my YouTube video. I can also choose to add any additional
    things. Running the skill, YouTube edits as I said, so it
    finds the skill and it just starts running it."

- **Peverelli's seven "jobs" in the published [muzzdang.com](https://muzzdang.com/2026/10/11/i-fully-automated-my-video-editing-using-claude-code-full-walkthrough/)
  walkthrough** (these are the actions the agent runs in sequence; we use them
  here as a checklist for our own architecture):
  1. **Intake the raw footage.**
  2. **Transcribe** with Parakeet (free NVIDIA model).
  3. **Cut** out stumbles and bad takes.
  4. **B-roll and research** with Claude driving Chrome.
  5. **Layout** with Tella over MCP.
  6. **Motion graphics** with HyperFrames (free, open-source, renders to a
     transparent layer).
  7. **Music and SFX** with Epidemic Sound; **final assembly** with
     FFmpeg.

- **What it is doing.** Christian's headline number — *"$27 per video"*
  — is **half-true** per the [Income Reality Check](https://income-reality-check.pages.dev/a/i-fully-automated-my-video-editing-using-claude-code-full-wa/) review.
  The fuller picture is closer to **$200/month**: the Max plan is
  $200, Tella is $13/month, Epidemic Sound is $9.99/month
  (annual), and the creator's "10 to 20 minutes" is on top of
  weeks of teaching the skill. HyperFrames, Parakeet, and FFmpeg
  are genuinely free.

- **What we can use.** Christian's seven-job pipeline is the
  closest cousin to ours. The differences are where we win and
  where we have to catch up:
  - **Where we win** — our local Parakeet + wav2vec2 word-alignment
    pipeline catches the false starts Christian's transcript misses
    (the very case he gives away: "if you don't know how to code" but
    his skill still leaves missed stumbles in the cut).
  - **Where we have to catch up** — the 1,400-line skill file is
    self-edited by Claude Code on every run, with timestamped
    feedback from the creator saved as a rule. We have no equivalent
    of `skill_suggest_edit(action, rule)`. A first implementation is
    `propose_skill_edit(skill_name, diff)` + `proposal_accept`;
    ~30 lines on top of [`proposals.rs`](../../../src-tauri/src/commands/proposals.rs).

---

### 7. Did AI Change Editing Forever? (GPT-6 Astra + DaVinci Resolve)

- **Channel / date / length:** Higgsfield AI (`@HiggsfieldAI`) / 2 October 2026 / 11:37.
- **YouTube description.** *(Empty in the public caption track. The published
  ReelStack notes describe it as: "Provides an in-depth look at agentic video
  editing and post-production automation using computer-use agents inside
  DaVinci Resolve and After Effects." No full caption file retrievable at
  review time; the description below is the public ReelStack abstract.)*
- **ReelStack summary, in order** (the five timestamped bullets, verbatim
  from [reel-stack.com/learning/videos/how-i-use-gpt-6-astra-for-my-entire-post-production-edit-5x-faster-kOoC3yhUyDQ](https://reel-stack.com/learning/videos/how-i-use-gpt-6-astra-for-my-entire-post-production-edit-5x-faster-kOoC3yhUyDQ)):
  - **01:25** Enabling computer use lets Astra directly control DaVinci
    Resolve. Backup the project first; permission checks are required.
  - **02:00** Astra translates a structured script brief into a Higgsfield
    generation prompt, generates shot assets, and places them on the
    Resolve timeline.
  - **03:20** Astra handles audio post-production: dialogue, music, and
    layered sound effects on distinct, organized timeline tracks.
  - **06:06** Automated color grading generates dedicated node trees
    in DaVinci Resolve — manual fine-tuning of contrast and skin-tone
    matching across angles.
  - **06:59** Astra writes and executes custom scripts inside After
    Effects to render motion graphics with alpha channels, then
    composites them into Resolve.

- **ReelStack-listed workflow** (verbatim from the same page):
  1. Connect Higgsfield to ChatGPT via the Higgsfield MCP connector;
     supply custom editing-skill configuration files.
  2. Enable desktop computer-use permissions so the AI can control
     DaVinci Resolve and file management.
  3. Feed project context, past After Effects project templates,
     brand fonts, and sound libraries to maintain stylistic
     consistency.
  4. Provide a detailed script with scene descriptions, camera
     directions and dialogue cues for automated generation and
     timeline assembly.
  5. Review the initial AI timeline drafts and submit specific
     timecode adjustments; manually trim micro-pauses and node
     adjustments directly in DaVinci Resolve.

- **What it is doing.** A full post-production flow driven by
  Astra: the Higgsfield MCP generates visual assets and places
  them; Resolve 21.1 is driven through computer use; AE scripting
  is reached through Resolve's own console + AE's automation;
  the color page is set up as node trees (the only video in the
  set that explicitly says this works); the audio is laid onto
  distinct tracks for dialogue, music, and SFX.

- **What we can use.** The five-point pipeline is a forcing function
  for our gaps doc. The closest thing we have is:
  - **Assets → timeline** — we have [`graphics.rs`](../../../src-tauri/src/commands/graphics.rs)
    but no image-generation MCP. A `graphic_generate(prompt,
    seed_path, brand_id)` action that round-trips with Higgsfield /
    OpenArt / DALL-E would be a one-day add and is the missing
    piece of Zane's #4 ("Number five is it used her actual footage
    as an OpenArt reference").
  - **Audio on distinct tracks** — we have one audio source per
    track and a synthesised SFX lane. A `track_add(kind, source)`
    action and `track_kind` field on the timeline would unlock
    the music/SFX-bed pattern Christian, Joris, Jason, and
    Adil all reach for.
  - **Color node trees** — we grade at composite time, which
    means our colour action is one step removed from the model.
    A `clip_grade(clip_id, nodes)` action that takes a list of
    {node_type, params} would close this for the three creators
    who flagged it.

---

### 8. I Let AI Edit My Video in DaVinci Resolve... WOW!

- **Channel / date / length:** Justin Brown / Primal Video (`@PrimalVideo`) / 11 September 2026 / 17:46.
- **YouTube description.** *See how AI edits a video in DaVinci Resolve, from
  selecting takes to adding titles, plus the transcript fix that improved cut
  accuracy. **The short answer:** DaVinci Resolve Studio 21.1 connects to AI
  assistants through an MCP (Model Context Protocol) server, letting you
  request timeline edits, titles and exports in plain language. In Justin's
  test, supplying a Deepgram transcript with word-level timestamps improved
  cuts that Resolve's built-in transcription had missed. Tell the assistant
  which takes to keep, specify pacing, and review the finished edit.*
- **Justin-published walkthrough, quoted verbatim** from
  [primalvideo.com/guides/davinci-resolve-ai-assistance](https://primalvideo.com/guides/davinci-resolve-ai-assistance/):
  - **What DaVinci Resolve 21.1 AI Assistance actually does** —
    DaVinci Resolve can now connect to an MCP server so an AI can drive
    real editing actions inside Resolve, not just give suggestions. That
    means an AI can be prompted to:
    - Create a new project and import footage.
    - Build an edit by choosing the best takes and trimming mistakes.
    - Add jump cuts and slow zooms (with consistent eye framing so
      cuts feel less jarring).
    - Generate titles and graphics timed to the spoken content.
    - Duplicate timelines and make branded variants.
    - Export a finished file and verify settings (resolution, fps,
      audio, duration, file size).
  - **A prompt structure that gets solid edits** — *"Use the last take for
    each section (even if the wording changes slightly). Keep pacing
    relatively quick, but allow a little breathing space when transitioning
    between sections. Add jump cuts and zoom-ins."*
  - **The fix: swap in a Deepgram transcript** — *"In this test, Deepgram
    improved cut accuracy because the transcript isn't just plain text. It
    can be a full JSON with: Gaps; Word-by-word timestamps; Timing data
    to help identify stumbles, repeats, and micro-pauses. Once the
    Deepgram transcript files were provided, the AI rebuilt a cleaner
    timeline in about 4 minutes 47 seconds, with noticeably better
    spacing and timing, without needing extra guidance beyond the
    original editing notes."*
  - **What you need before this works** — *DaVinci Resolve 21.1; for
    the transcript workflow shown here, a Deepgram API transcript
    with word-level timestamps.*

- **What it is doing.** A reproduction of video 6's Resolve-21.1-MCP
  test, with the explicit finding that **swapping in a better
  transcript with word-level timestamps** is the single highest
  leverage change. The fix took rebuild from ~5 minutes to 4 m
  47 s with better pacing and no extra prompt.

- **What we can use.** This is the strongest single validation
  for our Parakeet + wav2vec2 word-alignment pipeline. The
  published finding is *"a free local transcript with
  word-level timestamps beats the cloud transcript of a paid
  editor's built-in AI"*. Ours already does this; we should
  document it as the primary selling point in any
  skill-vs-skill comparison.

---

### 9. This is how I make videos now

- **Channel / date / length:** Convex (`@convex-dev`) / 20 October 2026 / 30:38.
- **YouTube description.** *"This is how I make my videos now. Claude Code
  does the editing with me in Tandem, a video editor I vibe coded just for
  me, and we go backwards and forwards on it like a vibe coding session but
  for video. Up until a few weeks ago every video on this channel was
  edited by hand in Filmora, and I don't think I could go back now. So
  for most of this video I edit THIS video, live, with Claude. The first
  pass, the B-roll, the section cards, a cartoon, the lot, mistakes and
  all. Tandem is up on GitHub and MIT licensed."*
- **Caption extracts, in order** (the captions are clean and timestamped,
  the only obstacle is the *natural-language "what I just said"* style —
  see the `[ts:920]` format):
  - *9:20* "And while it's doing that, I actually just move on working
    on a new video, so I've got a second video in process at the
    same time, this Convex 1.47 which just released."
  - *9:52* "I wanted to make sure the transcript was a first-class
    citizen. So, you might have noticed this track here, this
    transcript bar. As as you go through, it it highlights what's the
    words that are actually being said on the screen at that one
    time. You can also see on here on in the near play head um
    different edits that the at the Claude's made or the tran- the
    different sections that it's it's highlighted here. So, um
    this transcript is super important because it means that the
    agent or its sub-agents can always kind of like sync it up to
    what I'm saying and also get the context of what it is that
    I'm saying at any given time. So, the transcript being a
    fundamental part of uh Tandem, I think is super important to
    the to making it work so well."
  - *17:54* "I told you I mentioned Convex. Um, Tandem doesn't actually
    use Convex, but I can very easily imagine me adding Convex to
    Tandem in the near future, you know, if I want to have a
    project share between multiple editors."

- **What it is doing.** The creator (Jamie, a Convex dev) edits
  the very video you're watching in real time, with Claude
  Code driving a custom video editor called Tandem. Tandem is
  MIT-licensed on GitHub. The transcript is a first-class
  citizen: the playhead highlights the words being said, the
  agent reads the transcript to know what's happening, and the
  agent or its sub-agents can sync cuts to the words.

- **What we can use.** Tandem's design point — *the transcript is
  the bus between the model and the timeline* — is the same
  design we already follow: the [`transcript.rs`](../../../src-tauri/src/commands/transcript.rs)
  surface is the only one that any MCP agent can rely on to plan
  a cut, an arrangement, or a shorts pass. Where we differ is
  *the editor*: Tandem is a brand-new product Convex built, ours
  is a real NLE. The interesting open question is whether a
  sub-agent that lives in our MCP, with read/write to the
  transcript, the storyboard, and the `propose_cuts` action, can
  do what Convex's Claude does — drive a single-pass edit while
  the user is on a second project. The test for whether we are
  there is the "while it's doing that, I move on to the next
  video" line.

---

### +1. AI Just Changed Sound Design Forever  *(new in this set)*

- **Channel / date / length:** Zane Hoyer (`@zanehoyer`) / published
  approximately 1 October 2026 / 6:34 / 18.8K views.
- **YouTube description.** *(Not retrievable through any indexer at the time
  of review.)* Babbl Labs' creator profile lists the video as
  *"OpenAI 6:34 AI Just Changed Sound Design Forever"*, indicating
  OpenAI is the tool the video covers.
- **Transcript.** *Not retrievable at the time of review.* Prepublish
  has paused new transcripts; ReelStack does not have notes for
  this video; transcript.im, transcript.you, and the other public
  services returned 404 or access-challenge errors when probed.
- **Inferred content** *(labelled as such — not from the video itself
  but from Zane Hoyer's other public videos in the same period,
  which together describe the same workflow)*:
  - Zane's **"I Found the Perfect Music & SFX (Steal This)"** (June 2026)
    established the keyword-and-Epidemic-Sound pattern: *"There's
    no secret formula or crazy sound design theory to make your
    videos sound insane. It honestly just comes down to knowing
    exactly what to search for and having an ear for what
    actually sounds good."* He lists his go-to keywords: *camera*,
    *casino*, *video game* (for retention hooks), and the rule
    *"you don't want them to be super loud ... just enough so you
    can hear it in the background."*
  - Zane's **"How to Edit Premium 3D Documentary Animations"**
    (Jan 2026) and **"This AI Video Tool Is Terrifyingly Real
    (Higgsfield WAN 2.5)"** (Sep 2025) document his pattern of
    pulling SFX from the generated videos themselves — *"it
    already comes with sound effects. You can just add your own
    music"*.
  - Zane's **"ChatGPT vs Claude Motion Graphics (Scary Good)"**
    (16 Oct 2026, 5:02) extends the same logic to AI-generated
    sound: *"I found out when you stack models together, you can
    essentially supercharge them ... The reason I'm using these
    editing agents with a timeline is so that we can generate
    multiple little pieces, and it will piece it together on
    the timeline for us, as well as apply sound design."* The
    full caption is: *"By the way, you can use Claude with
    DaVinci Resolve, but I use it with InVideo editor because
    the sound design is better."* (See caption extract at
    ~4:39 in Zane's video.)
  - Taken together, the most likely content of *"AI Just Changed
    Sound Design Forever"* is: OpenAI's sound-effect generation
    (the **gpt-4o-mini** or **gpt-audio-mini** TTS-line
    sound-effect beta, the *ElevenLabs Sound Effects* v2 model,
    or the **Stable Audio** sound-effect model — Zane uses
    ElevenLabs and Stable Audio in adjacent videos), wired
    into the DaVinci timeline, replacing his earlier
    keyword-search-on-Epidemic-Sound workflow.

- **What we can use.** Zane's *"sound design gets more audience
  comments than the visual animation itself, which is the
  argument for giving it 50% or more of an editor's time"* (from
  the Modern Creator breakdown of his intros video) is the
  strongest one-line statement of the audio gap we already
  named. Two concrete pieces:
  - **`sfx_generate(prompt, duration_ms, gain_db)`** action
    behind our existing `soundEffects` track — agent calls it,
    generated WAV is placed into the Sound Effects lane
    with the same per-event rules our synthesised `click` and
    `whoosh` use today. One afternoon of work, three of the
    videos in this set use the same pattern.
  - **AI sound-design as a default preview** — when the
    agent cuts, the timeline shows SFX cues as placeholders
    that can be generated in place. This is the same trick
    Zane's "stack models" pattern achieves with
    ElevenLabs-in-ChatGPT; we can do it with
    `sfx_generate` and a dropdown of providers.

---

## Cross-cutting view

The nine videos can be read as nine different answers to one question: *what does the
agent need to see, and what does the agent need to be able to do, to cut a video?*

The answers are arranged below by what the agent **sees** and what the agent **does**,
plus a row for the *style asset* — the file (skill, project, prompt) the creator
maintains for the agent to read. Anything written in **bold** in the table is a
*capability we already ship today* (with the file path or module that does the
shipping). Anything in *italics* is a capability the video depends on and we do
not yet have.

| # | What the agent sees | What the agent does | Style asset |
|---|---|---|---|
| 1 Rourke | AE's UI (screen-scraped) | moves the mouse, types | a one-shot prompt |
| 2 Joris | the open Resolve project, the open invideo project | writes a Resolve Python script; runs a cloud edit | the prompt |
| 3 Jason | `CLAUDE.md` + 20+ skill files; the open NLE | edits on the timeline; routes through any of Premiere / DaVinci / Final Cut / CapCut | **a 1,400-line project folder** (we have [`skill.rs`](../../../src-tauri/src/mcp/skill.rs) but no blueprint) |
| 4 Zane | DaVinci Resolve 21.1 project + OpenArt + YouTube web + Pinterest | builds the story, picks B-roll, generates the graphics | *one master prompt stored in a Google Doc* |
| 5 Greg | the Resolve project | scripts the resolve console, takes screenshots | none — tests the new feature |
| 6 Christian | the raw footage + folder of MCP tools | runs seven jobs in order (intake → transcribe → cut → B-roll → layout → graphics → music/SFX) | a 1,400-line skill file that updates itself |
| 7 Higgsfield | the Resolve project + AE + the script brief | orchestrates across the three tools via computer use | *brand assets + AE templates uploaded each session* |
| 8 Justin | the Resolve project + a Deepgram JSON transcript | runs the standard Resolve-MCP cut flow | a prompt structure with three rules |
| 9 Convex | a transcript bar in the editor | edits live, with sub-agents on the transcript | the editor's source code (MIT-licensed) |
| +1 Zane | OpenAI sound-effect generation | adds cues to the timeline | keyword lists on Epidemic Sound |

### Where every creator uses the same idea

1. **The transcript is the bus.** All nine (and Convex most explicitly) treat the
   transcript as the only thing an agent can rely on to plan a cut. Eight of the
   nine read it; one of them (Joris) builds the rough cut from the recording
   without showing the agent the words. *Ours* is the most rigorous: word-aligned,
   gap-measured, sound-source-vetoed. That is the head start.

2. **The skill beats the model.** Six of the nine credit the *quality* of the result
   to the skill file (or its equivalent), not to the model. Three (Christian, Jason,
   Convex) update the skill itself. *Ours* is a fixed file
   ([`skill.rs`](../../../src-tauri/src/mcp/skill.rs)) — we don't accept edits to it from
   the agent.

3. **The "one prompt" beats the "many prompts."** Three of the nine (Zane #4,
   Christian #6, Convex #9) explicitly say *"I sent one prompt."* The rest
   describe a multi-prompt session, but the impression a viewer walks away with
   is *"the agent did the work, I just pushed the button."* Our
   [`propose_cuts`](../../../src-tauri/src/commands/proposals.rs) flow is closer to
   the multi-prompt style. A "single-proposal, multiple-actions" command would
   close this.

4. **The Colour page is broken in every Resolve flow.** Three of the nine say
   so explicitly: Greg #5 (*"The big downsides"*), Jason #3 (*"it can do
   that kind of work in Fusion but not in Color, which is silly"*), Adil #7
   (the only one who claims it works, and they may be using Computer Use). Ours
   grades at composite time in [`render/composite.wgsl`](../../../src-tauri/src/render/composite.wgsl),
   so we don't have this hole — but we also don't expose a typed
   `clip_grade` action.

5. **Sound is the gap nobody fills.** Christian's stack is the only one
   that lays a music track and SFX on a separate lane, and he does it by
   pulling in Epidemic Sound as a separate MCP. Zane #1 and the new #+1
   each spend a third of the video on sound design. We have a synthesised
   SFX lane ([`soundEffects`](../README.md#features)); we do not have
   music, we do not have generated SFX, and we do not have a beat grid
   to align the lot.

### Where every creator uses a *different* idea

- **Christian (1,400 lines of skill)** and **Jason (86k LoC project folder)** and
  **Convex (whole editor)** sit at three different points on the *build your
  own* curve. Christian is the lightest touch (skill file), Jason the
  middle (folder of glue), Convex the heaviest (entire editor). AeroEdits
  sits between Jason and Convex: a real editor (ours) but with a typed
  bridge the agent can read.
- **Six creators drive a real NLE** (Premiere, DaVinci, Final Cut, CapCut,
  After Effects, Resolve). **Two drive a render framework** (HyperFrames via
  Claude Code, or HyperFrames via Codex). **One drives a cloud editor**
  (Joris's flow B, invideo). We are the desktop-NLE category, which is
  also the one Primal Video (Justin) and Adil at Higgsfield are
  nominally in — but neither of them is actually editing in the desktop
  NLE the way we are; they are *operating* it.
- **One creator (Convex) closes the loop with the editor's own source** —
  Tandem is on GitHub, MIT-licensed, and the user can read it. No other
  video's stack is fully open. The closest is HyperFrames (free, open
  source) plus Claude Code (closed model) plus Epidemic Sound
  (subscription).

---

## What beats us, and what we can use

This section pairs each competitor capability with a one-line answer for AeroEdits
and the file path that already does the work, or the new file the work needs.

### Things that beat us

| # | Where they beat us | Severity | One-line answer for AeroEdits |
|---|---|---|---|
| 1 | **Premiere / DaVinci / FCP all have a vendor-shipped MCP.** Resolve 21.1 ships its own; Final Cut via community MCPs; Premiere via Adobe. | **High.** A creator on those NLEs can install an MCP in two minutes; a creator on ours reads a README. | Ship a one-page "Install our MCP" page that points at our existing [`aeroedits --mcp-stdio`](../../../src-tauri/src/mcp/cli.rs) entry. |
| 2 | **Cloud editors do not need a powerful local machine.** invideo's flow C is the same agent, the same prompt, run entirely in a browser. | Medium. We are desktop-first; users without a GPU or with a small machine are out. | Out of scope, but a "lite" export-only mode on a server-rendered preview would catch the browser-tab creators. |
| 3 | **Five-MCP stacks are normal.** Christian's seven jobs run in parallel against five different MCPs. Most users have one — ours. | Medium. Users who already have Tella and Epidemic Sound are paying for them; an extra MCP is friction. | Document the smallest set of MCPs AeroEdits will play nicely with (FFmpeg, yt-dlp, Scribe, OpenArt); recommend a starter profile. |
| 4 | **Every creator publishes a skill/prompt as a file.** We ship a `skill.rs`; the user can't author their own. | Medium. | `aeroedits skill new <deck>` + `aeroedits skill add <file>` is a one-day feature. |
| 5 | **Music + SFX on dedicated tracks.** Everyone else has it. We have one synthesised SFX lane. | Medium. | `track_add(kind, source)` and a `track_kind` field. |
| 6 | **Generated sound effects.** ElevenLabs SFX v2, OpenAI's TTS sound-effect beta, Stable Audio. | Medium. | `sfx_generate(prompt, duration_ms, gain_db)` action that returns a file path; the Sound Effects lane handles the rest. |
| 7 | **Color node trees via MCP.** Higgsfield claims it works; everyone else agrees it doesn't. | Low. | We grade at composite time. Adding `clip_grade(clip_id, nodes)` is the *defensive* move — covers the day a future Resolve release or Premiere release opens it. |
| 8 | **Image and B-roll generation as a first-class agent action.** Zane #4, Adil #7, Zane #+1. | Medium. | `graphic_generate(prompt, seed_path, brand_id)` round-trips with Higgsfield / OpenArt / DALL-E. The first two are the ones every creator is already paying for. |
| 9 | **Self-improving skill file.** Christian and Jason do it. | High. | `propose_skill_edit(skill_name, diff)` + `proposal_accept` on top of [`proposals.rs`](../../../src-tauri/src/commands/proposals.rs). |
| 10 | **The "while it's doing that, I move on" line.** Convex explicitly edits one video while the agent is still cutting the last. | High. | We already have a project journal. Adding an `agent_run(action_id, expectedRevision)` and a `agent_get(action_id)` that the agent can poll asynchronously is the smallest change. |
| 11 | **Vibe-coded editor with a transcript bar in the playhead.** Convex's Tandem. | Low. We are not going to write a video editor. | Our `storyboard` action ([`storyboard.rs`](../../../src-tauri/src/mcp/storyboard.rs)) is the closest equivalent. |
| 12 | **Real-time preview of agent-driven edits.** None of the videos do this, but Convex's transcript bar comes close. | Low. | Out of scope. |
| 13 | **Single-prompt that does everything.** Zane, Christian, Convex. | Medium. | A "single-proposal, multiple-actions" command that stages every step in [`propose_cuts`](../../../src-tauri/src/commands/proposals.rs) and accepts in one undo. |
| 14 | **The user's transcript has the word timing they need to plan a cut.** Justin's Deepgram JSON insight. | Low. | We do this already. |
| 15 | **Folder-watch ingest.** Pat's `OBS recordings` watcher. | Low. | `aeroedits-watch <dir> --profile <name>`. |

### Things that don't beat us — and are sometimes quietly *worse* than what we already do

- **Christian's headline $27 figure.** Corrected to $200/month elsewhere
  (Income Reality Check). Our price is $0 for the editor and a one-time
  model download for Parakeet — we are functionally cheaper than even
  the corrected figure, with no third-party subscriptions.
- **HyperFrames renders.** Free, but it does not have a frame-by-frame
  audio-aware graph. Our [`graphics.rs`](../../../src-tauri/src/commands/graphics.rs) is
  rendered by the same HyperFrames engine, but the *agent-facing* surface
  is structured (HTML+JSON input, not a free-form code-generation box).
  We win on predictability.
- **invideo's "AI edits without your machine."** Wins on no-machine,
  loses on the creative side — Joris uses it for cloud ingest but
  switches back to Resolve for the actual edit.
- **Christian's "I move on."** Convex literally edits this video live.
  Our [proposals_undo](../../../src-tauri/src/commands/proposals.rs) and the
  `expectedRevision` flow already make this safe *if* the agent
  is willing to drive async. The capability is one MCP action
  shape away.
- **Resolve's "transcript JSON with word timing."** We have wav2vec2
  on top of Parakeet, which gets us there for free. (See
  [README § Word alignment](../README.md#word-alignment).) Justin's
  point is that Deepgram beats Resolve's *built-in* transcript, not
  that ours doesn't beat Deepgram.

---

## Concrete next steps (eight items, ordered by leverage)

These are the eight moves that would close the largest gaps in the smallest
number of code-changes, given the AeroEdits surface as it stands today.

1. **`aeroedits skill new <deck>` and `aeroedits skill add <file>`.** ~30 lines on
   top of [`skill.rs`](../../../src-tauri/src/mcp/skill.rs). Closes gaps #4, #9.
2. **`propose_skill_edit(skill_name, diff)` + `proposal_accept`.** Reuse
   [`proposals.rs`](../../../src-tauri/src/commands/proposals.rs). Closes gap #9
   (self-improving skill).
3. **`sfx_generate(prompt, duration_ms, gain_db)` action.** ~50 lines, plus a
   provider dispatch (ElevenLabs SFX v2, OpenAI TTS sound-effect beta, Stable
   Audio). Closes gap #6 (generated SFX).
4. **`track_add(kind, source)` and a `track_kind` field on the timeline.**
   Closes gap #5 (music + SFX on dedicated tracks). Aligns the
   timeline model with what Christian, Adil, and Zane all reach for.
5. **`graphic_generate(prompt, seed_path, brand_id)` action.** ~50 lines plus
   a provider dispatch (Higgsfield, OpenArt, DALL-E). Closes gap #8
   (image / B-roll generation as a first-class agent action).
6. **`agent_run(action_id, expectedRevision)` + `agent_get(action_id)`.**
   ~50 lines on top of the existing journal. Closes gap #10 (the
   "while it's doing that, I move on" line) and gap #13 (single
   proposal, multiple actions).
7. **One-page "Install our MCP" page.** A static doc, not a code change.
   Closes gap #1 (vendor-MCP discoverability) at the cost of one
   page of documentation.
8. **`aeroedits-watch <dir> --profile <name>`.** ~30 lines on top of
   `claude.rs` / a new `watch.rs` module. Closes gap #15 (folder
   ingest).

## What we are not going to do

Two of the gaps above are explicitly *out* of AeroEdits' scope and the
nine videos are clear about why:

- **Cloud rendering** (Joris's flow B, Convex's plan). Our editor is
  on-device; rendering the export stays on-device. A cloud-render
  path is a different product.
- **Vibe-coded editor source code** (Convex's Tandem). We are not
  going to write a video editor. Our MCP *is* the typed bridge
  the agent reads; that is the position Jason takes with
  Claude Video Editor and the position Convex takes with
  Tandem. Ours is the same position, against a single
  pre-existing editor.

## Closing note

The nine videos tell one story: the agent that wins the cut is the
agent that has the best transcript, the cleanest MCP, the shortest
feedback loop, and the most reusable skill file. We have the best
transcript of the set, the cleanest MCP, and a publishable skill —
but the skill is read-only and the user can't author their own.

The eight items above are the smallest, highest-leverage moves we
could make in the next two weeks of work. None requires touching
the render pipeline, the compositor, the export, or the
on-device model download. All of them are *agent-surface* work,
which is where the nine videos say the leverage is.
