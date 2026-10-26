# 15 — The provenance record is written before the clip

- **Source:** [github.com/edigeh/broll-mcp](https://github.com/edigeh/broll-mcp)
- **Licence:** MIT (code) · 26★ · last push 2026-10-23 · captured 25 Oct 2026
- **Raw capture:** [evidence/2026-10-25/edigeh-broll-mcp.md](evidence/2026-10-25/edigeh-broll-mcp.md)
- **Evidence grade:** vendor claim, read from the source. Not installed, no clip was cut.

A small MCP server — four tools — that finds public-domain b-roll on the Internet Archive, cuts it,
and writes down exactly where it came from. It is the smallest project in this batch and the one
whose **one rule** we would most like to copy.

## The rule

> A JSON record sits next to every clip. It holds the source file MD5, the license copied verbatim,
> `commercial_use`, the data node, a timestamp and ready-to-paste attribution.
> **Provenance is written before the clip, so a clip never exists without its record.**

Not after. Not optionally. Before. The ordering is the design: a clip cannot be produced without
its rights already on disk.

And the licence is **never guessed**:

> License labels come only from an exact URL table covering PD, CC0, the PD Mark, and CC BY /
> BY-SA / BY-ND / BY-NC variants for versions 2.0–4.0. A URL outside the table is copied verbatim
> with the label `unrecognized` and `commercial_use: "unknown"`; **nothing is guessed.**

The rights note is equally careful: *"broll-mcp copies license information exactly as archive.org
publishes it, and it makes no legal judgement. `commercial_use: "unknown"` means archive.org states
no machine-readable license for that item."*

## Why this lands on us directly

Our **Q-SOUND** decision already writes this rule for music, and says it in almost the same words:

> **Music must never need credit.** Only tracks whose licence allows monetised use with no
> attribution (e.g. CC0 / public domain or an equivalent no-attribution licence) enter the built-in
> library. The app enforces it: **every track carries its licence, and a track that requires
> credit, forbids monetisation or has an unknown licence is refused** (user imports are flagged
> instead of silently accepted).

broll-mcp is the same rule applied to **footage**, and it brings the mechanism we have not written
down: a fixed table of recognisable licence URLs, a verbatim fallback, and `unknown` as a first
class answer. Our rule says a track with an unknown licence is *refused*. broll-mcp says the
provenance record is still written and the clip is still usable, but its record says `unknown`.

That difference matters for **b-roll in shorts**. Our decisions.md already allows it: a short may
carry "**B-roll, including outside video files**" ([SHT-5](https://github.com/AeroEdits/AeroEdits)).
B-roll from the open web is exactly where an unknown licence shows up. If our rule is "refused",
then a user can never use an outside file unless we can read its licence — and the honest answer
for most web video is that we cannot. broll-mcp's answer — *flag it, record it verbatim, attribute
it, and let the user decide* — is the one that matches "user imports are flagged instead of
silently accepted".

## Three more things worth having

**1. The clip is verified, not assumed.** "Cuts are re-encoded to H.264 MP4 with faststart. ffprobe
confirms the duration is within **±0.1 s** and the decode shows **zero errors**, and you get 3
check frames back. **The same request twice returns the same clip.**"

Three properties in one line: the length is *measured*, the output is *decoded and checked*, and
the operation is *deterministic*. The reason it re-encodes at all is stated plainly: *"Clips cut
with `-c copy` snap to keyframes. An 8 s request came back as 11.35 s."* Re-encoding costs quality
and buys exactness — and they state the trade rather than hiding it.

**2. The agent sees the footage before it cuts.** `inspect_item` returns **one keyframe image per
detected scene**, as an MCP image. *"so a vision model can pick the shot instead of guessing from
titles."* That is the same move as video-use's `timeline_view` ([13](13-the-llm-reads-the-video.md))
and as our own `storyboard` tool, which is already our second-largest token cost (3.6k with its
image).

**3. It is polite and it is honest about being polite.** archive.org's bot guidance is followed:
descriptive User-Agent, 1 req/s token bucket, at most 4 media connections, `Retry-After` support,
exponential backoff. And every tool result reports **`upstream_requests`** — a repeat search or
cut sends **0** upstream requests, and the agent can see that it did.

## Where it sits against AeroEdits

| | AeroEdits | broll-mcp |
|---|---|---|
| Where assets come from | the user's own files (`media_import`) | the Internet Archive's public domain |
| Rights rule | Q-SOUND: unknown licence → refused | unknown licence → recorded as `unknown`, user decides |
| Where the record lives | (not defined for footage) | a JSON file **next to the clip**, written first |
| Verification | the gate checks our graphics | ffprobe ±0.1 s, zero decode errors, 3 check frames |
| Determinism | the renderer is deterministic | *"the same request twice returns the same clip"* |

The row we have no answer for is the third one. **We have a licence rule for music and no
provenance format for footage at all.**

## What we would take

1. **The provenance format**, as a `media_import` companion. Source, licence verbatim, licence
   label, commercial use, attribution text, retrieval time, and the **checksum of the file we
   actually imported**. Written before the asset is usable.
2. **The exact-URL table**, extended to whatever our SFX and music sources turn out to be, with
   `unknown` as a legitimate value rather than a failure.
3. **"Unknown is not missing"** for rights: an unrecognised licence is a *recorded state*, not a
   crash. This is the same principle as finding [14](14-ffmpeg-skill-contract-and-evals.md)'s
   third capability state, applied to the law instead of the machine.

## Caveats

- **26★ and one maintainer.** Small, young, and moving. The archive.org interface it depends on
  could change.
- **Only public domain.** The trust tiers (`prelinger` and `nasa` are tier 1) and the licence
  table cover the Internet Archive's public-domain holdings. That is a narrow, safe slice — not a
  general stock-footage answer, and the footage is 1930s–1950s public-domain material, which
  limits how useful it is for a modern tutorial.
- **MIT covers the code only.** *"Footage belongs to its respective rights holders under the terms
  recorded in each clip's provenance file."*
- **Its licence rule is stricter than ours** — it refuses to classify a licence it does not
  recognise, which means a BY-NC clip is labelled and flagged rather than rejected. We would have
  to decide whether that is our rule too.
- The `commercial_use` field is derived from a URL table, not legal advice, and the README says so
  twice.
