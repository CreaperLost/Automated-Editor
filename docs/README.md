# AeroEdits docs: start here

Consolidated on 21 Oct 2026 from 61 overlapping files into the set below, and cleaned up on
24 Oct (built plans folded in and deleted, fixed gaps removed). Each fact lives in one place;
other docs link to it instead of restating it.

## Read in this order

| # | File | Answers |
|---|---|---|
| 1 | [product.md](product.md) | What AeroEdits is, who it is for, what it does today, how an agent edits with it, what is and isn't verified |
| 2 | [gaps.md](gaps.md) | What is missing or wrong, checked against the code, with the roadmap item that fixes each |
| 3 | [roadmap.md](roadmap.md) | **The plan:** Now / Next / Later, with what "done" means for each item. The backlog lives here |
| 4 | [decisions.md](decisions.md) | What George has decided, what we are not doing, and the open questions with recommendations |
| 5 | [architecture.md](architecture.md) | How it is built: the sequence model and clocks, MCP, graphics, layouts, performance rules, working notes |
| — | [dependencies.md](dependencies.md) | Every pinned toolchain, library, runtime and model, and how we review updates |
| — | [glossary.md](glossary.md) | The project's own words |

## Folders

| Folder | What goes there |
|---|---|
| [plans/](plans/) | One plan per piece of work that is being built or about to be: design, decisions, build log. [layouts](plans/layouts.md) (built; follow-ups LAY-1…4 open), [shorts](plans/shorts.md) (partly built; next phases), [evaluation](plans/evaluation.md) (EVAL-1: real agent edits, measured; runner in `eval/`), [local-review-eval](plans/local-review-eval.md) (measurement + data in `review-eval/`). Built plans are deleted; read one with `git log --diff-filter=D -- docs/plans/` |
| [contracts/](contracts/) | File formats shared with the AeroShoot recorder: [keyboard](contracts/keyboard-telemetry.md), [windows](contracts/window-telemetry.md) |
| [research/](research/README.md) | Competitors, creator workflows, the paused re-voice lab, and the raw evidence |
| [images/](images/) | The two diagrams (plain SVG with light and dark colours; edit by hand) |

## Keeping the docs honest

- **One home per fact.** Product facts in `product.md`, gaps in `gaps.md`, work in `roadmap.md`,
  decisions in `decisions.md`. Link, don't copy.
- **Plans are for work in flight.** When a plan's work ships, move its lasting design into
  `architecture.md`, its decisions into `decisions.md`, and delete the plan (git keeps it).
- **No new dated research files.** Add to `research/competitors.md` or
  `research/creator-workflows.md`; put raw material in `research/evidence/`.
- **Label evidence:** "the code shows", "we measured", "the vendor/creator claims".
- **Check links** after moving or deleting anything:
  `.venv\Scripts\python.exe scripts/check_doc_links.py` (exit 1 on a broken link).
- The pre-consolidation tree is kept in git as `refs/backup/docs-before-consolidation`
  (`git show refs/backup/docs-before-consolidation:docs/<path>` to read an old file).
- UI mockups in `output/ui-review/` (git-ignored) are style references, not specs.
