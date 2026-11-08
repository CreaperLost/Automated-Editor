# Roadmap

Updated 2 November 2026. This is the single backlog. Keep work IDs stable; remove shipped
items and put lasting behavior in [Product](product.md) or [Architecture](architecture.md).
Validation and measured results live in [Verification](verification.md).

## Next: STY-2 shorts style

<a id="sty-2"></a>
**STY-2 — Finish the portrait variant from George's references.** The style fields, My library
long-form style, reference proposals and sound set are built. GFX-8 is complete. Step 4 is the
remaining work, waiting for shorts references; it is not a request to rebuild the style system.

1. Break down the references into palette, typography, motion, useful graphic kinds and what
   to leave out. Keep the breakdown in the style.
2. Add portrait motion rules, kinds and linked library examples to the same style. Check fonts
   and files. The current style sound set remains opt-in; the first cues were judged weak.
3. Try one representative graphic on the 25 Oct take with the real face, captions and audio.
   Resolve the over-face placement tradeoff: lifting a low panel can clear platform UI while
   obscuring the mouth. Show the sample before building a full set.
4. Compare relevance, readability at phone size, timing and coherence. George must want to keep
   it. Then test another topic/aspect ratio and rerun EVAL-1 for workflow or quality changes.

The existing Style panel can start reference intake. The richer motion/kinds/examples fields
are available through the tools; dedicated editor lists for all these fields are still absent.

## Next: FCL faceless videos

<a id="fcl"></a>
Asked for on 7 Nov 2026. Bring George's Explainer Video Automation (`Desktop/Explainer-Video-Automation`,
Python, ~15k lines, built for an unattended VPS) into AeroEdits as a **Faceless** tab with two
modes: **Long** (5–8 min, 16:9) and **Short** (30–58 s, 9:16). AeroEdits replaces that
repository. Decisions are at the end.

**What that repository does.** Discover → ideas → triage and score → research dossier →
script → three reviews → metadata → voice → storyboard → scenes → render → thumbnail →
publish. Each video has one folder with one file per stage; a stage is skipped while its
inputs are unchanged. What carries over, by code inspection of that repository on 7 Nov:

- **Discovery** (`sources.py`): Google News angles, outlet RSS, Hacker News, Reddit, GitHub
  stars and Trending, Hugging Face papers, security feeds, lab sitemaps, OpenRouter and GitHub
  releases. One failing funnel never blocks the others; a funnel report says why each failed.
- **Ideas** (`ideas.py`, no model): groups reports of one event, scores outlets, attention,
  hooks and recency; viral repositories and papers ranked apart by what they gained.
- **Evidence** (`research.py`): the model proposes facts with exact quotes, then code alone
  checks that each quote appears word for word in the fetched page and every figure appears in
  the quote. Sources are grouped into independent origins; grades `ready`, `thin`, `empty`.
  This gate is the most valuable part and ports as code, unchanged in meaning.
- **Words** (`style/script/*.md`, `script.py`, `factcheck.py`, `metadata.py`): format rules,
  attention / cold-reader / fairness reviews by a different vendor, numbers must come from cited
  quotes, five title shapes, time words ("Just", "today") checked at release time, the source
  list built by code from cited fact ids.
- **Voice** (`voice.py`): ElevenLabs per scene, sped to 1.2x with `atempo`, word timings from
  the unsped audio, misheard names and numbers re-voiced once.
- **Pictures** (`style/frame.md`, `style/parts.js`, `storyboard.py`, `scenes.py`): the locked
  Broadsheet style and twelve part kinds (source, claim, comparison, chart, timeline, flow,
  artifact, gates, tiles, anchor, objects, flip3d), each entering and acting on spoken cue
  words, in one continuous HyperFrames stage with a camera moving between stations. Pacing
  (a change every 5–8 s, gaps over 12 s), reading time and variety are noted, not blocking.
  External assets: sourced logos, flags and maps with licences (`data/broadsheet-entities`),
  an asset catalogue, generated images.
- **Sound** (`sound.py`): generated openings and a looped beat about 18 dB under the voice.
- **Package** (`thumbnail.py`, `style/thumbnails`): six thumbnail templates chosen by title
  keywords, filled from a one-subject brief with reference images, size checked; Shorts use
  the opening frame. One thumbnail per video, no A/B.
- **Publish** (`publishing.py`, `zernio.py`): Zernio upload, private, draft or scheduled, a
  journal so a retry never posts twice, AI disclosure always on, the real visibility read from
  the YouTube entry, a reminder to check ads in Studio. `films/done.json`: never the same story
  twice, tests included.
- **Not carried over:** the hourly tick, fallback ladder, finished reserves, Discord heartbeat,
  dead-man's switch and budget ledger exist because nobody watches the VPS. In AeroEdits every
  step starts from George or his agent (decision 6).

**How it maps onto AeroEdits.** A faceless production is an empty project (they exist today)
whose picture is graphics and whose sound is generated narration. Narration lands as an Audio
asset with a transcript, so word anchors, never-chopped graphics, captions in shorts, chapters,
`check`, storyboard, versions, undo and GPU export all work unchanged. Who does each step:

| Step | Who | Where in AeroEdits |
|---|---|---|
| Discover, rank ideas, funnel report | App, no model | new `faceless/discover`, app-level store of stories and signals |
| Triage, pick | Agent (decision 1) | reads the ranked list through MCP; the Ideas panel for George |
| Fetch sources, verify quotes, origins, grade | App, no model | `faceless/evidence`; private addresses refused, PDFs with page numbers |
| Search, extract facts | Agent | submits facts; the app keeps only those the code verifies |
| Script and the three reviews | Agent | Faceless Long / Short presets; method from `style/script/*.md`; app validators |
| Title, description, tags | Agent writes, app checks | title rules, time words, sources built from fact ids |
| Voice and word timings | App, user's key | ElevenLabs or OpenRouter TTS per scene; Scribe timings already built |
| Beats | Agent | the existing plan table and `graphic_add`, cue words resolved by the app |
| Parts and look | App | Broadsheet as a style; parts as library templates passing the gate |
| Sound | App | generated cues and bed in the sound library; ducking exists |
| Render, loudness | App | existing export, −14 LUFS normalisation exists |
| Thumbnail | App, user's key; agent writes brief | templates and references kept with the style |
| Publish | App, user's key | Zernio client with journal; private by default |

**The Faceless tab.** A top-level tab beside Edit / Cleanup / Shorts / Clips that also works
with no project open, with a Long | Short switch and a stage rail showing each production's
stage, stale stages and spend. Panels, docked like the other workspaces:

1. **Ideas:** ranked stories, long and short fit, sources per story, funnel health, a badge
   for stories already made, "Make long" / "Make short" / "Short from this long".
2. **Research:** the dossier: facts with their quotes and pages, verified or refused and why,
   origins, grade, gaps.
3. **Script:** scenes with cited fact ids, estimated length against the format's range, review
   findings and the accepted revision.
4. **Voice:** a take per scene, play and regenerate one scene, misheard names and numbers.
5. **Visuals:** the existing timeline, preview and graphics panels, plus a beats list with
   pacing, reading time and variety notes.
6. **Package:** title, description, tags, chapters, the thumbnail with a 160 px preview.
7. **Publish:** account, private / scheduled / draft, status with real visibility, Studio
   link for the ads check.

### Work, in order

Sizes as in Deferred work. Each item is built and tested alone; the agent can drive each one
through MCP as soon as it exists.

| ID | Work and acceptance | Size |
|---|---|---|
| FCL-0 | **Built 7 Nov, not committed.** `src-tauri/src/faceless/`: a production is a project in `Documents/AeroEdits/Faceless` with `faceless/production.json` (format, topic, per stage the hash of what it read and wrote; outside undo). Seven stages, one file each in `faceless/`; a stage whose input changed is stale, and so is everything after it (`record_stage`). App-wide made list in `<config>/AeroEdits/faceless/made.json`; a new topic matching it warns. Start screen card → Faceless page (Long / Short, topic, productions with their stage rail); a production opens on its own Faceless tab (Production panel beside the editor's panels), shorts at 9:16. MCP `faceless` behind `more` (status, list, create, made). Checked in the dev app: a short made from the page opened at 9:16 on the Faceless tab, and `more faceless` listed it. | M |
| FCL-1 | **Built 7 Nov, not committed.** `faceless/discover.rs`, `ideas.rs`, `profile.rs`, `stories.rs`: Hacker News, OpenRouter models, GitHub releases, 53 feeds, Hugging Face, viral repositories, papers and lab sitemaps, plus X and Reddit through Treg (see Treg below), a week of stories in `<config>/AeroEdits/faceless/stories.json`, grouping and scores as in `ideas.py`, the channel profile built in (`aero-ai-news.json`, a user copy `faceless/profile.json` replaces it). Ideas list on the Faceless page with "Search the web", set aside, "Make long/short"; MCP `faceless` discover / ideas / setAside / create from an idea. Live on 7 Nov: 8 of 8 sources answered, 712 stories in 25 s. **Reworked 8 Nov after reading real results:** grouping weighs rare words and shared numbers ("$11 billion" = "$11B"), stems plurals, compares headline to headline and needs the group's lead or half the group (single-link chaining had joined OpenAI's round with Anthropic's IPO); scores have visible parts (coverage with X and Reddit once each, attention and its growth, fit from the profile's hooks with a negative "mainstream" hook for business, courts and schools, freshness, primary, low value), separate long and short rankings, and an editor's read (`triage`: importance, long and short fit, angle) worth 40%; pin, separate and join fix groups by hand; each search keeps the top 30 of both rankings so the list shows movement (▲, ▼, new). The Faceless page's list is now the ranking review for the chosen format (top 10, score bars, rate, split, join), and `faceless` `ideas` gives the agent the same table; the methods tell the agent to review and triage the top 10 before proposing. Measured on the real 7 Nov stories: wrong merges and chaining gone, the OpenAI round one story with 19-24 outlets, the Claude Code key leak first in long. *Still open:* some events still split when their headlines share little (the agent or user joins them); fit and attention are rough guesses until our own videos' numbers come in. | M |
| FCL-2 | **Built 7 Nov, not committed.** `faceless/evidence.rs`: the app fetches every page itself (public addresses only, each redirect checked, 3 MB, paywall and cookie-wall detection, an honest user agent), keeps its text in the production, and keeps a fact only when its quote is on that page and every number of the statement is in the quote; origins, corroboration and grades as in `research.py`; research is done only at `ready`. MCP `faceless_research` (fetch, source, add, remove, brief, read); a Research view in the Production panel. Checked live: a real quote kept, a planted "9 out of 10" refused. PDFs, Google News links and script-built pages are read through Treg when a key is set (see Treg below); `search` finds pages through Treg. *Still open:* sites that refuse automated readers stay unreadable and the agent finds the words elsewhere (The Hacker News reads fine since the user agent became honest). The Astra rebuild was not run. | M |
| FCL-3 | **Built 7 Nov, not committed.** Presets Faceless long and Faceless short (a production is put on one when made; the short one is 9:16 with captions), their method = the steps plus the explainer's `shared.md`, `long.md`/`short.md` and `review.md`. `faceless/script.rs`: a script is kept only when every scene cites real facts or is analysis, no number is missing from its cited quotes (title too), no sentence repeats and the length fits (long 875-1400 words, short 87-169 at 1.2x); style is noted, not refused. Reviews are saved with their findings; an open blocker keeps Review not done until the script no longer says it. MCP `faceless_script` (read, check, write, review, resolve). *Still open:* no Script panel in the UI yet (the Production panel shows the stage); an agent has not yet written a full script on a new story. | M |
| FCL-4 | **Built 8 Nov, not committed.** Voice through OpenRouter on the user's key with the explainer's locked voice (Will, `elevenlabs/eleven-v4-turbo`, $0.00004 a character: about 4 cents a short, 30 a long), one MP3 per scene kept with its fingerprint so only changed scenes (or `scenes` asked for) are paid again; `atempo` 1.2 to WAV, scenes joined with a 0.25 s lead and 0.45 s gaps (`faceless/voice.rs`); the narration is imported as a Mic sound and placed at 0 on the timeline in place of an earlier one (one undo step), then transcribed with the saved transcription settings (not Whisper: the app's own transcript is what graphics and captions use); model names and numbers the transcription heard differently are listed per scene (the explainer's QA), and a length outside the format names the pace to rewrite at. The stage is done when nothing is listed or the user accepts it. Voice in the profile (`voice`), MCP `faceless` `voice` / `acceptVoice`, Voice section in the Faceless panel. *Still open:* not run on a real script yet (one live speech call checked the model and voice). | M |
| FCL-5 | **8 Nov: the Broadsheet style and four parts built, not committed.** A built-in style `Broadsheet` (paper #F0E9DA, ink, vermilion #C8321F; Newsreader and IBM Plex Mono shipped in the app as OFL fonts, a new `data` font for labels and numbers; square corners, no shadows, no bounce; the frame.md rules, motion, kinds and sound policy), worn by both Faceless presets and taking the `broadsheet` image look. New templates `headline`, `quote` (marker sweep and margin note), `verdict` (stamp on the cue) and `bars`, each with a source tag; all four pass the real gate at 16:9, 9:16 and the split part. The other parts (timeline, flow, artifact, gates, tiles, anchor, objects, flip3d) map to existing templates or a graphic of the agent's own for now. Broadsheet style and parts: frame.md tokens as a style (16:9 and portrait variant, fonts as files), the twelve parts as library templates with entry and action cues, each passing the gate at both shapes. Licensed logos, flags and maps as library assets with source and licence. GFX-5 holds: one look per video, chosen like any style. First align HyperFrames: we pin 0.8.228, the explainer's parts were built on 0.9.17. *Done when* every part passes the gate and a contact sheet reads at phone size. **Image library built 7 Nov, not committed:** the explainer's 201 catalogued pictures (Broadsheet illustrations, the sourced flags, maps, wordmarks, portraits and pages, the 3D textures and turnarounds) plus a 68-picture gap set made in Flova (markets, courts, elections, an executive, humanoids, deepfakes, chips, space, social, products), and on 8 Nov the same 60 everyday subjects in four looks for the app's styles (light, dark, editorial, neon): 509 in all, one folder per look (`broadsheet`, `light`, `dark`, `editorial`, `neon`, and `sourced` shown with every look), in `src-tauri/assets/images` as WebP/SVG (25 MB at quality 80, built in by `build.rs`; the repo is the only copy, `scripts/add_images.py` adds a set, `assets/image-looks` keeps each look's rules and approved tile); a style takes the look naming it, else the nearest by colour, and `list` shows the project's look unless `look` is given, one merged `catalog.json` with what each depicts, where it goes and its licence (`LICENSES.md`); `library` `kind: image` list (groups, or `query` by what a beat shows), show (a board of a picture or group), use (copied into a graphic as `images/<file>`, one undo step); `check` flags a graphic carrying one that needs credit or a check, or one of another look than its style's. *Still open:* no Images panel in the app; the parts and the style itself. | L |
| FCL-6 | **Built 8 Nov, not committed.** `graphic_add` `actionWord` sets a part's `cue` from the narration's words, so it enters on one word and acts on another. `faceless/visuals.rs`: `faceless` `op: visuals` reads the graphics on the timeline and blocks on anything with nothing on screen; notes more than 12 s without a change (starts and cues), more words than 3 a second allows, the same template twice in a row, a start between words, fewer than 6 kinds in a long video; `fill: true` first stretches each graphic to meet the next (one undo step). Writes `beats.json`, records the stage when nothing blocks. Visuals section in the Faceless panel (check, close the gaps, the list with the words under each). Method steps 6-8 say how to plan, show (`plan: true`), place and check. *Still open:* generated images (user's key) and imported screenshots; not run by an agent on a real film. | L |
| FCL-7 | Sound: the explainer's generated openings and beat as library sounds, on by default in the Faceless presets, ducked under narration. *Done when* an export measures −14 LUFS with the bed under the voice. | S |
| FCL-8 | **Built 8 Nov, not committed, as part of PUB-1 made general:** the faceless package is the video's publish package, the same as any video's. `faceless/package.json` mirrors it and the stage is done when it has a title, description, tags and a thumbnail (after the visuals); an upload with a YouTube id records Publish. The method's step 7 says what the agent writes. *Still open:* thumbnail generation from a brief. | S |
| FCL-9 | Publish for faceless: PUB-2 from the Publish panel, with the faceless extras: time words checked at release time, sources rechecked before upload, made list updated. *Done when* a new faceless video uploads privately. Needs PUB-2. | S |
| FCL-10 | Automation: one `faceless` MCP tool behind `more` with operations (like `library`), skill text, and Preset Run for the free and paid app steps (discover, fetch, voice, render) that stops at each agent or George decision. EVAL-1 for one long and one short with corrections, time and cost. *Done when* both formats go from idea to private upload with only George's picks and approvals. | M |

**Publishing for every video (decided 7 Nov).** Title, description, thumbnail and upload are
not faceless-only: they live in the Export dialog for every video (tutorial, short, clip,
faceless), next to the finished exports, so any export can be packaged and uploaded there.
The faceless Package and Publish panels show the same fields.

| ID | Work and acceptance | Size |
|---|---|---|
| <a id="pub-1"></a>PUB-1 | Package in the Export dialog: title, description, tags and thumbnail per video and per short or clip, saved in the project. Chapters fill the description. Thumbnail: pick a frame, import an image, or generate one from a brief with the user's OpenRouter key using the style's templates and references; size check (1536×864 asked, 1280×720 or larger, under 2 MB), 160 px preview. Optional title and description drafts by the agent. *Done when* a tutorial export has all three saved and shown beside its delivery. **Built 7 Nov, not committed, except thumbnail generation:** `src-tauri/src/publish/`: a package per video in the project's `publish/package.json` (outside undo); an Upload button on each finished video in the Export dialog opens Publish: title (count to 100), description, tags, the AI label (on by default for faceless), "Add the chapters" (YouTube's format, only when YouTube would read them), a thumbnail from an image or a frame of the export, checked (1280×720 or larger, shrunk under 2 MB) with a 160 px preview. MCP `publish` set / thumbnail. **8 Nov: made general:** the package is a section of the Export page ("Title, description and thumbnail", saved as you type, with what it still needs), the same editor in Publish, a thumbnail from the edit's frame at the playhead (no export needed; MCP `thumbnail` with `atUs`), `publish` out of `more` into the main list, and the export steps of every preset ask the agent to write the package. *Still open:* generating a thumbnail from a brief (the explainer's templates and reference images need a home in the style first), and the dialog was not opened on a real export (unit tests only). | M |
| <a id="pub-2"></a>PUB-2 | Upload through Zernio. **Connecting:** Settings → Accounts → Zernio: a "Get a key" link, paste the key (stored like the other keys), then the connected YouTube channels are listed with a default; "Connect a channel" opens Zernio's dashboard in the browser and the list refreshes on return. **Uploading:** an Upload button on each finished export in the Export dialog: private, scheduled (time picker, past times refused) or draft; AI disclosure as a per-upload switch, on for faceless; category; the journal so a retry never posts twice; status with the YouTube entry's real visibility; a Studio link to check ads. Never public without George's click. MCP gets the same action. *Done when* a tutorial export uploads privately from the dialog, and a mocked interrupted upload makes no duplicate. **Built 7 Nov, not committed:** `publish/zernio.rs` and `publish/mod.rs`: File → Accounts (open Zernio, paste the key into the keychain, pick the channel; the only one is picked by itself); Publish uploads private, scheduled (at least 5 minutes ahead) or draft, `madeForKids` false, the journal written before posting with a stable idempotency key, a lost reply looked up instead of posted again (tested), status and Studio link per upload, a faceless upload goes on the made list. MCP `publish` upload (only on the user's go) and uploads. `open_url` opens only Zernio and YouTube pages. Zernio connected on 8 Nov (key in the keychain, channel Aero picked). *Still open:* the first live private upload, to be done by George from the Export dialog (the agent's attempt was stopped by its permission check). | M |

**Treg (8 Nov, not committed).** `faceless/treg.rs` calls `https://treg.to/call/<endpoint>` with
`X-Treg-Token` and a $0.05 ceiling per call (`X-Treg-Route-Max-Cost`); the key is in the
keychain (File → Accounts, or `TREG_API_KEY`). Used for: X (`treg.x.search.posts`, five
profile searches in X syntax with `min_faves:`) and Reddit (`crawl4ai.reddit.subreddit.posts`,
eight subreddits, top of the day) in the story search; `faceless_research` `search`
(`treg.web.search`); and reading what the app can't, through `treg.web.extract`: PDFs, Google
News links (followed to the publisher) and script-built pages. Sites that refuse automated
readers are not routed through Treg. Measured 8 Nov: a story search with X and Reddit cost
$0.0099 (19 posts kept from X, 120 from Reddit); a PDF, a Google News link and a web search
cost $0.0001 each. Responses are each provider's own shape, read tolerantly; a provider change
can need a field added.

**Research watchlist, filters and demand (8 Nov, not committed).** George's decisions: no
connected Google or YouTube accounts in Treg; searches only on his click or his agent's call (no
daily cap); X and Reddit (not Bluesky or Threads); TikTok and Instagram for shorts; no AI
visibility. Built: a watchlist in the profile (54 verified X accounts in labs, leaders,
research, news, security and creators groups, each with a likes floor; 14 verified YouTube
channels; TikTok and Instagram creators added from "Find creators"), searched in batched
`from:a OR from:b` X queries only `since:` the last search; filters before the agent (a lab's
post passes and is primary, leaders must be on subject, hashtags don't count, news and creators
must beat their own usual likes, unknown authors need 5k followers, each subreddit's top 5);
a shortlist of at most 15 per lane with why the rest were left out (`ideas` shows only it,
`all: true` everything); a $0.10 guard per search. "Check demand" for the top 10: YouTube videos
already on each story and their views per hour, the watched channels' videos against their
usual views (refreshed weekly), Google volume and trend for the subject, TikTok and reels for
shorts; demand adds to the score and saturation takes a little off. Watchlist editing, "Suggest
accounts" (who several tracked accounts follow) and "Find creators" in a Sources panel and
`faceless_sources`; each account's kept posts and top-10 hits; keyword suggestions with volume
and trend in Publish (`publish` `keywords`). Measured live on 8 Nov: a search $0.015 (X 9
queries, 7 of 24 posts kept; Reddit 32 of 40), demand for 10 stories $0.040 including the
channels' first refresh, X suggestions $0.0036.

Later, only when a real run needs it: a competitor watch beyond the demand check;
our own video stats feeding the ranking; native YouTube upload (Google keeps uploads from
unverified API projects private until audited, to confirm).

### Decisions

Decided by George on 7 Nov:

1. Writing, review and storyboard run in the user's agent over MCP; the app calls only paid
   media models (voice, images) with the user's keys.
2. Separate graphics on the timeline, not one continuous stage. A "stage" graphic kind can come
   later.
3. Upload through Zernio with an easy connection in Settings, and package plus upload in the
   Export dialog for every video (PUB-1, PUB-2).
4. Port the Python to Rust.
5. Funnels and topic scope belong to a channel profile; the Aero AI-news profile is built in.

6. AeroEdits replaces the explainer; the VPS pipeline is not kept. Nothing runs on its own:
   every step starts from George's click or his agent's call (Codex, Claude Code). No timer,
   no unattended mode, no fallback ladder or reserves, and no need to keep the explainer's
   file shapes.
7. FCL-0 to FCL-4 and PUB-1/2 start now; FCL-5 comes after STY-2's portrait work, which it
   reuses.

This widens [Product](product.md#purpose-and-success) beyond screen-recorded tutorials and
promotes "title/description drafts" and "cover frames" from the unscheduled ideas below. Update
Product once the decisions are made.

## Checks waiting on George

These do not block unrelated work.

- **FIX-9:** enable Transcription settings → AI agents → Allow, then verify a real Codex
  transcription session. Verify recorded authorization, not just response wording.
- <a id="cost-2"></a>**COST-2:** determine whether the Claude desktop app sends every tool
  upfront. Claude Code and Codex measurements already support on-demand schemas. Change tool
  exposure only if this client needs it; do not assume Codex refreshes lists mid-session.
- Listen to built-in music and real take matching. EQ strength (60% of the measured difference,
  capped at 4 dB) is still a tuning choice, and mood tags need ears.
- Renew the rejected ElevenLabs key in settings, or continue using OpenRouter fallback.
- Supply a longer real Greek recording: EL-B is only 13 seconds, so EL-A's synthetic case
  still carries most of the Greek evaluation.

## Unresolved engineering and review issues

Carry these into a relevant change rather than keeping a second gaps catalogue. Older reports
are evidence to reproduce, not newly verified defects.

| Issue | Next check or constraint |
|---|---|
| Direct actions check the project handle before acquiring the command lock | Recheck project switching under concurrent calls; proposals already validate under the lock. AGT-1 follow-up. |
| Stopping long analysis is supported; resuming is not | Add only for a job that proves expensive to restart. |
| No in-app time/provider-spend report | EVAL-1 measures agent spend; do not report that as the total cost of a video. |
| One run each favours fast review over thorough on four recordings ([Verification](verification.md#fast-against-thorough)) | Repeat before changing the long-form default or the method's advice; one model, 33 labelled spans. |
| `check` reads a graphic's frames at 3 to 6 moments and the Agent panel does not embed the sample clip | A brief overlap between moments can pass. Sample suspect graphics with `frame_render`. Whether a 9:16 graphic reads at phone size is still judged by eye: the check finds words under the platform's bars, not words too small. |
| Over-face layouts can hide captions or mouth or enter platform bands | Resolve in the STY-2 sample; a transparent composition root is not proof the face is visible. |
| A restart reworded with no shared opening words has no flag ("what you need to do is" → "you need to go"), so only a thorough review reads it | Add a labelled case first; guard deliberate repetition and meaningful connective words. A word repeat the reviewer is shown can still be kept wrongly. |
| Some settings lack “changed here” chips (layouts and short settings) | Address with the relevant panel work. |
| Graphic rerenders can take 20–40 s; orphaned unplaced graphic folders can remain after undo | Keep old renders playing; establish a real need before adding draft rendering or cleanup. |

## Deferred work

Order within each group is the suggested order. S ≈ two days, M ≈ a week, L > a week.
None supersedes the shorts style or George's next real edit.

| ID | Work and acceptance boundary | Size |
|---|---|---|
| <a id="loop-1"></a>LOOP-1 | Use accepted/rejected proposals, restored cuts and redone graphics to propose improvements to methods, styles and templates. User accepts each change; no hidden learning. | L |
| <a id="snd-2"></a>SND-2 | Opt-in cues at graphic events: event markers, sound-peak offsets, density limits and ducking, using the style's sound set. Improve the sounds before scaling. | M/L |
| <a id="ftg-2"></a>FTG-2 | Saved selects and B-roll from own footage: reviewed batch, video-only streams, free overlay track, narration untouched, word anchors. Not urgent (1 Nov). | M |
| <a id="ftg-3"></a>FTG-3 | Cache source descriptions/tags from sparse boards against fingerprints; OCR only if keyword search misses. | M |
| <a id="ftg-4"></a>FTG-4 | Natural take ordering, missing/duplicate checks and script-guided assembly with alternatives in one proposal. | M |
| <a id="ftg-6"></a>FTG-6 | Stock providers using the user's key; record licences and download only on acceptance. | M |
| <a id="ftg-5"></a>FTG-5 | Import external word transcripts only if trivially small; not currently needed. | S |
| <a id="lay-3"></a>LAY-3 | Bounded auto layouts on top of `pick`: few changes/minute, screen 2–8 s, camera 2–5 s, punch-in 1.5–4 s, review with list and storyboard. | M |
| <a id="fx-1"></a>FX-1 | Cached per-recording camera segmentation for background removal/blur/replacement; composite in wgpu. | L |
| <a id="lay-4"></a>LAY-4 | Presenter cut-out layout, dependent on FX-1's matte. | M |
| <a id="dlv-3"></a>DLV-3 | Standalone alpha graphics, then constrained FCP7 XML/OTIO handoff. Test in the destination editor and report what was flattened. | M→L |
| <a id="dlv-5"></a>DLV-5 | Real 16:9→9:16 reformat preserving UI targets, cursor relationships and reading time. | L |
| <a id="cost-3"></a>COST-3 | Cheaper narrow reviewers, retained only if EVAL-1 quality holds. Not urgent (1 Nov). | M |
| <a id="ver-1"></a>VER-1 | Repeat Windows bridge-to-export verification on macOS when macOS work resumes. | M |
| <a id="dep-2"></a>DEP-2 | Quarterly dependency review, next January 2027, or sooner for an advisory or measured gain; follow Verification. | S |
| <a id="dep-3"></a>DEP-3 | Tailwind 3 to 4, by hand or checking every change: the upgrade tool changes 266 places in 45 files and also rewrote the saved `EffectKind` value `"blur"`. Then compare every panel with the old build. | M |
| <a id="dep-4"></a>DEP-4 | `ort` 2.0 and parakeet-rs 0.4, when `ort` leaves release candidates: one ONNX Runtime environment handed to every model (seven call sites), a CUDA pack matching the newer ONNX Runtime, then the Parakeet, face detection and pack probes. | S-M |
| <a id="fx-5"></a>FX-5 | LUT import/simple grade with preview/export parity. Parked: not needed for the channel. | M |

AeroShoot work belongs in the recorder repo: exact segment boundaries (O03, overlap up to one
frame) and removal of the 10 ms system-audio gap at boundaries (O04). The editor tolerates both.

Promote unscheduled ideas only when a real edit needs them: alternate review timelines,
narration fixes, relinking/media bins, cross-project copying, arbitrary reference-range export,
cover frames, title/description drafts, reduced motion, versioned delivery safe zones and
music-beat boards. Do not resurrect dropped scope listed in [Product](product.md#out-of-scope).
