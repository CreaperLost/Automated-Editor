# Reference video scripts

Tooling behind [../transcript.md](../transcript.md), the full captions for the eleven reference
videos analysed in [../reference-video-review-2026-10-21.md](../../creator-workflows.md), and
behind [../video-dossier-2026-10-21.md](../video-dossier-2026-10-21.md), the metadata, **verbatim
descriptions and extracted links** for all nineteen videos in scope.

`transcript.md` is the committed artefact. These scripts exist so it can be regenerated rather than
becoming an untraceable blob — every claim in the review should be checkable against a caption.

## fetch_transcripts.py

Fetches YouTube's auto-generated caption tracks for the eleven videos and rewrites
`docs/research/evidence/transcript.md`.

```bash
python3 -m pip install --user youtube-transcript-api
python3 docs/research/evidence/scripts/fetch_transcripts.py
```

| Flag | Effect |
|---|---|
| `--refetch` | ignore the cache and re-download everything |
| `--only 5 7` | refresh only those 1-based video numbers |
| `--out PATH` | write somewhere other than `docs/research/evidence/transcript.md` |
| `--cache PATH` | use a different raw-caption cache |

Re-running without `--refetch` reads `.cache/raw.json` and only re-formats, so the Markdown can be
regenerated instantly and offline. The cache is disposable and git-ignored.

## fetch_dossier.py

The newer script (21 Oct 2026, second pass). It collects, for each of the nineteen videos in
scope, the caption track **and** the description, metadata, chapter list and every link found in
the description.

```bash
.venv\Scripts\python.exe docs/research/evidence/scripts/fetch_dossier.py
```

| Flag | Effect |
|---|---|
| `--refetch` | ignore the cache and re-download everything |
| `--only 13 14` | refresh only those 1-based video numbers |
| `--only-meta` | metadata only; skip captions entirely (fast, and the way around a caption block) |
| `--allow-partial` | write the output even if some videos failed |
| `--sleep N` | seconds between requests (default 3; raise it to 60–90 when YouTube is blocking) |
| `--dossier-out` / `--transcript-out` / `--cache` | alternate paths |

Outputs `docs/research/evidence/video-dossier-2026-10-21.md` (descriptions, links, metadata, chapters) and
`docs/research/evidence/transcripts-wave2-2026-10-21.md` (the caption tracks that were missing before).

**The descriptions are the point.** The earlier reviews were written from titles and chapter
lists because captions were blocked. The description is where these creators put the artefacts —
the skill, the prompt pack, the MCP server, the repo, the free-claim thread. The dossier copies
every description verbatim and lists every URL in it with the line it came from.

## Things that will bite you

**YouTube rate-limits aggressively.** Fetching all eleven in one burst gets the whole set back with
`IpBlocked`. This already happened once on 21 October 2026, immediately after the initial
collection. What to do:

1. Wait — usually a few minutes, sometimes longer.
2. Re-run. The script reports each failure and **refuses to write the output file** if any video is
   missing, so a blocked run can never silently overwrite a good `transcript.md`.
3. Fetch in smaller batches with `--only`, and pause between them.

**The caption block has two faces, and `--only-meta` is the escape.** After roughly a dozen
successful caption fetches on 21 Oct 2026, the `youtube-transcript-api` stopped working with
`IpBlocked` and never recovered that session. The two symptoms are worth telling apart:

- `youtube-transcript-api` raises `IpBlocked`.
- `yt-dlp --list-subs` with the default `web` player client says *"has no automatic captions"* —
  which is the block wearing a disguise, not a video without captions.
- Switching the player client (`--extractor-args "youtube:player_client=android,ios"`) makes the
  track **list** again, but the actual `timedtext` download then returns **HTTP 429**, and a
  direct fetch of that URL returns Google's *"your computer or network may be sending automated
  queries"* page. The block is on the IP, not on the client.

So the caption endpoint can be hard-blocked for a whole session. The **metadata path is
unaffected** — `yt-dlp` still reads descriptions, chapters and links while captions are blocked.
When captions are blocked, run `--only-meta` first and get the descriptions; try the captions
later with a long `--sleep`.

**The cache is the safety net.** If YouTube removes a video or its captions, the cached captions
still let the Markdown be regenerated. That is worth more than it sounds: these are the evidence
base for a whole research document.

**Do not "fix" the mangled brand names in bulk.** Higgsfield legitimately appears as *Higsfield*,
*Hicksfield* and *Hixson* in these captions. That is what the speech model produced; the file header
says so, and silently rewriting it would break the "quote a timestamp and check it" workflow the
review depends on. Fix a spelling only when you are annotating a specific finding, and say so there.

## check_doc_links.py

Not a fetcher — a guard. This repo is link-heavy: `docs/overview/themes.md` cross-references a dozen
sibling documents, and the research corpus added hundreds more links. A renamed or deleted file
breaks every link that pointed at it, and nothing in the build notices.

```bash
.venv\Scripts\python.exe scripts\check_doc_links.py --all
```

| Flag | Effect |
|---|---|
| `--path docs/analysis` | scan one subtree instead of the whole `docs/` tree |
| `--all` | also scan the repo-root `README.md` |

It checks inline links (`[text]` then `(target)`) and reference definitions `[id]: target`, resolving each
relative target against the document that contains it. It skips absolute `http(s)` and `mailto:`
links (checking them would make the run depend on the network) and fragment-only links (GitHub
regenerates those from headings). It exits 1 when anything is broken, so it works as a pre-commit
gate.

**It has already earned its keep.** The first run found five links pointing at source files that
had been moved — `src-tauri/src/mcp/graphics.rs` (which is `src-tauri/src/commands/graphics.rs`)
and `src-tauri/src/commands/check.rs` (which is `src-tauri/src/check.rs`). Both mistakes were
already *known and written down* in `overview/open-questions.md` as C9, which is exactly the
failure mode this tool exists to catch. The corpus now has **794 relative link targets and zero
broken**.

## Changing the video list

`VIDEOS` at the top of `fetch_transcripts.py` is the single source of truth: key, title, channel,
published date, YouTube video id, runtime. Add there and re-run. The order of that list is the order
of the sections in the output, and matches the numbering used throughout the review document — keep
them in step.

Video 7's channel is recorded as "Chase AI / Higgsfield" because the install link in its description
is a Higgsfield one and the channel was never confirmed. Its runtime is also unknown. Both are
placeholders; correct them if you can confirm.