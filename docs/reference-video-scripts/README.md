# Reference video scripts

Tooling behind [../transcript.md](../transcript.md), the full captions for the eleven reference
videos analysed in [../reference-video-review-2026-10-21.md](../reference-video-review-2026-10-21.md).

`transcript.md` is the committed artefact. These scripts exist so it can be regenerated rather than
becoming an untraceable blob — every claim in the review should be checkable against a caption.

## fetch_transcripts.py

Fetches YouTube's auto-generated caption tracks for the eleven videos and rewrites
`docs/transcript.md`.

```bash
python3 -m pip install --user youtube-transcript-api
python3 docs/reference-video-scripts/fetch_transcripts.py
```

| Flag | Effect |
|---|---|
| `--refetch` | ignore the cache and re-download everything |
| `--only 5 7` | refresh only those 1-based video numbers |
| `--out PATH` | write somewhere other than `docs/transcript.md` |
| `--cache PATH` | use a different raw-caption cache |

Re-running without `--refetch` reads `.cache/raw.json` and only re-formats, so the Markdown can be
regenerated instantly and offline. The cache is disposable and git-ignored.

## Things that will bite you

**YouTube rate-limits aggressively.** Fetching all eleven in one burst gets the whole set back with
`IpBlocked`. This already happened once on 21 October 2026, immediately after the initial
collection. What to do:

1. Wait — usually a few minutes, sometimes longer.
2. Re-run. The script reports each failure and **refuses to write the output file** if any video is
   missing, so a blocked run can never silently overwrite a good `transcript.md`.
3. Fetch in smaller batches with `--only`, and pause between them.

**The cache is the safety net.** If YouTube removes a video or its captions, the cached captions
still let the Markdown be regenerated. That is worth more than it sounds: these are the evidence
base for a whole research document.

**Do not "fix" the mangled brand names in bulk.** Higgsfield legitimately appears as *Higsfield*,
*Hicksfield* and *Hixson* in these captions. That is what the speech model produced; the file header
says so, and silently rewriting it would break the "quote a timestamp and check it" workflow the
review depends on. Fix a spelling only when you are annotating a specific finding, and say so there.

## Changing the video list

`VIDEOS` at the top of `fetch_transcripts.py` is the single source of truth: key, title, channel,
published date, YouTube video id, runtime. Add there and re-run. The order of that list is the order
of the sections in the output, and matches the numbering used throughout the review document — keep
them in step.

Video 7's channel is recorded as "Chase AI / Higgsfield" because the install link in its description
is a Higgsfield one and the channel was never confirmed. Its runtime is also unknown. Both are
placeholders; correct them if you can confirm.