# gloweaseco-leo/hyperdirector

- **Source:** https://github.com/gloweaseco-leo/hyperdirector
- **Kind:** github
- **Captured:** 2026-10-25T16:00:26
- **Evidence grade:** vendor or creator claim. Not tested in this repo. Quotes from a caption are what a creator *said*, not a measurement.

## Repository

- **Repo:** gloweaseco-leo/hyperdirector
- **Description:** Hermes Skill Pack for structured AI video production on HyperFrames — brief → storyboard → HTML → lint/render.
- **Stars:** 17
- **Forks:** 3
- **Open issues:** 0
- **Licence (SPDX):** Apache-2.0
- **Language:** JavaScript
- **Default branch:** main
- **Last push:** 2026-05-23T06:51:10Z
- **Archived:** False

## README

```
# HyperDirector

> Hermes video director enhancement pack — powered by HyperFrames.  
> 中文文档 → [README.zh-CN.md](./README.zh-CN.md) · Skill Pack 详解 → [hyperdirector/README.zh-CN.md](./hyperdirector/README.zh-CN.md)

HyperDirector is an Agent Skill Pack experiment that wraps creative video production into a structured, inspectable workflow: planning, artifact generation, validation, and delivery.

It is designed to make AI-generated video production more controllable, editable, and repeatable — from prompt to brief, storyboard, HTML project, QA checks, and final render.

**License:** [Apache License 2.0](./LICENSE) · **Contributing:** [CONTRIBUTING.md](./CONTRIBUTING.md) · **Security:** [SECURITY.md](./SECURITY.md) · **Releases:** [v0.1.0](./RELEASE_NOTES_v0.1.md) · [v0.1.1](./RELEASE_NOTES_v0.1.1.md)

---

## One-command install (Skill Pack only)

Installs **`hyperdirector/`** into the default Hermes skills path (`~/.hermes/skills/hyperdirector` on Unix, `%USERPROFILE%\.hermes\skills\hyperdirector` on Windows). Override the destination with **`HERMES_SKILLS_DIR`**.

**Security:** Users may inspect [`install.sh`](./install.sh) and [`install.ps1`](./install.ps1) before running the one-command installer.

**macOS / Linux / WSL:**

```bash
curl -fsSL https://raw.githubusercontent.com/gloweaseco-leo/hyperdirector/main/install.sh | bash
```

**Windows PowerShell:**

```powershell
irm https://raw.githubusercontent.com/gloweaseco-leo/hyperdirector/main/install.ps1 | iex
```

**Scope:** This installs the **HyperDirector Skill Pack only**. Real rendering still depends on your local Hermes setup, **HyperFrames CLI**, **Node.js** (≥22), **FFmpeg**, and a browser environment supported by HyperFrames. HyperDirector is a director layer **on top of** HyperFrames — not a replacement for it.

After install, verify tooling:

```bash
node ~/.hermes/skills/hyperdirector/scripts/check-hyperframes-env.js
```

(On Windows, use `%USERPROFILE%\.hermes\skills\hyperdirector\scripts\check-hyperframes-env.js`.)

---

## What It Does

HyperDirector takes a text prompt from Hermes and runs a structured video production workflow:

```
User prompt
  → Hermes + HyperDirector skill
    → brief.json → storyboard.json → DESIGN.md → index.html
      → npx hyperframes lint / preview / render
        → final.mp4 + editable HTML source
```

It wraps [HyperFrames](https://github.com/heygen-com/hyperframes) (HTML-to-video engine by HeyGen) with a director layer: capability judgment, brand memory, template selection, QA fix loop, and delivery packaging.

---

## Repository Layout

```
HyperDirector/
├── hyperdirector/        ← Hermes Skill Pack (install this into Hermes skills)
│   ├── SKILL.md          ← Hermes entry point
│   ├── docs/             ← User documentation (8 files, zh-CN)
│   ├── examples/         ← Complete demo projects
│   │   └── zh-CN/
│   │       ├── demo-article-to-video/
│   │       ├── demo-saas-product/
│   │       └── demo-github-repo/
│   ├── templates/        ← 3 built-in video templates
│   ├── workflows/        ← 7 workflow guides
│   ├── qa/               ← QA checklists and templates
│   ├── scripts/          ← Validation scripts (Node.js)
│   ├── schemas/          ← JSON Schema definitions
│   └── brand/            ← Brand Kit templates
├── output/               ← Generated video projects (not committed)
└── PRD.md                ← Product requirements document
```

---

## Prerequisites

| Dependency | Version | Install |
|---|---|---|
| Node.js | >= 22 | https://nodejs.org |
| HyperFrames CLI | latest | `npm install -g hyperframes` |
| FFmpeg | any recent | https://ffmpeg.org/download.html |
| Hermes | configured | see your Hermes docs |

---

## Quick Install

```bash
# 1. Verify environment
node hyperdirector/scripts/check-env.js

# 2. Copy skill to Hermes
cp -r hyperdirector/ ~/.hermes/skills/hyperdirector/

# 3. Verify skill loaded — ask Hermes:
#    "What templates does HyperDirector support?"

# 4. Set up Brand Kit
cp hyperdirector/brand/brand-kit.example.json ./brand-kit.json
# Edit brand-kit.json with your brand colors, fonts, CTA

# 5. Run first demo
open hyperdirector/examples/zh-CN/demo-article-to-video/output/preview.html
```

---

## Documentation

| Document | Description |
|---|---|
| [docs/quickstart.md](./hyperdirector/docs/quickstart.md) | Get started in 20 minutes |
| [docs/installation.md](./hyperdirector/docs/installation.md) | Full install guide (Node, FFmpeg, HyperFrames, Hermes) |
| [docs/first-video.md](./hyperdirector/docs/first-video.md) | Complete walkthrough: article → 30s video |
| [docs/brand-kit-setup.md](./hyperdirector/docs/brand-kit-setup.md) | Brand Kit configuration reference |
| [docs/template-guide.md](./hyperdirector/docs/template-guide.md) | Template selection and customization |
| [docs/faq.md](./hyperdirector/docs/faq.md) | Frequently asked questions |
| [docs/cursor-development-notes.md](./hyperdirector/docs/cursor-development-notes.md) | Developer notes: v0.1 scope, adding templates/workflows/rules |

---

## Templates (v0.1)

| Template | Aspect Ratio | Best For |
|---|---|---|
| `tiktok-vertical-kit` | 9:16 | WeChat Video, TikTok, YouTube Shorts |
| `saas-demo-kit` | 16:9 / 9:16 | Product demos, feature videos, launch announcements |
| `ai-knowledge-explainer-kit` | 9:16 | AI tutorials, open source project intros, tech explainers |

---

## Validation Scripts

```bash
node hyperdirector/scripts/check-env.js                    # check dependencies
node hyperdirector/scripts/validate-brand-kit.js <path>    # validate brand-kit.json
node hyperdirector/scripts/validate-brief.js <path>        # validate brief.json
node hyperdirector/scripts/validate-storyboard.js <path>   # validate storyboard.json
node hyperdirector/scripts/check-output-contract.js <dir>  # check output directory
node hyperdirector/scripts/check-composition-hazards.js <file.html>  # optional heuristic warnings (not lint)
```

---

*Powered by HyperFrames. Directed by Hermes. Packaged as HyperDirector.*
```

## Licence file

```
Apache License
                           Version 2.0, January 2004
                        http://www.apache.org/licenses/

   TERMS AND CONDITIONS FOR USE, REPRODUCTION, AND DISTRIBUTION

   1. Definitions.

      "License" shall mean the terms and conditions for use, reproduction,
      and distribution as defined by Sections 1 through 9 of this document.

      "Licensor" shall mean the copyright owner or entity authorized by
      the copyright owner that is granting the License.

      "Legal Entity" shall mean the union of the acting entity and all
      other entities that control, are controlled by, or are under common
      control with that entity. For the purposes of this definition,
      "control" means (i) the power, direct or indirect, to cause the
      direction or management of such entity, whether by contract or
      otherwise, or (ii) ownership of fifty percent (50%) or more of the
      outstanding shares, or (iii) beneficial ownership of such entity.

      "You" (or "Your") shall mean an individual or Legal Entity
      exercising permissions granted by this License.

      "Source" form shall mean the preferred form for making modifications,
      including but not limited to software source code, documentation
      source, and configuration files.

      "Object" form shall mean any form resulting from mechanical
      transformation or translation of a Source form, including but
      not limited to compiled object code, generated documentation,
      and conversions to other media types.

      "Work" shall mean the work of authorship, whether in Source or
      Object form, made available under the License, as indicated by a
      copyright notice that is included in or attached to the work
      (an example is provided in the Appendix below).

      "Derivative Works" shall mean any work, whether in Source or Object
      form, that is based on (or derived from) the Work and for which the
      editorial revisions, annotations, elaborations, or other modifications
      represent, as a whole, an original work of authorship. For the purposes
      of this License, Derivative Works shall not include works that remain
      separable from, or merely link (or bind by name) to the interfaces of,
      the Work and Derivative Works thereof.

      "Contribution" shall mean any work of authorship, including
      the original version of the Work and any modifications or additions
      to that Work or Derivative Works thereof, that is intentionally
      submitted to Licensor for inclusion in the Work by the copyright owner
      or by an individual or Legal Entity authorized to submit on behalf of
      the copyright owner. For the purposes of this definition, "submitted"
      means any form of electronic, verbal, or written communication sent
      to the Licensor or its representatives, including but not limited to
      communication on electronic mailing lists, source code control systems,
      and issue tracking systems that are managed by, or on behalf of, the
      Licensor for the purpose of discussing and improving the Work, but
      excluding communication that is conspicuously marked or otherwise
      designated in writing by the copyright owner as "Not a Contribution."

      "Contributor" shall mean Licensor and any individual or Legal Entity
      on behalf of whom a Contribution has been received by Licensor and
      subsequently incorporated within the Work.

   2. Grant of Copyright License. Subject to the terms and conditions of
      this License, each Contributor hereby grants to You a perpetual,
      worldwide, non-exclusive, no-charge, royalty-free, irrevocable
      copyright license to reproduce, prepare Derivative Works of,
      publicly display, publicly perform, sublicense, and distribute the
      Work and such Derivative Works in Source or Object form.

   3. Grant of Patent License. Subject to the terms and conditions of
      this License, each Contributor hereby grants to You a perpetual,
      worldwide, non-exclusive, no-charge, royalty-free, irrevocable
      (except as stated in this section) patent license to make, have made,
      use, offer to sell, sell, import, and otherwise transfer the Work,
      where such license applies only to those patent claims licensable
      by such Contributor that are necessarily infringed by their
      Contribution(s) alone or by combination of their Contribution(s)
      with the Work to which such Contribution(s) was submitted. If You
      institute patent litigation against any entity (including a
      cross-claim or counterclaim in a lawsuit) alleging that the Work
      or a Contribution incorporated within the Work constitutes direct
      or contributory patent infringement, then any patent licenses
      granted to You under this License for that Work shall terminate
      as of the date such litigation is filed.

   4. Redistribution. You may reproduce and distribute copies of the
      Work or Derivative Works thereof in any medium, with or without
      modifications, and in Source or Object form, provided that You
      meet the following conditions:

      (a) You must give any other recipients of the Work or
          Derivative Works a copy of this License; and

      (b) You must cause any modified files to carry prominent notices
          stating that You changed the files; and

      (c) You must retain, in the Source form of any Derivative Works
          that You distribute, all copyright, patent, trademark, and
          attribution notices from the Source form of the Work,
          excluding those notices that do not pertain to any part of
          the Derivative Works; and

      (d) If the Work includes a "NOTICE" text file as part of its
          distribution, then any Derivative Works that You distribute must
          include a readable copy of the attribution notices contained
          within such NOTICE file, excluding those notices that do not
          pertain to any part of the Derivative Works, in at least one
          of the following places: within a NOTICE text file distributed
          as part of the Derivative Works; within the Source form or
          documentation, if provided along with the Derivative Works; or,
          within a display generated by the Derivative Works, if and
          wherever such third-party notices normally appear. The contents
          of the NOTICE file are for informational purposes only and
          do not modify the License. You may add Your own attribution
          notices within Derivative Works that You distribute, alongside
          or as an addendum to the NOTICE text from the Work, provided
          that such additional attribution notices cannot be construed
          as modifying the License.

      You may add Your own copyright statement to Your modifications and
      may provide additional or different license terms and conditions
      for use, reproduction, or distribution of Your modifications, or
      for any such Derivative Works as a whole, provided Your use,
      reproduction, and distribution of the Work otherwise complies with
      the conditions stated in this License.

   5. Submission of Contributions. Unless You explicitly state otherwise,
      any Contribution intentionally submitted for inclusion in the Work
      by You to the Licensor shall be under the terms and conditions of
      this License, without any additional terms or conditions.
      Notwithstanding the above, nothing herein shall supersede or modify
      the terms of any separate license agreement you may have executed
      with Licensor regarding such Contributions.

   6. Trademarks. This License does not grant permission to use the trade
      names, trademarks, service marks, or product names of the Licensor,
      except as required for reasonable and customary use in describing the
      origin of the Work and reproducing the content of the NOTICE file.

  

[... truncated at 8000 chars ...]
```

> The repo's licence rule allows MIT, Apache-2.0, GPL-3.0 (as a separate component) and OpenRAIL. Non-commercial, research-only and revenue-capped licences are refused. Read the text, not the SPDX tag alone.

## File tree (138 files, filtered)

- `.github/ISSUE_TEMPLATE/bug_report.md`
- `.github/ISSUE_TEMPLATE/feature_request.md`
- `.github/pull_request_template.md`
- `CONTRIBUTING.md`
- `LICENSE`
- `README.md`
- `README.zh-CN.md`
- `RELEASE_NOTES_v0.1.1.md`
- `RELEASE_NOTES_v0.1.2-preview.md`
- `RELEASE_NOTES_v0.1.3-preview.md`
- `RELEASE_NOTES_v0.1.md`
- `SECURITY.md`
- `hyperdirector/AGENTS.md`
- `hyperdirector/CAPABILITY_BOUNDARY.md`
- `hyperdirector/CAPABILITY_BOUNDARY.zh-CN.md`
- `hyperdirector/README.md`
- `hyperdirector/README.zh-CN.md`
- `hyperdirector/RELEASE_NOTES_v0.1.md`
- `hyperdirector/SKILL.md`
- `hyperdirector/brand/brand-intake-form.md`
- `hyperdirector/brand/brand-kit-guide.md`
- `hyperdirector/brand/motion-language.example.md`
- `hyperdirector/docs/audio-workflow.md`
- `hyperdirector/docs/audio-workflow.zh-CN.md`
- `hyperdirector/docs/brand-kit-setup.md`
- `hyperdirector/docs/brand-kit-setup.zh-CN.md`
- `hyperdirector/docs/cursor-development-notes.md`
- `hyperdirector/docs/faq.md`
- `hyperdirector/docs/faq.zh-CN.md`
- `hyperdirector/docs/first-video.md`
- `hyperdirector/docs/first-video.zh-CN.md`
- `hyperdirector/docs/installation.md`
- `hyperdirector/docs/installation.zh-CN.md`
- `hyperdirector/docs/quickstart.md`
- `hyperdirector/docs/quickstart.zh-CN.md`
- `hyperdirector/docs/rendering-stability.zh-CN.md`
- `hyperdirector/docs/source-image-workflow.md`
- `hyperdirector/docs/source-image-workflow.zh-CN.md`
- `hyperdirector/docs/template-guide.md`
- `hyperdirector/docs/template-guide.zh-CN.md`
- `hyperdirector/examples/en/README.md`
- `hyperdirector/examples/zh-CN/demo-article-to-video/README.md`
- `hyperdirector/examples/zh-CN/demo-article-to-video/input/article.md`
- `hyperdirector/examples/zh-CN/demo-github-repo/README.md`
- `hyperdirector/examples/zh-CN/demo-github-repo/input/readme-sample.md`
- `hyperdirector/examples/zh-CN/demo-saas-product/README.md`
- `hyperdirector/examples/zh-CN/demo-saas-product/input/product-brief.md`
- `hyperdirector/prompts/01-capability-judge.md`
- `hyperdirector/prompts/02-intake-brief.md`
- `hyperdirector/prompts/03-storyboard-generator.md`
- `hyperdirector/prompts/04-visual-design.md`
- `hyperdirector/prompts/05-compose-hyperframes.md`
- `hyperdirector/prompts/06-qa-fixer.md`
- `hyperdirector/prompts/07-render-report.md`
- `hyperdirector/prompts/08-warm-iteration.md`
- `hyperdirector/qa/audio-qa-checklist.md`
- `hyperdirector/qa/final-delivery-checklist.md`
- `hyperdirector/qa/image-asset-checklist.md`
- `hyperdirector/qa/lint-fix-loop.md`
- `hyperdirector/qa/pre-render-checklist.md`
- `hyperdirector/qa/qa-report-template.md`
- `hyperdirector/qa/troubleshooting.md`
- `hyperdirector/rules/audio-director-rules.md`
- `hyperdirector/rules/common-errors-fix.md`
- `hyperdirector/rules/content-safety-rules.md`
- `hyperdirector/rules/gsap-deterministic-rules.md`
- `hyperdirector/rules/headless-rendering-stability.md`
- `hyperdirector/rules/hyperframes-core-rules.md`
- `hyperdirector/rules/image-assets-basics.md`
- `hyperdirector/rules/performance-checklist.md`
- `hyperdirector/rules/subtitle-safe-area.md`
- `hyperdirector/rules/template-authoring-rules.md`
- `hyperdirector/schemas/asset-manifest.schema.json`
- `hyperdirector/schemas/audio-manifest.schema.json`
- `hyperdirector/schemas/brand-kit.schema.json`
- `hyperdirector/schemas/brief.schema.json`
- `hyperdirector/schemas/caption-timeline.schema.json`
- `hyperdirector/schemas/edit-request.schema.json`
- `hyperdirector/schemas/output-contract.md`
- `hyperdirector/schemas/render-report.schema.json`
- `hyperdirector/schemas/storyboard.schema.json`
- `hyperdirector/scripts/check-composition-hazards.js`
- `hyperdirector/scripts/check-env.js`
- `hyperdirector/scripts/check-hyperframes-env.js`
- `hyperdirector/scripts/check-output-contract.js`
- `hyperdirector/scripts/leak-scan.js`
- `hyperdirector/scripts/validate-brand-kit.js`
- `hyperdirector/scripts/validate-brief.js`
- `hyperdirector/scripts/validate-storyboard.js`
- `hyperdirector/templates/ai-knowledge-explainer-kit/DESIGN.md`
- `hyperdirector/templates/ai-knowledge-explainer-kit/README.md`
- `hyperdirector/templates/ai-knowledge-explainer-kit/assets/README.md`
- `hyperdirector/templates/ai-knowledge-explainer-kit/customization-guide.md`
- `hyperdirector/templates/ai-knowledge-explainer-kit/prompt.md`
- `hyperdirector/templates/ai-knowledge-explainer-kit/template.html`
- `hyperdirector/templates/ai-knowledge-explainer-kit/variants/15s.md`
- `hyperdirector/templates/ai-knowledge-explainer-kit/variants/30s.md`
- `hyperdirector/templates/ai-knowledge-explainer-kit/variants/60s.md`
- `hyperdirector/templates/saas-demo-kit/DESIGN.md`
- `hyperdirector/templates/saas-demo-kit/README.md`
- `hyperdirector/templates/saas-demo-kit/assets/README.md`
- `hyperdirector/templates/saas-demo-kit/customization-guide.md`
- `hyperdirector/templates/saas-demo-kit/prompt.md`
- `hyperdirector/templates/saas-demo-kit/template.html`
- `hyperdirector/templates/saas-demo-kit/variants/15s.md`
- `hyperdirector/templates/saas-demo-kit/variants/30s.md`
- `hyperdirector/templates/saas-demo-kit/variants/60s.md`
- `hyperdirector/templates/tiktok-vertical-kit/DESIGN.md`
- `hyperdirector/templates/tiktok-vertical-kit/README.md`
- `hyperdirector/templates/tiktok-vertical-kit/assets/README.md`
- `hyperdirector/templates/tiktok-vertical-kit/customization-guide.md`
- `hyperdirector/templates/tiktok-vertical-kit/prompt.md`
- `hyperdirector/templates/tiktok-vertical-kit/template.html`
- `hyperdirector/templates/tiktok-vertical-kit/variants/15s.md`
- `hyperdirector/templates/tiktok-vertical-kit/variants/30s.md`
- `hyperdirector/templates/tiktok-vertical-kit/variants/60s.md`
- `hyperdirector/upstream/hyperframes-agent-native-rules.md`
- `hyperdirector/upstream/hyperframes-anti-patterns.md`
- `hyperdirector/upstream/hyperframes-command-patterns.md`
- `hyperdirector/upstream/hyperframes-docs-map.md`
- `hyperdirector/upstream/hyperframes-integration-positioning.md`
- `hyperdirector/upstream/hyperframes-principles.md`
- `hyperdirector/upstream/language-strategy.md`
- `hyperdirector/workflows/article-to-video.md`
- `hyperdirector/workflows/batch-video-production.md`
- `hyperdirector/workflows/data-chart-to-video.md`
- `hyperdirector/workflows/github-repo-to-video.md`
- `hyperdirector/workflows/one-prompt-video.md`
- `hyperdirector/workflows/prd-to-product-video.md`
- `hyperdirector/workflows/product-page-to-demo.md`
- `hyperdirector/workflows/warm-iteration.md`
