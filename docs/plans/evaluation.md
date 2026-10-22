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

## Open findings (not fixed)

- **ElevenLabs rejected its API key** on this PC: check the key in Settings > Transcription.
- **Dry runs in automatic mode** (T1): 5 of 19 calls are `apply: false` before each pass, by
  design (the "stop if it looks wrong" guard). Moving that guard into the app (apply unless it
  would cut more than a quarter...) would save those calls, about 10 % of T1's cost.
- **One graphic per call** (T2: 7 `graphic_add`): a list in one call would save 6 calls.
- **Graphics in a short cover the face 22 of 31.6 s** (full frame) and mix two looks (white
  library graphics, navy key points): a style kit (STY-1) and a "keep the face" rule for
  shorts are the fix, not a code change.
- **The key-point card is plain** (a navy slide, much empty space): GFX-3.
- **Undo history does not survive a restart** (SAFE-2).
- **No in-app run report** of time and provider spend (the other half of the old EVAL-1).

## Rerun

Same inputs, same messages (T1, T2, S1, then the user's answer to what the agent asks), a
fresh project. Compare calls, cost per turn and the corrections needed. Rerun after each cost
or quality change (COST-1..3, a lighter tool list, a cheaper model).
