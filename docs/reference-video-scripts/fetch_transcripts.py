#!/usr/bin/env python3
"""Fetch and format the reference-video transcripts in docs/transcript.md.

The transcripts in docs/transcript.md are YouTube's own auto-generated caption tracks,
pulled from the public videos listed in VIDEOS below. This script re-fetches them and
rewrites that file, so the research stays reproducible and refreshable.

Usage
-----
    python3 -m pip install --user youtube-transcript-api
    python3 docs/reference-video-scripts/fetch_transcripts.py            # refresh all
    python3 docs/reference-video-scripts/fetch_transcripts.py --refetch  # ignore cache
    python3 docs/reference-video-scripts/fetch_transcripts.py --only 5 7

Raw caption data is cached under .cache/ so re-formatting does not re-hit YouTube. The
cache is disposable; delete it or pass --refetch to rebuild it.

Provenance notes
----------------
- Transcripts are ASR output, not human captions. Brand and product names are routinely
  mangled (Higgsfield -> "Higsfield"/"Hicksfield", Opus -> "Appus"). docs/transcript.md
  says so in its header; do not quote spellings from it.
- If YouTube rate-limits or a video's captions are removed, the script reports the failure
  and continues. It never writes a partial file silently -- check the summary it prints.
- YouTube DOES rate-limit hard. Fetching all eleven in one burst returns IpBlocked for every one of
  them; this happened on the first run of this script, immediately after the original collection.
  The cache (.cache/raw.json) exists partly to make that unrecoverable-from-scratch situation a
  non-event. See README.md.
- Video 7's channel is credited as "Chase AI / Higgsfield" because the install link in the
  description is a Higgsfield one; the channel was not confirmed. Runtime is unknown.
"""

from __future__ import annotations

import argparse
import datetime as _dt
import json
import re
import sys
import unicodedata
from pathlib import Path

try:
    from youtube_transcript_api import YouTubeTranscriptApi
except ImportError:  # pragma: no cover
    sys.exit(
        "youtube-transcript-api is not installed.\n"
        "  python3 -m pip install --user youtube-transcript-api"
    )

HERE = Path(__file__).resolve().parent
DOCS = HERE.parent
OUT_DEFAULT = DOCS / "transcript.md"
CACHE_DEFAULT = HERE / ".cache" / "raw.json"

# key, title, channel, published, video id, runtime
VIDEOS: list[tuple[str, str, str, str, str, str]] = [
    (
        "01_joseph_reels_opus55",
        "How to Edit VIRAL Instagram Reels Using Opus 5.5! (New Style in 2026)",
        "Joseph | Video Editing",
        "11 Oct 2026",
        "5UnK3c-nlIs",
        "17:16",
    ),
    (
        "02_tao_opus55_motion_forever",
        "Claude Opus 5.5 Changed Motion Graphics Forever (Insane Results)",
        "Tao Prompts",
        "10 Oct 2026",
        "5XSwP0DaHF4",
        "20:42",
    ),
    (
        "03_tao_gpt6_astra_edit_guide",
        "The NEW Way to Edit Videos Using GPT-6 Astra | Full Guide",
        "Tao Prompts",
        "17 Sep 2026",
        "QwzVmGov9ec",
        "14:41",
    ),
    (
        "04_science_based_hyperframes_reels",
        "How I create viral animated reels using HyperFrames + Claude Code",
        "Science Based AI",
        "Sep 2026",
        "y4YupFxhAog",
        "14:29",
    ),
    (
        "05_marcin_astra_vs_fable_motion",
        "Is Astra 6 Actually Better Than Fable 5.1? (Motion Graphics Test)",
        "Marcin AI",
        "5 Sep 2026",
        "CgYGY0Mn1PE",
        "9:12",
    ),
    (
        "06_chase_astra_insane_motion_design",
        "GPT 6 Astra Is INSANE for Motion Design",
        "Chase AI",
        "8 Sep 2026",
        "eyDKoGU_vHk",
        "8:41",
    ),
    (
        "07_gpt6_astra_ae_minutes",
        "GPT-6 Astra + After Effects Creates Motion Graphics in Minutes!",
        "Chase AI / Higgsfield",
        "Sep 2026",
        "W3-RIZ-Ps64",
        "—",
    ),
    (
        "08_robonuggets_animate_skill",
        "GPT6 Astra one-shots motion graphics like never before (NEW Skill)",
        "Jay E | RoboNuggets",
        "15 Sep 2026",
        "Ce1Ym1HGyBA",
        "14:29",
    ),
    (
        "09_kaye_opus55_claude_motion",
        "How To Make Insane Motion Graphics With Opus 5.5 (Claude Motion)",
        "kaye.creatives",
        "14 Oct 2026",
        "AsvN_feasyY",
        "12:24",
    ),
    (
        "10_mattloui_opus55_remotion_capcut",
        "Claude Opus 5.5 Animated Every Motion Graphic in This Video (I Didn't Keyframe a Thing)",
        "Matt Loui",
        "15 Oct 2026",
        "Ll45R_Z8tQc",
        "15:23",
    ),
    (
        "11_chase_astra_unlocked_ae",
        "GPT-6 Astra Just Unlocked Motion Design + After Effects",
        "Chase AI",
        "14 Sep 2026",
        "C8dWdic-oK4",
        "8:36",
    ),
]

HEADER = """# Transcripts — AI video editing and motion graphics reference set

Collected {date}. YouTube's own auto-generated caption tracks, fetched with \
`youtube-transcript-api` (see [reference-video-scripts/](reference-video-scripts/)). Each paragraph \
is a merged run of captions; the timestamp is where that paragraph starts.

**On the text itself.** This is each creator's speech as YouTube's speech model heard it. Brand \
and product names are routinely mangled — Higgsfield appears as *Higsfield*, *Hicksfield* and \
*Hixson*, HyperFrames as *Hyperframes*, Opus as *Appus*. The sense is reliably intact; the \
spellings are not, so do not quote them from here. Bracketed `[music]` markers are the \
caption track's own.

Companions: [reference-video-review-2026-10-21.md](reference-video-review-2026-10-21.md) \
(what they do and why it matters) · \
[agent-video-gaps-and-proposals-2026-10-21.md](agent-video-gaps-and-proposals-2026-10-21.md) \
(what it means for AeroEdits).
"""


def fetch_one(video_id: str) -> dict:
    """Return {lang, snips:[{text,start,dur}]}. Handles both library API shapes."""
    api = YouTubeTranscriptApi()
    try:
        t = api.fetch(video_id)
        return {
            "lang": getattr(t, "language_code", "?"),
            "snips": [{"text": s.text, "start": s.start, "dur": s.duration} for s in t.snippets],
        }
    except AttributeError:  # older release, static method only
        raw = YouTubeTranscriptApi.get_transcript(video_id)
        return {
            "lang": "?",
            "snips": [{"text": s["text"], "start": s["start"], "dur": s.get("duration", 0)} for s in raw],
        }


def clean(text: str) -> str:
    text = unicodedata.normalize("NFKC", text).replace("\u200b", "")
    text = re.sub(r"\s+", " ", text).strip()
    # caption roll-up markers
    text = re.sub(r"^>>\s*", "", text)
    return re.sub(r"\s*>>\s*$", "", text).strip()


def stamp(seconds: float) -> str:
    seconds = int(seconds)
    minutes, secs = divmod(seconds, 60)
    hours, minutes = divmod(minutes, 60)
    return f"{hours}:{minutes:02d}:{secs:02d}" if hours else f"{minutes}:{secs:02d}"


def paragraphs(snips: list[dict], soft: int = 190, hard: int = 420, gap: float = 2.2) -> list[tuple[float, str]]:
    """Merge caption snippets into readable paragraphs.

    A paragraph ends at a sentence boundary past `soft` characters, at `hard` characters,
    or across a silence longer than `gap` seconds.
    """
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


def render(data: dict[str, dict], order: list[tuple], date: str) -> str:
    parts = [HEADER.format(date=date), "\n---\n"]
    for index, (key, title, channel, published, video_id, runtime) in enumerate(order, start=1):
        entry = data[key]
        paras = paragraphs(entry["snips"])
        words = sum(len(text.split()) for _, text in paras)
        url = f"https://www.youtube.com/watch?v={video_id}"
        parts.append(f"\n## {index}. {title}\n")
        parts.append(
            f"**{channel}** · {published} · {runtime} · [{url}]({url})  \n"
            f"_{words:,} words · {len(paras)} paragraphs · track: {entry['lang']}_\n"
        )
        parts.append("\n\n".join(f"**`[{stamp(t)}]`** {text}" for t, text in paras))
        parts.append("\n---\n")
    return "\n".join(parts)


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--out", type=Path, default=OUT_DEFAULT, help="output markdown path")
    ap.add_argument("--cache", type=Path, default=CACHE_DEFAULT, help="raw caption cache")
    ap.add_argument("--refetch", action="store_true", help="ignore the cache and re-download")
    ap.add_argument("--only", nargs="*", type=int, default=None, metavar="N", help="only these 1-based video numbers")
    args = ap.parse_args()

    data: dict[str, dict] = {}
    if args.cache.exists() and not args.refetch:
        data = json.loads(args.cache.read_text())
        print(f"cache: {len(data)} transcripts from {args.cache}")

    targets = VIDEOS
    if args.only:
        wanted = {i for i in args.only if 1 <= i <= len(VIDEOS)}
        targets = [v for i, v in enumerate(VIDEOS, start=1) if i in wanted]
        if not targets:
            sys.exit(f"--only: no video numbers in 1..{len(VIDEOS)}")

    failures = []
    for key, _title, _ch, _pub, video_id, _rt in targets:
        if key in data and not args.refetch:
            print(f"skip  {key} (cached)")
            continue
        try:
            data[key] = fetch_one(video_id)
            print(f"OK    {key:42s} {video_id}  {len(data[key]['snips']):5d} captions")
        except Exception as exc:  # noqa: BLE001 - report and continue
            reason = str(exc).split("\n")[0][:120]
            print(f"FAIL  {key:42s} {video_id}  {type(exc).__name__}: {reason}")
            failures.append(key)

    if failures:
        print(f"\n{len(failures)} failed: {', '.join(failures)}")
        print("Refusing to write --transcript.md with missing videos; fix or pass --only.")

    missing = [k for k, *_ in VIDEOS if k not in data]
    if missing:
        print(f"cache is incomplete, missing: {', '.join(missing)}")
        return 1

    args.cache.parent.mkdir(parents=True, exist_ok=True)
    args.cache.write_text(json.dumps(data))
    date = _dt.date.today().strftime("%-d %B %Y")
    args.out.write_text(render(data, VIDEOS, date))
    print(f"wrote {args.out} ({len(VIDEOS)} transcripts)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())