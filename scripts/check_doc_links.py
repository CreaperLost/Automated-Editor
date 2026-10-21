#!/usr/bin/env python3
"""Check that the relative Markdown links in the docs folder actually resolve.

This repo is link-heavy: `docs/roadmap.md`, `docs/gaps.md` and the plans cross-reference each
other by anchor, and the research folder adds more. A renamed or deleted file silently breaks
every link that pointed at it, and nothing else in the build notices.

Usage
-----
    .venv\\Scripts\\python.exe scripts/check_doc_links.py
    .venv\\Scripts\\python.exe scripts/check_doc_links.py --path docs/plans
    .venv\\Scripts\\python.exe scripts/check_doc_links.py --all

Exit code is 1 if any link is broken, so it can be used as a pre-commit gate.

What it checks
--------------
* every Markdown file under the scanned path (default: the whole `docs/` tree);
* every inline link `[text](target)` and reference definition `[id]: target`;
* relative targets only — absolute `http(s)://` and `mailto:` links are skipped, because
  checking them would make the run depend on the network;
* fragment-only links (`#anchor`) are skipped: GitHub regenerates those from headings and
  the exact slug rules are fussy to reimplement correctly.

What it does not check
----------------------
* that the *anchor* of a cross-document link exists in the target file;
* external links.
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
DOCS = REPO / "docs"

# [text](target) — target may contain spaces if wrapped in <>, which we normalise away.
INLINE_RE = re.compile(r"\[[^\]]*\]\(\s*(<[^>]+>|[^)\s]+)(?:\s+\"[^\"]*\")?\s*\)")
# [id]: target  (reference definitions)
REFDEF_RE = re.compile(r"^\s*\[[^\]]+\]:\s*(<[^>]+>|\S+)", re.MULTILINE)
# Fenced code blocks are skipped so shell snippets with brackets are not parsed as links.
FENCE_RE = re.compile(r"^```.*?^```", re.MULTILINE | re.DOTALL)


def iter_targets(text: str):
    body = FENCE_RE.sub("", text)
    for match in INLINE_RE.finditer(body):
        yield match.group(1).strip("<>")
    for match in REFDEF_RE.finditer(body):
        yield match.group(1).strip("<>")


def check(path: Path, scanned: list[tuple[Path, str]]) -> int:
    broken = 0
    for doc, target in scanned:
        if target.startswith(("http://", "https://", "mailto:", "#", "data:")):
            continue
        # Drop any query/fragment, then resolve against the document's own directory.
        clean = target.split("#", 1)[0].split("?", 1)[0]
        if not clean:
            continue
        resolved = (doc.parent / clean).resolve()
        if not resolved.exists():
            rel = doc.relative_to(REPO).as_posix()
            print(f"BROKEN  {rel:52s} -> {target}")
            broken += 1
    return broken


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--path", default="docs", help="directory to scan (default: docs)")
    ap.add_argument("--all", action="store_true", help="also scan the repo root README")
    args = ap.parse_args()

    root = (REPO / args.path).resolve()
    if not root.exists():
        sys.exit(f"not a directory: {root}")

    docs = sorted(root.rglob("*.md")) if root.is_dir() else [root]
    if args.all:
        readme = REPO / "README.md"
        if readme.exists():
            docs.append(readme)

    scanned: list[tuple[Path, str]] = []
    total = 0
    for doc in docs:
        try:
            text = doc.read_text(encoding="utf-8", errors="replace")
        except OSError as exc:
            print(f"UNREADABLE  {doc}: {exc}")
            continue
        for target in iter_targets(text):
            scanned.append((doc, target))
            total += 1

    broken = check(root, scanned)
    print("---")
    print(f"documents: {len(docs)} | relative link targets: {total} | broken: {broken}")
    return 1 if broken else 0


if __name__ == "__main__":
    raise SystemExit(main())
