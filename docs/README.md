# AeroEdits documentation

Reconciled 8 November 2026 against `be7179c`, starting from a clean working tree. Start with
[HANDOFF](HANDOFF.md) for machine context and standing decisions, then [Roadmap](ROADMAP.md)
for the next task. This consolidation changes documentation only and leaves it uncommitted.

| Document | Owns |
|---|---|
| [Handoff](HANDOFF.md) | Current inspected state, machine-only context, cloud access, standing decisions, questions for George |
| [Product](product.md) | Purpose, capabilities, settled scope and acceptance principles |
| [Roadmap](ROADMAP.md) | Remaining features, work order, acceptance boundaries and dependencies; existing CL/FCL/PUB IDs stay stable |
| [Bugs and performance gaps](gap.md) | Canonical detailed issue report: priorities, paths, evidence, impact, solutions, validation and fix questions; existing GAP IDs stay stable |
| [Architecture](architecture.md) | Current implementation, invariants and their known exceptions, operational/recorder contracts |
| [Verification](verification.md) | Fresh checks, dated measurements, retained audit outcomes, procedures and acceptance still missing |

Build/setup remains in the [root README](../README.md#development) and [AGENTS.md](../AGENTS.md).
The shipped editing instructions live in `src-tauri/src/mcp/skill/` and `preset/methods/`;
schemas, source and lockfiles own arguments, settings and versions.

## Maintenance

- Update the owner above. Cross-link related material; do not create another issue catalogue,
  feature plan, shipped-work log or provider/version inventory.
- Keep remaining work actionable. Move lasting shipped behavior to Product/Architecture and
  useful checks to Verification. Preserve stable IDs for remaining findings and leave short
  closure notes when their prior status would otherwise be ambiguous.
- Label current code inspection, newly executed tests, historical probes, missing checks and
  human judgement separately. Neither age nor lack of a reproduction closes an issue.
- Preserve [plans/eval](plans/eval/) and [plans/review-eval](plans/review-eval/): these are used
  evaluation runners/labelled data, not abandoned plans. English cases still reference private
  transcripts; the Greek transcripts are included. Do not delete them as backlog cleanup.
- Run `python scripts/check_doc_links.py --all` (`python3` on Linux). It checks file targets,
  not anchors, case portability or external URLs; inspect changed fragments and exact path case.

## Removed material

`audit.md`'s completed AUD-001..008 narratives were consolidated into
[Architecture](architecture.md#cache-and-save-ownership-contracts) and
[Verification](verification.md#retired-audit-material); current regression tests remain.
`cloud-prep-pr.md`'s stale PR body/checklist was consolidated into Handoff and Verification.
Roadmap's shipped faceless/publish implementation history and old Python inventory were
replaced by the current Architecture section and remaining work. GAP-001/014 retain short
superseded notes; GAP-005/006 retain evidence-backed closure notes in gap.md.

Older completed plans/research dossiers were already removed in the 1 Nov cleanup. Recover
originals from Git history (`git log --all -- docs/<old-path>`, then
`git show <commit>:docs/<old-path>`); a private recovery ZIP is not a build dependency.
