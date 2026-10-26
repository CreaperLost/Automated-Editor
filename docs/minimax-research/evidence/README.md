# Research intake

One capture per reference. Raw material only: what a creator said, what a repo contains,
what a page claims. Nothing here is a finding.

`INDEX.md` in this folder is the running table — one row per reference with a hand-written
verdict (*taken* / *taken with a change* / *refused* / *noted* / *not captured*). The raw capture
files are never edited; the verdict lives in the index.

**What this wave concluded.** The findings these captures were read for are one file each in
[../](../) — see [../README.md](../README.md) for the table.

Captures are made by `../scripts/intake.py`:

```bash
python3 docs/minimax-research/scripts/intake.py <url> [<url> ...]
```

The folder name is the capture date, so a batch stays together and the order of the
research is visible.

**What a capture is worth.** A vendor or creator claim, until something in this repo tests
it. Captions are speech recognition and mangle brand names; the description is where the
artefact usually is — the skill, the prompt pack, the repo, the MCP server.

**What happens next.** Read the capture, add what is new to the right section of
[../../research/competitors.md](../../research/competitors.md) or
[../../research/creator-workflows.md](../../research/creator-workflows.md),
turn anything actionable into a roadmap item in [../../roadmap.md](../../roadmap.md),
and add a decision to [../../decisions.md](../../decisions.md) only when something
genuinely blocks the work.
