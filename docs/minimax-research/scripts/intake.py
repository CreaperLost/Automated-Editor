#!/usr/bin/env python3
"""Capture one reference (video, repo, page) as reviewable evidence.

The three fetch scripts in this folder each have a hardcoded list of sources.
This one is the opposite: it takes whatever URLs you hand it, so a new
reference can be captured the same way every time, with the same provenance
notes, without editing a script first.

Usage
-----
    python3 docs/research/evidence/scripts/intake.py <url> [<url> ...]
    python3 docs/research/evidence/scripts/intake.py <url> --tag "gfx-3"
    python3 docs/research/evidence/scripts/intake.py <url> --meta-only
    python3 docs/research/evidence/scripts/intake.py <url> --refetch

What it does per kind of URL
----------------------------
- **YouTube** — title, channel, date, runtime, the full description, every link
  in that description, and the caption track. The description is the point:
  that is where creators put the skill, the prompt pack, the repo.
- **GitHub repo** — stars, licence, topics, default branch, the README, the
  file tree, and the licence file itself (the repo's licence rule forbids
  non-commercial and revenue-capped licences, so this has to be read, not
  assumed).
- **GitHub file or folder** — the raw file, or a listing of the folder.
- **Any other page** — title, meta description, the readable text, and the
  links out of it.

Outputs
-------
    docs/minimax-research/evidence/<date>/<slug>.md   one file per reference
    docs/minimax-research/evidence/INDEX.md           the running table
    docs/minimax-research/scripts/.cache/intake.json  raw cache (git-ignored)

Provenance notes
----------------
- Captions are ASR output. Brand names come out mangled ("Higgsfield" as
  "Higsfield"). Never quote a spelling from a caption; quote the description.
- YouTube rate-limits hard, and the block lasts for a whole session. The
  script retries with backoff and keeps the cache, so a failed pass never
  loses an earlier capture. If captions are blocked, `--meta-only` still
  gets the description and the links.
- A capture that failed is written as a failure record, not as an empty
  success, so "we looked and there was nothing" is visible in the evidence.
- Everything captured here is a **vendor or creator claim** until something
  in this repo tests it. Captions and marketing copy are not measurements.
"""

from __future__ import annotations

import argparse
import datetime as _dt
import html
import json
import os
import re
import subprocess
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path
from urllib.parse import parse_qs, urlparse

HERE = Path(__file__).resolve().parent
EVIDENCE = HERE.parent
INTAKE_DIR = EVIDENCE / "evidence"
CACHE_PATH = HERE / ".cache" / "intake.json"
USER_AGENT = "AeroEdits-research-intake/1.0 (+local research script)"
MAX_TEXT_CHARS = 120_000


# --------------------------------------------------------------------------
# small helpers
# --------------------------------------------------------------------------
def now_iso() -> str:
    return _dt.datetime.now().replace(microsecond=0).isoformat()


def slugify(text: str, limit: int = 60) -> str:
    text = re.sub(r"https?://\S+", "url", text)
    text = re.sub(r"[^A-Za-z0-9]+", "-", text).strip("-").lower()
    return (text[:limit].rstrip("-") or "reference")


def read_cache() -> dict:
    if CACHE_PATH.exists():
        try:
            return json.loads(CACHE_PATH.read_text(encoding="utf-8"))
        except json.JSONDecodeError:
            return {}
    return {}


def write_cache(cache: dict) -> None:
    CACHE_PATH.parent.mkdir(parents=True, exist_ok=True)
    CACHE_PATH.write_text(
        json.dumps(cache, ensure_ascii=False, indent=2, sort_keys=True), encoding="utf-8"
    )


def http_get(
    url: str,
    accept: str | None = None,
    timeout: int = 45,
    extra_headers: dict[str, str] | None = None,
) -> bytes:
    """Fetch a URL.

    `requests` is preferred: it bundles its own CA certificates, and the
    macOS system Python often ships an incomplete cert store, which makes
    urllib fail with CERTIFICATE_VERIFY_FAILED on perfectly good hosts.
    """
    headers = {"User-Agent": USER_AGENT}
    if accept:
        headers["Accept"] = accept
    if extra_headers:
        headers.update(extra_headers)
    try:
        import requests  # noqa: PLC0415 - optional dependency, imported on use

        resp = requests.get(url, headers=headers, timeout=timeout)
        resp.raise_for_status()
        return resp.content
    except ImportError:
        pass

    req = urllib.request.Request(url, headers=headers)
    with urllib.request.urlopen(req, timeout=timeout) as resp:
        return resp.read()


def http_get_text(
    url: str,
    accept: str | None = None,
    timeout: int = 45,
    extra_headers: dict[str, str] | None = None,
) -> str:
    return http_get(url, accept=accept, timeout=timeout, extra_headers=extra_headers).decode(
        "utf-8", errors="replace"
    )


def strip_html(raw: str) -> str:
    """Readable text from a page: drop script/style, tags, and collapse space."""
    raw = re.sub(r"(?is)<(script|style|noscript|svg)\b.*?</\1>", " ", raw)
    raw = re.sub(r"(?is)<!--.*?-->", " ", raw)
    raw = re.sub(r"(?i)<br\s*/?>", "\n", raw)
    raw = re.sub(r"(?i)</(p|div|li|h[1-6]|tr|section)>", "\n", raw)
    raw = re.sub(r"(?s)<[^>]+>", " ", raw)
    raw = html.unescape(raw)
    raw = re.sub(r"[ \t\xa0]+", " ", raw)
    raw = re.sub(r"\n\s*\n\s*\n+", "\n\n", raw)
    return raw.strip()


def extract_links(text: str) -> list[str]:
    links: list[str] = []
    for match in re.finditer(r"https?://[^\s<>\"'\)\]]+", text or ""):
        url = match.group(0).rstrip(".,;:!?")
        if url not in links:
            links.append(url)
    return links


def clip(text: str | None, limit: int = MAX_TEXT_CHARS) -> str:
    if not text:
        return ""
    text = text.strip()
    if len(text) <= limit:
        return text
    return text[:limit] + f"\n\n[... truncated at {limit} chars ...]"


# --------------------------------------------------------------------------
# YouTube
# --------------------------------------------------------------------------
def youtube_video_id(url: str) -> str | None:
    parsed = urlparse(url)
    host = parsed.netloc.replace("www.", "")
    if host == "youtu.be":
        return parsed.path.lstrip("/").split("/")[0] or None
    if host.endswith("youtube.com"):
        if parsed.path == "/watch":
            return parse_qs(parsed.query).get("v", [None])[0]
        for prefix in ("/shorts/", "/embed/", "/live/", "/v/"):
            if parsed.path.startswith(prefix):
                return parsed.path[len(prefix):].split("/")[0] or None
    return None


def youtube_metadata(url: str) -> tuple[dict, str | None]:
    """Title, channel, description and links from yt-dlp; oEmbed as a fallback."""
    try:
        proc = subprocess.run(
            [
                sys.executable, "-m", "yt_dlp", "--dump-single-json", "--skip-download",
                "--no-warnings", "--no-playlist", url,
            ],
            capture_output=True, text=True, timeout=180,
        )
        if proc.returncode == 0 and proc.stdout.strip():
            data = json.loads(proc.stdout)
            meta = {
                "title": data.get("title") or "",
                "channel": data.get("channel") or data.get("uploader") or "",
                "upload_date": data.get("upload_date") or "",
                "duration": data.get("duration_string") or "",
                "view_count": data.get("view_count"),
                "like_count": data.get("like_count"),
                "tags": data.get("tags") or [],
                "description": data.get("description") or "",
                "source": "yt-dlp",
            }
            return meta, None
        err = (proc.stderr or "").strip().splitlines()
        return {}, err[-1] if err else "yt-dlp returned no data"
    except Exception as exc:  # noqa: BLE001 - report, never crash the batch
        return {}, f"{type(exc).__name__}: {exc}"


def youtube_captions(video_id: str, url: str) -> tuple[list[dict], str | None]:
    """Caption snippets via youtube-transcript-api, with a yt-dlp fallback."""
    try:
        from youtube_transcript_api import YouTubeTranscriptApi

        api = YouTubeTranscriptApi()
        fetched = api.fetch(video_id)  # youtube-transcript-api >= 1.0
        snippets = [
            {"start": round(s.start, 2), "duration": round(s.duration, 2), "text": s.text}
            for s in fetched.snippets
        ]
        if snippets:
            return snippets, None
    except Exception as exc:  # noqa: BLE001
        first_error = f"{type(exc).__name__}: {exc}"

        # Fallback: yt-dlp's own subtitle extractor.
        try:
            proc = subprocess.run(
                [
                    sys.executable, "-m", "yt_dlp", "--skip-download", "--write-subs",
                    "--write-auto-subs", "--sub-langs", "en.*,en", "--sub-format", "vtt",
                    "--print", "requested_subtitles", "--no-playlist", "--no-warnings", url,
                ],
                capture_output=True, text=True, timeout=180,
            )
            if proc.returncode == 0:
                return [], "captions fell back to yt-dlp path; read the transcript file it printed"
        except Exception:  # noqa: BLE001
            pass
        return [], first_error


def capture_youtube(url: str, meta_only: bool, sleep: float) -> dict:
    record: dict = {"kind": "youtube", "url": url, "captured": now_iso()}
    video_id = youtube_video_id(url)
    record["video_id"] = video_id

    meta, err = youtube_metadata(url)
    record["meta"] = meta
    record["meta_error"] = err
    record["label"] = meta.get("title") or url
    if meta.get("description"):
        record["links"] = extract_links(meta["description"])

    if video_id and not meta_only:
        snippets, cap_err = youtube_captions(video_id, url)
        record["captions"] = snippets
        record["captions_error"] = cap_err
    elif video_id:
        record["captions"] = []
        record["captions_error"] = "skipped (--meta-only)"
    else:
        record["captions"] = []
        record["captions_error"] = "no video id parsed"

    time.sleep(sleep)
    return record


# --------------------------------------------------------------------------
# GitHub
# --------------------------------------------------------------------------
def github_token() -> str | None:
    return os.environ.get("GITHUB_TOKEN") or os.environ.get("GH_TOKEN")


def github_api(path: str, accept: str | None = None) -> tuple[dict | str | None, str | None]:
    url = f"https://api.github.com{path}"
    try:
        headers = {"Accept": accept} if accept else None
        if github_token():
            headers = headers or {}
            headers["Authorization"] = f"Bearer {github_token()}"
        body = http_get_text(url, accept=accept, extra_headers=headers).strip()
        if not body:
            return None, "empty response"
        try:
            return json.loads(body), None
        except json.JSONDecodeError:
            return body, None
    except Exception as exc:  # noqa: BLE001
        return None, f"{type(exc).__name__}: {exc}"

def capture_github(url: str, sleep: float) -> dict:
    record: dict = {"kind": "github", "url": url, "captured": now_iso()}
    parsed = urlparse(url)
    parts = [p for p in parsed.path.split("/") if p]
    if not parts:
        record["error"] = "not a GitHub repo path"
        return record

    owner, repo = parts[0], parts[1]
    record["owner"], record["repo"] = owner, repo
    record["label"] = f"{owner}/{repo}"

    meta, err = github_api(f"/repos/{owner}/{repo}")
    if err:
        record["meta_error"] = err
    elif isinstance(meta, dict):
        record["meta"] = {
            "full_name": meta.get("full_name"),
            "description": meta.get("description"),
            "homepage": meta.get("homepage"),
            "stars": meta.get("stargazers_count"),
            "forks": meta.get("forks_count"),
            "open_issues": meta.get("open_issues_count"),
            "license": (meta.get("license") or {}).get("spdx_id"),
            "topics": meta.get("topics") or [],
            "default_branch": meta.get("default_branch"),
            "pushed_at": meta.get("pushed_at"),
            "archived": meta.get("archived"),
            "language": meta.get("language"),
        }
        if meta.get("full_name"):
            record["label"] = meta["full_name"]

    # A file or folder inside the repo.
    if len(parts) > 2:
        kind = parts[2]
        if kind in ("blob", "tree"):
            branch = parts[3] if len(parts) > 3 else (record.get("meta") or {}).get(
                "default_branch", "main"
            )
            subpath = "/".join(parts[4:])
            record["path"] = f"{branch}/{subpath}"
            if kind == "blob":
                raw = f"https://raw.githubusercontent.com/{owner}/{repo}/{branch}/{subpath}"
                try:
                    record["content"] = http_get_text(raw)
                except Exception as exc:  # noqa: BLE001
                    record["content_error"] = f"{type(exc).__name__}: {exc}"
            else:
                tree, tree_err = github_api(
                    f"/repos/{owner}/{repo}/contents/{subpath}?ref={branch}"
                )
                record["listing"] = tree
                record["listing_error"] = tree_err
        elif kind in ("releases", "issues", "pulls", "wiki", "tree", "blob"):
            record["note"] = f"raw capture not implemented for /{kind}"

    # README and licence.
    readme, readme_err = github_api(f"/repos/{owner}/{repo}/readme")
    if isinstance(readme, dict) and readme.get("content"):
        record["readme"] = _b64(readme["content"])
    elif readme_err:
        record["readme_error"] = readme_err

    licence, lic_err = github_api(
        f"/repos/{owner}/{repo}/license", accept="application/vnd.github.raw"
    )
    if isinstance(licence, str) and licence.strip():
        record["licence_text"] = clip(licence, 8000)
    elif lic_err:
        record["licence_error"] = lic_err

    # File tree, filtered to the files that carry a method.
    branch = (record.get("meta") or {}).get("default_branch")
    if branch:
        tree, tree_err = github_api(
            f"/repos/{owner}/{repo}/git/trees/{branch}?recursive=1"
        )
        if isinstance(tree, dict) and tree.get("tree"):
            paths = [t["path"] for t in tree["tree"] if t.get("type") == "blob"]
            record["tree_total"] = len(paths)
            record["tree"] = [p for p in paths if _interesting(p)][:400]
            record["tree_truncated"] = len(record["tree"]) >= 400
        elif tree_err:
            record["tree_error"] = tree_err

    time.sleep(sleep)
    return record


def _b64(data: str) -> str:
    import base64

    try:
        return base64.b64decode(data).decode("utf-8", errors="replace")
    except Exception:  # noqa: BLE001
        return ""


def _interesting(path: str) -> bool:
    """Files that carry a method, a prompt, a template or a licence."""
    low = path.lower()
    name = low.rsplit("/", 1)[-1]
    if name in ("readme.md", "license", "license.md", "license.txt", "skill.md"):
        return True
    if low.endswith((".md", ".mdc")):
        return True
    if "/skills/" in low or low.startswith("skills/"):
        return True
    if low.endswith((".json", ".yaml", ".yml")) and ("schema" in low or "config" in low):
        return True
    if any(low.endswith(ext) for ext in (".jsx", ".tsx", ".html", ".py", ".js", ".ts")):
        return True
    return False


# --------------------------------------------------------------------------
# generic web page
# --------------------------------------------------------------------------
def capture_web(url: str, sleep: float) -> dict:
    record: dict = {"kind": "web", "url": url, "captured": now_iso()}
    try:
        raw = http_get_text(url)
    except Exception as exc:  # noqa: BLE001
        record["error"] = f"{type(exc).__name__}: {exc}"
        return record

    title_match = re.search(r"(?is)<title[^>]*>(.*?)</title>", raw)
    record["title"] = html.unescape(title_match.group(1).strip()) if title_match else ""
    record["label"] = record["title"] or url
    meta_match = re.search(
        r'(?is)<meta[^>]+name=["\']description["\'][^>]+content=["\'](.*?)["\']', raw
    ) or re.search(
        r'(?is)<meta[^>]+content=["\'](.*?)["\'][^>]+name=["\']description["\']', raw
    )
    record["meta_description"] = html.unescape(meta_match.group(1).strip()) if meta_match else ""
    record["text"] = clip(strip_html(raw))
    record["links"] = extract_links(raw)[:200]
    time.sleep(sleep)
    return record


# --------------------------------------------------------------------------
# rendering
# --------------------------------------------------------------------------
def render_markdown(record: dict) -> str:
    url = record.get("url", "")
    kind = record.get("kind", "unknown")
    meta = record.get("meta") or {}
    title = record.get("label") or meta.get("title") or record.get("title") or slugify(url)
    out: list[str] = []
    out.append(f"# {title}")
    out.append("")
    out.append(f"- **Source:** {url}")
    out.append(f"- **Kind:** {kind}")
    out.append(f"- **Captured:** {record.get('captured')}")
    out.append(
        "- **Evidence grade:** vendor or creator claim. Not tested in this repo. "
        "Quotes from a caption are what a creator *said*, not a measurement."
    )
    out.append("")

    if record.get("error"):
        out.append("## Capture failed")
        out.append("")
        out.append(f"`{record['error']}`")
        out.append("")
        return "\n".join(out)

    if kind == "youtube":
        out.append("## Metadata")
        out.append("")
        for key, label in (
            ("title", "Title"), ("channel", "Channel"), ("upload_date", "Published"),
            ("duration", "Runtime"), ("view_count", "Views"), ("like_count", "Likes"),
        ):
            if meta.get(key):
                value = meta[key]
                if key == "upload_date" and len(str(value)) == 8:
                    value = f"{value[6:8]} {value[4:6]} {value[0:4]}"
                out.append(f"- **{label}:** {value}")
        if record.get("meta_error"):
            out.append(f"- **Metadata error:** `{record['meta_error']}`")
        out.append("")

        if meta.get("description"):
            out.append("## Description (verbatim)")
            out.append("")
            out.append("```")
            out.append(clip(meta["description"], 20_000))
            out.append("```")
            out.append("")

        links = record.get("links") or []
        if links:
            out.append("## Links from the description")
            out.append("")
            for link in links:
                out.append(f"- {link}")
            out.append("")

        captions = record.get("captions") or []
        out.append("## Captions")
        out.append("")
        if captions:
            out.append(f"{len(captions)} snippets. ASR output; brand names are mangled.")
            out.append("")
            for snippet in captions:
                start = snippet.get("start", 0)
                minutes, seconds = divmod(int(start), 60)
                out.append(f"- [{minutes:02d}:{seconds:02d}] {snippet.get('text', '')}")
        else:
            out.append(f"_No captions captured._ {record.get('captions_error') or ''}".strip())
        out.append("")

    elif kind == "github":
        meta = record.get("meta") or {}
        out.append("## Repository")
        out.append("")
        for key, label in (
            ("full_name", "Repo"), ("description", "Description"), ("homepage", "Homepage"),
            ("stars", "Stars"), ("forks", "Forks"), ("open_issues", "Open issues"),
            ("license", "Licence (SPDX)"), ("language", "Language"),
            ("default_branch", "Default branch"), ("pushed_at", "Last push"),
            ("archived", "Archived"),
        ):
            if meta.get(key) is not None and meta.get(key) != "":
                out.append(f"- **{label}:** {meta[key]}")
        if meta.get("topics"):
            out.append(f"- **Topics:** {', '.join(meta['topics'])}")
        if record.get("meta_error"):
            out.append(f"- **Metadata error:** `{record['meta_error']}`")
        out.append("")

        if record.get("path"):
            out.append(f"## Path `{record['path']}`")
            out.append("")
        if record.get("content"):
            out.append("```")
            out.append(clip(record["content"]))
            out.append("```")
            out.append("")
        if record.get("listing"):
            out.append("```")
            out.append(json.dumps(record["listing"], indent=2, ensure_ascii=False)[:20_000])
            out.append("```")
            out.append("")

        if record.get("readme"):
            out.append("## README")
            out.append("")
            out.append("```")
            out.append(clip(record["readme"]))
            out.append("```")
            out.append("")

        if record.get("licence_text"):
            out.append("## Licence file")
            out.append("")
            out.append("```")
            out.append(record["licence_text"])
            out.append("```")
            out.append("")
            out.append(
                "> The repo's licence rule allows MIT, Apache-2.0, GPL-3.0 (as a separate "
                "component) and OpenRAIL. Non-commercial, research-only and revenue-capped "
                "licences are refused. Read the text, not the SPDX tag alone."
            )
            out.append("")

        if record.get("tree"):
            out.append(f"## File tree ({record.get('tree_total', '?')} files, filtered)")
            out.append("")
            for path in record["tree"]:
                out.append(f"- `{path}`")
            out.append("")

    else:
        if record.get("title"):
            out.append(f"**Title:** {record['title']}")
            out.append("")
        if record.get("meta_description"):
            out.append(f"**Meta description:** {record['meta_description']}")
            out.append("")
        if record.get("text"):
            out.append("## Page text")
            out.append("")
            out.append(clip(record["text"]))
            out.append("")
        links = record.get("links") or []
        if links:
            out.append("## Links")
            out.append("")
            for link in links:
                out.append(f"- {link}")
            out.append("")

    return "\n".join(out)


def existing_verdicts() -> dict[str, str]:
    """The verdicts already written by hand in the index, keyed by URL.

    The index is rebuilt from the cache on every run, and the cache is
    git-ignored, so a verdict written by hand in the table would be lost the
    next time a reference is captured. Read the current table first and carry
    the verdicts forward.
    """
    INDEX = INTAKE_DIR / "INDEX.md"
    if not INDEX.exists():
        return {}
    verdicts: dict[str, str] = {}
    for line in INDEX.read_text(encoding="utf-8").splitlines():
        # A date cell starts the row; do not require an exact amount of space
        # after the pipe, so a hand-edited row still parses.
        if not re.match(r"^\|\s*\d{4}-\d{2}-\d{2}", line):
            continue
        # Split on unescaped pipes only: a title may contain "|" (escaped as
        # "\|"), and splitting on every pipe would shift every cell along.
        cells = [cell.strip() for cell in re.split(r"(?<!\\)\|", line)]
        # cells: ['', '<date>', '<reference>', '<kind>', '<verdict>', '<evidence>', '']
        if len(cells) < 6:
            continue
        match = re.search(r"\((https?://[^)]+)\)", cells[2])
        if match and cells[4] and cells[4] != "—":
            verdicts[match.group(1)] = cells[4]
    return verdicts


def update_index(records: list[dict]) -> Path:
    """The running table of every reference captured."""
    INDEX = INTAKE_DIR / "INDEX.md"
    lines = ["# Research intake index", ""]
    lines.append(
        "Every reference captured by `scripts/intake.py`, newest first. One row each: "
        "what it is, what it claims, and what we took from it. The evidence file holds "
        "the raw capture."
    )
    lines.append("")
    lines.append("| Captured | Reference | Kind | Verdict | Evidence |")
    lines.append("|---|---|---|---|---|")

    rows: list[tuple[str, dict]] = []
    cache = read_cache()
    for key, record in cache.items():
        if record.get("kind") == "index":
            continue
        rows.append((record.get("captured", ""), record))
    rows.sort(key=lambda item: item[0], reverse=True)

    kept = existing_verdicts()
    for captured, record in rows:
        title = record.get("label") or record.get("title") or record["url"]
        title = title.replace("|", "\\|")  # a pipe would break the table
        verdict = record.get("verdict") or kept.get(record["url"], "—")
        # record["file"] is relative to docs/minimax-research/, so make the
        # link relative to the index, which lives in evidence/.
        rel = record.get("file", "")
        try:
            rel = os.path.relpath(EVIDENCE / rel, INTAKE_DIR).replace(os.sep, "/")
        except ValueError:
            rel = Path(rel).name
        lines.append(
            f"| {captured[:10]} | [{title}]({record['url']}) | {record.get('kind')} | "
            f"{verdict} | [{Path(rel).name}]({rel}) |"
        )

    lines.append("")
    lines.append(
        "A verdict of *taken*, *taken with a change*, *refused* or *noted* is written by "
        "hand after reading the capture; the raw file itself is never edited. A capture "
        "that failed (here: GitHub's 60-requests-per-hour limit) is marked *not captured* "
        "and is retried with `GITHUB_TOKEN` set."
    )
    lines.append("")
    INDEX.parent.mkdir(parents=True, exist_ok=True)
    INDEX.write_text("\n".join(lines), encoding="utf-8")
    return INDEX


# --------------------------------------------------------------------------
# main
# --------------------------------------------------------------------------
def capture_one(url: str, meta_only: bool, sleep: float) -> dict:
    host = urlparse(url).netloc.lower()
    if "youtube.com" in host or "youtu.be" in host:
        return capture_youtube(url, meta_only, sleep)
    if "github.com" in host:
        return capture_github(url, sleep)
    return capture_web(url, sleep)


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Capture a reference (video, repo, page) as reviewable evidence."
    )
    parser.add_argument("urls", nargs="+", help="one or more URLs")
    parser.add_argument("--tag", default="", help="a short label for this batch, e.g. gfx-3")
    parser.add_argument("--meta-only", action="store_true", help="skip captions")
    parser.add_argument("--refetch", action="store_true", help="ignore the cache")
    parser.add_argument("--sleep", type=float, default=2.0, help="seconds between requests")
    parser.add_argument(
        "--out", default="", help="write the Markdown here instead of intake/<date>/"
    )
    args = parser.parse_args()

    cache = read_cache()
    date = _dt.date.today().isoformat()
    batch_dir = INTAKE_DIR / date
    batch_dir.mkdir(parents=True, exist_ok=True)

    failures = 0
    for url in args.urls:
        key = url.rstrip("/")
        cached = cache.get(key)
        if cached and not args.refetch:
            print(f"cached  {url}")
            continue

        print(f"capture {url}")
        try:
            record = capture_one(url, args.meta_only, args.sleep)
        except Exception as exc:  # noqa: BLE001 - one bad URL must not kill the batch
            failures += 1
            record = {"kind": "error", "url": url, "captured": now_iso(),
                      "error": f"{type(exc).__name__}: {exc}"}
            print(f"  FAILED: {record['error']}")
            cache[key] = record
            write_cache(cache)
            continue

        slug = slugify(record.get("label") or url)
        filename = f"{slug}.md"
        path = Path(args.out) if args.out else batch_dir / filename
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(render_markdown(record), encoding="utf-8")

        try:
            rel = str(path.relative_to(EVIDENCE))
        except ValueError:
            rel = str(path)
        record["file"] = rel
        record["tag"] = args.tag
        cache[key] = record
        write_cache(cache)

        if record.get("error") or record.get("meta_error"):
            failures += 1
            print(f"  partial: {rel}")
        else:
            print(f"  ok: {rel}")

    index = update_index(cache)
    print(f"index: {index}")
    return 1 if failures else 0


if __name__ == "__main__":
    raise SystemExit(main())
