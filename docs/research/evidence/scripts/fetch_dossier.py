#!/usr/bin/env python3
"""Build a full dossier for the nineteen AI-video-editing reference videos.

For every video this collects, in one pass:

* the full caption track (youtube-transcript-api, falling back to yt-dlp's subtitle
  extractor) — cached as raw caption snippets so the Markdown can be rebuilt offline;
* the video metadata and, most importantly, the **description** — the description is
  where these creators put their prompts, repos, skills, MCPs and preset links;
* the **links** found in that description, each with the line it came from, because
  "the thing in the description" is usually the thing worth stealing.

Usage
-----
    .venv\\Scripts\\python.exe docs/research/evidence/scripts/fetch_dossier.py
    .venv\\Scripts\\python.exe docs/research/evidence/scripts/fetch_dossier.py --refetch
    .venv\\Scripts\\python.exe docs/research/evidence/scripts/fetch_dossier.py --only 3 7
    .venv\\Scripts\\python.exe docs/research/evidence/scripts/fetch_dossier.py --only-meta

Outputs
-------
    docs/research/evidence/video-dossier-2026-10-21.md          metadata, descriptions, extracted links
    docs/research/evidence/transcripts-wave2-2026-10-21.md      captions for the ten videos whose
                                               captions were never captured before

Provenance notes
----------------
- Captions are ASR output. Brand names are mangled (Opus -> "Appus", HyperFrames ->
  "Hyperframes"). Never quote a spelling from the transcripts; quote the descriptions
  and the links instead.
- YouTube rate-limits hard. A burst of requests returns IpBlocked / "Sign in to
  confirm you're not a bot" for *every* item in the batch. The script therefore:
    * sleeps between requests,
    * retries with exponential backoff,
    * keeps the cache, so a failed pass never loses earlier work,
    * refuses to write a partial Markdown file unless --allow-partial is passed.
- See README.md in this folder for the long version of the rate-limit story.
"""

from __future__ import annotations

import argparse
import datetime as _dt
import json
import os
import re
import subprocess
import sys
import time
import unicodedata
from pathlib import Path

HERE = Path(__file__).resolve().parent
DOCS = HERE.parent
OUT_DOSSIER_DEFAULT = DOCS / "video-dossier-2026-10-21.md"
OUT_TRANSCRIPT_DEFAULT = DOCS / "transcripts-wave2-2026-10-21.md"
CACHE_DEFAULT = HERE / ".cache" / "dossier.json"

# key, title, channel, published, video id, runtime
VIDEOS: list[tuple[str, str, str, str, str, str]] = [
    # --- batch A: the motion-graphics set (captions already in docs/research/evidence/transcript.md) ---
    (
        "01_tao_opus55_motion_forever",
        "Claude Opus 5.5 Changed Motion Graphics Forever (Insane Results)",
        "Tao Prompts",
        "10 Oct 2026",
        "5XSwP0DaHF4",
        "20:42",
    ),
    (
        "02_tao_gpt6_astra_edit_guide",
        "The NEW Way to Edit Videos Using GPT-6 Astra | Full Guide",
        "Tao Prompts",
        "17 Sep 2026",
        "QwzVmGov9ec",
        "14:41",
    ),
    (
        "03_science_based_hyperframes_reels",
        "How I create viral animated reels using HyperFrames + Claude Code",
        "Science Based AI",
        "Sep 2026",
        "y4YupFxhAog",
        "14:29",
    ),
    (
        "04_marcin_astra_vs_fable_motion",
        "Is Astra 6 Actually Better Than Fable 5.1? (Motion Graphics Test)",
        "Marcin AI",
        "5 Sep 2026",
        "CgYGY0Mn1PE",
        "9:12",
    ),
    (
        "05_chase_astra_insane_motion_design",
        "GPT 6 Astra Is INSANE for Motion Design",
        "Chase AI",
        "8 Sep 2026",
        "eyDKoGU_vHk",
        "8:41",
    ),
    (
        "06_gpt6_astra_ae_minutes",
        "GPT-6 Astra + After Effects Creates Motion Graphics in Minutes!",
        "Chase AI / Higgsfield",
        "Sep 2026",
        "W3-RIZ-Ps64",
        "—",
    ),
    (
        "07_robonuggets_animate_skill",
        "GPT6 Astra one-shots motion graphics like never before (NEW Skill)",
        "Jay E | RoboNuggets",
        "15 Sep 2026",
        "Ce1Ym1HGyBA",
        "14:29",
    ),
    (
        "08_opus55_claude_motion_workflow",
        "How I make insane motion graphics with Claude Opus 5.5 (my exact workflow)",
        "—",
        "—",
        "H6lBZ7AfDDE",
        "—",
    ),
    (
        "09_mattloui_opus55_remotion_capcut",
        "Claude Opus 5.5 Animated Every Motion Graphic in This Video (I Didn't Keyframe a Thing)",
        "Matt Loui",
        "15 Oct 2026",
        "Ll45R_Z8tQc",
        "15:23",
    ),
    (
        "10_chase_astra_unlocked_ae",
        "GPT-6 Astra Just Unlocked Motion Design + After Effects",
        "Chase AI",
        "14 Sep 2026",
        "C8dWdic-oK4",
        "8:36",
    ),
    # --- batch B: the AI-editing set (captions never captured: YouTube was blocked) ---
    (
        "11_higgsfield_astra_resolve_post",
        "Did AI Change Editing Forever? (GPT-6 Astra + DaVinci Resolve)",
        "Higgsfield AI",
        "2 Oct 2026",
        "kOoC3yhUyDQ",
        "11:36",
    ),
    (
        "12_rourke_astra_after_effects_tutorial",
        "How To Make Motion Graphics With AI (After Effects Full Tutorial)",
        "Rourke",
        "28 Sep 2026",
        "mcpAbTHFIRE",
        "15:53",
    ),
    (
        "13_joris_resolve_mcp_vs_invideo",
        "Yes, AI Edits Your Video Now BUT... | DaVinci Resolve MCP VS Cloud Based invideo Editor Workflows",
        "Joris Hermans",
        "17 Oct 2026",
        "4nQGiRCUlDg",
        "15:32",
    ),
    (
        "14_jason_claude_code_editing",
        "I Finally Solved AI Video Editing (Full Guide + FREE Setup)",
        "Jason Cooperson",
        "8 Oct 2026",
        "sGt1ACELN4k",
        "21:19",
    ),
    (
        "15_zane_codex_openart_entire_video",
        "AI Edited My ENTIRE Video in DaVinci Resolve (One Prompt)",
        "Zane Hoyer",
        "8 Oct 2026",
        "P_NL_hg-y_4",
        "6:06",
    ),
    (
        "16_zane_sound_design_forever",
        "AI Just Changed Sound Design Forever",
        "Zane Hoyer",
        "28 Sep 2026",
        "8D9aWgGAkew",
        "6:34",
    ),
    (
        "17_greg_resolve_ai_update",
        "This NEW AI Update just changed DaVinci Resolve forever",
        "Greg Edits Video",
        "10 Sep 2026",
        "r3p2QkIr0PA",
        "17:33",
    ),
    (
        "18_convex_tandem_editor",
        "This is how I make videos now",
        "Convex (Mike Cann)",
        "20 Oct 2026",
        "3LIQ5GFmED8",
        "30:38",
    ),
    (
        "19_christian_claude_code_hyperframes",
        "I Fully Automated My Video Editing Using Claude Code (Full Walkthrough)",
        "Christian Peverelli",
        "4 Oct 2026",
        "HzXD4GVqXwM",
        "16:58",
    ),
]

# videos whose captions live in docs/research/evidence/transcript.md already (batch A minus the new #08)
WAVE2_KEYS = {
    "08_opus55_claude_motion_workflow",
    "11_higgsfield_astra_resolve_post",
    "12_rourke_astra_after_effects_tutorial",
    "13_joris_resolve_mcp_vs_invideo",
    "14_jason_claude_code_editing",
    "15_zane_codex_openart_entire_video",
    "16_zane_sound_design_forever",
    "17_greg_resolve_ai_update",
    "18_convex_tandem_editor",
    "19_christian_claude_code_hyperframes",
}

URL_RE = re.compile(r"https?://[^\s<>\"'\)\]]+", re.IGNORECASE)
BARE_DOMAIN_RE = re.compile(
    r"(?<![\w/@.])((?:www\.)?[a-z0-9][a-z0-9-]{1,}\.(?:com|io|ai|dev|org|net|co|app|xyz|sh|me|gg|studio|site|link|tools|page|link?)[^\s<>\"'\)\]]*)",
    re.IGNORECASE,
)


# --------------------------------------------------------------------------- utils


def clean(text: str) -> str:
    text = unicodedata.normalize("NFKC", text).replace("\u200b", "")
    text = re.sub(r"\s+", " ", text).strip()
    text = re.sub(r"^>>\s*", "", text)
    return re.sub(r"\s*>>\s*$", "", text).strip()


def stamp(seconds: float) -> str:
    seconds = int(seconds)
    minutes, secs = divmod(seconds, 60)
    hours, minutes = divmod(minutes, 60)
    return f"{hours}:{minutes:02d}:{secs:02d}" if hours else f"{minutes}:{secs:02d}"


def paragraphs(snips: list[dict], soft: int = 190, hard: int = 420, gap: float = 2.2) -> list[tuple[float, str]]:
    out: list[tuple[float, str]] = []
    buf: list[str] = []
    start: float | None = None
    last: float | None = None

    def flush() -> None:
        nonlocal buf, start
        if buf:
            out.append((start, " ".join(buf)))
        buf = []
        start = None

    for snip in snips:
        text = clean(snip["text"])
        if not text:
            continue
        if start is None:
            start = snip["start"]
        if last is not None and snip["start"] - last > gap and buf:
            flush()
            start = snip["start"]
        buf.append(text)
        last = snip["start"]
        size = sum(len(c) + 1 for c in buf)
        ends_sentence = bool(re.search(r"[.!?…][\"')\]]?$", buf[-1]))
        if (ends_sentence and size >= soft) or size >= hard:
            flush()
    flush()
    return out


def today_label() -> str:
    """'21 October 2026' on every platform (strftime('%-d') is glibc-only)."""
    today = _dt.date.today()
    return f"{today.day} {today.strftime('%B %Y')}"


def fmt_duration(seconds: int | None) -> str:
    if not seconds:
        return "—"
    minutes, secs = divmod(int(seconds), 60)
    hours, minutes = divmod(minutes, 60)
    return f"{hours}:{minutes:02d}:{secs:02d}" if hours else f"{minutes}:{secs:02d}"


def extract_links(description: str) -> list[dict]:
    """Every URL in the description, with the source line for context."""
    links: list[dict] = []
    seen: set[str] = set()
    for raw_line in description.splitlines():
        line = raw_line.strip()
        if not line:
            continue
        for match in URL_RE.finditer(line):
            url = match.group(0).rstrip(".,;:!?")
            url = re.sub(r"\\$", "", url)
            if url in seen:
                continue
            seen.add(url)
            links.append({"url": url, "context": line[:300]})
    return links


# ----------------------------------------------------------------------- fetching


def fetch_transcript_yta(video_id: str) -> dict:
    from youtube_transcript_api import YouTubeTranscriptApi

    api = YouTubeTranscriptApi()
    try:
        t = api.fetch(video_id)
        return {
            "lang": getattr(t, "language_code", "?"),
            "source": "youtube-transcript-api",
            "snips": [{"text": s.text, "start": s.start, "dur": s.duration} for s in t.snippets],
        }
    except AttributeError:  # older release, static method only
        raw = YouTubeTranscriptApi.get_transcript(video_id)
        return {
            "lang": "?",
            "source": "youtube-transcript-api",
            "snips": [{"text": s["text"], "start": s["start"], "dur": s.get("duration", 0)} for s in raw],
        }


def fetch_transcript_ytdlp(video_id: str, workdir: Path) -> dict:
    """Fallback: let yt-dlp pull the auto-generated subtitle track as json3.

    The default `web` client stops reporting captions once YouTube rate-limits the
    IP ("Sorry... your computer or network may be sending automated queries"), but the
    `android` player client keeps listing the track. The timedtext download itself can
    still 429, so this path is a fallback, not a guarantee.
    """
    workdir.mkdir(parents=True, exist_ok=True)
    cmd = [
        sys.executable, "-m", "yt_dlp",
        "--skip-download", "--no-warnings", "--quiet",
        "--extractor-args", "youtube:player_client=android,ios",
        "--write-subs", "--write-auto-subs",
        "--sub-langs", "en-orig,en.*,en",
        "--sub-format", "json3",
        "-o", str(workdir / f"{video_id}.%(ext)s"),
        f"https://www.youtube.com/watch?v={video_id}",
    ]
    proc = subprocess.run(cmd, capture_output=True, text=True, timeout=240)
    files = sorted(workdir.glob(f"{video_id}*.json3"))
    if not files:
        raise RuntimeError((proc.stderr or proc.stdout or "no subtitle file")[-300:])
    data = json.loads(files[0].read_text(encoding="utf-8"))
    snips = []
    for event in data.get("events", []):
        text = "".join(seg.get("utf8", "") for seg in event.get("segs", []))
        text = clean(text)
        if not text:
            continue
        snips.append({"text": text, "start": event.get("tStartMs", 0) / 1000.0, "dur": (event.get("dDurationMs", 0) / 1000.0)})
    for path in files:
        path.unlink(missing_ok=True)
    if not snips:
        raise RuntimeError("subtitle track empty")
    return {"lang": "en", "source": "yt-dlp", "snips": snips}


def fetch_meta(video_id: str) -> dict:
    import yt_dlp

    opts = {
        "quiet": True,
        "no_warnings": True,
        "skip_download": True,
        "noplaylist": True,
        "extract_flat": False,
    }
    with yt_dlp.YoutubeDL(opts) as ydl:
        info = ydl.extract_info(f"https://www.youtube.com/watch?v={video_id}", download=False)
    return {
        "title": info.get("title"),
        "channel": info.get("channel") or info.get("uploader"),
        "uploader_id": info.get("uploader_id"),
        "upload_date": info.get("upload_date"),
        "timestamp": info.get("timestamp"),
        "duration": info.get("duration"),
        "view_count": info.get("view_count"),
        "like_count": info.get("like_count"),
        "description": info.get("description") or "",
        "chapters": info.get("chapters") or [],
        "tags": info.get("tags") or [],
        "categories": info.get("categories") or [],
        "webpage_url": info.get("webpage_url"),
        "availability": info.get("availability"),
    }


def try_fetch(fn, *args, attempts: int = 4, base_wait: float = 20.0, **kwargs):
    last = None
    for attempt in range(1, attempts + 1):
        try:
            return fn(*args, **kwargs)
        except Exception as exc:  # noqa: BLE001
            last = exc
            msg = str(exc)
            blocked = "IpBlocked" in msg or "blocked" in msg.lower() or "not a bot" in msg.lower()
            if attempt == attempts:
                break
            wait = base_wait * (2 ** (attempt - 1))
            print(f"      retry {attempt}/{attempts - 1} after {int(wait)}s ({type(exc).__name__})", flush=True)
            time.sleep(wait)
    raise last  # type: ignore[misc]


# ------------------------------------------------------------------------ rendering


def render_dossier(data: dict, order: list[tuple], date: str) -> str:
    parts = [
        "# Video dossier — nineteen AI editing and motion-graphics reference videos\n",
        f"Collected {date}. Descriptions and metadata read live from YouTube with `yt-dlp`; "
        "captions pulled with `youtube-transcript-api`. Regenerate with "
        "[scripts/fetch_dossier.py](scripts/fetch_dossier.py).\n",
        "**Why this file exists.** The reviews in this folder were written from titles, "
        "descriptions and chapter lists, because YouTube blocked caption downloads on 21 Oct 2026. "
        "The descriptions are where these creators put the actual artefacts — prompts, repos, "
        "skills, MCP servers, preset packs, Google Docs, free-claim threads. Everything below is "
        "copied from the creator, not paraphrased.\n",
        "---\n",
    ]
    for index, (key, title, channel, published, video_id, runtime) in enumerate(order, start=1):
        entry = data.get(key)
        if not entry:
            continue
        meta = entry.get("meta") or {}
        url = f"https://www.youtube.com/watch?v={video_id}"
        duration = fmt_duration(meta.get("duration"))
        runtime_str = runtime if runtime != "—" and runtime != duration else duration
        parts.append(f"\n## {index}. {title}\n")
        parts.append(
            f"**{meta.get('channel') or channel}** · {published} · {runtime_str} · "
            f"{(meta.get('view_count') or 0):,} views · [{url}]({url})  \n"
            f"_key: `{key}` · video id: `{video_id}`_\n"
        )
        if entry.get("transcript"):
            t = entry["transcript"]
            paras = paragraphs(t["snips"])
            words = sum(len(text.split()) for _, text in paras)
            where = (
                "[transcripts-wave2-2026-10-21.md](transcripts-wave2-2026-10-21.md)"
                if key in WAVE2_KEYS
                else "[transcript.md](transcript.md)"
            )
            parts.append(
                f"**Captions:** {words:,} words · {len(paras)} paragraphs · "
                f"track `{t.get('lang')}` via {t.get('source')}. Full text: {where}\n"
            )
        else:
            parts.append("**Captions:** unavailable — see the failure note below.\n")

        chapters = meta.get("chapters") or []
        if chapters:
            parts.append("\n### Chapters\n")
            for ch in chapters:
                start = ch.get("start_time", 0)
                title_ch = ch.get("title", "")
                parts.append(f"- `{stamp(start)}` {title_ch}")
            parts.append("")

        desc = meta.get("description") or ""
        if desc.strip():
            parts.append("\n### Description (verbatim)\n")
            parts.append("```text")
            parts.append(desc.strip())
            parts.append("```")

        links = extract_links(desc)
        if links:
            parts.append(f"\n### Links in the description ({len(links)})\n")
            for link in links:
                parts.append(f"- <{link['url']}>")
                ctx = link["context"]
                if ctx and ctx != link["url"]:
                    parts.append(f"  - _{ctx}_")
            parts.append("")
        else:
            parts.append("\n### Links in the description\n\n_None._\n")

        tags = meta.get("tags") or []
        if tags:
            parts.append(f"\n**Tags:** {', '.join(tags)}\n")
        parts.append("\n---\n")
    return "\n".join(parts)


def render_transcripts(data: dict, order: list[tuple], date: str) -> str:
    parts = [
        "# Transcripts — wave 2 reference videos (captions never captured before)\n",
        f"Collected {date}. YouTube's own auto-generated caption tracks, fetched with "
        "`youtube-transcript-api` (falling back to `yt-dlp`'s subtitle extractor) — see "
        "[scripts/fetch_dossier.py](scripts/fetch_dossier.py). "
        "Each paragraph is a merged run of captions; the timestamp is where that paragraph starts.\n",
        "**On the text itself.** This is each creator's speech as YouTube's speech model heard it. "
        "Brand and product names are routinely mangled — Opus appears as *Appus*, HyperFrames as "
        "*Hyperframes*, DaVinci Resolve as *Davinchi*. The sense is reliably intact; the spellings "
        "are not, so quote the descriptions and links, not these spellings. Bracketed `[music]` "
        "markers are the caption track's own.\n",
        "Videos 1–10 of the original motion-graphics set keep their captions in "
        "[transcript.md](transcript.md); this file covers the ten videos that were missing them.\n",
        "---\n",
    ]
    for index, (key, title, channel, published, video_id, runtime) in enumerate(order, start=1):
        if key not in WAVE2_KEYS:
            continue
        entry = data.get(key)
        if not entry or not entry.get("transcript"):
            continue
        t = entry["transcript"]
        paras = paragraphs(t["snips"])
        words = sum(len(text.split()) for _, text in paras)
        url = f"https://www.youtube.com/watch?v={video_id}"
        parts.append(f"\n## {index}. {title}\n")
        parts.append(
            f"**{channel}** · {published} · {runtime} · [{url}]({url})  \n"
            f"_{words:,} words · {len(paras)} paragraphs · track: {t.get('lang')} "
            f"({t.get('source')})_\n"
        )
        parts.append("\n\n".join(f"**`[{stamp(ts)}]`** {text}" for ts, text in paras))
        parts.append("\n---\n")
    return "\n".join(parts)


# ----------------------------------------------------------------------------- main


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--dossier-out", type=Path, default=OUT_DOSSIER_DEFAULT)
    ap.add_argument("--transcript-out", type=Path, default=OUT_TRANSCRIPT_DEFAULT)
    ap.add_argument("--cache", type=Path, default=CACHE_DEFAULT)
    ap.add_argument("--refetch", action="store_true")
    ap.add_argument("--only", nargs="*", type=int, default=None, metavar="N")
    ap.add_argument("--only-meta", action="store_true", help="skip captions, metadata only")
    ap.add_argument("--allow-partial", action="store_true", help="write even if some videos failed")
    ap.add_argument("--sleep", type=float, default=3.0, help="seconds between requests")
    args = ap.parse_args()

    data: dict[str, dict] = {}
    if args.cache.exists() and not args.refetch:
        try:
            data = json.loads(args.cache.read_text(encoding="utf-8"))
            print(f"cache: {len(data)} entries from {args.cache}", flush=True)
        except Exception as exc:  # noqa: BLE001
            print(f"cache unreadable ({exc}); starting fresh", flush=True)
            data = {}

    order = VIDEOS
    if args.only:
        wanted = {i for i in args.only if 1 <= i <= len(VIDEOS)}
        order = [v for i, v in enumerate(VIDEOS, start=1) if i in wanted]
        if not order:
            sys.exit(f"--only: no video numbers in 1..{len(VIDEOS)}")

    failures: list[str] = []
    workdir = HERE / ".cache" / "subs"
    for key, title, _ch, _pub, video_id, _rt in order:
        entry = data.setdefault(key, {})
        if args.refetch:
            entry = {}
            data[key] = entry

        if not entry.get("meta"):
            try:
                entry["meta"] = try_fetch(fetch_meta, video_id, attempts=3, base_wait=15)
                print(f"META  {key:44s} ok", flush=True)
            except Exception as exc:  # noqa: BLE001
                entry["meta_error"] = f"{type(exc).__name__}: {str(exc)[:200]}"
                print(f"META  {key:44s} FAIL {type(exc).__name__}", flush=True)
                failures.append(key + " (meta)")
            time.sleep(args.sleep)

        if not args.only_meta and not entry.get("transcript"):
            ok = False
            for fn, label in ((fetch_transcript_yta, "yta"), (fetch_transcript_ytdlp, "ytdlp")):
                try:
                    kwargs = {"workdir": workdir} if label == "ytdlp" else {}
                    entry["transcript"] = try_fetch(fn, video_id, attempts=3, base_wait=25, **kwargs)
                    ok = True
                    print(f"CAPS  {key:44s} ok via {label} ({len(entry['transcript']['snips'])} snips)", flush=True)
                    break
                except Exception as exc:  # noqa: BLE001
                    entry.setdefault("transcript_errors", []).append(f"{label}: {type(exc).__name__}: {str(exc)[:160]}")
                    print(f"CAPS  {key:44s} fail via {label}: {type(exc).__name__}", flush=True)
            if not ok:
                failures.append(key + " (captions)")
            time.sleep(args.sleep)

        args.cache.parent.mkdir(parents=True, exist_ok=True)
        args.cache.write_text(json.dumps(data, ensure_ascii=False), encoding="utf-8")

    if failures:
        print(f"\n{len(failures)} failures: {', '.join(failures)}", flush=True)

    missing = [k for k, *_ in VIDEOS if k not in data or not (data[k].get("meta") or data[k].get("transcript"))]
    if missing and not args.allow_partial:
        print(f"refusing to write partial output; missing: {', '.join(missing)}")
        print("re-run (cache is kept) or pass --allow-partial.")
        return 1

    date = today_label()
    args.dossier_out.write_text(render_dossier(data, VIDEOS, date), encoding="utf-8")
    print(f"wrote {args.dossier_out}", flush=True)
    args.transcript_out.write_text(render_transcripts(data, VIDEOS, date), encoding="utf-8")
    print(f"wrote {args.transcript_out}", flush=True)
    return 0


if __name__ == "__main__":
    os.environ.setdefault("PYTHONIOENCODING", "utf-8")
    raise SystemExit(main())
