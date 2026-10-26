# geekjourneyx/hyperframes-motion-director

- **Source:** https://github.com/geekjourneyx/hyperframes-motion-director
- **Kind:** github
- **Captured:** 2026-10-25T16:00:21
- **Evidence grade:** vendor or creator claim. Not tested in this repo. Quotes from a caption are what a creator *said*, not a measurement.

## Repository

- **Repo:** geekjourneyx/hyperframes-motion-director
- **Description:** Agent Skill for Chinese-first HyperFrames motion-video production from articles, products, websites, and README files.
- **Homepage:** https://jieni.ai/skills
- **Stars:** 468
- **Forks:** 46
- **Open issues:** 0
- **Licence (SPDX):** AGPL-3.0
- **Language:** JavaScript
- **Default branch:** main
- **Last push:** 2026-07-26T08:28:32Z
- **Archived:** False
- **Topics:** agent-skill, article-to-video, chinese-first, hyperframes, image-generation, motion-video, readme-to-video, video-generation, video-production, website-to-video

## README

```
<div align="center">

# HyperFrames Motion Director

**An Agent Skill for planning, producing, and reviewing HyperFrames motion videos.**

<img src="skills/hyperframes-motion-director/assets/banner.png" alt="HyperFrames Motion Director" width="100%">

[![License: AGPL-3.0](https://img.shields.io/badge/License-AGPL--3.0-blue.svg)](./LICENSE)
[![Agent Skill](https://img.shields.io/badge/Agent%20Skill-HyperFrames-c96442.svg)](./skills/hyperframes-motion-director/SKILL.md)
[![Node.js](https://img.shields.io/badge/Node.js-18%2B-141413.svg)](https://nodejs.org/)

</div>

## What It Is

HyperFrames Motion Director is an Agent Skill for turning a product story, article, README, website, or launch message into a structured HyperFrames motion-video production.

It defaults to Chinese-first vertical promo videos: `9:16`, `1080x1920`, concise screen copy, controlled visual assets, readable hold frames, and a review step before delivery.

The skill keeps production in two phases:

1. Create a brief/design proposal and wait for confirmation.
2. Generate assets, build the composition, validate snapshots/renders, and write a review report.

## Install

```bash
npx skills add geekjourneyx/hyperframes-motion-director
```

## Use

After installation, ask your Agent for a HyperFrames motion video, product launch film, article-to-video piece, or kinetic typography promo.

Example:

```text
Turn this README into a 12-second Chinese vertical HyperFrames promo.
Start with the brief/design proposal and wait for confirmation.
```

## Local Development

Create a motion-video production scaffold from this repository:

```bash
node skills/hyperframes-motion-director/scripts/create_project.mjs ./my-motion-film
```

Create a project with timing and motion maps:

```bash
node skills/hyperframes-motion-director/scripts/create_project.mjs ./my-motion-film --with-timing --with-motion
```

## Outputs

The scaffold creates four core production files:

```text
BRIEF_DESIGN_PROPOSAL.md  Direction, format, visual plan, motion plan
DESIGN.md                 Visual system, asset rules, layout contracts
STORYBOARD.md             Beats, screen copy, timing, transitions
REVIEW_REPORT.md          Checks, snapshots, issues, remaining risks
```

It also creates three design-engineering contracts:

```text
SCENE_SCHEMA.json         Structured scenes, slots, layout contracts, timing, snapshots
VECTOR_TEMPLATES.json     Approved SVG scene systems, icon/decor rules, rejection tests
MOTION_PRIMITIVES.json    Approved GSAP/SVG/CSS motion vocabulary, plugin policy, selection rules
```

Optional files:

```text
BEAT_MAP.json             Music, voiceover, or exact timing map
MOTION_MAP.json           GSAP choreography and transition map
```

## What It Enforces

<img src="skills/hyperframes-motion-director/assets/features.png" alt="Background, typography, and motion layers" width="100%">

- A confirmed brief before implementation.
- Chinese vertical-video defaults, with documented overrides for other platforms.
- Image Gen assets with a clear role, quiet text zone, crop-safe area, and local path.
- Background and component counts derived from the actual story, product mechanism, and proof needs rather than a fixed quota.
- Exact supplied style sources inspected directly, with a clear boundary between visual language and product narrative.
- Independent vertical background plates plus source-driven component sheets, with separate hero generations added when needed.
- Asset libraries checked for narrative-anchor, product-proof, and transition-carrier roles, plus distinct component combinations.
- Named transparent cutouts, dark/light contact-sheet review, and a manifest that records accepted assets and revisions.
- Text-over-background layout contracts before animation.
- Scene schema, vector template, and motion primitive contracts before implementation.
- GSAP choreography contracts: labels, position parameters, plugin registration, transform aliases, and performance rules.
- Motion that guides attention instead of repeating static slide patterns.
- Final-MP4 checks for composite-layer ownership, visible transparent-asset bounds, exported font fit, and entry/hold/exit frames.
- Validation, snapshots, and review notes before final delivery.
- Compact artifact writing: no self-talk, generic pitch language, or unrelated commentary.

## Validate This Skill

Check the skill package:

```bash
node skills/hyperframes-motion-director/scripts/check-structure.mjs
```

Check a generated project:

```bash
node skills/hyperframes-motion-director/scripts/check_assets.mjs <project-dir>
node skills/hyperframes-motion-director/scripts/check_assets.mjs <project-dir> --strict
node skills/hyperframes-motion-director/scripts/check_assets.mjs <project-dir> --require-premium-assets
node skills/hyperframes-motion-director/scripts/validate_image_assets.mjs <project-dir>
node skills/hyperframes-motion-director/scripts/validate_artifacts.mjs <project-dir>
node skills/hyperframes-motion-director/scripts/validate_design_engineering.mjs <project-dir>
```

For implemented HyperFrames compositions, also run the strongest checks supported by the local HyperFrames CLI, such as validate, inspect, snapshot, and render.

## Releases

Stable tags publish GitHub Releases automatically after the tagged source passes the repository checks. A `main` push also repairs the newest stable tag when its Release is missing.

See [RELEASING.md](RELEASING.md) for the complete release procedure and recovery rules.

## Repository Structure

```text
skills/hyperframes-motion-director/SKILL.md      Main Agent Skill instructions
skills/hyperframes-motion-director/templates/    Brief, design, storyboard, review, schema, vector, motion maps
skills/hyperframes-motion-director/references/   Workflow, visual standards, design engineering, GSAP choreography, layout, audio sync, stability
skills/hyperframes-motion-director/scripts/      Project scaffold and validation helpers
skills/hyperframes-motion-director/evals/        Trigger prompts and evaluation cases
skills/hyperframes-motion-director/assets/       Skill and README visual assets
```

## Author

- Website: [jieni.ai](https://jieni.ai)
- GitHub: [geekjourneyx](https://github.com/geekjourneyx)
- X: [@seekjourney](https://x.com/seekjourney)
- WeChat Official Account: 极客杰尼

## License

[GNU Affero General Public License v3.0](./LICENSE)
```

## Licence file

```
GNU AFFERO GENERAL PUBLIC LICENSE
                       Version 3, 19 November 2007

 Copyright (C) 2007 Free Software Foundation, Inc. <https://fsf.org/>
 Everyone is permitted to copy and distribute verbatim copies
 of this license document, but changing it is not allowed.

                            Preamble

  The GNU Affero General Public License is a free, copyleft license for
software and other kinds of works, specifically designed to ensure
cooperation with the community in the case of network server software.

  The licenses for most software and other practical works are designed
to take away your freedom to share and change the works.  By contrast,
our General Public Licenses are intended to guarantee your freedom to
share and change all versions of a program--to make sure it remains free
software for all its users.

  When we speak of free software, we are referring to freedom, not
price.  Our General Public Licenses are designed to make sure that you
have the freedom to distribute copies of free software (and charge for
them if you wish), that you receive source code or can get it if you
want it, that you can change the software or use pieces of it in new
free programs, and that you know you can do these things.

  Developers that use our General Public Licenses protect your rights
with two steps: (1) assert copyright on the software, and (2) offer
you this License which gives you legal permission to copy, distribute
and/or modify the software.

  A secondary benefit of defending all users' freedom is that
improvements made in alternate versions of the program, if they
receive widespread use, become available for other developers to
incorporate.  Many developers of free software are heartened and
encouraged by the resulting cooperation.  However, in the case of
software used on network servers, this result may fail to come about.
The GNU General Public License permits making a modified version and
letting the public access it on a server without ever releasing its
source code to the public.

  The GNU Affero General Public License is designed specifically to
ensure that, in such cases, the modified source code becomes available
to the community.  It requires the operator of a network server to
provide the source code of the modified version running there to the
users of that server.  Therefore, public use of a modified version, on
a publicly accessible server, gives the public access to the source
code of the modified version.

  An older license, called the Affero General Public License and
published by Affero, was designed to accomplish similar goals.  This is
a different license, not a version of the Affero GPL, but Affero has
released a new version of the Affero GPL which permits relicensing under
this license.

  The precise terms and conditions for copying, distribution and
modification follow.

                       TERMS AND CONDITIONS

  0. Definitions.

  "This License" refers to version 3 of the GNU Affero General Public License.

  "Copyright" also means copyright-like laws that apply to other kinds of
works, such as semiconductor masks.

  "The Program" refers to any copyrightable work licensed under this
License.  Each licensee is addressed as "you".  "Licensees" and
"recipients" may be individuals or organizations.

  To "modify" a work means to copy from or adapt all or part of the work
in a fashion requiring copyright permission, other than the making of an
exact copy.  The resulting work is called a "modified version" of the
earlier work or a work "based on" the earlier work.

  A "covered work" means either the unmodified Program or a work based
on the Program.

  To "propagate" a work means to do anything with it that, without
permission, would make you directly or secondarily liable for
infringement under applicable copyright law, except executing it on a
computer or modifying a private copy.  Propagation includes copying,
distribution (with or without modification), making available to the
public, and in some countries other activities as well.

  To "convey" a work means any kind of propagation that enables other
parties to make or receive copies.  Mere interaction with a user through
a computer network, with no transfer of a copy, is not conveying.

  An interactive user interface displays "Appropriate Legal Notices"
to the extent that it includes a convenient and prominently visible
feature that (1) displays an appropriate copyright notice, and (2)
tells the user that there is no warranty for the work (except to the
extent that warranties are provided), that licensees may convey the
work under this License, and how to view a copy of this License.  If
the interface presents a list of user commands or options, such as a
menu, a prominent item in the list meets this criterion.

  1. Source Code.

  The "source code" for a work means the preferred form of the work
for making modifications to it.  "Object code" means any non-source
form of a work.

  A "Standard Interface" means an interface that either is an official
standard defined by a recognized standards body, or, in the case of
interfaces specified for a particular programming language, one that
is widely used among developers working in that language.

  The "System Libraries" of an executable work include anything, other
than the work as a whole, that (a) is included in the normal form of
packaging a Major Component, but which is not part of that Major
Component, and (b) serves only to enable use of the work with that
Major Component, or to implement a Standard Interface for which an
implementation is available to the public in source code form.  A
"Major Component", in this context, means a major essential component
(kernel, window system, and so on) of the specific operating system
(if any) on which the executable work runs, or a compiler used to
produce the work, or an object code interpreter used to run it.

  The "Corresponding Source" for a work in object code form means all
the source code needed to generate, install, and (for an executable
work) run the object code and to modify the work, including scripts to
control those activities.  However, it does not include the work's
System Libraries, or general-purpose tools or generally available free
programs which are used unmodified in performing those activities but
which are not part of the work.  For example, Corresponding Source
includes interface definition files associated with source files for
the work, and the source code for shared libraries and dynamically
linked subprograms that the work is specifically designed to require,
such as by intimate data communication or control flow between those
subprograms and other parts of the work.

  The Corresponding Source need not include anything that users
can regenerate automatically from other parts of the Corresponding
Source.

  The Corresponding Source for a work in source code form is that
same work.

  2. Basic Permissions.

  All rights granted under this License are granted for the term of
copyright on the Program, and are irrevocable provided the stated
conditions are met.  This License explicitly affirms your unlimited
permission to run the unmodified Program.  The output from running a
covered work is covered by this License only if the output, given its
content, constitutes a covered work.  This License acknowledges your
rights of fair use or other equivalent, as provided by copyright law.

  You may make, run and propagate covered works that you do not
convey, without conditions so long as your license otherwise remains
in force.  You may convey covered works to others for the sole purpose
of having them make modifications exclusively for you, or provide you
with facilities for running those works, provided that you comply with
the terms of this License in conveying all material for which you do
not control copyright.  Those thus making or running the covered works
for you must do so exclusively on your behalf, under your direction
and 

[... truncated at 8000 chars ...]
```

> The repo's licence rule allows MIT, Apache-2.0, GPL-3.0 (as a separate component) and OpenRAIL. Non-commercial, research-only and revenue-capped licences are refused. Read the text, not the SPDX tag alone.

## File tree (46 files, filtered)

- `AGENTS.md`
- `CHANGELOG.md`
- `LICENSE`
- `README.md`
- `RELEASING.md`
- `docs/superpowers/specs/2026-07-26-tag-release-reconciliation-design.md`
- `skills/hyperframes-motion-director/SKILL.md`
- `skills/hyperframes-motion-director/assets/banner.png`
- `skills/hyperframes-motion-director/assets/features.png`
- `skills/hyperframes-motion-director/evals/evals.json`
- `skills/hyperframes-motion-director/evals/trigger-prompts.md`
- `skills/hyperframes-motion-director/references/asset-choreography-and-render-qa.md`
- `skills/hyperframes-motion-director/references/audio-sync.md`
- `skills/hyperframes-motion-director/references/design-engineering.md`
- `skills/hyperframes-motion-director/references/gsap-choreography.md`
- `skills/hyperframes-motion-director/references/hyperframes-stability.md`
- `skills/hyperframes-motion-director/references/imagegen-asset-pipeline.md`
- `skills/hyperframes-motion-director/references/motion-background-system.md`
- `skills/hyperframes-motion-director/references/motion-craft.md`
- `skills/hyperframes-motion-director/references/premium-product-promo.md`
- `skills/hyperframes-motion-director/references/text-over-background-layout.md`
- `skills/hyperframes-motion-director/references/visual-standard.md`
- `skills/hyperframes-motion-director/references/workflow.md`
- `skills/hyperframes-motion-director/scripts/build_review_pack.mjs`
- `skills/hyperframes-motion-director/scripts/check-structure.mjs`
- `skills/hyperframes-motion-director/scripts/check_assets.mjs`
- `skills/hyperframes-motion-director/scripts/check_release.mjs`
- `skills/hyperframes-motion-director/scripts/create_project.mjs`
- `skills/hyperframes-motion-director/scripts/lib/image-assets.mjs`
- `skills/hyperframes-motion-director/scripts/tests/check_release.test.mjs`
- `skills/hyperframes-motion-director/scripts/tests/validate_image_assets.test.mjs`
- `skills/hyperframes-motion-director/scripts/validate_artifacts.mjs`
- `skills/hyperframes-motion-director/scripts/validate_design_engineering.mjs`
- `skills/hyperframes-motion-director/scripts/validate_image_assets.mjs`
- `skills/hyperframes-motion-director/templates/ASSET_MANIFEST.template.json`
- `skills/hyperframes-motion-director/templates/BEAT_MAP.template.json`
- `skills/hyperframes-motion-director/templates/BRIEF_DESIGN_PROPOSAL.template.md`
- `skills/hyperframes-motion-director/templates/DESIGN.template.md`
- `skills/hyperframes-motion-director/templates/MOTION_MAP.template.json`
- `skills/hyperframes-motion-director/templates/MOTION_PRIMITIVES.template.json`
- `skills/hyperframes-motion-director/templates/REVIEW_REPORT.template.md`
- `skills/hyperframes-motion-director/templates/SCENE_SCHEMA.template.json`
- `skills/hyperframes-motion-director/templates/STORYBOARD.template.md`
- `skills/hyperframes-motion-director/templates/VECTOR_TEMPLATES.template.json`
