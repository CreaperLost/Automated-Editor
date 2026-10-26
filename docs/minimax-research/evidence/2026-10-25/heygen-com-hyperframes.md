# heygen-com/hyperframes

- **Source:** https://github.com/heygen-com/hyperframes
- **Kind:** github
- **Captured:** 2026-10-25T15:56:45
- **Evidence grade:** vendor or creator claim. Not tested in this repo. Quotes from a caption are what a creator *said*, not a measurement.

## Repository

- **Repo:** heygen-com/hyperframes
- **Description:** Write HTML. Render video. Built for agents.
- **Homepage:** https://hyperframes.dev
- **Stars:** 71842
- **Forks:** 6492
- **Open issues:** 285
- **Licence (SPDX):** Apache-2.0
- **Language:** TypeScript
- **Default branch:** main
- **Last push:** 2026-10-25T13:53:21Z
- **Archived:** False
- **Topics:** ai, ai-agents, animation, claude-code, ffmpeg, framework, gsap, html, html-to-video, mcp, motion-graphics, puppeteer, rendering, typescript, video-editor

## README

```
<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="docs/logo/dark.svg">
    <source media="(prefers-color-scheme: light)" srcset="docs/logo/light.svg">
    <img alt="HyperFrames" src="docs/logo/light.svg" width="300">
  </picture>
</p>

<p align="center">
  <a href="https://www.npmjs.com/package/hyperframes"><img src="https://img.shields.io/npm/v/hyperframes.svg?style=flat" alt="npm version"></a>
  <a href="https://www.npmjs.com/package/hyperframes"><img src="https://img.shields.io/npm/dm/hyperframes.svg?style=flat" alt="npm downloads"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-Apache%202.0-blue.svg" alt="License"></a>
  <a href="https://nodejs.org"><img src="https://img.shields.io/badge/node-%3E%3D22-brightgreen" alt="Node.js"></a>
  <a href="https://discord.gg/EbK98HBPdk"><img src="https://img.shields.io/badge/Discord-Join-5865F2?logo=discord&logoColor=white" alt="Discord"></a>
</p>

<p align="center"><b>Write HTML. Render video. Built for agents.</b></p>

<p align="center">
  <a href="https://www.hyperframes.dev/studio">Studio</a> |
  <a href="https://hyperframes.heygen.com/quickstart">Quickstart</a> |
  <a href="https://hyperframes.heygen.com/showcase">Showcase</a> |
  <a href="https://www.hyperframes.dev/">Playground</a> |
  <a href="https://hyperframes.heygen.com/catalog/blocks/data-chart">Catalog</a> |
  <a href="https://hyperframes.heygen.com/introduction">Docs</a> |
  <a href="https://discord.gg/EbK98HBPdk">Discord</a>
</p>

<p align="center">
  <img src="https://github.com/user-attachments/assets/f6ff9fae-f33d-4f68-bd54-f3ed4ba6473b" alt="HyperFrames app opening from the Dock: native Mac and Linux app now available" width="800">
</p>

HyperFrames is an open-source framework for turning HTML, CSS, media, and seekable animations into deterministic MP4 videos. Use it locally with the CLI, from AI coding agents with skills, or as the rendering core behind hosted authoring workflows.

Using Claude Code, Codex, Cursor, or another coding agent? Run `npx hyperframes skills update` once, then ask your agent to make a video.

## Quick Start

### With an AI coding agent

For Claude Code, install the versioned plugin:

```bash
claude plugin marketplace add heygen-com/hyperframes
claude plugin install hyperframes@hyperframes
```

To enable auto-update for the **hyperframes** marketplace, run `claude` in a
terminal in your project. In Claude Code's terminal UI, run `/plugin`, then select
**Marketplaces** → **hyperframes** → **Enable auto-update**. Afterward, use
`/hyperframes:hyperframes`. See the [plugin guide](docs/guides/plugins.mdx)
for Copilot, VS Code, Cursor, Gemini CLI, updates, and migration.

For standalone skills (including OpenCode), use:

```bash
npx skills add heygen-com/hyperframes
```

> The picker opens with nothing pre-selected — the **Core Skills** group is all you need; `/slideshow`, `/remotion-to-hyperframes` and `/figma` install on demand. Agents and non-interactive runs should use `npx hyperframes skills update` instead — it installs exactly the core set, whereas `skills add --all` installs all 7 published skills. The six repo-internal skills under `.claude/skills` / `.agents/skills` are excluded by default. For the full published set use `npx hyperframes skills`.
>
> `skills add` resolves the skills.sh registry blob, which can lag `main` by hours. `npx hyperframes skills update` installs from the current `main`, so reach for it when you need the newest copy of a skill.

Run project-local installs from your video project's directory. From a HyperFrames
checkout, use `npx hyperframes skills update` for a global install instead: a
project-local installer target can overlap the tracked `skills/` source and
overwrite uncommitted edits. See [contributor setup](CONTRIBUTING.md#getting-started).

Try a prompt like:

> Using `/hyperframes`, create a 10-second product intro with a fade-in title, a background video, and subtle background music.

The skills teach agents the HyperFrames production loop: plan the video, write valid HTML, wire seekable animations, add media, lint, preview, and render. They work with Claude Code, Codex, Cursor, Gemini CLI, IBM Bob, and other coding agents that support skills.

## Skills

HyperFrames ships 7 skills. Make any video by asking for it; the `/hyperframes` skill carries the contract and the references (film, captions, overlays, motion, look). Read it first.

For standalone installation, default to the **core set** (`/hyperframes`, `/hyperframes-cli`, `/hyperframes-registry`, `/media-use`). `npx hyperframes skills update` installs exactly that from anywhere; the interactive picker (`npx skills add heygen-com/hyperframes`) lists it as the "Core Skills" group, nothing pre-selected. The picker is interactive-only — a non-interactive or agent run without `--skill` installs all 7. Use `npx skills add heygen-com/hyperframes --all` to install the complete published set deliberately (skips the picker), or `npx skills add heygen-com/hyperframes --skill <name>` for just one (bare name, no leading `/`).

Standalone installs stay lean after that: `npx hyperframes init` installs the **core set** only when it is missing and never touches a skill you already have (`npx hyperframes skills update` refreshes them, and never removes one); `/figma`, `/slideshow` and `/remotion-to-hyperframes` install **on demand** with `npx hyperframes skills update <name>`. Nothing re-pulls the full set behind your back.

### Plugin packages

Plugins bundle the full skill catalog and use their agent's update manager.
`bun run package:agent-plugin` builds the committed portable ZIP, source metadata,
and SHA-256 checksum. See the [maintainer checklist](docs/contributing/agent-plugins.mdx).

### Upload to Codex

Build the upload-ready Codex plugin archive from the committed `HEAD` version of the manifest, brand assets, and skills:

```bash
bun run package:codex-plugin
```

This writes `dist/hyperframes-plugin.zip` with a `hyperframes/` root folder and fails if the archive exceeds Codex's 100 MB upload limit.

### Core set

| Skill                   | Use when                                                                                                                                                                                                                                                                                            |
| ----------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `/hyperframes`          | **Read first** for any request to make, edit, animate or render a video, motion graphic, captions or overlays with HyperFrames (HTML rendered to video). Holds the render contract and the work loop, and points to the references for format, motion, look, captions, whole films and storyboards. |
| `/hyperframes-cli`      | The HyperFrames CLI by job — scaffold, check, look at frames, edit the timeline, preview, render locally or in the cloud, publish, project history — plus reading its JSON and recovering from build or render failures.                                                                            |
| `/hyperframes-registry` | About 450 ready-made blocks and components installed as editable source — transitions, overlays, titles, kinetic type, captions, charts, maps, 3D, camera moves. Use before hand-building any named visual; also for `hyperframes add` and `catalog`.                                               |
| `/media-use`            | Get or make any media for a video — music, sound effects, images, icons, logos, voiceover, avatar and generated video, transcripts, cutouts, colour grades and looks — with HeyGen or free local tools, and mix placed audio. Also for footage that looks dark, flat or dull.                       |

### On demand

| Skill                      | Use when                                                                                                                                                                                              |
| -------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `/figma`                   | Bring Figma into a video — rendered assets, brand tokens, components, storyboard frames as reconstructed motion, shaders and connector-assisted motion. Use when you paste a figma.com link.          |
| `/slideshow`               | A presentation, pitch deck or interactive deck — discrete slides, fragment reveals, branching, hotspot navigation, presenter mode with speaker notes. Output is a navigable deck, not a rendered MP4. |
| `/remotion-to-hyperframes` | Bringing existing video code or footage into HyperFrames (Remotion first; also Motion Canvas, Lottie, Three.js, canvas, other tools' renders), verified frame by frame.                               |

For visual design handoff workflows, see the [Claude Design guide](https://hyperframes.heygen.com/guides/claude-design) and [Open Design guide](https://hyperframes.heygen.com/guides/open-design).

### Manually with the CLI

```bash
npx hyperframes init my-video
cd my-video
npx hyperframes preview      # preview in browser with live reload
npx hyperframes render       # render to MP4
```

**Requirements:** Node.js 22+, FFmpeg

## What You Can Build

Need ideas? Browse the [Showcase](https://hyperframes.heygen.com/showcase) for finished videos you can watch, read, run, and remix.

- Product launch videos and feature announcements
- PR walkthroughs with animated code diffs, narration, and captions
- Data visualizations, chart races, and map animations
- Social videos with kinetic captions, overlays, and music
- Docs-to-video, PDF-to-video, and site-tour explainers
- Reusable motion graphics for automated content pipelines

## Frame.md

**frame.md — your design system, ready for video.**

Every brand has a `design.md`. None of them were written for a camera. `frame.md` is the missing translation layer: it takes your web-context design spec and inverts it for the frame — the same tokens, the same rules, but rewritten so an AI agent can compose a promo video without guessing at scale or reaching for web chrome.

The output is a `DESIGN.md` superset your whole toolchain can read. Atoms stay sacred. Composition stays free. Numbers come from the script.

<table>
  <tr>
    <td width="50%" align="center">
      <a href="https://www.hyperframes.dev/design/biennale-yellow"><img src="https://static.heygen.ai/hyperframes-oss/docs/images/design-templates/biennale-yellow.png" alt="Biennale Yellow" width="100%"></a>
      <br><b><a href="https://www.hyperframes.dev/design/biennale-yellow">Biennale Yellow</a></b>
    </td>
    <td width="50%" align="center">
      <a href="https://www.hyperframes.dev/design/blockframe"><img src="https://static.heygen.ai/hyperframes-oss/docs/images/design-templates/blockframe.png" alt="BlockFrame" width="100%"></a>
      <br><b><a href="https://www.hyperframes.dev/design/blockframe">BlockFrame</a></b>
    </td>
  </tr>
  <tr>
    <td width="50%" align="center">
      <a href="https://www.hyperframes.dev/design/blue-professional"><img src="https://static.heygen.ai/hyperframes-oss/docs/images/design-templates/blue-professional.png" alt="Blue Professional" width="100%"></a>
      <br><b><a href="https://www.hyperframes.dev/design/blue-professional">Blue Professional</a></b>
    </td>
    <td width="50%" align="center">
      <a href="https://www.hyperframes.dev/design/bold-poster"><img src="https://static.heygen.ai/hyperframes-oss/docs/images/design-templates/bold-poster.png" alt="Bold Poster" width="100%"></a>
      <br><b><a href="https://www.hyperframes.dev/design/bold-poster">Bold Poster</a></b>
    </td>
  </tr>
  <tr>
    <td width="50%" align="center">
      <a href="https://www.hyperframes.dev/design/broadside"><img src="https://static.heygen.ai/hyperframes-oss/docs/images/design-templates/broadside.png" alt="Broadside" width="100%"></a>
      <br><b><a href="https://www.hyperframes.dev/design/broadside">Broadside</a></b>
    </td>
    <td width="50%" align="center">
      <a href="https://www.hyperframes.dev/design/capsule"><img src="https://static.heygen.ai/hyperframes-oss/docs/images/design-templates/capsule.png" alt="Capsule" width="100%"></a>
      <br><b><a href="https://www.hyperframes.dev/design/capsule">Capsule</a></b>
    </td>
  </tr>
  <tr>
    <td width="50%" align="center">
      <a href="https://www.hyperframes.dev/design/cartesian"><img src="https://static.heygen.ai/hyperframes-oss/docs/images/design-templates/cartesian.png" alt="Cartesian" width="100%"></a>
      <br><b><a href="https://www.hyperframes.dev/design/cartesian">Cartesian</a></b>
    </td>
    <td width="50%" align="center">
      <a href="https://www.hyperframes.dev/design/cobalt-grid"><img src="https://static.heygen.ai/hyperframes-oss/docs/images/design-templates/cobalt-grid.png" alt="Cobalt Grid" width="100%"></a>
      <br><b><a href="https://www.hyperframes.dev/design/cobalt-grid">Cobalt Grid</a></b>
    </td>
  </tr>
  <tr>
    <td width="50%" align="center">
      <a href="https://www.hyperframes.dev/design/coral"><img src="https://static.heygen.ai/hyperframes-oss/docs/images/design-templates/coral.png" alt="Coral" width="100%"></a>
      <br><b><a href="https://www.hyperframes.dev/design/coral">Coral</a></b>
    </td>
    <td width="50%" align="center">
      <a href="https://www.hyperframes.dev/design/creative-mode"><img src="https://static.heygen.ai/hyperframes-oss/docs/images/design-templates/creative-mode.png" alt="Creative Mode" width="100%"></a>
      <br><b><a href="https://www.hyperframes.dev/design/creative-mode">Creative Mode</a></b>
    </td>
  </tr>
</table>

Browse and remix them all at [hyperframes.dev/design](https://www.hyperframes.dev/design).

## How It Works

Define a video as HTML. Add data attributes for timing and tracks. Use GSAP, CSS, Lottie, Three.js, Anime.js, WAAPI, or your own frame adapter for seekable animation.

```html
<div id="stage" data-composition-id="launch" data-start="0" data-width="1920" data-height="1080">
  <video
    class="clip"
    data-start="0"
    data-duration="6"
    data-track-index="0"
    src="intro.mp4"
    muted
    playsinline
  ></video>

  <h1 id="title" class="clip" data-start="1" data-duration="4" data-track-index="1">Launch day</h1>

  <audio
    data-start="0"
    data-duration="6"
    data-track-index="2"
    data-volume="0.5"
    src="music.wav"
  ></audio>

  <script src="https://cdn.jsdelivr.net/npm/gsap@3/dist/gsap.min.js"></script>
  <script>
    const tl = gsap.timeline({ paused: true });
    tl.from("#title", { opacity: 0, y: 40, duration: 0.8 }, 1);
    window.__timelines = window.__timelines || {};
    window.__timelines.launch = tl;
  </script>
</div>
```

Preview instantly in the browser. Render locally or in Docker. The renderer seeks each frame in headless Chrome and encodes the result with FFmpeg, so the same input produces the same video.

## HyperFrames Stack

HyperFrames is the open-source rendering engine, plus a growing set of tools around HTML-native video creation.

| Piece                                           | Status              | What it does                                                                                      |
| ----------------------------------------------- | ------------------- | ------------------------------------------------------------------------------------------------- |
| CLI                                             | Available           | Scaffold, preview, lint, inspect, and render local video projects                                 |
| Core / Engine / Producer                        | Available           | Parse compositions, drive headless Chrome, encode video, and mix audio                            |
| Catalog                                         | Available           | Reusable blocks and components for transitions, overlays, captions, charts, maps, and effects     |
| Agent skills                                    | Available           | Teach coding agents the video-production patterns that generic web docs miss                      |
| Studio                                          | Available, evolving | Browser surface for previewing and editing compositions                                           |
| AWS Lambda rendering                            | Available           | Deploy a distributed render stack and drive renders from your laptop or CI                        |
| [hyperframes.dev](https://www.hyperframes.dev/) | Available           | Community playground for previewing, iterating, sharing, and rendering HTML-native video projects |
| [frame.md](https://www.hyperframes.dev/design)  | Available           | Invert your design system for the camera — a DESIGN.md superset an agent can compose video from   |

## Catalog

Install ready-to-use blocks and components:

```bash
npx hyperframes add flash-through-white   # shader transition
npx hyperframes add instagram-follow      # social overlay
npx hyperframes add data-chart            # animated chart
```

Browse the catalog at [hyperframes.heygen.com/catalog](https://hyperframes.heygen.com/catalog/blocks/data-chart).

## Why HyperFrames?

- **HTML-native:** compositions are HTML files with data attributes. No React requirement, no proprietary timeline format.
- **Agent-friendly:** agents already write HTML, and the CLI is non-interactive by default.
- **Deterministic:** same input, same frames, same output. Built for CI, regression tests, and automated rendering.
- **No build step:** an `index.html` composition plays as-is and can be previewed directly in the browser.
- **Adapter-based animation:** bring GSAP, CSS animations, Lottie, Three.js, Anime.js, WAAPI, or a custom runtime.
- **Open source:** Apache 2.0 license, with no per-render fees or commercial-use thresholds.

## HyperFrames vs Remotion

HyperFrames is inspired by [Remotion](https://www.remotion.dev). Both tools render video with headless Chrome and FFmpeg. The main difference is the authoring model: Remotion's bet is React components; HyperFrames' bet is plain HTML that humans and agents can both write easily.

|                          | **HyperFrames**                       | **Remotion**                            |
| ------------------------ | ------------------------------------- | --------------------------------------- |
| Authoring                | HTML + CSS + seekable animation       | React components                        |
| Build step               | None; `index.html` plays as-is        | Bundler required                        |
| Agent handoff            | Plain HTML files                      | JSX / React project                     |
| Library-clock animations | Seekable, frame-accurate via adapters | Wall-clock animation patterns need care |
| Distributed rendering    | Local and AWS Lambda render paths     | Remotion Lambda, mature cloud renderer  |
| License                  | Apache 2.0                            | Source-available Remotion License       |

Read the full comparison in the [HyperFrames vs Remotion guide](https://hyperframes.heygen.com/guides/hyperframes-vs-remotion).

## Documentation

Full documentation: [hyperframes.heygen.com/introduction](https://hyperframes.heygen.com/introduction)

- [Quickstart](https://hyperframes.heygen.com/quickstart)
- [Showcase](https://hyperframes.heygen.com/showcase)
- [Guides](https://hyperframes.heygen.com/guides/gsap-animation)
- [API Reference](https://hyperframes.heygen.com/packages/core)
- [Catalog](https://hyperframes.heygen.com/catalog/blocks/data-chart)
- [Examples](https://hyperframes.heygen.com/examples)
- [AWS Lambda rendering](https://hyperframes.heygen.com/deploy/aws-lambda)

## Packages

| Package                                                          | Description                                                       |
| ---------------------------------------------------------------- | ----------------------------------------------------------------- |
| [`hyperframes`](packages/cli)                                    | CLI for creating, previewing, linting, and rendering compositions |
| [`@hyperframes/core`](packages/core)                             | Types, parsers, generators, linter, runtime, and frame adapters   |
| [`@hyperframes/engine`](packages/engine)                         | Seekable page-to-video capture engine using Puppeteer and FFmpeg  |
| [`@hyperframes/producer`](packages/producer)                     | Full rendering pipeline for capture, encode, and audio mix        |
| [`@hyperframes/studio`](packages/studio)                         | Browser-based composition editor UI                               |
| [`@hyperframes/player`](packages/player)                         | Embeddable `<hyperframes-player>` web component                   |
| [`@hyperframes/shader-transitions`](packages/shader-transitions) | WebGL shader transitions for compositions                         |
| [`@hyperframes/aws-lambda`](packages/aws-lambda)                 | AWS Lambda SDK and deployment surface for distributed renders     |

## Community

HyperFrames is used in production at [HeyGen](https://www.heygen.com), with community examples from teams like [tldraw](https://tldraw.com), [TanStack](https://tanstack.com), and others in [ADOPTERS.md](ADOPTERS.md). Open a PR if your team is using HyperFrames.

- Questions and ideas: [Discord](https://discord.gg/EbK98HBPdk)
- Bugs and feature requests: [GitHub Issues](https://github.com/heygen-com/hyperframes/issues)
- User research: [Book a casual 30-minute conversation with the HyperFrames team](https://calendar.google.com/calendar/u/0/appointments/schedules/AcZssZ2cSpKoDgmcmRrgekrnrgqmvPT8W6F2Zg6e7MY7IJqaZKwpn_I0NdTHkN390iguMepE_NVg8ezb?gv=true) — no preparation or sales pitch
- Security reports: [SECURITY.md](SECURITY.md)
- Contributions: [CONTRIBUTING.md](CONTRIBUTING.md)

## Development Note

The repo uses [Git LFS](https://git-lfs.com) for golden regression-test baselines under `packages/producer/tests/**/output.mp4` (about 240 MB of `.mp4` files). If you're cloning the full repo for development, install Git LFS first:

```bash
# macOS
brew install git-lfs

# Ubuntu / Debian
sudo apt install git-lfs

# Windows
winget install GitHub.GitLFS

# Then, once per machine
git lfs install
```

If you only need source files, you can skip LFS content:

```bash
GIT_LFS_SKIP_SMUDGE=1 git clone https://github.com/heygen-com/hyperframes.git
```

## License

[Apache 2.0](LICENSE)
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
      submitted to the Licensor for inclusion in the Work by the copyright owner
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
      on behalf of whom a Contribution has been received by the Licensor and
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
          within such NOTICE file, excluding any notices that do not
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
      origin of the Work and reproducing the content of the NOTICE fil

[... truncated at 8000 chars ...]
```

> The repo's licence rule allows MIT, Apache-2.0, GPL-3.0 (as a separate component) and OpenRAIL. Non-commercial, research-only and revenue-capped licences are refused. Read the text, not the SPDX tag alone.

## File tree (9189 files, filtered)

- `.agents/skills/README.md`
- `.agents/skills/captions-overlay/SKILL.md`
- `.agents/skills/changelog-video/SKILL.md`
- `.agents/skills/changelog-video/assets/bg-pattern.mp4`
- `.agents/skills/changelog-video/assets/bgm.mp3`
- `.agents/skills/changelog-video/assets/fonts/ABCSolarDisplay-Bold.woff2`
- `.agents/skills/changelog-video/assets/fonts/TT_Norms_Pro_Bold.woff2`
- `.agents/skills/changelog-video/assets/fonts/TT_Norms_Pro_Medium.woff2`
- `.agents/skills/changelog-video/assets/fonts/TT_Norms_Pro_Normal.woff2`
- `.agents/skills/changelog-video/assets/fonts/tt_norms_pro_mono_regular-webfont.woff2`
- `.agents/skills/changelog-video/examples/master-skeleton.html`
- `.agents/skills/changelog-video/examples/script-tokens.json`
- `.agents/skills/changelog-video/references/build-spec.md`
- `.agents/skills/changelog-video/references/lexicon.json`
- `.agents/skills/changelog-video/references/script-voice.md`
- `.agents/skills/changelog-video/references/visualization-registry.md`
- `.agents/skills/changelog-video/scripts/align-captions.mjs`
- `.agents/skills/cut-the-curve/SKILL.md`
- `.agents/skills/cut-the-curve/examples/gsap-implementation.md`
- `.agents/skills/motion-doctrine/SKILL.md`
- `.agents/skills/motion-doctrine/references/seam-gate.md`
- `.agents/skills/motion-doctrine/scripts/seam-gate.mjs`
- `.agents/skills/motion-doctrine/scripts/seam-stamp.mjs`
- `.agents/skills/oversized-cursor/SKILL.md`
- `.agents/skills/seam-craft/SKILL.md`
- `.claude/skills/README.md`
- `.claude/skills/captions-overlay/SKILL.md`
- `.claude/skills/changelog-video/SKILL.md`
- `.claude/skills/changelog-video/assets/bg-pattern.mp4`
- `.claude/skills/changelog-video/assets/bgm.mp3`
- `.claude/skills/changelog-video/assets/fonts/ABCSolarDisplay-Bold.woff2`
- `.claude/skills/changelog-video/assets/fonts/TT_Norms_Pro_Bold.woff2`
- `.claude/skills/changelog-video/assets/fonts/TT_Norms_Pro_Medium.woff2`
- `.claude/skills/changelog-video/assets/fonts/TT_Norms_Pro_Normal.woff2`
- `.claude/skills/changelog-video/assets/fonts/tt_norms_pro_mono_regular-webfont.woff2`
- `.claude/skills/changelog-video/examples/master-skeleton.html`
- `.claude/skills/changelog-video/examples/script-tokens.json`
- `.claude/skills/changelog-video/references/build-spec.md`
- `.claude/skills/changelog-video/references/lexicon.json`
- `.claude/skills/changelog-video/references/script-voice.md`
- `.claude/skills/changelog-video/references/visualization-registry.md`
- `.claude/skills/changelog-video/scripts/align-captions.mjs`
- `.claude/skills/cut-the-curve/SKILL.md`
- `.claude/skills/cut-the-curve/examples/gsap-implementation.md`
- `.claude/skills/motion-doctrine/SKILL.md`
- `.claude/skills/motion-doctrine/references/seam-gate.md`
- `.claude/skills/motion-doctrine/scripts/seam-gate.mjs`
- `.claude/skills/motion-doctrine/scripts/seam-stamp.mjs`
- `.claude/skills/oversized-cursor/SKILL.md`
- `.claude/skills/seam-craft/SKILL.md`
- `.github/ISSUE_TEMPLATE/config.yml`
- `.github/codeql/codeql-config.yml`
- `.github/pull_request_template.md`
- `.github/workflows/fixtures/windows-canary.html`
- `ADOPTERS.md`
- `AGENTS.md`
- `CLAUDE.md`
- `CODE_OF_CONDUCT.md`
- `CONTRIBUTING.md`
- `CREDITS.md`
- `DESIGN.md`
- `DOCS_GUIDELINES.md`
- `LICENSE`
- `README.md`
- `SECURITY.md`
- `TRIAGE.md`
- `commitlint.config.js`
- `docs/AGENTS.md`
- `docs/contracts/2026-10-07-composition-probe.html`
- `docs/contracts/2026-10-07-document-idle-readiness.html`
- `docs/guides/claude-design-hyperframes.md`
- `docs/guides/claude-design-send-to-hyperframes.md`
- `docs/guides/frame-sources.md`
- `docs/guides/imported-film-html.md`
- `docs/guides/open-design-hyperframes.md`
- `docs/schema/hyperframes.json`
- `docs/schema/registry-item.json`
- `docs/schema/registry.json`
- `docs/snippets/advanced-path-grid.jsx`
- `docs/snippets/catalog-detail.jsx`
- `docs/snippets/catalog-gallery.jsx`
- `docs/snippets/docs-video.jsx`
- `docs/snippets/example-player.jsx`
- `docs/snippets/hover-video.jsx`
- `docs/snippets/install-command.jsx`
- `docs/snippets/quickstart-continuation-grid.jsx`
- `docs/snippets/replica-compare.jsx`
- `docs/snippets/workflow-chooser.jsx`
- `docs/template-gallery.js`
- `examples/aws-lambda/README.md`
- `examples/gcp-cloud-run/README.md`
- `examples/k8s-jobs/README.md`
- `knip.config.ts`
- `packages/aws-lambda/README.md`
- `packages/aws-lambda/scripts/_formatBytes.ts`
- `packages/aws-lambda/scripts/_handlerBanner.ts`
- `packages/aws-lambda/scripts/build-zip.test.ts`
- `packages/aws-lambda/scripts/build-zip.ts`
- `packages/aws-lambda/scripts/pin-handler-zip.test.ts`
- `packages/aws-lambda/scripts/pin-handler-zip.ts`
- `packages/aws-lambda/scripts/probe-beginframe.test.ts`
- `packages/aws-lambda/scripts/probe-beginframe.ts`
- `packages/aws-lambda/scripts/probe-webgpu-adapter.ts`
- `packages/aws-lambda/scripts/stage-arm64-image.test.ts`
- `packages/aws-lambda/scripts/stage-arm64-image.ts`
- `packages/aws-lambda/scripts/verify-zip-size.ts`
- `packages/aws-lambda/src/cdk/HyperframesRenderStack.contract.test.ts`
- `packages/aws-lambda/src/cdk/HyperframesRenderStack.snapshot.test.ts`
- `packages/aws-lambda/src/cdk/HyperframesRenderStack.ts`
- `packages/aws-lambda/src/cdk/index.ts`
- `packages/aws-lambda/src/chromium.test.ts`
- `packages/aws-lambda/src/chromium.ts`
- `packages/aws-lambda/src/contentStore.ts`
- `packages/aws-lambda/src/events.ts`
- `packages/aws-lambda/src/formatExtension.ts`
- `packages/aws-lambda/src/handler.test.ts`
- `packages/aws-lambda/src/handler.ts`
- `packages/aws-lambda/src/index.ts`
- `packages/aws-lambda/src/s3PlanV2Publisher.test.ts`
- `packages/aws-lambda/src/s3PlanV2Publisher.ts`
- `packages/aws-lambda/src/s3Stitch.test.ts`
- `packages/aws-lambda/src/s3Stitch.ts`
- `packages/aws-lambda/src/s3Transport.test.ts`
- `packages/aws-lambda/src/s3Transport.ts`
- `packages/aws-lambda/src/sdk/__fixtures__/fakeS3.ts`
- `packages/aws-lambda/src/sdk/costAccounting.test.ts`
- `packages/aws-lambda/src/sdk/costAccounting.ts`
- `packages/aws-lambda/src/sdk/deploySite.test.ts`
- `packages/aws-lambda/src/sdk/deploySite.ts`
- `packages/aws-lambda/src/sdk/getRenderProgress.test.ts`
- `packages/aws-lambda/src/sdk/getRenderProgress.ts`
- `packages/aws-lambda/src/sdk/index.ts`
- `packages/aws-lambda/src/sdk/prebuiltHandler.ts`
- `packages/aws-lambda/src/sdk/pushImageToEcr.test.ts`
- `packages/aws-lambda/src/sdk/pushImageToEcr.ts`
- `packages/aws-lambda/src/sdk/renderToLambda.test.ts`
- `packages/aws-lambda/src/sdk/renderToLambda.ts`
- `packages/aws-lambda/src/sdk/validateConfig.test.ts`
- `packages/aws-lambda/src/sdk/validateConfig.ts`
- `packages/aws-lambda/src/siteManifest.test.ts`
- `packages/aws-lambda/src/siteManifest.ts`
- `packages/aws-lambda/tsconfig.build.json`
- `packages/aws-lambda/tsconfig.json`
- `packages/cli/README.md`
- `packages/cli/scripts/build-runtime.ts`
- `packages/cli/scripts/gen-skills-manifest.ts`
- `packages/cli/scripts/sync-agent-dirs.ts`
- `packages/cli/src/api.test.ts`
- `packages/cli/src/api.ts`
- `packages/cli/src/api/removeBackground.ts`
- `packages/cli/src/api/transcribe.ts`
- `packages/cli/src/audio/analysis/analyze.test.ts`
- `packages/cli/src/audio/analysis/analyze.ts`
- `packages/cli/src/audio/analysis/decode.ts`
- `packages/cli/src/audio/analysis/fileTime.ts`
- `packages/cli/src/audio/analysis/format.test.ts`
- `packages/cli/src/audio/analysis/format.ts`
- `packages/cli/src/audio/analysis/providers/api.test.ts`
- `packages/cli/src/audio/analysis/providers/api.ts`
- `packages/cli/src/audio/analysis/providers/apple.ts`
- `packages/cli/src/audio/analysis/providers/beatThis.ts`
- `packages/cli/src/audio/analysis/providers/localProviders.test.ts`
- `packages/cli/src/audio/analysis/providers/shared.ts`
- `packages/cli/src/audio/analysis/run.test.ts`
- `packages/cli/src/audio/analysis/run.ts`
- `packages/cli/src/audio/analysis/targets.test.ts`
- `packages/cli/src/audio/analysis/targets.ts`
- `packages/cli/src/audio/providers.test.ts`
- `packages/cli/src/audio/providers.ts`
- `packages/cli/src/audio/read/effects.test.ts`
- `packages/cli/src/audio/read/effects.ts`
- `packages/cli/src/audio/read/format.ts`
- `packages/cli/src/audio/read/music.test.ts`
- `packages/cli/src/audio/read/music.ts`
- `packages/cli/src/audio/read/pitch.ts`
- `packages/cli/src/audio/read/read.test.ts`
- `packages/cli/src/audio/read/read.ts`
- `packages/cli/src/audio/read/signal.test.ts`
- `packages/cli/src/audio/read/signal.ts`
- `packages/cli/src/audio/read/stems.ts`
- `packages/cli/src/audio/read/types.ts`
- `packages/cli/src/audio/read/voice.test.ts`
- `packages/cli/src/audio/read/voice.ts`
- `packages/cli/src/auth/_test-utils.ts`
- `packages/cli/src/auth/browser.ts`
- `packages/cli/src/auth/client.test.ts`
- `packages/cli/src/auth/client.ts`
- `packages/cli/src/auth/errors.test.ts`
- `packages/cli/src/auth/errors.ts`
- `packages/cli/src/auth/index.ts`
- `packages/cli/src/auth/loopback.test.ts`
- `packages/cli/src/auth/loopback.ts`
- `packages/cli/src/auth/oauth.test.ts`
- `packages/cli/src/auth/oauth.ts`
- `packages/cli/src/auth/paths.test.ts`
- `packages/cli/src/auth/paths.ts`
- `packages/cli/src/auth/pkce.test.ts`
- `packages/cli/src/auth/pkce.ts`
- `packages/cli/src/auth/resolver.test.ts`
- `packages/cli/src/auth/resolver.ts`
- `packages/cli/src/auth/scrub.test.ts`
- `packages/cli/src/auth/scrub.ts`
- `packages/cli/src/auth/store.test.ts`
- `packages/cli/src/auth/store.ts`
- `packages/cli/src/auth/user.test.ts`
- `packages/cli/src/auth/user.ts`
- `packages/cli/src/background-removal/dependency-compat.test.ts`
- `packages/cli/src/background-removal/inference.test.ts`
- `packages/cli/src/background-removal/inference.ts`
- `packages/cli/src/background-removal/manager.test.ts`
- `packages/cli/src/background-removal/manager.ts`
- `packages/cli/src/background-removal/pipeline.rotation.test.ts`
- `packages/cli/src/background-removal/pipeline.test.ts`
- `packages/cli/src/background-removal/pipeline.ts`
- `packages/cli/src/background-removal/pipeline.windowsHide.test.ts`
- `packages/cli/src/backgroundChecksWorker.test.ts`
- `packages/cli/src/backgroundChecksWorker.ts`
- `packages/cli/src/browser/chromeOrphans.fixture.ts`
- `packages/cli/src/browser/chromeOrphans.process.test.ts`
- `packages/cli/src/browser/colorGradingShaderStages.browser.test.ts`
- `packages/cli/src/browser/downloadStall.fixture.ts`
- `packages/cli/src/browser/downloadStall.process.test.ts`
- `packages/cli/src/browser/ffmpeg.test.ts`
- `packages/cli/src/browser/ffmpeg.ts`
- `packages/cli/src/browser/gpuPolicy.test.ts`
- `packages/cli/src/browser/gpuPolicy.ts`
- `packages/cli/src/browser/installFacts.test.ts`
- `packages/cli/src/browser/installFacts.ts`
- `packages/cli/src/browser/installInChild.test.ts`
- `packages/cli/src/browser/installInChild.ts`
- `packages/cli/src/browser/installProtocol.ts`
- `packages/cli/src/browser/installRun.ts`
- `packages/cli/src/browser/launch.fixture.ts`
- `packages/cli/src/browser/launch.noListen.test.ts`
- `packages/cli/src/browser/launch.process.test.ts`
- `packages/cli/src/browser/launch.test.ts`
- `packages/cli/src/browser/launch.ts`
- `packages/cli/src/browser/linuxDeps.test.ts`
- `packages/cli/src/browser/linuxDeps.ts`
- `packages/cli/src/browser/macosOldChromeCrash.test.ts`
- `packages/cli/src/browser/macosOldChromeCrash.ts`
- `packages/cli/src/browser/manager.test.ts`
- `packages/cli/src/browser/manager.ts`
- `packages/cli/src/browser/preflight.test.ts`
- `packages/cli/src/browser/preflight.ts`
- `packages/cli/src/browser/sandboxDenied.test.ts`
- `packages/cli/src/browser/sandboxDenied.ts`
- `packages/cli/src/browser/unzipArchive.ts`
- `packages/cli/src/browser/unzipFallback.test.ts`
- `packages/cli/src/browser/windowsChromeWatchdog.ts`
- `packages/cli/src/browser/windowsCrash.test.ts`
- `packages/cli/src/browser/windowsCrash.ts`
- `packages/cli/src/capture/agentPromptGenerator.test.ts`
- `packages/cli/src/capture/agentPromptGenerator.ts`
- `packages/cli/src/capture/animationCataloger.degrade.test.ts`
- `packages/cli/src/capture/animationCataloger.ts`
- `packages/cli/src/capture/assetCataloger.test.ts`
- `packages/cli/src/capture/assetCataloger.ts`
- `packages/cli/src/capture/assetDownloader.test.ts`
- `packages/cli/src/capture/assetDownloader.ts`
- `packages/cli/src/capture/browserLaunchArgs.test.ts`
- `packages/cli/src/capture/browserLaunchArgs.ts`
- `packages/cli/src/capture/captureAttempt.test.ts`
- `packages/cli/src/capture/captureAttempt.ts`
- `packages/cli/src/capture/captureCompositionFrame.browserFallback.test.ts`
- `packages/cli/src/capture/captureCompositionFrame.lateImage.browser.test.ts`
- `packages/cli/src/capture/captureCompositionFrame.lateSubresources.browser.test.ts`
- `packages/cli/src/capture/captureCompositionFrame.prelaunch.browser.test.ts`
- `packages/cli/src/capture/captureCompositionFrame.slowScript.browser.test.ts`
- `packages/cli/src/capture/captureCompositionFrame.test.ts`
- `packages/cli/src/capture/captureCompositionFrame.ts`
- `packages/cli/src/capture/captureCompositionFrame.webglDraws.browser.test.ts`
- `packages/cli/src/capture/captureCompositionFrame.windowsHide.test.ts`
- `packages/cli/src/capture/captureErrors.ts`
- `packages/cli/src/capture/captureFile.test.ts`
- `packages/cli/src/capture/captureFile.ts`
- `packages/cli/src/capture/captureFontValidation.test.ts`
- `packages/cli/src/capture/captureFontValidation.ts`
- `packages/cli/src/capture/captureImageValidation.test.ts`
- `packages/cli/src/capture/captureImageValidation.ts`
- `packages/cli/src/capture/captureProxy.browser.test.ts`
- `packages/cli/src/capture/captureProxy.test.ts`
- `packages/cli/src/capture/captureProxy.ts`
- `packages/cli/src/capture/captureTimeout.test.ts`
- `packages/cli/src/capture/captureTimeout.ts`
- `packages/cli/src/capture/captureWatchdog.test.ts`
- `packages/cli/src/capture/captureWatchdog.ts`
- `packages/cli/src/capture/colorGrading.chromium.test.ts`
- `packages/cli/src/capture/contactSheet.test.ts`
- `packages/cli/src/capture/contactSheet.ts`
- `packages/cli/src/capture/contentExtractor.file-race.test.ts`
- `packages/cli/src/capture/contentExtractor.test.ts`
- `packages/cli/src/capture/contentExtractor.ts`
- `packages/cli/src/capture/coreExtractionPhase.test.ts`
- `packages/cli/src/capture/coreExtractionPhase.ts`
- `packages/cli/src/capture/designStyleExtractor.buttons.test.ts`
- `packages/cli/src/capture/designStyleExtractor.shadows.test.ts`
- `packages/cli/src/capture/designStyleExtractor.ts`
- `packages/cli/src/capture/faviconRanker.test.ts`
- `packages/cli/src/capture/faviconRanker.ts`
- `packages/cli/src/capture/filterExtractedScripts.test.ts`
- `packages/cli/src/capture/filterExtractedScripts.ts`
- `packages/cli/src/capture/fontMetadataExtractor.test.ts`
- `packages/cli/src/capture/fontMetadataExtractor.ts`
- `packages/cli/src/capture/htmlExtractor.test.ts`
- `packages/cli/src/capture/htmlExtractor.ts`
- `packages/cli/src/capture/iconClassifier.test.ts`
- `packages/cli/src/capture/iconClassifier.ts`
- `packages/cli/src/capture/index.ts`
- `packages/cli/src/capture/index.watchdog.test.ts`
- `packages/cli/src/capture/launchCaptureBrowser.signal.test.ts`
- `packages/cli/src/capture/launchCaptureBrowser.test.ts`
- `packages/cli/src/capture/launchCaptureBrowser.ts`
- `packages/cli/src/capture/lazyScrollForCapture.test.ts`
- `packages/cli/src/capture/lazyScrollForCapture.ts`
- `packages/cli/src/capture/lottieArchiveEntryLimit.test.ts`
- `packages/cli/src/capture/lottieArchivePreflight.ts`
- `packages/cli/src/capture/lottieDiscovery.test.ts`
- `packages/cli/src/capture/lottieDiscovery.ts`
- `packages/cli/src/capture/lottiePreviewRequests.test.ts`
- `packages/cli/src/capture/lottiePreviewRequests.ts`
- `packages/cli/src/capture/lottieValidation.test.ts`
- `packages/cli/src/capture/lottieValidation.ts`
- `packages/cli/src/capture/mediaCapture.test.ts`
- `packages/cli/src/capture/mediaCapture.ts`
- `packages/cli/src/capture/navigateForCapture.browser.test.ts`
- `packages/cli/src/capture/navigateForCapture.test.ts`
- `packages/cli/src/capture/navigateForCapture.ts`
- `packages/cli/src/capture/navigationPhase.ts`
- `packages/cli/src/capture/pageBlockDetection.test.ts`
- `packages/cli/src/capture/pageBlockDetection.ts`
- `packages/cli/src/capture/partialCapture.test.ts`
- `packages/cli/src/capture/partialCapture.ts`
- `packages/cli/src/capture/postExtractionPhase.test.ts`
- `packages/cli/src/capture/postExtractionPhase.ts`
- `packages/cli/src/capture/publicNetwork.fixture.ts`
- `packages/cli/src/capture/publicNetwork.integration.test.ts`
- `packages/cli/src/capture/publicNetwork.process.test.ts`
- `packages/cli/src/capture/publicNetwork.test.ts`
- `packages/cli/src/capture/publicNetwork.ts`
- `packages/cli/src/capture/readBoundedResponse.test.ts`
- `packages/cli/src/capture/readBoundedResponse.ts`
- `packages/cli/src/capture/responseRecord.test.ts`
- `packages/cli/src/capture/responseRecord.ts`
- `packages/cli/src/capture/scaffolding.test.ts`
- `packages/cli/src/capture/scaffolding.ts`
- `packages/cli/src/capture/screenshotCapture.degrade.test.ts`
- `packages/cli/src/capture/screenshotCapture.priority.test.ts`
- `packages/cli/src/capture/screenshotCapture.test.ts`
- `packages/cli/src/capture/screenshotCapture.ts`
- `packages/cli/src/capture/tokenExtractor.test.ts`
- `packages/cli/src/capture/tokenExtractor.ts`
- `packages/cli/src/capture/types.ts`
- `packages/cli/src/capture/userAgent.ts`
- `packages/cli/src/cli.commands.test.ts`
- `packages/cli/src/cli.dotEnv.test.ts`
- `packages/cli/src/cli.lifecycle.test.ts`
- `packages/cli/src/cli.ts`
- `packages/cli/src/cli.workspaceBuildGate.test.ts`
- `packages/cli/src/cliDispatchTestUtils.ts`
- `packages/cli/src/cloud/_gen/client.ts`
- `packages/cli/src/cloud/_gen/types.ts`
- `packages/cli/src/cloud/ansi.test.ts`
- `packages/cli/src/cloud/ansi.ts`
- `packages/cli/src/cloud/auth.ts`
- `packages/cli/src/cloud/detectAspectRatio.test.ts`
- `packages/cli/src/cloud/detectAspectRatio.ts`
- `packages/cli/src/cloud/download.test.ts`
- `packages/cli/src/cloud/download.ts`
- `packages/cli/src/cloud/errors.test.ts`
- `packages/cli/src/cloud/errors.ts`
- `packages/cli/src/cloud/generatedClient.contract.test.ts`
- `packages/cli/src/cloud/index.test.ts`
- `packages/cli/src/cloud/index.ts`
- `packages/cli/src/cloud/offload.test.ts`
- `packages/cli/src/cloud/offload.ts`
- `packages/cli/src/cloud/offloadTrim.ts`
- `packages/cli/src/cloud/parsing.test.ts`
- `packages/cli/src/cloud/parsing.ts`
- `packages/cli/src/cloud/poll.test.ts`
- `packages/cli/src/cloud/poll.ts`
- `packages/cli/src/cloud/statusColor.ts`
- `packages/cli/src/cloud/upload.test.ts`
- `packages/cli/src/cloud/upload.ts`
- `packages/cli/src/commands/_examples.ts`
- `packages/cli/src/commands/add.test.ts`
- `packages/cli/src/commands/add.ts`
- `packages/cli/src/commands/audio.ts`
- `packages/cli/src/commands/auth.ts`
- `packages/cli/src/commands/auth/login.test.ts`
- `packages/cli/src/commands/auth/login.ts`
- `packages/cli/src/commands/auth/logout.test.ts`
- `packages/cli/src/commands/auth/logout.ts`
- `packages/cli/src/commands/auth/refresh.ts`
- `packages/cli/src/commands/auth/status-guidance.ts`
- `packages/cli/src/commands/auth/status-user.test.ts`
- `packages/cli/src/commands/auth/status.test.ts`
- `packages/cli/src/commands/auth/status.ts`
- `packages/cli/src/commands/batchRender.test.ts`
- `packages/cli/src/commands/batchRender.ts`
