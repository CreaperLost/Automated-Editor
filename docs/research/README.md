# Research

Background for the roadmap. Nothing in this folder is a decision; decisions are in
[decisions.md](../decisions.md), the work is in [roadmap.md](../roadmap.md), and what the
research asks of the product is in [requirements.md](../requirements.md).

| File | What it holds |
|---|---|
| [synthesis.md](synthesis.md) | **The research, consolidated (30 Oct):** the sources and how far to trust them, the shared pipeline, fourteen findings, what isn't supported, the landscape in brief, corrections, references |
| [competitors.md](competitors.md) | Per product and toolkit (Resolve, Premiere MCPs, ChatCut, Gling, Borumi, Tella, Higgsfield/AE, Tandem, Herk Edit Kit, Astra Premiere, HyperFrames Studio, the 362-repo catalogue, MotionVideo): what we take, what we refuse |
| [revoice.md](revoice.md) | The paused re-voicing / lip-sync lab: what was tested, the licence rule, when to revisit (its scripts are in git) |
| [evidence/](evidence/) | Raw captions, the video dossier, the four ecosystem ingests, the full V01/V02 caption tracks in `raw/`, and the scripts that fetch captions. Read only to check a claim |

**Evidence grades used across the docs.** *Code*: checked in this repo. *Log*: our own
measurements. *Vendor/creator claim*: from someone else's page, docs or speech, never tested here.

**Adding research.** Don't add another review file. Add a finding to `synthesis.md` (or a product
to `competitors.md`), put raw material in `evidence/`, and turn anything actionable into a
requirement and a roadmap item.

The per-video write-ups, `creator-workflows.md` and `ai-styles.md` were folded into
`synthesis.md` on 30 Oct and deleted; read them with
`git show refs/backup/research-before-synthesis:docs/<path>`.
