# Evaluation (EVAL-1)

The baseline every cost or quality change is measured against ([EVAL-1](../roadmap.md#eval-1)).
Two English inputs, the milestones in order: a long-form tutorial edited with the `tutorial`
template, then a short from a take with the `shorts` template. Greek is later (LNG-4).

## How a run works

- **Inputs** (copies; the originals are never touched):
  - Long-form: `AeroShootRec/Untitled 22 Oct 2026 2.aero`, 7:10, screen + camera + mic +
    system sound + telemetry. A crypto airdrop walkthrough (TX Flow squads, fee credits,
    PaperTrade).
  - Take: `AeroShootRec/Untitled 22 Oct 2026 3.aero`, 1:40, camera + mic, recorded in two
    segments. About Hermes Agent (a personal AI assistant), so a different topic from the
    long-form; its graphics come from the library, not the video.
- **App:** the dev build (`AEROEDITS_MCP_DEV=dev`), a new project made from the long-form copy.
- **Agent:** Claude Code headless (`claude -p`, Sonnet 5.5), the app's MCP server only:
  `--strict-mcp-config --tools "" --allowedTools mcp__aeroedits --setting-sources project
  --disable-slash-commands`, from an empty folder, so no repo, skills or memory leak in. Each
  user message is one `claude -p` (resumed with `--resume <session>`); the stream is logged
  as JSON lines. A resumed session reports its running total cost; per-turn cost below is the
  difference.
- **The user's part** was played by Claude: the messages George would send, accepting
  batches in the Agent panel, and judging the result from frames and the audio's levels (no
  listening).

## Results (22 Oct 2026)

| Turn | What the user asked | Calls | Wall | Agent cost |
|---|---|---|---|---|
| T1 | Edit the tutorial automatically, stop before export | 19 | 3:11 | $0.285 |
| T2 | Add graphics too (correction: the saved answers said none) | 15 | 2:46 | $0.122 |
| S1 | Make a short from this take, footage and graphics under it | 13 | 1:00 | $0.139 |
| S2 | Different topic: library and new graphics, fix the cut ending, a real title | 26 | 1:43 | $0.317 |
| S3 | (Accepted the batch) go on — cut short when Claude rebuilt the app | 4 | 0:37 | $0.049 |
| S4 | Same, again | 6 | 2:53 | $0.090 |

- **Long-form tutorial: $0.41, 34 calls, about 6 min** of agent time for a finished edit:
  7:10 → 5:45, 114 mouth noises muted, 121 pauses (70 s) cut, 25 fillers and 5 retakes cut
  (16 found by the OpenRouter review), camera bubble with one camera-only section, 11 zooms,
  6 key-point cards and an end card, 6 chapters, `check` clean.
- **Short from the take: $0.55, 49 calls, about 6 min**, of which about $0.30 was S2 redoing
  work after Claude restarted the app (undo history is lost on restart, SAFE-2). A clean run
  would be about S1 + one proposal + S4: **≈ $0.30, ≈ 25 calls**. Result: 31.6 s, cleaned
  (tight pauses, mouth noises, one retake), four graphics proposed as one batch (two library
  Hermes graphics, two new key points), accepted, then checked at 9:16 and one fixed.
- **Provider spend (not tracked by the app yet):** two transcriptions on OpenRouter Scribe
  (ElevenLabs rejected its key, so the app fell back), one OpenRouter filler review. Graphics
  render locally.
- **Where the agent's money goes:** each call re-reads the whole context from the cache
  (~21k tokens: the CLI's own ~15k, the tool list ~18k shared, the template and the
  conversation). So **the number of calls drives the cost** more than any one answer;
  T1's 19 calls read 420k cached tokens.
- **Human corrections:** two. Graphics were off in the saved answers (T2), and the clipped
  last word of the take (S2). Both are on the app side, not the agent's judgement.
- **Agent judgement that held up:** it saw that the take and the video were about different
  things and stopped instead of forcing unrelated footage under it; it checked the
  storyboard and fixed placeholder text in a library graphic; it said plainly what it had not
  heard or seen.

## Found and fixed in this round

- **The take's last word was clipped** (S1). The tight gap pass refines word edges at a
  fixed -42 dB; George's mic speaks around -26 to -42 dB, so the fade of "them" (-43 dB) read
  as silence. Word edges now stay at least 24 dB below the speech level (`below_speech_db`):
  a loud mic is unchanged, a quiet one keeps its tails. Rerun in S2: the short ends "…define
  them."
- **"Word w-33 is outside the short"** for words that are in it (S2): a word whose first
  milliseconds a pause cut took was looked up by its exact start. Words now map by the part
  the short keeps, as the transcript view does.
- **No way to rename a short**, so the agent rebuilt the short to change its placeholder
  title (S2). `shorts` has `op: rename`; the template names a take short from its transcript,
  never after the video.
- **The first chapter read 0:00.15** in the agent's answers (T1, T2) and it asked George to
  move it. The export already writes 0:00; the answers now say so too.
- **Graphics rendered twice in shorts** (storyboard tiles round to 1082x1920, export
  1080x1920): one render per graphic now. Checked in S4: every short graphic has a single
  1080x1920 render (a second only for the one whose text changed).

## Run 2 (23 Oct): fewer calls

What changed: in automatic mode the cutting passes take `guard: true` and the app applies its
own "looks wrong" check (a pass cutting more than a quarter of the video, the cleanup more
than a third of what was recorded, mouth noises with no transcript), so there is no separate
dry run; a take is not guarded. `graphic_add` takes `items`, all in one call and one undo
step. Same inputs, same first messages, a fresh project.

| | Run 1 | Run 2 | |
|---|---|---|---|
| T1 tutorial edit | 19 calls, 3:11, $0.285 | 13 calls, 2:33, $0.264 | −32 % calls |
| T2 graphics | 15 calls, 2:46, $0.122 | 7 calls, 2:29, $0.094 | −53 % calls |
| **Tutorial** | **34 calls, 5:57, $0.407** | **20 calls, 5:02, $0.358** | **−41 % calls, −12 % cost** |
| Short (all turns) | 49 calls, $0.55 (with a restart Claude caused) | 42 calls, $0.48 | not like for like |

- **Same edit:** 70 s of pauses cut, 28 fillers and retakes (30 in run 1; the AI review varies),
  12 zooms, six graphics added in one call. The first chapter now reads 0:00, so the agent no
  longer raises it.
- **Calls fell 41 %, cost only 12 %.** Each extra call costs little: it re-reads the cached
  context at a tenth of the price. Most of a turn's cost is **writing** the context once: about
  52k tokens in T1 (Claude Code's own prompt, the 18k-token tool list, the template, the
  transcript). The next saving is a smaller context: tools on demand (COST-2), shorter
  templates, reading the transcript once.
- **The short varies with the agent's choices**, so it is not like for like: run 2 went
  straight to proposing the Hermes library graphics (S1), but its own proposal overlapped two
  graphics, and fixing that took 23 calls ($0.26) because **a graphic's time or length on a
  short can't be changed**: the agent removed both and proposed them again. That is the next
  capability gap for shorts. The last turn fixed leftover template text on all three library
  graphics.
- Found in the rerun and fixed: the guard held `fillers` on the take (it counted the take's
  recording against the long video). A take is not guarded now; recorded length counts only
  recordings on the video's timeline.
- These runs use George's Claude plan: run 2's last turn hit the session limit and ran again
  after the reset.

## Open findings (not fixed)

- **ElevenLabs rejected its API key** on this PC: check the key in Settings > Transcription.
- ~~A graphic on a short can't be moved or lengthened~~: fixed 23 Oct (below).
- **The take short keeps its placeholder title** ("Take short") until asked, although the
  template says to rename it after the transcript.
- **Graphics in a short cover the face 22 of 31.6 s** (full frame) and mix two looks (white
  library graphics, navy key points): a style kit (STY-1) and a "keep the face" rule for
  shorts are the fix, not a code change.
- **The key-point card is plain** (a navy slide, much empty space): GFX-3.
- **Undo history does not survive a restart** (SAFE-2).
- **No in-app run report** of time and provider spend (the other half of the old EVAL-1).

## After run 2 (23 Oct): a smaller context, short graphics

George's call: $1-2 a video is fine, so the cost work stops here; no third run.

- **Rarely needed tools behind `more`:** 18 tools an edit rarely needs leave the list agents
  are sent on every connection; `more` names them, gives a tool's arguments and runs it. The
  list goes from 75,260 to 53,828 characters (about 5,300 tokens less per session). Not
  `tools/list_changed`: Codex doesn't refresh a tool list mid-session.
- **A shorter automatic guide:** `template` with `auto: true` leaves out the questions and dry
  runs a step-by-step run needs, and the thorough review unless chosen; export steps come on
  request. 20,440 → 14,025 characters for the tutorial.
- **The transcript** was already compact (lines with the first and last word id), so it stays.
- **Short graphics:** `graphic_set` with a take short moves (`moveToWord` from the take) and
  lengthens graphics on the short's own timeline, so the 23-call detour of run 2's S2 is one
  call; the app refuses an under-take batch whose graphics would overlap, with the times.

## Rerun

Same inputs, same messages (T1, T2, S1, then the user's answer to what the agent asks), a
fresh project. Compare calls, cost per turn and the corrections needed. Rerun after each cost
or quality change (COST-1..3, a lighter tool list, a cheaper model).
