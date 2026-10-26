# Vincentwei1021/video-shotcraft

- **Source:** https://github.com/Vincentwei1021/video-shotcraft
- **Kind:** github
- **Captured:** 2026-10-25T15:07:51
- **Evidence grade:** vendor or creator claim. Not tested in this repo. Quotes from a caption are what a creator *said*, not a measurement.

## Repository

- **Repo:** Vincentwei1021/video-shotcraft
- **Description:** AI video skill for Claude Code & Codex — cinematic product videos with Remotion: 152 shot recipe cards, 209 motion previews, a production-ready template
- **Homepage:** https://vincentwei1021.github.io/video-shotcraft/
- **Stars:** 13522
- **Forks:** 1235
- **Open issues:** 12
- **Licence (SPDX):** Apache-2.0
- **Language:** TypeScript
- **Default branch:** main
- **Last push:** 2026-10-17T17:50:53Z
- **Archived:** False
- **Topics:** agent-skills, ai-agents, ai-video, claude-code, claude-code-skills, claude-skills, codex, motion-design, motion-graphics, product-video, promo-video, remotion, video-generation, video-production

## README

```
<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="./assets/brand/logo-mark-reverse.svg">
  <source media="(prefers-color-scheme: light)" srcset="./assets/brand/logo-mark.svg">
  <img alt="video-shotcraft logo" src="./assets/brand/logo-mark.svg" width="96" height="96">
</picture>

# video-shotcraft

**English** | [中文](README_CN.md) | [日本語](README_JA.md) | [Русский](README_RU.md)

[![Gallery](https://img.shields.io/badge/Gallery-live%20previews-d3923c)](https://vincentwei1021.github.io/video-shotcraft/)
[![Showcase](https://img.shields.io/badge/Showcase-community%20videos-7c3aed)](https://vincentwei1021.github.io/video-shotcraft/showcase.html)

[![GitHub stars](https://img.shields.io/github/stars/Vincentwei1021/video-shotcraft)](https://github.com/Vincentwei1021/video-shotcraft/stargazers)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue)](LICENSE)
[![Claude Code](https://img.shields.io/badge/Claude%20Code-skill-d97757)](SKILL.md)
[![Codex](https://img.shields.io/badge/Codex-skill-10a37f)](SKILL.md)
[![AtomGit Star](https://atomgit.com/VincentWei/video-shotcraft/star/badge.svg)](https://atomgit.com/VincentWei/video-shotcraft)

<a href="https://trendshift.io/repositories/88911?utm_source=repository-badge&amp;utm_medium=badge&amp;utm_campaign=badge-repository-88911" target="_blank" rel="noopener noreferrer"><img src="https://trendshift.io/api/badge/repositories/88911" alt="Vincentwei1021%2Fvideo-shotcraft | Trendshift" width="250" height="55"/></a>

<img src="./assets/readme/cover.webp" width="100%" alt="Six Gallery shots, left to right, top to bottom: graze-face-tour, logo-shrink-wordmark-lockup, panel-grid-moves, product-card-progressive-assemble, space-camera-moves · exploded-view, spotlight-hero-card">

[**▶ Browse all 150 shots in the Gallery**](https://vincentwei1021.github.io/video-shotcraft/) · [**🎬 Watch the Showcase**](https://vincentwei1021.github.io/video-shotcraft/showcase.html)

</div>

## What it does

Tell Claude Code or Codex "make a promo for my product". The agent captures your real pages, picks shots
from a library of tuned motion recipes, storyboards the film, animates it with
[Remotion](https://www.remotion.dev/), cuts it to the beat, adds sound design, checks every frame and hands you
a finished MP4 you can keep editing.

It is not a video generator. Every frame is drawn by code from your own screenshots and copy, so the result is
sharp, on-brand and fully editable.

**How it works**

![Eight-second workflow loop: product → capture real pages → storyboard with shot recipe cards → animate in Remotion → beat sync and sound design → render and frame QA → MP4, Motion Workbench and JianYing draft](assets/readme/workflow.gif)

## What's new

> [!IMPORTANT]
> ### 🎬 2026-10 · Shot library refresh
> All 216 demos were polished and reviewed. The best versions now make up
> **124 cards / 150 shots**, with fresh Gallery previews and video-shotcraft branding.

- **2026-09 · [Motion Workbench](#motion-workbench).** Keep editing the delivered film in a CapCut-style browser editor.
- **2026-08 · [video-talkcraft](https://github.com/Vincentwei1021/video-talkcraft)**, the narration-video
  sibling: script + voiceover in, every motion beat locked to the voice
  ([78 narration previews](https://vincentwei1021.github.io/video-talkcraft/)).
- **2026-08 · JianYing (CapCut CN) export.** The final film becomes an editable JianYing draft: per-shot clips,
  native caption tracks, separate SFX/BGM tracks. Verified on JianYing Pro 11.2 for macOS
  ([guide](references/jianying-export.md)).

## ❤️ Sponsors

<table>
<tr>
<td align="center" valign="middle" width="25%">
  <a href="https://go.apimart.ai/tw-video-shotcraft"><img src="./assets/readme/sponsors/apimart.png" width="200" alt="APIMart logo"></a><br>
  <a href="https://go.apimart.ai/tw-video-shotcraft"><b>APIMart</b></a>
</td>
<td>
  <p>Thanks to APIMart for sponsoring this project! APIMart is a low-cost API platform for AI image &amp; video generation — GPT-Image-2 from $0.006/image, 160+ images per dollar. One async API covers both image and video: submit a task, get an ID, fetch results via polling or callback. Batch tens of thousands of images without timeouts, switch models without changing code. Pay-as-you-go with no monthly fee — <a href="https://go.apimart.ai/tw-video-shotcraft">sign up here</a> to get started.</p>
</td>
</tr>
<tr>
<td align="center" valign="middle" width="25%">
  <a href="https://www.packyapi.ai/register?aff=l9IZ"><img src="./assets/readme/sponsors/packycode.png" width="200" alt="PackyCode logo"></a><br>
  <a href="https://www.packyapi.ai/register?aff=l9IZ"><b>PackyCode</b></a>
</td>
<td>
  <p>Access leading AI models through PackyCode with one API endpoint and one API key. Enjoy fast, reliable access with automatic failover and dedicated high-speed routes for Codex and Claude Code.</p>
  <p>Get started with $1 in free credits, a discount on your first top-up, and savings of up to 80% on eligible routes. Pay in RMB with no currency conversion markups or extra top-up fees.</p>
  <p><a href="https://www.packyapi.ai/register?aff=l9IZ">Get started with PackyCode →</a></p>
</td>
</tr>
<tr>
<td align="center" valign="middle" width="25%">
  <a href="https://aws.amazon.com/"><img src="./assets/readme/sponsors/aws.svg" width="200" alt="AWS logo"></a><br>
  <a href="https://aws.amazon.com/"><b>AWS</b></a>
</td>
<td>
  <p>Amazon Web Services (AWS) offers reliable, scalable cloud computing services, including compute, storage and AI.</p>
  <p><a href="https://aws.amazon.com/">Explore AWS →</a></p>
</td>
</tr>
</table>

## Install

Built and tuned for Claude Code and Codex. The simplest way is to hand your agent the link:

```text
Install this skill for me: https://github.com/Vincentwei1021/video-shotcraft
```

Or with the [skills](https://skills.sh/) CLI:

```bash
npx skills add Vincentwei1021/video-shotcraft
```

Or by hand:

```bash
git clone https://github.com/Vincentwei1021/video-shotcraft.git
cd video-shotcraft
ln -s "$(pwd)" ~/.claude/skills/video-shotcraft   # Claude Code
ln -s "$(pwd)" ~/.codex/skills/video-shotcraft    # Codex
```

In mainland China, clone from the AtomGit mirror if GitHub is slow:
`git clone https://atomgit.com/VincentWei/video-shotcraft.git`.

## Use

Just describe the film. No command needed.

| Say | What happens |
|---|---|
| `Use video-shotcraft to make a promo for my product.` | The agent introduces the Ink Press template and asks whether to use it, then captures your pages and builds the film |
| `Make a launch film for this repo with the Ink Press template.` | The fastest path: your screenshots, copy and logo dropped into a validated template |
| `Use the deck-deal-flyin and row-embed shot cards to present this feature.` | You pick the shots by name (copy names straight from the Gallery) |
| `Design a product close-up inspired by spotlight-hero-card.` | One shot, adapted to your product |

## Shot library

124 recipe cards in 10 categories, 150 shots in total. Search, filter and copy card names in the
[Gallery](https://vincentwei1021.github.io/video-shotcraft/).

<table>
<tr>
<td align="center" width="50%"><b>Opening & Brand</b> · 11 shots<br><img src="./assets/readme/examples/opening.webp" width="100%" alt="text-as-mask: a dashboard seen through giant letters"><br><sub>text-as-mask: the product shows through the title</sub></td>
<td align="center" width="50%"><b>Camera</b> · 13 shots<br><img src="./assets/readme/examples/camera.webp" width="100%" alt="exploded-view: a page tilts in 3D and its parts fly apart"><br><sub>exploded-view: the page tilts and comes apart</sub></td>
</tr>
<tr>
<td align="center" width="50%"><b>Transitions</b> · 19 shots<br><img src="./assets/readme/examples/transition.webp" width="100%" alt="card-flock-tumble: cards tumble into a giant word"><br><sub>card-flock-tumble: a flock of cards becomes the headline</sub></td>
<td align="center" width="50%"><b>Typography</b> · 25 shots<br><img src="./assets/readme/examples/typography.webp" width="100%" alt="flying-words: feature words fly through a tunnel"><br><sub>flying-words: a word tunnel resolves into the motto</sub></td>
</tr>
<tr>
<td align="center" width="50%"><b>UI Entrance</b> · 22 shots<br><img src="./assets/readme/examples/ui-entrance.webp" width="100%" alt="card-stack: a fan of recipe cards deals out"><br><sub>card-stack: a fan of cards deals into place</sub></td>
<td align="center" width="50%"><b>Interaction</b> · 13 shots<br><img src="./assets/readme/examples/interaction.webp" width="100%" alt="palette-theme-ripple: a theme change ripples across the app"><br><sub>palette-theme-ripple: a new theme ripples across the UI</sub></td>
</tr>
<tr>
<td align="center" width="50%"><b>Data</b> · 9 shots<br><img src="./assets/readme/examples/data.webp" width="100%" alt="axis-rescale-shock: a chart spikes and its axis rescales"><br><sub>axis-rescale-shock: the spike breaks the axis</sub></td>
<td align="center" width="50%"><b>Light & Emphasis</b> · 23 shots<br><img src="./assets/readme/examples/effects.webp" width="100%" alt="spotlight-sweep: a spotlight reveals the title"><br><sub>spotlight-sweep: a stage light finds the title</sub></td>
</tr>
<tr>
<td align="center" width="50%"><b>Rhythm</b> · 8 shots<br><img src="./assets/readme/examples/rhythm.webp" width="100%" alt="paparazzi-flash: three flashes cut between crops"><br><sub>paparazzi-flash: three flashes, three crops, one number</sub></td>
<td align="center" width="50%"><b>Outro</b> · 7 shots<br><img src="./assets/readme/examples/outro.webp" width="100%" alt="input-morph-assemble: an input box becomes the logo"><br><sub>input-morph-assemble: the chat box builds the logo</sub></td>
</tr>
</table>

Each card lives in `references/shots/<category>/<card>.md`, with its Remotion implementation in
`demos/<category>/<card>/` (see [demos/README.md](demos/README.md) for how to wire one into your project).

## Motion Workbench

After delivery the skill opens a CapCut-style editor in your browser
(`node workbench/scripts/open.mjs <project>`). The film is split into shot, transition, caption and SFX tracks
exactly as authored. Select any shot to edit its copy, font sizes and colours, move, trim or speed-ramp clips,
drag in any of the 150 library shots, then export with Remotion. Preview and render are frame-identical.

![Motion Workbench](workbench/docs/overview.png)

[Workbench guide, every panel with screenshots](workbench/GUIDE.md) (Chinese) ·
[Integration contract](references/workbench.md)

## What you get

| Output | Details |
|---|---|
| Final film | `MP4`, 1920×1080, 30 fps, h264, with music and sound design |
| Remotion project | The full source of your film: every shot is a React component you or the agent can change |
| Motion Workbench | Opens after delivery: tracks, inspector, retime, swap shots, export ([details](#motion-workbench)) |
| JianYing draft | Optional: an editable JianYing (CapCut CN) project with per-shot clips, captions and audio tracks |
| Social publishing | Optional, when you ask: publish the finished film via Upload-Post to TikTok, Instagram, YouTube, LinkedIn, X and more. Confirm caption and platforms, then dry-run before posting ([guide](references/publish-upload-post.md)) |

Also in the box: reusable Remotion components (2.5D page camera, captions, flash cuts, digit rolls), 149 SFX and
5 BGM tracks, page-capture scripts, and the production method itself (capture, visual direction, storyboarding,
sound design, beat sync, final QA) in [references/](references/pipeline.md).

## Showcase

**Share your film.** Made something with video-shotcraft?
[Submit it](https://github.com/Vincentwei1021/video-shotcraft/issues/new?template=showcase.yml) with the
Showcase issue form. After review it appears on the
[Showcase page](https://vincentwei1021.github.io/video-shotcraft/showcase.html).

## Requirements

| Needs | For |
|---|---|
| Node.js and npm | Remotion rendering, the template and the workbench |
| ffmpeg / ffprobe | Frame checks and final QA |
| [uv](https://docs.astral.sh/uv/) | Only for BGM beat analysis (runs a one-off Python script) |
| puppeteer | Only for page capture; the capture script installs it on demand |

Remotion downloads its own headless Chrome on first render. Tested on macOS; Linux works with the notes below.

<details>
<summary><b>Headless Linux / CI notes</b></summary>

Rendering on a headless Linux box (tested: 2 cores, Node 22) hits three walls:

1. **Concurrency cap.** `remotion still/render` fails with "Maximum for --concurrency is 2" on low-core
   machines. Pass `--concurrency=1`.
2. **Old headless removal.** Recent Chrome/Chromium dropped the old headless mode, so pointing Remotion at
   system Chromium fails to launch. Use a chrome-headless-shell binary instead of full Chrome.
3. **Blocked CDN.** If remotion.media is unreachable (common in mainland China), the automatic headless-shell
   download fails. Pass `--browser-executable=<path-to-local chrome-headless-shell>`.

With these three flags, frame renders from the bundled template work.

</details>

<details>
<summary><b>Repository structure</b></summary>

```text
video-shotcraft/
├── SKILL.md                 # Agent entry point and core production rules
├── references/
│   ├── pipeline.md          # End-to-end production workflow
│   ├── shots/               # 124 shot recipe cards in 10 functional categories
│   ├── sequences/           # Reusable full-video structures and sequence patterns
│   ├── aesthetic-rules.md   # Visual QA criteria
│   ├── music-beat-sync.md   # BGM analysis and beat-sync methodology
│   ├── sound-design.md      # Sound-design guidance and examples
│   ├── jianying-export.md   # JianYing (CapCut CN) project-export guide
│   ├── publish-upload-post.md # Optional social publishing (Upload-Post API)
│   └── workbench.md         # Motion workbench: manifest contract + editability rules
├── demos/                   # Remotion reference implementations (same categories)
├── gallery/                 # Static motion-preview Gallery
├── template/                # Runnable complete video template
├── jianying-export/         # JianYing draft installers (mac tested / win untested)
├── publish/                 # Optional social publisher (stdlib-only Python) + offline tests
├── workbench/               # Post-delivery motion workbench (Vite + Remotion Player)
└── assets/
    ├── lib/                 # Reusable Remotion components
    ├── scripts/             # Page-asset capture scripts
    └── audio/               # Audio assets
        ├── bgm/             # 5 BGM options
        └── sfx/<category>/  # 149 SFX across 16 scene categories
```

For the full workflow see [SKILL.md](SKILL.md), the [production pipeline](references/pipeline.md) and the
[visual QA criteria](references/aesthetic-rules.md).

</details>

## Audio and assets

SFX are organized into 16 scene and material categories (`transition` `impact` `riser` `camera` `ui` `text`
`paper` `film` `light` `data` `scifi` `mech` `glass` `fluid` `crowd` `counter`): pick the category first, then
the timbre. See [sound-design.md](references/sound-design.md) for the index and per-file usage, and
[ATTRIBUTION.md](assets/audio/ATTRIBUTION.md) for sources and licenses.

Product screenshots bundled with the template are demonstration assets. Replace them with screenshots of your
product before publishing, and check whether any product, customer or personal data needs to be anonymized.

## License

- Code and docs: [Apache-2.0](LICENSE).
- Audio under `assets/audio/`: each file under its own license, see [ATTRIBUTION.md](assets/audio/ATTRIBUTION.md).
- [Remotion](https://www.remotion.dev/) has its own
  [license](https://github.com/remotion-dev/remotion/blob/main/LICENSE.md): free for individuals and small
  teams, companies may need a paid license.

## Acknowledgements

Many recipes were distilled by studying the motion language of outstanding official product films, including
promos from **ClickUp, Perplexity, Slack, Notion, Figma, Framer, Bear, Raycast, Pitch, Miro, Superhuman and
Loom**. The cards document techniques (timing, easing, choreography) re-implemented from scratch; no footage,
artwork or brand assets from these films are included. All trademarks belong to their owners, and none of these
companies are affiliated with or endorse this project.

Thanks also to **[Remotion](https://www.remotion.dev/)**, which powers every demo and template here;
**[Mixkit](https://mixkit.co/)**, the source of the bundled SFX and music under its free license; the game-feel
and animation communities whose published principles inform several cards; and **Claude Code**, with which this
library was built, iterated and QA'd using the same workflow the skill teaches.

## Follow me

<p>
  <a href="https://x.com/VincentWei93"><img alt="Follow Vincent on X" src="https://img.shields.io/badge/X-Follow_Me-000000?style=for-the-badge&logo=x&logoColor=white"></a>
  <a href="https://www.douyin.com/user/MS4wLjABAAAAK1pkjBxilk2Oi_9h_vFyD-lTAu9CTlvhmOtkosDvvxg"><img alt="Follow Vincent on Douyin" src="https://img.shields.io/badge/Douyin-Follow_Me-000000?style=for-the-badge&logo=tiktok&logoColor=white"></a>
  <a href="https://xhslink.cn/m/At9iP2d5C1V"><img alt="Follow Vincent on Red Note" src="https://img.shields.io/badge/Red_Note-Follow_Me-FF2442?style=for-the-badge&logo=xiaohongshu&logoColor=white"></a>
</p>

## Star history

<a href="https://www.star-history.com/?repos=Vincentwei1021%2Fvideo-shotcraft&type=date&legend=top-left">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/chart?repos=Vincentwei1021/video-shotcraft&type=date&theme=dark&legend=top-left&sealed_token=DQ8_yn0k8in6tP80CRd9Ghuk1fcdEW7poFh9ticGB3wMNO-E_i6g51sUiQWCAQYP0u0bjRweuIfGoRS8FnrIz86oFp1lcl5zu2vrEJrQOoNvwdUSwmm8XNPkAiln1o-EBAX0uU8k6ReIlSRufGLqpoxsWshMSZ9mmok6ox5XXIUO77b7zOgp2yRIH6yR" />
    <source media="(prefers-color-scheme: light)" srcset="https://api.star-history.com/chart?repos=Vincentwei1021/video-shotcraft&type=date&legend=top-left&sealed_token=DQ8_yn0k8in6tP80CRd9Ghuk1fcdEW7poFh9ticGB3wMNO-E_i6g51sUiQWCAQYP0u0bjRweuIfGoRS8FnrIz86oFp1lcl5zu2vrEJrQOoNvwdUSwmm8XNPkAiln1o-EBAX0uU8k6ReIlSRufGLqpoxsWshMSZ9mmok6ox5XXIUO77b7zOgp2yRIH6yR" />
    <img alt="Star History Chart" src="https://api.star-history.com/chart?repos=Vincentwei1021/video-shotcraft&type=date&legend=top-left&sealed_token=DQ8_yn0k8in6tP80CRd9Ghuk1fcdEW7poFh9ticGB3wMNO-E_i6g51sUiQWCAQYP0u0bjRweuIfGoRS8FnrIz86oFp1lcl5zu2vrEJrQOoNvwdUSwmm8XNPkAiln1o-EBAX0uU8k6ReIlSRufGLqpoxsWshMSZ9mmok6ox5XXIUO77b7zOgp2yRIH6yR" />
  </picture>
</a>
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

## File tree (1361 files, filtered)

- `.github/scripts/showcase-publish.py`
- `LICENSE`
- `README.md`
- `README_CN.md`
- `README_JA.md`
- `README_RU.md`
- `SKILL.md`
- `assets/audio/ATTRIBUTION.md`
- `assets/audio/AUDITION-2026-07-27.md`
- `assets/brand/BRAND.md`
- `assets/lib/Caption.tsx`
- `assets/lib/ClipCard.tsx`
- `assets/lib/DigitRoll.tsx`
- `assets/lib/FlashCut.tsx`
- `assets/lib/FlatPanel.tsx`
- `assets/lib/PageCam.tsx`
- `assets/lib/VerticalTicker.tsx`
- `assets/lib/helpers/__tests__/helpers.test.ts`
- `assets/lib/helpers/camera.tsx`
- `assets/lib/helpers/motion.ts`
- `assets/lib/helpers/rand.ts`
- `assets/lib/helpers/shake.ts`
- `assets/readme/sponsors/ATTRIBUTION.md`
- `assets/scripts/smoke-render-demos.py`
- `demos/README.md`
- `demos/_fixtures/Brand.tsx`
- `demos/_fixtures/Fixtures.tsx`
- `demos/_fixtures/Look.tsx`
- `demos/_fixtures/Motion.tsx`
- `demos/_fixtures/PageCam2D.tsx`
- `demos/_fixtures/Polish.tsx`
- `demos/_legacy/README.md`
- `demos/_legacy/_fixtures/Fixtures.tsx`
- `demos/_legacy/_fixtures/Motion.tsx`
- `demos/_legacy/camera/depth-layer-moves/DollyZoomReal.tsx`
- `demos/_legacy/camera/tension-camera-moves/DutchRollToLevel.tsx`
- `demos/_legacy/camera/tension-camera-moves/SlowPushIn.tsx`
- `demos/_legacy/data/avatar-grid-radial-build-colorize/AvatarGridRadialBuildColorize.tsx`
- `demos/_legacy/data/counter-confetti/CounterConfetti.tsx`
- `demos/_legacy/data/cycle-glass-node-morph/CycleGlassNodeMorph.tsx`
- `demos/_legacy/data/hatch-depth/HatchDepth.tsx`
- `demos/_legacy/data/odometer-digit-roll/OdometerDigitRoll.tsx`
- `demos/_legacy/data/particle-celebrate-hits/ConfettiCrossfire.tsx`
- `demos/_legacy/data/particle-celebrate-hits/CounterTickSparks.tsx`
- `demos/_legacy/data/particle-sand-fill/ParticleSandFill.tsx`
- `demos/_legacy/data/ring-diagram-annotation-reveal/RingDiagramAnnotationReveal.tsx`
- `demos/_legacy/effects/dashboard-glow-highlight-pill/DashboardGlowHighlightPill.tsx`
- `demos/_legacy/effects/fui-hud-moves/LineUnfoldPanel.tsx`
- `demos/_legacy/effects/icon-performance-moves/PopBurstConfirm.tsx`
- `demos/_legacy/effects/light-play-moves/HalationBloom.tsx`
- `demos/_legacy/effects/line-boil/LineBoil.tsx`
- `demos/_legacy/effects/riso-print-hits/RisoBeatPump.tsx`
- `demos/_legacy/interaction/ai-stream-response/StreamResponse.tsx`
- `demos/_legacy/interaction/chip-lift-to-user-pill/ChipLiftToUserPill.tsx`
- `demos/_legacy/interaction/collab-cursor-moves/CursorDialogueDuet.tsx`
- `demos/_legacy/interaction/command-palette-summon/CommandPaletteSummon.tsx`
- `demos/_legacy/interaction/input-trigger-moves/CursorPerformancePunchIn.tsx`
- `demos/_legacy/interaction/input-trigger-moves/KeycapSmashCut.tsx`
- `demos/_legacy/outro/edit-hook-moves/LogoStingButton.tsx`
- `demos/_legacy/rhythm/beat-cut-moves/BeatCutAccelerando.tsx`
- `demos/_legacy/rhythm/montage-rhythm-moves/DominoCascade.tsx`
- `demos/_legacy/rhythm/montage-rhythm-moves/DropBlackoutSlam.tsx`
- `demos/_legacy/rhythm/montage-rhythm-moves/WrightTripleCut.tsx`
- `demos/_legacy/rhythm/panel-grid-moves/FlipGridReflow.tsx`
- `demos/_legacy/rhythm/panel-grid-moves/GridFlashMosaic.tsx`
- `demos/_legacy/rhythm/rhythm-interrupt-moves/StrobeBlackFrames.tsx`
- `demos/_legacy/rhythm/smear-multiples/SmearMultiples.tsx`
- `demos/_legacy/rhythm/spectrum-morph-ui/SpectrumMorphUi.tsx`
- `demos/_legacy/rhythm/speed-ramp-freeze/SpeedRampReal.tsx`
- `demos/_legacy/rhythm/trailer-grammar-moves/CardFootageCadence.tsx`
- `demos/_legacy/rhythm/trailer-grammar-moves/SmashCut.tsx`
- `demos/_legacy/transition/bubble-swarm-takeover/BubbleSwarmTakeover.tsx`
- `demos/_legacy/transition/color-block-step-wipe/ColorBlockStepWipe.tsx`
- `demos/_legacy/transition/cube-navigation/CubeNavigation.tsx`
- `demos/_legacy/transition/page-turn-transitions/CubeRotate.tsx`
- `demos/_legacy/transition/paper-plane-messenger/PaperPlaneMessenger.tsx`
- `demos/_legacy/transition/shot-transitions/BlackCardTransition.tsx`
- `demos/_legacy/transition/shot-transitions/DarkTunnelTransition.tsx`
- `demos/_legacy/transition/shot-transitions/FocusHandoffTransition.tsx`
- `demos/_legacy/transition/shot-transitions/MaskWipeReal.tsx`
- `demos/_legacy/transition/shot-transitions/WhipPanReal.tsx`
- `demos/_legacy/transition/transition-travel/SharedElementMorph.tsx`
- `demos/_legacy/transition/wipe-transitions/ClockWipe.tsx`
- `demos/_legacy/typography/cel-flash-stomp/CelFlashStomp.tsx`
- `demos/_legacy/typography/scramble/Scramble.tsx`
- `demos/_legacy/typography/split-flap-title/SplitFlapFlip.tsx`
- `demos/_legacy/typography/type-assembly-moves/TrackingExpandReveal.tsx`
- `demos/_legacy/typography/type-entrance-moves/ScrambleDecode.tsx`
- `demos/_legacy/typography/typewriter-moves/TerminalTypewriter.tsx`
- `demos/_legacy/typography/typing-code-block/TypingCodeBlock.tsx`
- `demos/_legacy/ui-entrance/element-body-moves/AxialStretch.tsx`
- `demos/_legacy/ui-entrance/floating-glossy-label-pills/FloatingGlossyLabelPills.tsx`
- `demos/_legacy/ui-entrance/morph-from-primitive/MorphFromPrimitive.tsx`
- `demos/_legacy/ui-entrance/paper-craft-moves/MaskingTapeSlap.tsx`
- `demos/_legacy/ui-entrance/paper-craft-moves/PopupBookRise.tsx`
- `demos/_legacy/ui-entrance/platform-hinge-rise/PlatformHingeRise.tsx`
- `demos/_legacy/ui-entrance/runway-ground-skim/RunwayGroundSkim.tsx`
- `demos/_legacy/ui-entrance/skeleton-reveal/SkeletonReveal.tsx`
- `demos/_legacy/ui-entrance/svg-shape-morph/SvgShapeMorph.tsx`
- `demos/_legacy/ui-entrance/wall-reveal-moves/GridWaveFlip.tsx`
- `demos/camera/basic-3d-scene/Basic3DScene.tsx`
- `demos/camera/crash-zoom-punch/CrashImpactReal.tsx`
- `demos/camera/crash-zoom-punch/CrashZoomReal.tsx`
- `demos/camera/cursor-flyover/CursorFlyover.tsx`
- `demos/camera/depth-layer-moves/MultiplaneReal.tsx`
- `demos/camera/graze-face-tour/GrazeFaceTour.tsx`
- `demos/camera/overhead-camera-moves/OverheadTabletopDrop.tsx`
- `demos/camera/overhead-camera-moves/TiltReveal.tsx`
- `demos/camera/space-camera-moves/DroneDiveLanding.tsx`
- `demos/camera/space-camera-moves/ExplodedView.tsx`
- `demos/camera/steep-tilt-glide/SteepTiltGlide.tsx`
- `demos/camera/tension-camera-moves/BulletTimeFreezeOrbit.tsx`
- `demos/camera/tension-camera-moves/PullBackIsolation.tsx`
- `demos/camera/terminal-3d/Terminal3D.tsx`
- `demos/data/before-after-slider-scrub/BeforeAfterSliderScrub.tsx`
- `demos/data/chart-live-moves/AxisRescaleShockV2.tsx`
- `demos/data/chart-live-moves/OscilloscopeStreamV2.tsx`
- `demos/data/chart-live-moves/UnitDotSwarmRegroupV2.tsx`
- `demos/data/gauge-readout-moves/NeedleSweepSelftest.tsx`
- `demos/data/gauge-readout-moves/TapeScrollFixedPointer.tsx`
- `demos/data/scroll-brake-moves/BrakeReticleLock.tsx`
- `demos/data/scroll-brake-moves/ChangelogScrollBrake.tsx`
- `demos/data/timeline-travel/TimelineTravel.tsx`
- `demos/effects/assemble-then-type-flyin/AssembleThenTypeFlyin.tsx`
- `demos/effects/aurora-bloom-bg-flip/AuroraBloomBgFlip.tsx`
- `demos/effects/brand-frame-snap/BrandFrameSnap.tsx`
- `demos/effects/fui-hud-moves/ReticleLockOn.tsx`
- `demos/effects/glow-flyline-moves/FlylineArc.tsx`
- `demos/effects/glow-flyline-moves/GlowOrbAmbient.tsx`
- `demos/effects/glow-flyline-moves/OrbFlylineRelay.tsx`
- `demos/effects/icon-performance-moves/AttentionBounce.tsx`
- `demos/effects/impact-feedback/AnimeImpact.tsx`
- `demos/effects/impact-feedback/HitCounter.tsx`
- `demos/effects/light-play-moves/SheenSweepRetry.tsx`
- `demos/effects/light-play-moves/SpotlightSweepReveal.tsx`
- `demos/effects/radial-ripple-phone-chips/RadialRipplePhoneChips.tsx`
- `demos/effects/riso-print-hits/RisoMisregistrationHit.tsx`
- `demos/effects/scan-bracket-sweep/ScanBracketSweep.tsx`
- `demos/effects/scanline-annotate-focus/ScanlineAnnotateFocus.tsx`
- `demos/effects/scanline-assemble-flyin/ScanlineAssembleFlyin.tsx`
- `demos/effects/slam-entrance-moves/ImpactBurstKit.tsx`
- `demos/effects/slam-entrance-moves/KanadaPerspectiveSnap.tsx`
- `demos/effects/slam-entrance-moves/ScoreSlam.tsx`
- `demos/effects/spotlight-sweep-moves/CornerSpotlightReveal.tsx`
- `demos/effects/spotlight-sweep-moves/GlowWakeSleepPanel.tsx`
- `demos/effects/spotlight-sweep-moves/SlideSpotlightPan.tsx`
- `demos/interaction/autolayout-gap-dial/AutolayoutGapDial.tsx`
- `demos/interaction/canvas-materialize-moves/DiagramCascadeBuild.tsx`
- `demos/interaction/canvas-materialize-moves/PanelToCanvasMaterialize.tsx`
- `demos/interaction/chip-grid-single-select-blackout/ChipGridSingleSelectBlackout.tsx`
- `demos/interaction/collab-cursor-moves/CursorCastEnsemble.tsx`
- `demos/interaction/glass-pill-dictation-typing/GlassPillDictationTyping.tsx`
- `demos/interaction/hashtag-to-pill-materialize/HashtagToPillMaterialize.tsx`
- `demos/interaction/picker-carousel-feature-cycle/PickerCarouselFeatureCycle.tsx`
- `demos/interaction/segmented-thumb-hero/SegmentedThumbHero.tsx`
- `demos/interaction/theme-switch-moves/PaletteThemeRipple.tsx`
- `demos/interaction/theme-switch-moves/ThemeSweepToggle.tsx`
- `demos/interaction/type-and-filter/TypeAndFilter.tsx`
- `demos/interaction/voice-waveform-live/VoiceWaveformLive.tsx`
- `demos/opening/crane-rise-reveal/CraneRiseReveal.tsx`
- `demos/opening/dataviz-landscape-open/DatavizLandscapeOpen.tsx`
- `demos/opening/fracture/Fracture.tsx`
- `demos/opening/icon-field-colorize/IconFieldColorize.tsx`
- `demos/opening/letterspace-materialize/LetterspaceMaterialize.tsx`
- `demos/opening/magician-card-flourish/MagicianCardFlourish.tsx`
- `demos/opening/orbit-ring-title-open/OrbitRingTitleOpen.tsx`
- `demos/opening/spotlight-hero-card/SpotlightHeroCard.tsx`
- `demos/opening/stroke-segment-build/StrokeSegmentBuild.tsx`
- `demos/opening/text-as-mask/TextAsMask.tsx`
- `demos/outro/grain-dissolve/GrainDissolve.tsx`
- `demos/outro/logo-shrink-wordmark-lockup/LogoShrinkWordmarkLockup.tsx`
- `demos/outro/neon-triple-marquee/NeonTripleMarquee.tsx`
- `demos/outro/outro-group-photo-launch/OutroGroupPhotoLaunch.tsx`
- `demos/outro/ui-strip-away-outro/UiStripAwayOutro.tsx`
- `demos/outro/ui-to-brand-morph/IconFlipBloomLogo.tsx`
- `demos/outro/ui-to-brand-morph/InputMorphsIntoLogo.tsx`
- `demos/rhythm/beat-cut-moves/PaparazziFlash.tsx`
- `demos/rhythm/beat-step-list-theme-cycle/BeatStepListThemeCycle.tsx`
- `demos/rhythm/panel-grid-moves/ComicPanelSplit.tsx`
- `demos/rhythm/quad-split-parallel-scenes/QuadSplitParallelScenes.tsx`
- `demos/rhythm/rhythm-interrupt-moves/JumpCutPunchIn.tsx`
- `demos/rhythm/sakuga-timing-shift/SakugaTimingShift.tsx`
- `demos/rhythm/speed-ramp-freeze/FreezeAnnotateReal.tsx`
- `demos/rhythm/trailer-grammar-moves/TrailerBumper.tsx`
- `demos/transition/bottom-push-stack-wipe/BottomPushStackWipe.tsx`
- `demos/transition/card-flip-reveal/CardFlipReveal.tsx`
- `demos/transition/card-flock-tumble/CardFlockTumble.tsx`
- `demos/transition/circle-match-iris/CircleMatchIris.tsx`
- `demos/transition/gradient-transition/GradientTransition.tsx`
- `demos/transition/line-carry-transition/LineCarryTransition.tsx`
- `demos/transition/mosaic-reframe/MosaicReframe.tsx`
- `demos/transition/page-turn-transitions/BarnDoorSplit.tsx`
- `demos/transition/print-texture-transitions/InkBleedReveal.tsx`
- `demos/transition/shot-transitions/PortalWipeV2.tsx`
- `demos/transition/shot-transitions/WhipBrakeReal.tsx`
- `demos/transition/tear-streak-transitions/GlitchDisplace.tsx`
- `demos/transition/transition-hidden-cut/InvisibleCut.tsx`
- `demos/transition/transition-hidden-cut/LightLeakBurn.tsx`
- `demos/transition/transition-hidden-cut/VersusSlam.tsx`
- `demos/transition/transition-travel/LetterformZoom.tsx`
- `demos/transition/white-flash-logo-simplify-cut/WhiteFlashLogoSimplifyCut.tsx`
- `demos/transition/wipe-transitions/BlindsSlice.tsx`
- `demos/typography/blur-slide/BlurSlide.tsx`
- `demos/typography/brace-expand/BraceExpand.tsx`
- `demos/typography/brand-ink-open/BrandInkOpen.tsx`
- `demos/typography/countdown-arc-scatter/CountdownArcScatter.tsx`
- `demos/typography/flying-words/FlyingWords.tsx`
- `demos/typography/glitch-cycle/GlitchCycle.tsx`
- `demos/typography/gradient-word-sweep/GradientWordSweep.tsx`
- `demos/typography/lead-word-zoom-assemble/LeadWordZoomAssemble.tsx`
- `demos/typography/marker-underline-title/MarkerUnderlineTitle.tsx`
- `demos/typography/outline-word-fill/OutlineWordFill.tsx`
- `demos/typography/paper-title-card/PaperTitleCard.tsx`
- `demos/typography/pill-chip-slot-cycle-handled/PillChipSlotCycleHandled.tsx`
- `demos/typography/pill-slot-cycle/PillSlotCycle.tsx`
- `demos/typography/text-column-converge/TextColumnConverge.tsx`
- `demos/typography/title-demote-to-label/TitleDemoteToLabel.tsx`
- `demos/typography/type-assembly-moves/LetterformDriftAssembly.tsx`
- `demos/typography/type-assembly-moves/SplitTextStagger.tsx`
- `demos/typography/type-assembly-moves/TextOnPath.tsx`
- `demos/typography/type-entrance-moves/LetterDropPhysics.tsx`
- `demos/typography/type-rhythm-sync/FontWeightPump.tsx`
- `demos/typography/type-rhythm-sync/KaraokeFillSync.tsx`
- `demos/typography/typewriter-moves/TypewriterErrorRetype.tsx`
- `demos/typography/vertical-word-roll-blur-cycle/VerticalWordRollBlurCycle.tsx`
- `demos/typography/word-relay-filmstrip/WordRelayFilmstrip.tsx`
- `demos/typography/word-relay-geometry/WordRelayGeometry.tsx`
- `demos/ui-entrance/avatar-bracket-carousel/AvatarBracketCarousel.tsx`
- `demos/ui-entrance/bezier-source-converge-merge/BezierSourceConvergeMerge.tsx`
- `demos/ui-entrance/card-stack/CardStack.tsx`
- `demos/ui-entrance/carousel-3d/Carousel3D.tsx`
- `demos/ui-entrance/clipcard-looping/ClipCardLooping.tsx`
- `demos/ui-entrance/cloner-depth-echo/ClonerDepthEcho.tsx`
- `demos/ui-entrance/deck-deal-flyin/DeckDealFlyin.tsx`
- `demos/ui-entrance/doc-park-left-pill-deal/DocParkLeftPillDeal.tsx`
- `demos/ui-entrance/document-typewriter-reveal/DocumentTypewriterReveal.tsx`
- `demos/ui-entrance/draw-svg-trace/DrawSvgTrace.tsx`
- `demos/ui-entrance/element-body-moves/ContactShadowLift.tsx`
- `demos/ui-entrance/integration-hub-map/IntegrationHubMap.tsx`
- `demos/ui-entrance/list-reveal/ListReveal.tsx`
- `demos/ui-entrance/list-stack-press/ListStackPress.tsx`
- `demos/ui-entrance/neon-frame-forerun/NeonFrameForerun.tsx`
- `demos/ui-entrance/neon-frame-orbit-drop/NeonFrameForerunOrbit.tsx`
- `demos/ui-entrance/page-waterfall-wall/PageWaterfallWall.tsx`
- `demos/ui-entrance/page-waterfall-wall/VerticalTicker.tsx`
- `demos/ui-entrance/product-card-progressive-assemble/ProductCardProgressiveAssemble.tsx`
- `demos/ui-entrance/radial-wave/RadialWave.tsx`
- `demos/ui-entrance/research-card-stack-scroll/ResearchCardStackScroll.tsx`
- `demos/ui-entrance/row-embed/RowEmbed.tsx`
- `demos/ui-entrance/value-stagger-gradient/ValueStaggerGradient.tsx`
- `demos/ui-entrance/wall-reveal-moves/BentoLightUp.tsx`
- `demos/ui-entrance/wall-reveal-moves/WireframeDrawOn.tsx`
- `gallery/app.js`
- `gallery/assemble-media.py`
- `gallery/build-seo.py`
- `gallery/index.html`
- `gallery/library.html`
- `gallery/showcase.html`
- `gallery/showcase.js`
- `gallery/source/assemble-then-type-flyin.md`
- `gallery/source/aurora-bloom-bg-flip.md`
- `gallery/source/autolayout-gap-dial.md`
- `gallery/source/avatar-bracket-carousel.md`
- `gallery/source/basic-3d-scene.md`
- `gallery/source/beat-cut-moves.md`
- `gallery/source/beat-step-list-theme-cycle.md`
- `gallery/source/before-after-slider-scrub.md`
- `gallery/source/bezier-source-converge-merge.md`
- `gallery/source/blur-slide.md`
- `gallery/source/bottom-push-stack-wipe.md`
- `gallery/source/brace-expand.md`
- `gallery/source/brand-frame-snap.md`
- `gallery/source/brand-ink-open.md`
- `gallery/source/canvas-materialize-moves.md`
- `gallery/source/card-flip-reveal.md`
- `gallery/source/card-flock-tumble.md`
- `gallery/source/card-stack.md`
- `gallery/source/carousel-3d.md`
- `gallery/source/chart-live-moves.md`
- `gallery/source/chip-grid-single-select-blackout.md`
- `gallery/source/circle-match-iris.md`
- `gallery/source/cloner-depth-echo.md`
- `gallery/source/collab-cursor-moves.md`
- `gallery/source/countdown-arc-scatter.md`
- `gallery/source/crane-rise-reveal.md`
- `gallery/source/crash-zoom-punch.md`
- `gallery/source/cursor-flyover.md`
- `gallery/source/dataviz-landscape-open.md`
- `gallery/source/deck-deal-flyin.md`
- `gallery/source/depth-layer-moves.md`
- `gallery/source/doc-park-left-pill-deal.md`
- `gallery/source/document-typewriter-reveal.md`
- `gallery/source/draw-svg-trace.md`
- `gallery/source/element-body-moves.md`
- `gallery/source/flying-words.md`
- `gallery/source/fracture.md`
- `gallery/source/fui-hud-moves.md`
- `gallery/source/gauge-readout-moves.md`
- `gallery/source/glass-pill-dictation-typing.md`
- `gallery/source/glitch-cycle.md`
- `gallery/source/glow-flyline-moves.md`
- `gallery/source/gradient-transition.md`
- `gallery/source/gradient-word-sweep.md`
- `gallery/source/grain-dissolve.md`
- `gallery/source/graze-face-tour.md`
- `gallery/source/hashtag-to-pill-materialize.md`
- `gallery/source/icon-field-colorize.md`
- `gallery/source/icon-performance-moves.md`
- `gallery/source/impact-feedback.md`
- `gallery/source/integration-hub-map.md`
- `gallery/source/lead-word-zoom-assemble.md`
- `gallery/source/letterspace-materialize.md`
- `gallery/source/light-play-moves.md`
- `gallery/source/line-carry-transition.md`
- `gallery/source/list-reveal.md`
- `gallery/source/list-stack-press.md`
- `gallery/source/logo-shrink-wordmark-lockup.md`
- `gallery/source/magician-card-flourish.md`
- `gallery/source/marker-underline-title.md`
- `gallery/source/mosaic-reframe.md`
- `gallery/source/neon-frame-forerun.md`
- `gallery/source/neon-frame-orbit-drop.md`
- `gallery/source/neon-triple-marquee.md`
- `gallery/source/orbit-ring-title-open.md`
- `gallery/source/outline-word-fill.md`
- `gallery/source/outro-group-photo-launch.md`
- `gallery/source/overhead-camera-moves.md`
- `gallery/source/page-turn-transitions.md`
- `gallery/source/page-waterfall-wall.md`
- `gallery/source/panel-grid-moves.md`
- `gallery/source/paper-title-card.md`
- `gallery/source/picker-carousel-feature-cycle.md`
- `gallery/source/pill-chip-slot-cycle-handled.md`
- `gallery/source/pill-slot-cycle.md`
- `gallery/source/print-texture-transitions.md`
- `gallery/source/product-card-progressive-assemble.md`
- `gallery/source/quad-split-parallel-scenes.md`
- `gallery/source/radial-ripple-phone-chips.md`
- `gallery/source/radial-wave.md`
- `gallery/source/research-card-stack-scroll.md`
- `gallery/source/rhythm-interrupt-moves.md`
- `gallery/source/riso-print-hits.md`
- `gallery/source/row-embed.md`
- `gallery/source/sakuga-timing-shift.md`
- `gallery/source/scan-bracket-sweep.md`
- `gallery/source/scanline-annotate-focus.md`
- `gallery/source/scanline-assemble-flyin.md`
- `gallery/source/scroll-brake-moves.md`
- `gallery/source/segmented-thumb-hero.md`
- `gallery/source/shot-transitions.md`
- `gallery/source/slam-entrance-moves.md`
- `gallery/source/space-camera-moves.md`
- `gallery/source/speed-ramp-freeze.md`
- `gallery/source/spotlight-hero-card.md`
- `gallery/source/spotlight-sweep-moves.md`
- `gallery/source/steep-tilt-glide.md`
- `gallery/source/stroke-segment-build.md`
- `gallery/source/tear-streak-transitions.md`
- `gallery/source/tension-camera-moves.md`
- `gallery/source/terminal-3d.md`
- `gallery/source/text-as-mask.md`
- `gallery/source/text-column-converge.md`
- `gallery/source/theme-switch-moves.md`
- `gallery/source/timeline-travel.md`
- `gallery/source/title-demote-to-label.md`
- `gallery/source/trailer-grammar-moves.md`
- `gallery/source/transition-hidden-cut.md`
- `gallery/source/transition-travel.md`
- `gallery/source/type-and-filter.md`
- `gallery/source/type-assembly-moves.md`
- `gallery/source/type-entrance-moves.md`
- `gallery/source/type-rhythm-sync.md`
- `gallery/source/typewriter-moves.md`
- `gallery/source/ui-strip-away-outro.md`
- `gallery/source/ui-to-brand-morph.md`
- `gallery/source/value-stagger-gradient.md`
- `gallery/source/vertical-word-roll-blur-cycle.md`
- `gallery/source/voice-waveform-live.md`
- `gallery/source/wall-reveal-moves.md`
- `gallery/source/white-flash-logo-simplify-cut.md`
- `gallery/source/wipe-transitions.md`
- `gallery/source/word-relay-filmstrip.md`
- `gallery/source/word-relay-geometry.md`
- `gallery/sync-from-cards.py`
- `gallery/translations.js`
- `jianying-export/aifl_promo.py`
- `jianying-export/mac_draft.py`
- `jianying-export/smoke_test.py`
- `jianying-export/windows_draft.py`
- `publish/test_upload_post.py`
- `publish/upload_post.py`
- `references/aesthetic-rules.md`
- `references/final-review.md`
- `references/guided-free-creation.md`
- `references/jianying-export.md`
- `references/music-beat-sync.md`
- `references/pipeline.md`
- `references/publish-upload-post.md`
- `references/sequences/promo-energy-arc.md`
- `references/shots/ATTRIBUTION.md`
