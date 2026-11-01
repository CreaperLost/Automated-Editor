# motionvideo.xyz — the community prompt catalogue, and the agent-first editor behind it

Pulled 27 October 2026 from <https://motionvideo.xyz/en>.
Source repo [motionvideohq/motionvideo.xyz](https://github.com/motionvideohq/motionvideo.xyz), commit `09c10894db64` (2026-10-26T23:36:33+05:30). MIT.
Data via the site's own public endpoint `GET /api/catalog` (catalogue stamp `2026-10-24T21:04:06.396Z`, 922 KB) and `GET /api/catalog/directories` (139 KB).
Sitemap: 5,022 URLs = 9 locales x 558 pages each.

**Everything about the products below is a vendor or maintainer claim.** We read the public
catalogue and the open-source site; we did not buy or run the skill pack, and we did not
install any renderer.

## What the site actually is

Two separate things share the domain.

| | The directory (open source, MIT) | The skill pack (sold) |
|---|---|---|
| What | 500 AI-made motion videos with the prompt behind each, plus tool/studio/skill directories | Agent skills, scene starters, motion tokens, renderer guides |
| Price | free | **$29 launch, $49 after the first 100 purchases**, one-time |
| Licence | MIT (site source only) | *"The MotionVideo skill pack, brand name, and logomark are not covered by this license"* |
| Stack | TanStack Start (React 19), Cloudflare Workers + D1 + R2, Drizzle ORM, Better Auth, Dodo Payments, PostHog | plain markdown + code |

Built by **Aniket Pawar** ([@alaymanguy), who also runs Shadcn Labs. The site's own words:
*"Motion design, written in code. A directory of motion videos made with AI and code, with
the prompts and skills behind them. Plus agent skills that teach your coding agent timing,
easing, and choreography."*

The pitch, verbatim: *"Good motion is mostly judgment: how long to hold, when things overlap,
which easing makes a move feel intentional. That judgment usually lives in keyframe tools and
years of practice. MotionVideo writes it down for your agent."* And: *"There is no hosted
service, no credits, and no subscription."*

## The catalogue

**500 videos, 445 distinct creators**, dated 2026-08-29 to 2026-10-15 — seven weeks.

| Field | Value |
|---|---|
| Prompts published | 494 of 500; `promptShared` true on 497 |
| Prompt length | min 11 chars, median 176, max 16,225 |
| Model | Opus 5.5 468, Fable 5.5 10, unstated 9, Gemini 4 Argon 8, GPT-6 Astra 5 |
| Reasoning effort | unstated 461, max 25, high 7, medium 7 |
| Types | prompt 494, skill 7 |
| Formats | 1920x1080 321, 1280x720 47, 1080x1920 36, 1080x1080 20, 1440x1440 17, 1080x1350 8 |

### Content tags

| Tag | Videos |
|---|---:|
| kinetic-type | 228 |
| product-ui | 191 |
| music | 121 |
| shapes | 104 |
| diagrams | 87 |
| characters | 86 |
| particles | 65 |
| charts | 49 |
| phone | 33 |
| photos | 30 |
| code | 30 |

### Stacks

Recorded on 132 of 500 entries. Unstated is the largest group.

| Stack | Videos |
|---|---:|
| (unstated) | 368 |
| Remotion | 20 |
| HTML | 16 |
| HTML + Playwright | 12 |
| HyperFrames | 12 |
| HTML + Canvas | 8 |
| JavaScript | 8 |
| Three.js | 6 |
| Canvas | 4 |
| HTML + Three.js | 3 |
| HTML + Playwright + ffmpeg | 3 |
| JavaScript + Playwright + FFmpeg | 3 |
| GSAP | 2 |
| HTML + SVG + Canvas | 2 |
| Python + ffmpeg | 2 |
| SVG | 2 |
| HTML + WebGL2 | 2 |
| fframes + Rust + SVG | 1 |
| HyperFrames + Three.js | 1 |
| OpenEdit | 1 |
| WebGL + Three.js | 1 |
| Manim, edge-tts | 1 |
| Canvas 2D | 1 |
| HTML canvas + Playwright + ffmpeg | 1 |
| HTML + Canvas + Web Audio API | 1 |
| HTML + Canvas + Playwright | 1 |
| React + Remotion | 1 |
| GSAP + Three.js + HTML | 1 |
| WebGL | 1 |
| Canvas + Web Audio | 1 |
| HTML, Playwright | 1 |
| SVG/Canvas | 1 |
| GSAP + SVG | 1 |
| HyperFrames + GSAP | 1 |
| Canvas + Playwright + ffmpeg | 1 |
| Python | 1 |
| WebGL2 + Canvas 2D + Web Audio | 1 |
| HTML/JS, Playwright, ffmpeg | 1 |
| Blender | 1 |
| Remotion, React | 1 |
| HTML + GSAP | 1 |
| Remotion + SVG + Canvas | 1 |
| Remotion + ffmpeg | 1 |
| Remotion + HTML | 1 |

**HyperFrames appears 12 times, Remotion 20** — the two renderers AeroEdits knows, in a
catalogue that otherwise spans HTML, Canvas, Three.js, GSAP, SVG, WebGL2, Python and
fframes. One entry is tagged `OpenEdit` (*"Talking-head clip edit", Sabba Keynejad, prompt:
*"Cut a raw talking-head clip into a punchy, fun edit with subtitles, graphics and music"*)
— the only entry in the whole catalogue that cuts existing footage.

### The `iterations` field: one-shot or not

Present on 117 entries.

| Value | Count |
|---|---:|
| one-shot | 102 |
| few-rounds | 13 |
| many-rounds | 2 |

This is the only iteration data in this whole line of research, and it cuts against the
one-shot framing: of the entries that say, **102 are one-shot and 15 needed rounds**. The
two `many-rounds` entries are Anthony Riera's product-film skill (Remotion) and toyoshim's
ray-marching piece.

### Data-quality note: the dedup is incomplete

The catalogue ships its own `removed` list with 12 entries, each a duplicate of another
(slug, `duplicateOf`, `tweetUrl`). But hashing the published prompts finds **six groups of
entries still sharing an identical prompt** (>200 chars):

- `twoclipping-5cba86`, `demonugc-4c5753`, `dzhohola-72ebb4`, `hicallmechai-79b945`, `nezbuilds-008d14`, `__morse-7ea664`, `sanjeevn72-9de2cb` — different creators
- `twoclipping-221cab`, `beamnxw-375dbf` — different creators
- `twoclipping-6dd14e`, `raphaelaubryy-b20885` — different creators
- `op7418-2864b1`, `op7418-0267c8`
- `1littlecoder-9fef89`, `souravbhar871-61f424` — different creators
- `leanzubrezki-8846f7`, `hsanat-51b0ff`, `drrickio-dee911`, `pbteja1998-82c12e`, `moritzkremb-b0b5e5` — different creators

One of those pairs is `twoclipping-221cab` and `beamnxw-375dbf`: the beamnxw entry carries a
byte-identical prompt. One prompt is shared by five handles. So: **do not treat a
distinct video as a distinct prompt.** Read the corpus as ~490 distinct prompts at best,
and check the `handle` before citing anything as a creator's own wording.

## The structured prompt schema

A convention has emerged across the community. 29 published prompts use at least four of
these six sections:

| Section | What it carries |
|---|---|
| `<inputs>` | what to ask the user for, and the defaults to use if they skip it |
| `<direction>` | the look: palette, typography, reference, banned moves |
| `<structure>` | the beat sheet — what happens, in order |
| `<build>` | how to build it: stack, frame maths, determinism, render |
| `<gotchas>` | the traps, written down before they happen |
| `<start>` | the first instruction to the agent |

Section frequency across the 494 published prompts: `structure` 32, `inputs` 31, `direction`
31, `build` 30, `gotchas` 29, `start` 26.

The fullest example, from `notdwd-c2037d` (10,419 chars), opens like this:

```text
<inputs>
Use the attached PHYLLA video as the primary reference. Inspect its compositions, typography,
scene changes, motion, and audio before building.
Ask me for any available original image assets, PHYLLA logo, brand fonts, Higgsfield/Katana
branding, and soundtrack. Ask only for information that cannot be recovered from the reference.

If I skip anything, use these defaults:

Brand: PHYLLA.
Duration: 30 seconds.
Frame rate: 24 fps.
Main film: 1920 x 1080.
Showcase version: 2560 x 1536.
Typography: a bold grotesk sans serif paired with a high-contrast italic serif.
</inputs>

<direction>
Create a complete launch film for PHYLLA using Higgsfield Katana and Claude Motion.
The story: a word behaves like a seed. A person supplies it, intelligence responds, and an
entire world grows from that small beginning.

Starting palette:

Near-black: #080A09.
...
</direction>
```

This is the same shape as the [beamnxw protocol](beamnxw-my-works-2026-10-27.md) — inputs
first, then direction, then structure, then build, then gotchas — and it is the shape
Tao used in video 04. It appears to be how this community writes prompts now.

## The seven skills in the catalogue

Seven entries carry a `skillUrl`, and five carry an install command — the catalogue doubles
as a skill registry.

| Entry | Skill | Install |
|---|---|---|
| `lexnlin-6161a6` (Leon Lin) | https://github.com/Leonxlnx/cinetic | npx skills add Leonxlnx/cinetic |
| `anthonyriera-9b1b2a` (Anthony Riera) | https://github.com/Rieranthony/product-film-skill | npx skills add Rieranthony/product-film-skill |
| `superalesha-3b3cf8` (Alexey Fateev) | https://github.com/alesha-pro/tools/tree/main/skills/hand-drawn-canvas-animation | — |
| `jake11moran-a269c4` (Jake Moran) | https://github.com/heygen-com/hyperframes-community-skills/tree/master/skills/session-story | npx skills add heygen-com/hyperframes-community-skills --skill session-story |
| `hanifproduktif-baade0` (Hanif | AI For Productivity) | https://github.com/buildwithhanif/claude-animation-skill | — |
| `buildfastwithai-53234e` (Build Fast with AI) | https://github.com/buildfastwithai/buildfast-skills/tree/main/generative-film-skill | npx skills add buildfastwithai/buildfast-skills --skill generative-film |
| `demitiyageekzen-d51652` (geekzen) | https://github.com/alchaincyf/huashu-art-motion | npx skills add alchaincyf/huashu-art-motion |

Three of them are already in our research: `buildwithhanif/claude-animation-skill`
(synthesis.md), `op7418/guizang-product-video-skill` (the 362-skill catalogue), and
`heygen-com/hyperframes-community-skills` — **a community skills repo for the renderer we
pin**.

## The directories

### Tools (339)

| Category | Count |
|---|---:|
| Skills | 173 |
| Articles | 65 |
| Editor | 48 |
| AI | 19 |
| Motion | 16 |
| Resources | 15 |
| Mockup | 3 |

Renderers named across the directory: Remotion 66, HyperFrames 45, ffmpeg 56, Manim 8,
Rive 7, After Effects 6, Canvas 5, Three.js 3, GSAP 3, Lottie 3, Motion Canvas 2, Blender 2,
Playwright 2, WebGL 1, Editframe 1, fframes 1, Premiere 1, CapCut 1, Puppeteer 1.

#### The 48 `Editor` entries — the peer set

| Tool | Pricing | What it says it is |
|---|---|---|
| [Aloud](https://www.aloud.sh/) | free | Feedback your coding agent can run |
| [Better Shot](https://www.bettershot.site/) | open-source | Capture clearly. Share beautifully. |
| [CapCut](https://www.capcut.com/) | freemium | All-in-one AI video editor |
| [ChatCut](https://chatcut.io/) | freemium | Your AI video editing assistant |
| [Clapper](https://clapper.archastro.tryintern.dev/) | open-source | Write videos in React. Render them to MP4. |
| [CleanShot](https://cleanshot.com/) | paid | There are average capture apps. And there's CleanShot. |
| [Clipchamp](https://clipchamp.com/en/) | freemium | Create professional videos in the browser |
| [Creavit Studio](https://creavit.studio/) | freemium | Recording, editing, and motion in one app |
| [CyberCut](https://cybercut.ai/) | paid | Let everyone easily create viral videos |
| [DemoFilm](https://www.demofilm.app/) | freemium | Raw recordings in. Cinematic demos out. |
| [Descript](https://www.descript.com/) | freemium | Edit video as easily as editing text |
| [Diffusion Studio](https://diffusion.studio/) | open-source | The video editor your agents can drive |
| [Dina](https://www.dina.so/) | freemium | Make every video worth watching |
| [EditDatVid](https://github.com/cneuralnetwork/editdatvid) | open-source | A private, browser-native video editor |
| [Editframe](https://editframe.com/) | freemium | Build Video With Code |
| [Evercut](https://evercut.app/) | paid | Capture, edit, and share anything on your screen |
| [FocuSee](https://focusee.imobie.com/) | paid | Auto-editing screen recorder for Windows and Mac |
| [HyperFrames](https://www.hyperframes.dev/) | open-source | The video editor built for agents |
| [Kinara](https://www.usekinara.com/) | paid | A local video editor for Claude and Codex |
| [Kite](https://kite.video/) | freemium | Turn screen recordings into feature films |
| [Lumary](https://lumary.app/) | paid | Your screen. Your studio. |
| [Matte](https://matte.app/) | freemium | Polished app demos in minutes |
| [Minshot](https://minshot.fehey.com/) | free | A native macOS screenshot tool |
| [Omascreen](https://omascreen.com/) | free | Make beautiful screen recordings with Omarchy |
| [OpenChatCut](https://github.com/0xsline/openchatcut) | open-source | An open video timeline your coding agent can edit |
| [OpenCut](https://opencut.app/) | open-source | The open source video editor |
| [OpenScreenShot](https://openscreenshot.app/) | open-source | Click. Captured. Done. |
| [OpenShot](https://www.openshot.org/) | open-source | Create stunning videos, easily |
| [OpusClip](https://www.opus.pro/) | freemium | 1 long video, 10 viral clips. Create 10x faster. |
| [PilotCut](https://www.pilotcut.com/) | freemium | The video editor that does the editing |
| [Powermove](https://trypowermove.com/) | open-source | Shape your video editor |
| [Prequel](https://prequel.sh/) | freemium | Create cinematic screen recordings from Mac |
| [Recordly](https://recordly.dev/) | open-source | Make beautiful screen recordings |
| [Screeen](https://screeen.co/) | freemium | Studio-quality screen recordings in minutes |
| [Screen Movie](https://screen.movie/) | paid | Create beautiful movies from your screen |
| [Screen Studio](https://screen.studio/) | paid | Beautiful screen recordings in minutes |
| [ScreenCam](https://thescreen.cam/) | paid | Capture sharp. Direct the frame. |
| [ScreenCursor](https://screencursor.com/) | paid | Screen recorder with auto zoom effects |
| [ScreenKite](https://screenkite.com/r/dingyi) | freemium | Record your screen. Let AI edit it. |
| [ScreenSage Pro](https://screensage.pro/) | freemium | Professional screen recordings, ready in minutes |
| [Screenflare](https://screenflare.app/) | freemium | Create screen recordings people want to watch |
| [Shotbase](https://shotbase.com/) | paid | Capture, edit, organize, and share in one place |
| [SmoothCut](https://smoothcut.studio/) | freemium | Automatic zooms, without a video editor |
| [VEED](https://www.veed.io/) | freemium | Create pro videos in the blink of AI |
| [Video Editor](https://videoeditor.fyi/) | free | A web screen recorder. No signups, no tracking |
| [Viewport](https://viewport.theresonance.studio/) | paid | Screen recording for designers |
| [What'Sub](https://whatsub.equal2.app/) | paid | Fast, accurate captions without the cloud |
| [fframes](https://fframes.studio/) | open-source | Video vibe coding framework that is actually fast |

**Eight of them are agent-first editors we have not tracked:**

| Tool | Pricing | Why it matters |
|---|---|---|
| [HyperFrames](https://www.hyperframes.dev/) | open-source | **our pinned runtime's vendor now ships a product editor** — see below |
| [OpenChatCut](https://github.com/0xsline/openchatcut) | open-source | an open video timeline your coding agent can edit |
| [Diffusion Studio](https://diffusion.studio/) | open-source | the video editor your agents can drive |
| [Kinara](https://www.usekinara.com/) | paid | a local video editor for Claude and Codex |
| [Aloud](https://www.aloud.sh/) | free | feedback your coding agent can run — an agent-facing review loop |
| [Editframe](https://editframe.com/) | freemium | build video with code; one of the pack's four renderers |
| [fframes](https://fframes.studio/) | open-source | video vibe coding framework; also one of the pack's four renderers |
| [Clapper](https://clapper.archastro.tryintern.dev/) | open-source | write videos in React, render them to MP4 |

Also new: `Powermove` ("shape your video editor"), `Mortiflix` ("a local motion studio, from
brief to animatic", open source), `launchvideo.dev` ("launch videos, animated by your agent"),
`PilotCut`, `CyberCut`, `Dina`, `SmoothCut` ("automatic zooms, without a video editor"),
`ScreenKite` ("record your screen. let AI edit it"), `Aloud`.

### Studios and designers (49)

49 entries: agencies and independent motion designers for product films, brand work,
UI motion, explainers and 3D. `specialties` values seen: `product-films`, `brand`,
`ui-motion`, `explainers`. This is the human supply side of the same market.

### The `Skills` category: 173 entries

A registry of agent skills with a name, a URL and, for many, an `npx skills add` line.
Names include **Remotion Skills** (official), Lemo-Opuscar, Guizang Product Video, Motion
Graphics Skills, MiniMax H3 Video Reverse, Huashu Art Motion, One Prompt Video, Director,
Whiteboard Video Skill, Oil Motion, Jacky Motion, motionmaxxing, Product Launch Motion.

## The skill pack

Sold separately. What it contains, in the site's words:

| Part | What it is |
|---|---|
| Agent skills | instruction files covering storyboarding, choreography, pacing and typography in motion, loaded by your agent only when it needs them |
| Scene starters | ready-to-fork compositions for showreels, title sequences, launch films and changelog clips |
| Motion tokens | spring, easing and timing values tuned for motion design, kept in one place |
| Renderer guides | setup notes for **Remotion, HyperFrames, Editframe and fframes**, so the same scene plan works with your stack |

How it works: accept the GitHub invite and copy the skills folder into your project;
describe the piece in plain words; the agent storyboards the beats and timing before writing
any scene code; it animates each scene with your fonts, colours and components and renders
locally; you ask for tweaks in plain words (*"hold the logo longer"*) and render again.

*"You stay in charge of the cut. Review the storyboard, ask for changes in plain words, and
render again."* — the same chat-based, plain-words feedback model AeroEdits chose on 24 Oct.

## HyperFrames Studio — the finding that matters most

AeroEdits pins [heygen-com/hyperframes](https://github.com/heygen-com/hyperframes) 0.8.162 as
a **renderer library**. That project now also ships a product:

> **HyperFrames Studio** — "The video editor built for agents. Built by the team who open
> sourced HyperFrames." — [hyperframes.studio](https://hyperframes.studio), by **HeyGen**.

Its own page reports **73,765 GitHub stars, 6,689 forks, 121 contributors and 809,606 npm
downloads a week**. The open-source project we pinned is the same codebase, and the vendor
that owns it now has a commercial editor aimed squarely at agents.

That is a different situation from every other entry in this research: not a competitor that
might build our product, but **our dependency's owner already has it, aimed at our buyers**.

## Caveats

- **Every claim is the vendor's.** Nothing here was installed, run or benchmarked by us.
- **The catalogue is scraped, not authored.** Each entry pairs a video with a prompt and a
  `tweetUrl` to the creator's post; the prompt is presented as the creator's own. The site
  also accepts submissions, so the corpus is a mix.
- **The dedup is incomplete** (six groups of identical prompts survive), so counts here are
  upper bounds on distinct work.
- **Model and stack are recorded for a minority of entries** — 132 of 500 carry a stack, 39 a
  model is not stated; the model field is populated on 491 of 500 but 9 are unnamed.
- **The `iterations` field is on 117 of 500** and its meaning is the site's own label.
- **`api.motionvideo.xyz` does not resolve** and `/mcp` returns 404, so the machine-facing
  API and MCP server described in the repo's `docs/api-migration.md` are not live yet.
- **The skill pack is behind a paywall**; we read its marketing page, not its files.
- **The site is young**: created 11 Oct 2026 for the repo, first videos dated 29 Aug 2026.

## Re-fetching

```bash
curl -o catalog.json https://motionvideo.xyz/api/catalog
curl -o directories.json https://motionvideo.xyz/api/catalog/directories
git clone --depth 1 https://github.com/motionvideohq/motionvideo.xyz.git
```
