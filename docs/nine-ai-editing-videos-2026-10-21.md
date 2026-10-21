# Nine AI-editing videos: what they do, what beats us, what to take (21 Oct 2026)

**Evidence status — read first.** YouTube blocked every caption request on 21 Oct 2026
(`IpBlocked` from `youtube-transcript-api`, and "Sorry…" rate-limit pages from the timedtext
endpoint even inside a browser session). All nine videos *have* English auto-captions, but none
could be downloaded in this pass. This review is therefore built from each video's **title,
description and chapter list**, read live from the watch pages, plus what earlier research in
`docs/` already established about the same tools. Claims that need the spoken content are
marked *(unverified — needs transcript)*. To fill the gap, add these ids to `VIDEOS` in
[reference-video-scripts/fetch_transcripts.py](reference-video-scripts/fetch_transcripts.py)
and run it in small `--only` batches once the block lifts (see that folder's README).

## The nine videos

| # | Video | Channel | Length | Published | What it shows |
|---|---|---|---|---|---|
| 1 | [Did AI Change Editing Forever? (GPT-6 Astra + DaVinci Resolve)](https://youtu.be/kOoC3yhUyDQ) | Higgsfield AI | 11:36 | 2 Oct | Higgsfield MCP in ChatGPT; Astra gets footage, script and references; grade in Resolve, motion graphics in After Effects, sound design and mix; long-form → Shorts; full film → 60 s trailer. Sells an "editing skill + SFX library". |
| 2 | [How To Make Motion Graphics With AI (After Effects Full Tutorial)](https://youtu.be/mcpAbTHFIRE) | Rourke | 15:53 | 28 Sep | GPT Astra 6 designs and animates, then the human finishes in After Effects. Names the failure modes: floaty easing, misaligned shapes, comps that fall apart when edited. Ships PDF prompt sheets. |
| 3 | [Yes, AI Edits Your Video Now BUT… Resolve MCP vs invideo](https://youtu.be/4nQGiRCUlDg) | Joris Hermans | 15:32 | 17 Oct | Head-to-head: DaVinci Resolve MCP driven by ChatGPT vs the cloud invideo Editor driven by ChatGPT and "Agent Two" (invideo's remote-agents bridge). |
| 4 | [I Finally Solved AI Video Editing (Full Guide + FREE Setup)](https://youtu.be/sGt1ACELN4k) | Jason Cooperson | 21:19 | 8 Oct | A Claude Code project folder (claimed 86k lines, 335 commits) that edits for hours unattended from raw footage + one prompt, then takes a few revisions. Targets Premiere, Resolve, Final Cut and CapCut. **"Presets: the secret sauce"** chapter. Cost and time-saved chapters. |
| 5 | [AI Edited My ENTIRE Video in DaVinci Resolve (One Prompt)](https://youtu.be/P_NL_hg-y_4) | Zane Hoyer | 6:06 | 8 Oct | Codex + OpenArt MCP on a client's raw footage: silence removal, captions, grade, music + SFX sync, generated images/B-roll, and **editable Fusion motion graphics**. Claims $700 spent refining one master prompt (published as a Google Doc). |
| 6 | [AI Just Changed Sound Design Forever](https://youtu.be/8D9aWgGAkew) | Zane Hoyer | 6:34 | 28 Sep | Agent maps the editor's **own SFX library** onto the timeline (claimed ~40 min saved per video); GPT-6 Astra vs invideo on identical prompts for first-pass sound. |
| 7 | [This NEW AI Update just changed DaVinci Resolve forever](https://youtu.be/r3p2QkIr0PA) | Greg Edits Video | 17:33 | 10 Sep | Resolve's built-in AI: project setup, project clean-up, colour grading, real-time editing, troubleshooting, **controlling your screen**, plus a "big downsides" chapter. |
| 8 | [This is how I make videos now](https://youtu.be/3LIQ5GFmED8) | Convex (Mike Cann) | 30:38 | 20 Oct | **Tandem**, a vibe-coded Mac editor with MCP (MIT, [github.com/mikecann/tandem](https://github.com/mikecann/tandem)). Claude does the first pass; he reviews and **leaves a comment on the timeline** where it missed something; B-roll, section cards, a Seedance cartoon; then **turns the session into a skill**. |
| 9 | [I Fully Automated My Video Editing Using Claude Code](https://youtu.be/HzXD4GVqXwM) | Christian Peverelli | 16:58 | 4 Oct | Tella.tv recordings + Claude Code + **HyperFrames** (HTML-based motion graphics) via a free editing skill. |

## Patterns across the nine

1. **The host NLE is where the human finishes.** Videos 1, 2, 5 and 7 all insist the output stays
   editable (Resolve timelines, Fusion comps, AE projects). "AI made it and I can't touch it" is
   the failure everyone advertises against.
2. **Taste lives in presets and libraries, not prompts.** Video 4's "secret sauce" is presets;
   video 6 maps a *personal* SFX library; video 1 sells a skill + SFX pack; video 5 sells a master
   prompt. The model is the commodity; the curated assets and rules are the product.
3. **Long unattended runs, then short revision rounds.** Video 4 lets Claude run for hours and
   then sends "a couple small revisions"; video 8 does the same interactively.
4. **Review by pointing at the timeline.** Video 8's comment-on-the-cut is the clearest example of
   anchoring feedback to a time range instead of describing it in chat.
5. **Sessions become skills.** Videos 4, 8 and 9 all end with the workflow packaged as a reusable
   skill/folder.
6. **Cost and time are reported.** Videos 4 and 5 show the bill; that is becoming an expectation.

## Where they are ahead of AeroEdits

Compared with what [overview/current-state.md](overview/current-state.md) lists as built:

| Gap | Who shows it | Notes |
|---|---|---|
| **Timeline comments the agent reads** | 8 (Tandem) | Our feedback loop is chat + Agent panel batches. A comment pinned to a time range, readable by the agent as a tool, would make revision rounds precise. Overlaps the review theme in [overview/themes.md](overview/themes.md). |
| **User-owned SFX library with auto-placement** | 6, 1, 5 | Map a folder of the user's own sounds (tagged whoosh/hit/riser/click) onto cuts, graphic entrances and zooms. Pure local, fits our privacy stance. |
| **Edit presets / style profiles** | 4, 5 | A saved, named bundle (pacing, caption style, zoom rules, music ducking, graphic templates) the agent applies, rather than re-deriving taste each run. |
| **Export to an editable NLE timeline** | 1, 3, 4, 5 | Hand-off to Resolve/Premiere/FCP as XML/OTIO so a pro can finish. See [davinci-resolve-research-2026-10-21.md](davinci-resolve-research-2026-10-21.md). |
| **Long unattended runs with a report** | 4 | Agent works for hours, writes what it did and what it was unsure about, user reviews once. Needs checkpointing and a run summary. |
| **"Turn this session into a skill"** | 8, 9, 4 | A tool that writes the user's accepted decisions into a personal template next to our `templates/tutorial.md`. |
| **Cost + time report per edit** | 4, 5 | We already run models on the user's keys; surfacing spend per run is cheap. |
| **Long-form → vertical shorts, film → trailer** | 1 | Repurposing as a first-class job. *(unverified — needs transcript for how selection works)* |
| **AI-generated cartoons/B-roll inserts** | 5, 8 | OpenArt/Seedance generation inserted as slots. Optional cloud step on the user's keys. |

## Where AeroEdits is already ahead or equal

- **Agent calls the same code the buttons call, under revision checks and undo** (see
  [overview/ai-process-today.md](overview/ai-process-today.md)). Resolve/AE MCPs in videos 1–3
  and 5 script a host app with no batch history; video 7's own chapter on "big downsides" is
  likely about exactly this *(unverified)*.
- **Local-first.** Video 3's invideo path and video 1's Higgsfield path are cloud editors.
- **Tutorial-specific structure** (beat plan, slots) is more opinionated than the general
  "one prompt" demos.

## What to take (recommended order)

1. **Timeline comments as an agent tool** (from 8). Smallest change with the biggest effect on
   revision quality. Agent tools: `list_comments`, `resolve_comment`.
2. **Personal SFX library mapping** (from 6). Folder + tags → placement rules on cut / zoom /
   graphic-in events; one batch, undoable.
3. **Style presets** (from 4, 5). Named JSON profile the skill loads first; the "save session as
   preset" action covers pattern 5.
4. **Run report with cost and duration** (from 4, 5). Attach to each agent batch.
5. **NLE export (OTIO/FCPXML)** — already in the Resolve research doc; these videos raise its
   priority because four of nine treat the pro NLE as the finishing step.

Lower priority: generated cartoon/B-roll inserts, trailer mode.

## Follow-ups

- Fetch the nine transcripts when YouTube lifts the block, and re-check every *(unverified)* line.
- Tandem is MIT and on GitHub: worth reading its MCP tool list and comment model directly.
- Video 5's master prompt and video 4's free blueprint are public; compare them with our
  `SKILL.md`.
