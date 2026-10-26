# edigeh/broll-mcp

- **Source:** https://github.com/edigeh/broll-mcp
- **Kind:** github
- **Captured:** 2026-10-25T15:56:53
- **Evidence grade:** vendor or creator claim. Not tested in this repo. Quotes from a caption are what a creator *said*, not a measurement.

## Repository

- **Repo:** edigeh/broll-mcp
- **Description:** MCP server to find, preview and cut public-domain b-roll from the Internet Archive, with a provenance record for every clip
- **Stars:** 26
- **Forks:** 2
- **Open issues:** 0
- **Licence (SPDX):** MIT
- **Language:** Python
- **Default branch:** main
- **Last push:** 2026-10-23T13:58:34Z
- **Archived:** False
- **Topics:** b-roll, ffmpeg, internet-archive, mcp, model-context-protocol, public-domain, python, stock-footage

## README

```
<div align="center">

# 🎞️ broll-mcp

**Find, preview and cut public-domain b-roll from the Internet Archive, from inside any MCP agent.**

Every clip lands on disk with a provenance record next to it: source, license, exact segment and attribution text.

[![CI](https://github.com/edigeh/broll-mcp/actions/workflows/ci.yml/badge.svg)](https://github.com/edigeh/broll-mcp/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
![Python 3.12+](https://img.shields.io/badge/python-3.12%2B-3776AB?logo=python&logoColor=white)
![MCP](https://img.shields.io/badge/MCP-stdio-8A2BE2)
![No API key](https://img.shields.io/badge/API%20key-not%20needed-success)

</div>

---

```text
You:    "I need 8 seconds of children eating bananas, ideally black and white."

Agent → search_footage("children eating bananas")       5 ranked items, license + tier on each
Agent → inspect_item("AboutBan1935")                    scenes with a keyframe image per scene
Agent → extract_clip("AboutBan1935", start_s=300, 8s)   verified H.264 MP4 + provenance JSON
Agent → get_provenance(clip_id)                         "About Bananas" (1935), Prelinger Archives
                                                        via Internet Archive · Public Domain
```

The agent sees the footage before it cuts: `inspect_item` returns real keyframe images, so a vision model can pick the shot instead of guessing from titles.

## Why

archive.org holds decades of public-domain film (Prelinger, NASA, newsreels, stock footage), but using it from an agent is awkward:

- Raw item metadata can exceed **1 MB** for one item. That's a huge token cost before you've seen a single frame.
- Licenses differ **per item**, even inside one collection.
- Clips cut with `-c copy` snap to keyframes. An 8 s request came back as **11.35 s**.

broll-mcp handles all of this behind four small tools. Responses are compact and cached, and the server applies polite rate limits. Each cut is re-encoded and checked against the requested length.

## Features

| | |
|---|---|
| 🔎 **Brief-driven search** | Plain-language briefs become archive.org queries. A two-stage search first requires every term, then matches any term. Local ranking adds collection trust tiers and a black-and-white preference. Results come back 5 per page. |
| 🖼️ **Visual inspection** | Scene detection runs over HTTPS (PySceneDetect, with ffmpeg as fallback) and returns one keyframe per scene as an MCP image. Items that hold hundreds of clips return a paged clip list with thumbnails. |
| ✂️ **Frame-accurate cuts** | Cuts are re-encoded to H.264 MP4 with faststart. ffprobe confirms the duration is within ±0.1 s and the decode shows zero errors, and you get 3 check frames back. The same request twice returns the same clip. |
| 📜 **Provenance by design** | A JSON record sits next to every clip. It holds the source file MD5, the license copied verbatim, `commercial_use`, the data node, a timestamp and ready-to-paste attribution. Provenance is written before the clip, so a clip never exists without its record. |
| 🤝 **Polite to archive.org** | Follows [archive.org's bot guidance](https://archive.org/developers/bots.html): a descriptive User-Agent, a 1 req/s token bucket, at most 4 media connections, `Retry-After` support and exponential backoff. |
| 💾 **Cache-first** | Uses a SQLite WAL cache. Repeating a search, inspection or cut sends **0** upstream requests, and every tool result reports `upstream_requests`. |

## Quick start

**Requirements:** Python 3.12+, [uv](https://docs.astral.sh/uv/), and `ffmpeg` / `ffprobe` built with HTTPS support (`brew install ffmpeg`, `apt install ffmpeg`).

### Claude Code

```bash
claude mcp add broll -- uvx --from git+https://github.com/edigeh/broll-mcp broll-mcp
```

### Claude Desktop, Cursor and other MCP hosts

```json
{
  "mcpServers": {
    "broll": {
      "command": "uvx",
      "args": ["--from", "git+https://github.com/edigeh/broll-mcp", "broll-mcp"],
      "env": { "BROLL_AGENT_MODEL": "claude-opus-5-5" }
    }
  }
}
```

### From a clone

```bash
git clone https://github.com/edigeh/broll-mcp && cd broll-mcp
uv sync
uv run broll-mcp          # stdio server; point your host at this command
```

Try it without an agent using the [MCP Inspector](https://github.com/modelcontextprotocol/inspector):

```bash
npx @modelcontextprotocol/inspector --cli uv run broll-mcp \
  --method tools/call --tool-name search_footage --tool-arg 'brief=aerial highway 1950s'
```

## Tools

| Tool | What it does | Upstream cost |
|---|---|---|
| `search_footage(brief, filters?, cursor?)` | Ranked items with title, year, trust tier, license, runtime, tile image URL and a short *why it matched*. Filters: `max_tier`, `license` (`stated` · `commercial_ok` · `any`), `collections`, `year_from/to`, `color`. | ≤ 4 requests at the default `max_tier` (2 at tier 1, 6 at tier 3), 0 when cached |
| `inspect_item(identifier, file?, window?, page_size?, cursor?)` | **Scene mode:** scenes with timings plus a keyframe image each, a shotlist excerpt and license. **Clip-list mode:** for items with many separate clips, a paged list with thumbnails. | a few requests, 0 when cached |
| `extract_clip(identifier, file, start_s, duration_s, label?, audio?, brief?)` | Cuts 1–60 s into `BROLL_HOME/clips/`, verifies it and writes the provenance JSON. Returns 3 check frames. | ~3 requests, 0 on repeat |
| `get_provenance(clip_id \| identifier, cursor?)` | Reads the record for a clip, or for an item and the clips already cut from it. Local only. | always 0 |

**Resource:** `broll://stats` reports request counts by endpoint, cache contents, the active scene detector and the ffmpeg version.

Every tool declares an `outputSchema` and returns `structuredContent`. Errors come back as tool results with a stable code, such as `NO_RESULTS`, `ITEM_NOT_FOUND`, `FILE_NOT_IN_ITEM`, `RANGE_OUT_OF_BOUNDS`, `THROTTLED` or `CUT_VERIFY_FAILED`. Each message tells the agent how to recover.

## Provenance record

```jsonc
// clips/AboutBan1935__0300.00-0308.00__bananas__4ee00c.provenance.json
{
  "schema": "broll-mcp/provenance/1",
  "clip_id": "clp_20261023_4ee00c",
  "clip_sha256": "78e33da7…",
  "source": {
    "identifier": "AboutBan1935",
    "title": "About Bananas",
    "file": "AboutBan1935.mp4",
    "file_md5": "de484b1b573d28be4860f00a593f4416",
    "derived_from": "AboutBan1935.mpeg",
    "collections": ["prelinger"],
    "tier": 1,
    "date": "1935",
    "creator": "Castle Films"
  },
  "license": {
    "licenseurl": "http://creativecommons.org/licenses/publicdomain/",
    "license_label": "Public Domain (CC PD dedication)",
    "commercial_use": "allowed",
    "attribution_required": "no",
    "determined_from": "licenseurl"
  },
  "segment": { "start_s": 300.0, "duration_s": 8.0, "verified_duration_s": 7.975 },
  "retrieval": { "retrieved_at": "2026-10-23T12:28:30Z", "data_node": "dn600301.us.archive.org", "user_agent": "broll-mcp/0.1.0 (…)" },
  "processing": { "ffmpeg": "8.1.1", "audio": false, "reencoded": true },
  "attribution_text": "\"About Bananas\" (1935), Prelinger Archives via Internet Archive, https://archive.org/details/AboutBan1935. License: Public Domain (CC PD dedication)."
}
```

License labels come only from an exact URL table covering PD, CC0, the PD Mark, and CC BY / BY-SA / BY-ND / BY-NC variants for versions 2.0–4.0. A URL outside the table is copied verbatim with the label `unrecognized` and `commercial_use: "unknown"`; nothing is guessed.

## Configuration

| Variable | Default | Purpose |
|---|---|---|
| `BROLL_HOME` | platform data dir (`~/Library/Application Support/broll-mcp`, `~/.local/share/broll-mcp`) | Cache, keyframes, clips. The server writes nowhere else. |
| `BROLL_AGENT_MODEL` | `unspecified` | Model name sent in the User-Agent, as archive.org asks of AI agents. |
| `BROLL_CACHE_MAX_MB` | `2048` | Keyframe cache budget; older keyframes are evicted first. |

Collection trust tiers live in `BROLL_HOME/tiers.toml`. Tier 1 is `prelinger` and `nasa`, tier 2 is `stock_footage` and `feature_films`, and everything else is tier 3. Edit the file to tune ranking and the `max_tier` filter.

## How it works

```text
 MCP host ──stdio──► server.py ── jsonschema validation, result shaping, images
                        │
        ┌───────────────┼──────────────────┬────────────────────┐
   SearchService   SceneService        ClipService          SQLite cache (WAL)
   two-stage query  PySceneDetect /     cut → ffprobe verify  searches · items · scenes
   + local ranking  ffmpeg, keyframes   → provenance → clip   clips · provenance · request_log
        └───────────────┴─────── ArchiveClient ────────────────┘
                     UA · 1 req/s bucket · 4 media slots · backoff
                                    │
                     archive.org (advancedsearch · metadata · download)
```

The design rests on live archive.org responses captured in [`research/`](research/), which the offline tests also use as fixtures.

## Development

```bash
uv sync
uv run pytest                 # 155 offline tests (mock transport, captured metadata)
uv run ruff check . && uv run ruff format --check .
uv run python scripts/e2e_smoke.py /tmp/broll-e2e   # live end-to-end over stdio against archive.org
```

See [CONTRIBUTING.md](CONTRIBUTING.md).

## A note on rights

broll-mcp copies license information exactly as archive.org publishes it, and it makes no legal judgement. `commercial_use: "unknown"` means archive.org states no machine-readable license for that item. Check the source item before you publish footage, especially material that carries only a rights statement.

## License

[MIT](LICENSE) for the code. Footage belongs to its respective rights holders under the terms recorded in each clip's provenance file.
```

## Licence file

```
MIT License

Copyright (c) 2026 edigeh

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

> The repo's licence rule allows MIT, Apache-2.0, GPL-3.0 (as a separate component) and OpenRAIL. Non-commercial, research-only and revenue-capped licences are refused. Read the text, not the SPDX tag alone.

## File tree (78 files, filtered)

- `CONTRIBUTING.md`
- `LICENSE`
- `README.md`
- `research/analyze_meta.py`
- `scripts/e2e_smoke.py`
- `src/broll_mcp/__init__.py`
- `src/broll_mcp/archive.py`
- `src/broll_mcp/cache.py`
- `src/broll_mcp/catalog.py`
- `src/broll_mcp/clips.py`
- `src/broll_mcp/config.py`
- `src/broll_mcp/detect_worker.py`
- `src/broll_mcp/errors.py`
- `src/broll_mcp/items.py`
- `src/broll_mcp/media.py`
- `src/broll_mcp/scenes.py`
- `src/broll_mcp/schemas.py`
- `src/broll_mcp/search.py`
- `src/broll_mcp/server.py`
- `tests/__init__.py`
- `tests/conftest.py`
- `tests/test_archive.py`
- `tests/test_briefs.py`
- `tests/test_catalog.py`
- `tests/test_clips.py`
- `tests/test_scenes.py`
- `tests/test_search.py`
- `tests/test_server.py`
