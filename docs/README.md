# AeroEdits docs

Five documents for the work ahead. Consolidated 1 November 2026 from the working tree,
including that day's uncommitted updates. Implementation status is separate from verification;
these notes do not claim a new test run.

| Document | Owns |
|---|---|
| [Product](product.md) | Purpose, current capabilities, settled decisions and acceptance criteria |
| [Roadmap](roadmap.md) | Next work, unresolved issues, dependencies and deferred work |
| [Architecture](architecture.md) | Implementation invariants, working practices and recorder contracts |
| [Verification](verification.md) | Repeatable evaluation, measured baselines, validation gaps and dependency review |

Build, setup and repository layout remain in the [root README](../README.md).
The shipped agent instructions remain in `src-tauri/src/mcp/skill/` and
`src-tauri/src/preset/methods/`; tool schemas and lockfiles are authoritative for arguments and versions.

## Keeping this small

- Update the document that owns the fact. Do not add parallel gaps, requirements, decisions,
  shipped-work logs, per-feature plans or research dossiers.
- Keep a short design and acceptance test with the active roadmap item. When it ships,
  remove the item and retain only lasting behavior in Product or Architecture and useful
  measurement in Verification. Keep existing work IDs stable.
- Distinguish code inspection, measured results and user judgement. Competitor claims are
  not proof of our quality or feature gaps.
- Keep evaluation scripts and labelled data in [plans/eval](plans/eval/) and
  [plans/review-eval](plans/review-eval/). Their paths stay stable for existing runners;
  these folders contain test support, not additional plans.
- Check links after changes: `.venv\Scripts\python.exe scripts/check_doc_links.py --all`.

## Older material

Completed plans, dated audits, research transcripts, competitor inventories and the shipped-work
log were removed after extracting the decisions and remaining work. Use Git history when the
original investigation matters: `git log --all -- docs/<old-path>`, then
`git show <commit>:docs/<old-path>`. A recovery ZIP outside the repository preserves the prior documentation content, including
uncommitted changes reconstructed from the initial diff (line endings normalized):
`C:/Users/George/.codex/visualizations/2026/11/01/01a19913-c032-7df3-b15d-45d2d0663e16/docs-before-cleanup.zip`.
