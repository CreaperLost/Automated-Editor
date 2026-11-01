# awesome-claude-video-skills — catalogue and measured test

Pulled 27 October 2026 from <https://github.com/zhuyansen/awesome-claude-video-skills>.
Repo commit `b69fc040b38b` (2026-10-27T15:05:56Z) — Daily update 2026-10-27.
The list itself is generated: `data/skills.json` carries a `generated` stamp of **2026-10-27** and names its own source as `https://agentskillshub.top/best/claude-video-skills/`.

Two things are in here: the **catalogue** (362 repos, ten types, each security-graded) and
the **measured test** the maintainer ran on 16 of them. The test is the valuable part — it is
the controlled experiment our own creator-video research said did not exist.

## Where the test comes from

The test harness lives in a **separate repo**, `zhuyansen/agent-skills-hub`, under `ops/video-runs/`:
`in/brief.md` (the brief), `score_video.py` (the four checks), `measure_video.py` (the file
metrics), `evidence.py` (the per-run evidence pages), `results.json` and `RESULTS.md`.

## The brief (verbatim)

```text
Brief: a 15-second promo video for Tidepool

Tidepool is an app that tells swimmers whether the water at their local beach is clean enough
to swim in today. Make a 15-second promo for it. Use the on-screen copy below exactly, in
this order; the look and the motion are yours.

## On-screen copy, in order

1. Know before you swim.
2. Water quality at 2,400 beaches, checked every morning.
3. Good. Fair. Stay out.
4. Rain warnings tell you how long to wait.
5. Tidepool. Check my beach.

## Requirements

- One file: `video.mp4`, H.264, 1920 x 1080, 30 frames per second, 15 seconds long
  (14.5 to 15.5 is fine).
- Every line of the copy is on screen long enough to read. No other sentences, taglines,
  logos of real companies or claims.
- No voice-over. Music or sound is optional; if you add any, make it in code or with tools
  already on this machine.
- No stock footage or downloaded images are needed: draw with code, shapes, type and colour.
- This machine has Node 22, Python 3, Chromium (/usr/bin/chromium) and ffmpeg. There are no API
  keys for video, image or voice models.
```

## The four checks (from `score_video.py`)

Every check is read from the delivered file, not from a model's opinion:

| Check | How | Threshold |
|---|---|---|
| `playable` | ffprobe opens the file | — |
| `spec` | ffprobe | 14.5–15.5 s, 1920x1080, 30 fps, h264 |
| `copy` | a VLM reads the five lines from frames sampled at 2/s; 3 passes, majority vote; the reader is calibrated on the control video first | whole line legible in ≥1 sampled frame |
| `moves` | mean frame difference over sampled frames | > 0.004, and not mostly dark frames |

The script's own comment: *"How good a video looks is not scored: the videos are on the
evidence pages."* A VLM's quality opinion is deliberately kept secondary.

## The measured results (from `results.json`)

`Held frames` is `still_share`: the fraction of sampled frame *pairs* that were identical.
`Motion` is `mean_change`. `Text added` is any on-screen sentence that was not in the brief.

_Results table transcribed from `RESULTS.md` (the JSON was fetched read-only; see the
provenance note below)._

| Skill | ★ | Checks | Lines readable | Sound | Held frames | Motion | Minutes |
|---|---:|---:|---:|---|---:|---:|---:|
| octocat/Hello-World (control, no skill) | 0 | 4/4 | 5/5 | yes | 3% | 0.0247 | 4.4 |
| vincentsch/explainroo | 523 | 4/4 | 5/5 | yes | 0% | 0.0140 | 2.0 |
| wshuyi/remotion-video-skill | 394 | 4/4 | 5/5 | no | 10% | 0.0113 | 3.4 |
| vibe-motion/skills | 1,315 | 4/4 | 5/5 | yes | 7% | 0.0710 | 4.4 |
| remotion-dev/skills | 5,005 | 4/4 | 5/5 | no | 34% | 0.0238 | 4.5 |
| Barty-Bart/motion-graphics | 580 | 4/4 | 5/5 | no | 48% | 0.0382 | 4.7 |
| digitalsamba/claude-code-video-toolkit | 2,198 | 4/4 | 5/5 | yes | 24% | 0.0125 | 5.5 |
| diffusionstudio/lottie | 5,493 | 4/4 | 5/5 | no | 34% | 0.0103 | 5.6 |
| hypit-ai/hypit | 20,799 | 4/4 | 5/5 | yes | 0% | 0.0266 | 6.5 |
| iart-ai/motion-skills | 610 | 4/4 | 5/5 | yes | 10% | 0.0579 | 6.5 |
| charlie947/motion-graphics-skills | 174 | 4/4 | 5/5 | yes | 28% | 0.0167 | 7.0 |
| howseen-ai/claude-motion-design | 408 | 4/4 | 5/5 | yes | 14% | 0.0368 | 7.9 |
| feitangyuan/onetake | 2,080 | 4/4 | 5/5 | yes | 45% | 0.0080 | 8.5 |
| calesthio/OpenMontage | 64,239 | 4/4 | 5/5 | no | 14% | 0.0935 | 8.6 |
| op7418/guizang-product-video-skill | 762 | 4/4 | 5/5 | yes | 7% | 0.0728 | 8.6 |
| echris6/motion-video-kit | 1,080 | 4/4 | 5/5 | no | 3% | 0.0453 | 21.8 |
| heygen-com/hyperframes | 60,291 | 4/4 | 5/5 | no | 38% | 0.0108 | 23.5 |

Every one of the seventeen runs — sixteen skills plus the no-skill control — passed all four
checks, had all five lines readable, and added no text that was not in the brief.

## The academic grounding (`SOURCES.md`)

The method is not invented; each check is tied to published work, read at source on
2026-10-11.

| Source | What it gives |
|---|---|
| TheoremExplainAgent (Ku et al., ACL 2025) · arXiv 2502.19400 | Render success is the first metric (93.8% for o3-mini); automatic metric vs human Spearman falls from 0.72 (visual relevance) to 0.14–0.17 (Table 4). |
| Code2Video (Chen et al., 2025) · arXiv 2510.01174 | Code-rendered video beats pixel video models on layout and teaching value (Table 1); people penalise layout errors more than the VLM judge does (Sec. 5.3). |
| MoVer (Ma and Agrawala, SIGGRAPH 2025) · arXiv 2502.13372 | 58.8% of animations correct with no iteration, 93.6% with up to 50 corrections — report first try and after retries apart. |
| Evaluating Design Video Generation (Deganutti et al., ICML 2026 workshop) · arXiv 2605.16223 | Text recoverability: OCR on frames at 2 fps, exact match in ≥1 frame; OCR reads only 0.659 of perfect renders, so calibrate on a known-good render (Sec. 4.5, Table 7). |
| Graphic-Design-Bench (Deganutti et al., 2026) · arXiv 2604.04192 | VLMs misjudge duration by about 6 s and classify motion type below 13%: measure duration and motion from the file, not with a model. |
| VBench (Huang et al., 2023) · arXiv 2311.17982 | Motion must be measured beside consistency: a static video scores well on consistency alone. |
| SkillsBench (Li et al., 2026 v4) · arXiv 2602.12670 | Same task with and without the skill, deterministic verifiers; curated skills +16.6 points. |
| HeyGen Code2Video Bench (vendor post, 2026-09-18) | Not peer reviewed; the author builds the renderer and the judge. General VLM judges agreed with people 75% of the time (self-reported). |

## The catalogue

| Type | Count |
|---|---:|
| Promo & demos | 69 |
| Frameworks & toolkits | 64 |
| Explainers | 64 |
| Editing | 47 |
| Shorts & social | 42 |
| Motion graphics | 25 |
| Stories & animation | 20 |
| Music videos | 13 |
| Scripts & learning | 10 |
| Avatars | 8 |
| **Total** | **362** |

Security grades: **353 safe**, **9 caution**. The grade is of the
repo's README and install steps, not of the code. *No entry is graded unsafe.*

Language: 138 Python, 86 JavaScript, 73 TypeScript, 33 HTML, 6 Shell, 2 Go, 1 Rust, 1 Swift, 1 PowerShell, 1 Go Template, 1 CSS.

Licence: 236 MIT, 45 NOASSERTION, 26 Apache-2.0, 8 AGPL-3.0, 2 MIT-0, 1 CC-BY-SA-4.0.

Stars: 272,634 total; median 25; 3 above 10k, 26 in 1k–10k, 74 in 100–1k, 225 in 5–50.

### The Editing type, by stars (the ones shaped like an editor)

| Repo | ★ | Grade | Language | What it does |
|---|---:|---|---|---|
| [FireRedTeam/FireRed-OpenStoryline](https://github.com/FireRedTeam/FireRed-OpenStoryline) | 3,568 | safe | Python | FireRed-OpenStoryline is an AI video editing agent that transforms manual editing into intention-driven directing through natural language interaction, LLM-powered planning, and precise tool orchestration. It facilitates transparent, human-in-the-loop creation with reusable Style Skills for consistent, professional storytelling. |
| [Agentchengfeng/chengfeng-videocut-skills](https://github.com/Agentchengfeng/chengfeng-videocut-skills) | 3,142 | safe | JavaScript | 用 Claude Code Skills 做的视频剪辑 Agent |
| [0xsline/OpenChatCut](https://github.com/0xsline/OpenChatCut) | 2,715 | safe | TypeScript | Open-source, local-first conversational AI video editor with a professional multi-track timeline, Agent Skills, MCP integration, and Remotion rendering. |
| [JimLiu/baocut](https://github.com/JimLiu/baocut) | 1,545 | safe | TypeScript | BaoCut: an AI video agent with an editable timeline. Transcribe, subtitle, translate, dub, cut and animate. |
| [Barty-Bart/motion-graphics](https://github.com/Barty-Bart/motion-graphics) | 883 | safe | HTML | Motion-graphics skills for Claude Code and Codex. |
| [cartesiancs/cartcut](https://github.com/cartesiancs/cartcut) | 818 | safe | TypeScript | Video Editor for AI agents, built on the belief that open source can beat commercial tools |
| [hetpatel-11/Adobe_Premiere_Pro_MCP](https://github.com/hetpatel-11/Adobe_Premiere_Pro_MCP) | 768 | safe | TypeScript | Adobe Premiere Pro MCP. Tools for AI-driven video editing via MCP, for Codex, Claude, and other MCP clients. |
| [zenstory-ai/video-recap-skills](https://github.com/zenstory-ai/video-recap-skills) | 592 | safe | Python | Claude Code / Codex skills that turn a video into a Chinese narration recap (视频解说): scene detection, ASR, VLM, script, TTS, ffmpeg assembly, optional editable JianYing / CapCut draft export (剪映草稿导出). Local ffmpeg + one MiMo key, no GPU. \| 用 Claude Code skills 把视频做成中文解说成片，可选一键导出可编辑剪映草稿。 |
| [leancoderkavy/premiere-pro-mcp](https://github.com/leancoderkavy/premiere-pro-mcp) | 440 | safe | TypeScript | Adobe Premiere Pro MCP — independent, local-first server for supported workflows. Connect Claude, Codex, or Cursor through a local CEP bridge. |
| [YeJe-cpu/SeeCut](https://github.com/YeJe-cpu/SeeCut) | 394 | safe | JavaScript | An AI editor that watches its own cut: talking-head / AI-avatar video → auto-edited short video. 网感口播精剪：数字人/真人口播 → AI 自动精剪成网感动效短视频 → 可选剪映分层草稿 |
| [hassancs91/claude-youtube-editor](https://github.com/hassancs91/claude-youtube-editor) | 351 | safe | TypeScript | Record the talking head, Claude Code does the rest: the cut, the visuals, the voice, the sound effects, the thumbnail, and the YouTube upload. Every screen moment is built as Remotion TSX code, never screen-recorded. |
| [AgriciDaniel/claude-shorts](https://github.com/AgriciDaniel/claude-shorts) | 240 | safe | Python | Interactive longform-to-shortform video creator — Claude Code skill with Remotion-rendered animated captions, AI segment scoring, cursor tracking, and audio-aware boundary snapping |
| [erduo1998-cell/erduo-broll-loop-engineering](https://github.com/erduo1998-cell/erduo-broll-loop-engineering) | 232 | safe | TypeScript | SRT 驱动的双后端 B-roll Agent Skill：自动路由 HyperFrames / Remotion，集成 152 张 Shotcraft 镜头卡 |
| [znyupup/ai-video-editing-skill](https://github.com/znyupup/ai-video-editing-skill) | 166 | safe | Python | AI Agent Skill for automated vlog editing. Feed raw footage, get a finished video. Powered by ffmpeg + Whisper + Vision API. |
| [blixvip/easyedit](https://github.com/blixvip/easyedit) | 162 | safe | Python | Type a movie name and get a fan edit: a captioned speech cut, then a montage cut to the beat. |
| [naive-kun/naive-video-skill](https://github.com/naive-kun/naive-video-skill) | 135 | safe | Python | A Codex skill for turning talking-head videos into captioned, animated final videos. |
| [ayushozha/AdobePremiereProMCP](https://github.com/ayushozha/AdobePremiereProMCP) | 128 | safe | Go | An open-source bridge between AI assistants and Adobe Premiere Pro, for supported editing workflows. Go, Rust, Python, and TypeScript. |
| [Cassette-Editor/oh-my-cassette](https://github.com/Cassette-Editor/oh-my-cassette) | 121 | safe | Python | 你的随身 AI 剪辑搭档 \| Pocket AI co-editor for video montage — AI video editing plugin & MCP server for Claude Code, Codex, Hermes & OpenCode |
| [Monet-AI-Editor/Monet](https://github.com/Monet-AI-Editor/Monet) | 119 | safe | TypeScript | Edit Videos and Design Images with Claude code or Codex |
| [louisedesadeleer/cut-video](https://github.com/louisedesadeleer/cut-video) | 114 | safe | Python | Claude Code skill: tighten long recordings — remove silences, ums, dead air. Preserves laughs and comedic pauses. Fast on Apple Silicon. |
| [AKMessi/vex](https://github.com/AKMessi/vex) | 95 | safe | Python | claude code for video editing |
| [Xiangyu-CAS/rea-capcut](https://github.com/Xiangyu-CAS/rea-capcut) | 89 | safe | TypeScript | ReaCut — REA × CapCut: an independent, open-source video editor rebuilt from REA-assisted, authorized local behavior and format research. No copied CapCut proprietary code or assets. Offline desktop app + Codex plugin. |
| [JUNKDOGE-JOE/after-effects-mcp](https://github.com/JUNKDOGE-JOE/after-effects-mcp) | 87 | safe | JavaScript | AI-powered Adobe After Effects automation via MCP. Chat in a CEP panel or connect external agents, with 16 tools for editing, previews, recovery, reusable scripts, and multi-instance workflows. |
| [kurbaitaev/ghost-editor](https://github.com/kurbaitaev/ghost-editor) | 81 | safe | Python | AI video editor for talking-head reels: 7 styles, face-safe captions, motion scenes, reverse-engineer any reference edit. A Claude Code / agent skill on HyperFrames. |
| [krusemediallc/video-editor-agent](https://github.com/krusemediallc/video-editor-agent) | 50 | safe | Python | Claude Code skill pack: edit short-form videos end to end — style-clone a reference reel, build a branded motion-graphics edit, ElevenLabs sound design, frame-level QA, timeline-comment review loop |
| [hahadu4520/vlog-cut](https://github.com/hahadu4520/vlog-cut) | 49 | safe | Python | Narration-driven video editing pipeline for Claude Code · 给 Claude Code 用的「按文案剪辑」流水线 |
| [alisadiq-ai/favstash-ai-video-editing-skills](https://github.com/alisadiq-ai/favstash-ai-video-editing-skills) | 46 | safe | JavaScript | Create Instagram Reels, TikToks and YouTube Shorts with adaptive AI editing, captions and motion graphics. Connect FavStash for planning, publishing and analytics. |
| [ops120/video-recap-skills-plus](https://github.com/ops120/video-recap-skills-plus) | 45 | caution | Python | Clip any video into a narration recap with claude code skill｜用claude code skill把任何视频剪辑成中文解说视频，支持剪映导出 |
| [darrenli6/JJKoubo](https://github.com/darrenli6/JJKoubo) | 40 | safe | JavaScript | JJ Koubo (JJ口播) is a local-first collection of video editing Skills for talking-head, interview, knowledge-sharing, and other videos centered on spoken content. The Skills can be used through Codex or other compatible Agent environments. |
| [Kappaemme-git/codex-video-short-maker-skill](https://github.com/Kappaemme-git/codex-video-short-maker-skill) | 35 | safe | Python |  |
| [skailabinfo/claude-video-harness](https://github.com/skailabinfo/claude-video-harness) | 21 | safe | TypeScript | Монтаж вертикальных роликов «говорящая голова» в Claude Code + Remotion: клонируй, ответь на вопросы — пайплайн настроится под тебя |
| [ZiadAbdelkarim/beat-synced-edit](https://github.com/ZiadAbdelkarim/beat-synced-edit) | 14 | safe | Python | Automatic beat-synced video editing: feed it a song and raw footage, it analyzes beats, energy, and scenes, then cuts a ready-to-post edit. Pure Python/ffmpeg CLI, plus a bundled Claude Code skill for natural-language editing. |
| [jincheng2026/jc-remotion-skills](https://github.com/jincheng2026/jc-remotion-skills) | 14 | safe | TypeScript | Remotion Koubo Skill — 口播视频的 AI 成片工作流（Codex / Claude Code 双端）：粗剪+SRT 进，带 MG 包装、音效、母带的成片出 |
| [PoetCoderJun/MotionTalk](https://github.com/PoetCoderJun/MotionTalk) | 12 | safe | Python | Speak your ideas. Turn a recording into a concise, polished video with your Agent. 保留原声，让好想法变成好视频。 |
| [cocolayuan/videoclip-AI-skill](https://github.com/cocolayuan/videoclip-AI-skill) | 12 | safe | Python | Claude Skill-powered video editing toolkit - 视频全自动剪辑 |
| [onuryildriim/claude-kurgu-atolyesi](https://github.com/onuryildriim/claude-kurgu-atolyesi) | 12 | safe | JavaScript | Claude Code ile konuşan-kafa YouTube videolarını kodla kurgulama atölyesi — Remotion + FFmpeg, kendi marka kimliğinle. |
| [davidbfrogman/davecto-cutroom](https://github.com/davidbfrogman/davecto-cutroom) | 11 | safe | JavaScript | Edit your talking-head videos with Claude: a local editor, a notes loop, captions and animations in your own brand. Claude Code plugin for macOS. |
| [misbahsy/tiktok-ig-shorts](https://github.com/misbahsy/tiktok-ig-shorts) | 11 | safe | Python | Generate Viral Tiktok and Instagram Shorts using Hyperframes with Claude and Codex |
| [Fagan1024/smart-video-editor](https://github.com/Fagan1024/smart-video-editor) | 10 | safe | Python | AI video editing skill that watches your footage before cutting. Vision model analyzes every frame, scores usability, drops bad takes, reorders by narrative, then renders with ffmpeg. For Claude Code, Cola, OpenClaw. 会看画面的 AI 自动剪辑 Skill——先用视觉模型看懂素材再决定剪哪几秒。 |
| [nomicsadmin/editor-video-ia](https://github.com/nomicsadmin/editor-video-ia) | 10 | safe | JavaScript | Editor de vídeo dentro do Claude Code: manda o vídeo, o briefing e a referência, recebe o reels editado. Corte pela fala, legenda palavra a palavra, telas na palavra certa. Roda no seu Mac. |
| [qingyunAGI/qingyun-cine-skill](https://github.com/qingyunAGI/qingyun-cine-skill) | 10 | safe | Python | qingyun-cine-skill: PLAN-FIRST cinematic video editing Codex skill for trailers, beat-sync edits, promos, and highlights. |
| [rahlquist/hermes-video-toolchain](https://github.com/rahlquist/hermes-video-toolchain) | 8 | caution | Shell | Complete free/open-source video production pipeline for Hermes Agent: OpenScreen capture + HyperFrames authoring + headless post-processing, wired into one skill. |
| [griffinwooldridge/talking-head-editor](https://github.com/griffinwooldridge/talking-head-editor) | 7 | safe | JavaScript | A Claude skill that edits raw talking-head footage: tight cuts, clean voice, captions and motion graphics as code |
| [brookejlacey/boring-tools](https://github.com/brookejlacey/boring-tools) | 6 | safe | Python | Agent-driven video editing where the edit is text. No timeline, no project file, nothing uploaded. ffmpeg + whisper + python, and a SKILL.md your agent reads. |
| [hahadu4520/newcut-video-clipping-skill](https://github.com/hahadu4520/newcut-video-clipping-skill) | 6 | safe | JavaScript | Open-source Codex skill for semantic Chinese video clipping with hooks, captions, cleanup, and FFmpeg rendering |
| [jackjls/jtxvideo-skill](https://github.com/jackjls/jtxvideo-skill) | 5 | safe | Python | Codex + HyperFrames 口播视频制作 Skill：按 SRT、结构、分镜、设计、渲染的审核流程生成 AI 金融短视频 |
| [vincentventalon/claude-code-video-editing-skill](https://github.com/vincentventalon/claude-code-video-editing-skill) | 5 | safe | Python | Les skills Claude Code que j'utilise vraiment sur mes sites et mes vidéos, une nouvelle chaque mardi (montage vidéo, SEO Search Console…) |

### The full catalogue, all 362

| Repo | ★ | Type | Grade | Language | Licence | Description |
|---|---:|---|---|---|---|---|
| [heygen-com/hyperframes](https://github.com/heygen-com/hyperframes) | 73,569 | general | safe | TypeScript | Apache-2.0 | Write HTML. Render video. Built for agents. |
| [calesthio/OpenMontage](https://github.com/calesthio/OpenMontage) | 72,338 | general | safe | Python | AGPL-3.0 | World's first open-source, agentic video production system. 12 production pipelines, 100+ tools, 700+ agent skill and production-knowledge files. Turn your AI coding assistant into a full video production studio. |
| [hypit-ai/hypit](https://github.com/hypit-ai/hypit) | 26,010 | general | safe | TypeScript | NOASSERTION | Clone any viral video with AI agents. Not just a script, the whole workflow: swap the face, the words, the B-roll, ship 100 variants in one command, and get your 100M views. |
| [lemomo-ai/lemo-opuscar](https://github.com/lemomo-ai/lemo-opuscar) | 8,439 | story | safe | JavaScript | MIT | A toolbox of film styles made in code for Claude Code: 59 ready to use, and your own. Each style is a style prompt plus a demo film made entirely in code by Claude Opus 5.5, with no video model. By Lemomo. · 一套用代码做片的风格工具箱：59 种现成风格，也能做出你自己的 |
| [EverMind-AI/Raven](https://github.com/EverMind-AI/Raven) | 5,869 | explainer | safe | Python | Apache-2.0 | Direct a two-minute hand-drawn animated film from a repository's Git history: story, character, storyboard, HTML review studio, then a 4K/60fps MP4. |
| [remotion-dev/skills](https://github.com/remotion-dev/skills) | 5,774 | general | safe | TypeScript | — | Agent Skills |
| [diffusionstudio/lottie](https://github.com/diffusionstudio/lottie) | 5,632 | motion | safe | TypeScript | MIT | Generate production-ready Lottie animations with Claude Code or Codex |
| [FireRedTeam/FireRed-OpenStoryline](https://github.com/FireRedTeam/FireRed-OpenStoryline) | 3,568 | editing | safe | Python | Apache-2.0 | FireRed-OpenStoryline is an AI video editing agent that transforms manual editing into intention-driven directing through natural language interaction, LLM-powered planning, and precise tool orchestration. It facilitates transparent, human-in-the-loop creation with reusable Style Skills for consistent, professional storytelling. |
| [feitangyuan/onetake](https://github.com/feitangyuan/onetake) | 3,356 | promo | safe | Python | NOASSERTION | Motion films that never cut to the next slide: every beat grows out of the one before, one continuous camera, continuity measured by an oracle. Product launch films and feature demos, made with a coding agent. |
| [Agentchengfeng/chengfeng-videocut-skills](https://github.com/Agentchengfeng/chengfeng-videocut-skills) | 3,142 | editing | safe | JavaScript | Apache-2.0 | 用 Claude Code Skills 做的视频剪辑 Agent |
| [NarratorAI-Studio/narrator-ai-cli-skill](https://github.com/NarratorAI-Studio/narrator-ai-cli-skill) | 3,077 | general | safe | — | MIT | AI 解说大师 — Agent skill；封装 narrator-ai-cli 供 Claude/Codex 等工具调用 |
| [cclank/lanshu-create-ai-presenter-video](https://github.com/cclank/lanshu-create-ai-presenter-video) | 2,819 | avatar | safe | HTML | MIT | Provider-neutral Codex Skill for producing verified AI presenter videos from a script and an authorized presenter image. |
| [0xsline/OpenChatCut](https://github.com/0xsline/OpenChatCut) | 2,715 | editing | safe | TypeScript | AGPL-3.0 | Open-source, local-first conversational AI video editor with a professional multi-track timeline, Agent Skills, MCP integration, and Remotion rendering. |
| [JohnHeibel/PDoomVideo](https://github.com/JohnHeibel/PDoomVideo) | 2,642 | music | safe | JavaScript | — | Source code for the Claude Opus 5.5 music video for I'm Upping My P(doom) |
| [nolangz/pixel2motion](https://github.com/nolangz/pixel2motion) | 2,522 | motion | safe | Python | MIT | AI logo animation skill: turn raster logos into smooth SVG animation, animated HTML demos, GIF/video previews, and motion QA evidence. |
| [Alisa0808/vox-director](https://github.com/Alisa0808/vox-director) | 2,487 | explainer | safe | Python | MIT | Turn one topic into a finished Vox-style paper-collage explainer/ad video — automated end to end on Atlas Cloud + ffmpeg. An agent skill. |
| [digitalsamba/claude-code-video-toolkit](https://github.com/digitalsamba/claude-code-video-toolkit) | 2,355 | general | safe | Python | MIT | AI-native video production toolkit for Claude Code |
| [gnipbao/story-to-handdrawn-video](https://github.com/gnipbao/story-to-handdrawn-video) | 2,307 | story | safe | HTML | MIT | Agent skill: convert Chinese story copy or ordered images into a hand-drawn diary-comic animation (silent MP4 picture track). |
| [JohnHeibel/ClaudeAnimationBase](https://github.com/JohnHeibel/ClaudeAnimationBase) | 2,090 | story | safe | JavaScript | MIT | A starter kit for animating hand-painted cartoons with Claude: p5.js + p5.brush, the Clawd character, 31 acted emotions and a guide for the model |
| [jtydhr88/screenwriting-skills](https://github.com/jtydhr88/screenwriting-skills) | 1,836 | craft | safe | Python | MIT | Professional agent skills for screenwriting, television writing and dramaturgy |
| [JimLiu/baocut](https://github.com/JimLiu/baocut) | 1,545 | editing | safe | TypeScript | NOASSERTION | BaoCut: an AI video agent with an editable timeline. Transcribe, subtitle, translate, dub, cut and animate. |
| [pyang5166/gbro-collage-broll](https://github.com/pyang5166/gbro-collage-broll) | 1,539 | motion | safe | Python | MIT | 半调纸拼贴 B-roll 生成 skill：三闸门审批，Gemini Omni Flash 首尾帧组装动画 \| Editorial halftone paper-collage B-roll agent skill |
| [vibe-motion/skills](https://github.com/vibe-motion/skills) | 1,406 | general | safe | Python | — | agent skills for vibe motion |
| [echris6/motion-video-kit](https://github.com/echris6/motion-video-kit) | 1,235 | promo | safe | Python | MIT | Claude Code skill kit for premium AI-assisted business videos: independent critic loop, motion principles from 28 launch films, quality bar, sound design, business offers, Three.js patterns, scripts |
| [adithya-s-k/manim_skill](https://github.com/adithya-s-k/manim_skill) | 1,221 | explainer | safe | Python | MIT | Agent skills for Manim to create 3Blue1Brown style animations. |
| [eternityspring/reelbench-skills](https://github.com/eternityspring/reelbench-skills) | 1,171 | craft | safe | JavaScript | Apache-2.0 | Learning notes and tooling skills for AI video - AI 视频相关的学习与工具 skill |
| [alexgreensh/anidoodle](https://github.com/alexgreensh/anidoodle) | 1,127 | motion | safe | TypeScript | Apache-2.0 | Art and animation, written as code. Illustrations, loops, interactive web art, launch-videos and scored films in dozens of styles, identical on every render. |
| [Tejashmakwana/motionmaxxing](https://github.com/Tejashmakwana/motionmaxxing) | 1,075 | promo | safe | JavaScript | Apache-2.0 | Looksmaxxing for motion graphics. An agent skill that turns AI slop videos into films that look made by a motion designer. |
| [iart-ai/motion-skills](https://github.com/iart-ai/motion-skills) | 1,033 | general | safe | HTML | MIT | 50 open-source skills that teach your AI coding agent to make motion graphics, animation & video — kinetic typography, data-viz, explainers, TikTok/Reels, WebGL, Manim. 14 installable packs. By iart.ai, the AI motion agent. |
| [Barty-Bart/motion-graphics](https://github.com/Barty-Bart/motion-graphics) | 883 | editing | safe | HTML | NOASSERTION | Motion-graphics skills for Claude Code and Codex. |
| [op7418/guizang-product-video-skill](https://github.com/op7418/guizang-product-video-skill) | 874 | promo | safe | Python | AGPL-3.0 | 归藏 product video skill：复用真实产品组件和设计语言，用代码制作软件更新宣传片。包含分镜文案、原创配乐、动作音效与视频渲染，支持 Claude Code 和 Codex。 |
| [cartesiancs/cartcut](https://github.com/cartesiancs/cartcut) | 818 | editing | safe | TypeScript | MIT | Video Editor for AI agents, built on the belief that open source can beat commercial tools |
| [ythx-101/live-panel-skill](https://github.com/ythx-101/live-panel-skill) | 809 | motion | safe | JavaScript | MIT | Config-driven animated architecture diagrams: turn one JSON file into a terminal-style, always-running diagram or a light-theme infographic that moves. Outputs H.264 mp4 or a live web page; also a Claude Code style skill (SKILL.md). |
| [hetpatel-11/Adobe_Premiere_Pro_MCP](https://github.com/hetpatel-11/Adobe_Premiere_Pro_MCP) | 768 | editing | safe | TypeScript | MIT | Adobe Premiere Pro MCP. Tools for AI-driven video editing via MCP, for Codex, Claude, and other MCP clients. |
| [bangtutorial/bang-motion](https://github.com/bangtutorial/bang-motion) | 736 | general | safe | HTML | MIT | Agent skill for browser motion graphics — openers, promos, bumpers, kinetic typography, and five explainer styles that move like video, not slides. Single index.html output. Works with Claude Code, Codex, Gemini CLI, Cursor. |
| [howseen-ai/claude-motion-design](https://github.com/howseen-ai/claude-motion-design) | 701 | promo | caution | Python | MIT | A Claude Code skill to make motion design videos in pure code: HTML + Playwright + ffmpeg. No After Effects, no Remotion license. |
| [vincentsch/explainroo](https://github.com/vincentsch/explainroo) | 663 | explainer | safe | JavaScript | MIT | Explainer videos and product demos made by your AI agent. Free and open source: a local voice (Kokoro), word timing (Whisper) and a canvas renderer turn a script into a narrated MP4. |
| [GTKottman/mortiflix-oss](https://github.com/GTKottman/mortiflix-oss) | 657 | explainer | safe | JavaScript | AGPL-3.0 | A motion design studio on your own machine: Claude makes the video step by step, you approve every stage. Bring your own Claude Code or API key. |
| [zenstory-ai/video-recap-skills](https://github.com/zenstory-ai/video-recap-skills) | 592 | editing | safe | Python | MIT | Claude Code / Codex skills that turn a video into a Chinese narration recap (视频解说): scene detection, ASR, VLM, script, TTS, ffmpeg assembly, optional editable JianYing / CapCut draft export (剪映草稿导出). Local ffmpeg + one MiMo key, no GPU. \| 用 Claude Code skills 把视频做成中文解说成片，可选一键导出可编辑剪映草稿。 |
| [adunext/adu-motion-video](https://github.com/adunext/adu-motion-video) | 572 | shorts | safe | JavaScript | MIT | 阿杜的 AI 剪辑 Skill｜全系列模板已升级为 MotionGit .motion 开放格式，下载后直接发给助手即可创作。下载：https://motiongit.com/@adunext ｜精选阿杜与 Opus 5.5 制作的代码级动画模板，适合口播、知识讲解，支持多平台、多助手。 |
| [heygen-com/skills](https://github.com/heygen-com/skills) | 521 | avatar | safe | Shell | MIT | HeyGen AI agent skills — avatar creation and video production via the v3 Video Agent pipeline |
| [geekjourneyx/hyperframes-motion-director](https://github.com/geekjourneyx/hyperframes-motion-director) | 468 | promo | safe | JavaScript | AGPL-3.0 | Agent Skill for Chinese-first HyperFrames motion-video production from articles, products, websites, and README files. |
| [leancoderkavy/premiere-pro-mcp](https://github.com/leancoderkavy/premiere-pro-mcp) | 440 | editing | safe | TypeScript | MIT | Adobe Premiere Pro MCP — independent, local-first server for supported workflows. Connect Claude, Codex, or Cursor through a local CEP bridge. |
| [wshuyi/remotion-video-skill](https://github.com/wshuyi/remotion-video-skill) | 433 | explainer | caution | Python | — | A Claude Code Skill for creating programmatic videos with Remotion framework |
| [hi-nikola/hand-drawn-explainer-video-nikola](https://github.com/hi-nikola/hand-drawn-explainer-video-nikola) | 431 | explainer | safe | Python | Apache-2.0 | 中文手绘知识讲解视频 Codex Skill：逐笔故事、双语义岛、让怪诞小黑动起来与程序动画 |
| [charlie947/motion-graphics-skills](https://github.com/charlie947/motion-graphics-skills) | 430 | promo | safe | Shell | MIT | 13 Claude Code skills for launch-grade motion graphics. Every frame is code. No After Effects, no video generator. |
| [actionow-ai/vvv](https://github.com/actionow-ai/vvv) | 415 | general | safe | Python | MIT | Build a video template once, reuse it across coding agents. Editable Remotion templates and an Agent Skill for Claude Code, Codex and 25+ agents. |
| [YeJe-cpu/SeeCut](https://github.com/YeJe-cpu/SeeCut) | 394 | editing | safe | JavaScript | NOASSERTION | An AI editor that watches its own cut: talking-head / AI-avatar video → auto-edited short video. 网感口播精剪：数字人/真人口播 → AI 自动精剪成网感动效短视频 → 可选剪映分层草稿 |
| [Yuuhann1999/agent-storyboard](https://github.com/Yuuhann1999/agent-storyboard) | 377 | shorts | safe | JavaScript | MIT | 本地优先的 Agent 视频分镜工作台：让 Codex、Claude Code 通过 MCP 写分镜、生成图片/视频素材和配音，并自动回填。Local-first storyboard workbench for AI agents (Codex, Claude Code). |
| [lazypay/Archscribe](https://github.com/lazypay/Archscribe) | 371 | motion | safe | Python | MIT | Premium hand-drawn, dark-background animated architecture & process diagrams. Outputs editable .excalidraw + PNG + genuinely animated GIF. Codex/Claude skill. |
| [AbubakrChan/product-launch-motion](https://github.com/AbubakrChan/product-launch-motion) | 369 | promo | safe | JavaScript | MIT | Claude Code skill for Dribbble-level product launch videos from code. One shape that never cuts, springs, a cursor that drives every change, beat-synced SFX, 60 fps motion blur. v2: use with Claude Opus 5.5. |
| [aaronyi97/image-story-video-wizard](https://github.com/aaronyi97/image-story-video-wizard) | 365 | story | safe | Python | MIT | A confirmation-gated Codex and WorkBuddy skill for audio-first image-story video production. |
| [shuyicc/MathLens](https://github.com/shuyicc/MathLens) | 358 | explainer | safe | Python | — | MathLens 是一个专注于数学题目视频讲解的 Agent Skill。你只需粘贴一道数学题（图片或文字），它就能自动完成从题目分析、可视化讲解、配音脚本到 Manim 动画视频的全流程制作。单条视频1-10 分钟，成本 0.2-1 元以内。 |
| [hassancs91/claude-youtube-editor](https://github.com/hassancs91/claude-youtube-editor) | 351 | editing | safe | TypeScript | MIT | Record the talking head, Claude Code does the rest: the cut, the visuals, the voice, the sound effects, the thumbnail, and the YouTube upload. Every screen moment is built as Remotion TSX code, never screen-recorded. |
| [tuzhechen2005/opus-video-skills](https://github.com/tuzhechen2005/opus-video-skills) | 351 | story | safe | JavaScript | NOASSERTION | Video-making skills for Claude Opus 5.5 in Claude Code — every frame and note generated in code, one skill per style: hand-painted animation, kinetic-typography reels \| 用 Claude Opus 5.5 以代码制作视频的技能合集 |
| [GTAI-1/flow-mcp](https://github.com/GTAI-1/flow-mcp) | 345 | general | safe | TypeScript | — | Drive Google Flow from Claude, Codex or a local control panel, on Mac or Windows — using your Google AI subscription credits instead of the pay-per-use video API. No browser extension. |
| [YIAN6557/eian-riso-broll](https://github.com/YIAN6557/eian-riso-broll) | 341 | motion | safe | JavaScript | NOASSERTION | 孔版双色风格的口播信息动画 Skill：逐字稿 → 1080×1920 竖屏 mp4 · Risograph-style info animations for talking-head videos |
| [FavioVazquez/showtime](https://github.com/FavioVazquez/showtime) | 304 | general | safe | Python | MIT | A local video studio for your coding agent. Describe a video in one sentence: your agent directs, your machine renders. Claude Code, Codex, Cursor, Devin and more. No API keys, no uploads. |
| [Kianzzz/xilo-opus-video](https://github.com/Kianzzz/xilo-opus-video) | 297 | promo | safe | Python | MIT | 用 Claude Opus 5.5 写代码做视频：先给三个方案和预览，确认后逐帧渲染成片 \| Canvas / WebGL / Three.js / Remotion / HyperFrames \| Claude Code Skill |
| [Anil-matcha/vox-ai-motion-graphics-generator](https://github.com/Anil-matcha/vox-ai-motion-graphics-generator) | 294 | explainer | safe | Python | — | 🎬 Turn any topic into a finished Vox-style paper-collage explainer / motion graphics video — script, collage keyframes, animation, voice-over, music & captions, all automated. An agent skill for Claude Code, Codex & other coding agents. |
| [gnipbao/minimax-h3-video-reverse-skill](https://github.com/gnipbao/minimax-h3-video-reverse-skill) | 284 | craft | safe | Python | MIT | MiniMax H3 视频反推 Skill：真实镜头、首帧与动态提示词、AUTO/T2VA/I2V/DUAL，附中文教程与验证工具。 |
| [hassancs91/claude-faceless-shorts-creator](https://github.com/hassancs91/claude-faceless-shorts-creator) | 283 | shorts | safe | TypeScript | MIT | A faceless YouTube-Shorts factory driven by Claude Code: pure-TSX Remotion visuals, ElevenLabs voice with word-exact captions, library-first sound design. 12 complete example shorts included. |
| [Hainrixz/editor-pro-max](https://github.com/Hainrixz/editor-pro-max) | 280 | general | safe | TypeScript | NOASSERTION | AI-powered video editor by @soyenriquerocha — built with Remotion + Claude Code. Describe videos in natural language, get professional MP4s. 25 components, 9 templates, 7 skills. |
| [liyue-aigc/xianxia-cinematic-video-director](https://github.com/liyue-aigc/xianxia-cinematic-video-director) | 273 | story | safe | — | — | Codex Skill for Eastern xianxia storyboards, camera movement and video continuity. |
| [MegaTroll222/VOX-COLLAGE-BROLL](https://github.com/MegaTroll222/VOX-COLLAGE-BROLL) | 270 | motion | safe | — | NOASSERTION | Turn one spoken line into a paper-collage explainer video — Claude Code skill + MaxFusion MCP. English adaptation of gbro-collage-broll by pyang5166. |
| [tugrawork-creator/saas-motion-kit](https://github.com/tugrawork-creator/saas-motion-kit) | 246 | promo | safe | Python | NOASSERTION | Promo & motion videos for software products with HyperFrames + Claude Code. No two films should feel the same: tone matrix, variety audit, 24-transition atlas, 100 storyboard themes. |
| [kangarooking/director-skills](https://github.com/kangarooking/director-skills) | 245 | general | safe | Python | MIT | 导演Skill：面向 AI 视频创作的开源 Agent Skills \| Director Skills: Open-source Agent Skills for AI video creation. |
| [AgriciDaniel/claude-shorts](https://github.com/AgriciDaniel/claude-shorts) | 240 | editing | safe | Python | MIT | Interactive longform-to-shortform video creator — Claude Code skill with Remotion-rendered animated captions, AI segment scoring, cursor tracking, and audio-aware boundary snapping |
| [erduo1998-cell/erduo-broll-loop-engineering](https://github.com/erduo1998-cell/erduo-broll-loop-engineering) | 232 | editing | safe | TypeScript | MIT | SRT 驱动的双后端 B-roll Agent Skill：自动路由 HyperFrames / Remotion，集成 152 张 Shotcraft 镜头卡 |
| [edenfunf/reelmimic](https://github.com/edenfunf/reelmimic) | 223 | story | safe | JavaScript | MIT | Show it a video you love. Get a new video in the same style. An AI crew (Claude Code or Codex) plans, builds and reviews it with you. |
| [EGSECDA/vocaloid-style-mv-pipeline](https://github.com/EGSECDA/vocaloid-style-mv-pipeline) | 222 | music | safe | JavaScript | MIT | Production pipeline and Claude Code skill for Vocaloid-style hand-drawn (手書き) lyric music videos: deterministic Canvas2D/WebGL2 animation engine, JIZURA lyric motion, AI illustration, Blender toon 3D, headless rendering to 16:9 and 9:16. |
| [wyuzhi/qisi-video-remix](https://github.com/wyuzhi/qisi-video-remix) | 185 | craft | safe | — | MIT | 视频创意改编 Agent Skill：参考视频 → 剧本 → 分镜 → 分段生成提示词。By qisi. |
| [Finderchangchang/promo-video-skill](https://github.com/Finderchangchang/promo-video-skill) | 173 | promo | safe | JavaScript | Apache-2.0 | 精酿 BrewReel：让 DeepSeek 这类便宜模型也能做出好看的竖版宣传片。写一份产品简报，AI 挑镜头、写文案，一条命令出片。3 种配方、6 个行业、广告法校验，开源可商用。 |
| [iart-ai/motion-design-skills](https://github.com/iart-ai/motion-design-skills) | 168 | general | safe | HTML | MIT | Motion design fundamentals, engines, and brand elements as installable Claude Code skills — timing, typography, color, composition, After Effects, Remotion, logos, and backgrounds. |
| [HRuiCcc/RuiC-motion-reel](https://github.com/HRuiCcc/RuiC-motion-reel) | 167 | motion | safe | Python | NOASSERTION | 用代码生成 15 秒动态图形成片 · Agent Skill。交付 1920×1080 / 30fps。自研渲染引擎：2× 超采样矢量/文字 + 真 3D 点云 + 丝网印四色分色 + 代码合成配乐，不用任何外部美术素材、不用 AE。15 张风格牌，已做过暗色科技 / riso 印刷 / 纸艺 / 银盐暗房 / 恒星普朗克配色 / 氰版蓝图 / 紫外光刻，成片见 docs/films/ |
| [znyupup/ai-video-editing-skill](https://github.com/znyupup/ai-video-editing-skill) | 166 | editing | safe | Python | MIT | AI Agent Skill for automated vlog editing. Feed raw footage, get a finished video. Powered by ffmpeg + Whisper + Vision API. |
| [blixvip/easyedit](https://github.com/blixvip/easyedit) | 162 | editing | safe | Python | MIT | Type a movie name and get a fan edit: a captioned speech cut, then a montage cut to the beat. |
| [10291124jcjc-dotcom/ancient-rap-battle](https://github.com/10291124jcjc-dotcom/ancient-rap-battle) | 156 | music | safe | Python | MIT | 古人说唱 Battle：一句话做一期 AI 历史人物说唱对决视频的 Claude Code Skill（Suno + Midjourney + MiniMax H3 对口型） |
| [Sunwood-ai-labs/hyperframes-motion-reel-skill](https://github.com/Sunwood-ai-labs/hyperframes-motion-reel-skill) | 154 | promo | safe | — | MIT | 🎬 Claude Code skill for showreel-grade, beat-synced motion graphics with HyperFrames (HTML + GSAP + SVG + Canvas, Apache-2.0) — 120 BPM / 60 FPS to MP4 |
| [mileson/chaojifeng-video-design](https://github.com/mileson/chaojifeng-video-design) | 150 | general | safe | HTML | MIT | 超级峰的视频设计方法论｜面向 AI Agent 的视频主线、分镜、声画节奏、制作与交付验收 Skill |
| [Mr-funny/hbg-life-simulation](https://github.com/Mr-funny/hbg-life-simulation) | 149 | story | safe | JavaScript | MIT | HBG Agent Skill for Chinese life-simulation narrative videos with consistent comic IP, rapid multi-life openings, Edge TTS, synchronized captions, zoom/pan motion and final MP4 QA. |
| [Moessif/code-video-showcase](https://github.com/Moessif/code-video-showcase) | 145 | general | safe | JavaScript | MIT | A pure-code beat-synced showcase-video Agent Skill: HTML/Canvas/SVG/Three.js frame-by-frame + Python numpy/scipy score + FFmpeg. No generative models. |
| [haiderfarooq3/khan-explainer](https://github.com/haiderfarooq3/khan-explainer) | 144 | explainer | safe | JavaScript | MIT | Claude Code skill: Khan Academy style explainer videos. A pen draws while a voice explains. Free, local, no API keys. |
| [video-db/skills](https://github.com/video-db/skills) | 139 | general | safe | Python | MIT | Server-side video workflows for agents: ingest, understand, search, edit, stream. |
| [chuspeeism/dashi-animationstyle](https://github.com/chuspeeism/dashi-animationstyle) | 139 | explainer | safe | HTML | NOASSERTION | 把一段中文知识口播做成 7 种画风的讲解动画（Claude Code skill）：画面由 Claude 写 HTML/Canvas 代码逐帧画出，不用生图模型 |
| [Liamrjohnston/remotion-motion-graphics-skill](https://github.com/Liamrjohnston/remotion-motion-graphics-skill) | 139 | motion | safe | JavaScript | MIT | Production-ready Remotion motion graphics skills for AI video workflows |
| [naive-kun/naive-video-skill](https://github.com/naive-kun/naive-video-skill) | 135 | editing | safe | Python | MIT | A Codex skill for turning talking-head videos into captioned, animated final videos. |
| [Upload-Post/avatar-mix](https://github.com/Upload-Post/avatar-mix) | 135 | avatar | safe | Python | MIT | Generate 16:9 & 9:16 videos with your HeyGen avatar, animated backgrounds (HyperFrames), music, SFX and Hormozi captions — then publish them to every social network. Reusable skill. |
| [unclejobs-ai/motion-video-skill](https://github.com/unclejobs-ai/motion-video-skill) | 129 | story | safe | TypeScript | MIT | Claude Code 스킬: 모션 그래픽·영상 제작 순서와 단계별 점검 하네스 |
| [bestagentkits/motion-video-skill](https://github.com/bestagentkits/motion-video-skill) | 128 | promo | safe | JavaScript | MIT | Agent skill that produces beat-synced 1080p motion-graphic videos in HyperFrames (HTML + GSAP) with an AI voice-over, karaoke captions, SFX and generated music. MIT. |
| [ayushozha/AdobePremiereProMCP](https://github.com/ayushozha/AdobePremiereProMCP) | 128 | editing | safe | Go | MIT | An open-source bridge between AI assistants and Adobe Premiere Pro, for supported editing workflows. Go, Rust, Python, and TypeScript. |
| [Joker-chichi/claude-mv-skill](https://github.com/Joker-chichi/claude-mv-skill) | 127 | music | safe | JavaScript | NOASSERTION | Claude Code skill for hand-painted animation and lyric music videos |
| [jhartquist/claude-remotion-kickstart](https://github.com/jhartquist/claude-remotion-kickstart) | 122 | general | safe | TypeScript | MIT | Create videos programmatically with Claude Code and Remotion |
| [killkli/minimax-h3-colab-skill](https://github.com/killkli/minimax-h3-colab-skill) | 121 | general | safe | Python | — | Standalone Codex skill for MiniMax H3 Colab reference-to-video generation |
| [Cassette-Editor/oh-my-cassette](https://github.com/Cassette-Editor/oh-my-cassette) | 121 | editing | safe | Python | MIT | 你的随身 AI 剪辑搭档 \| Pocket AI co-editor for video montage — AI video editing plugin & MCP server for Claude Code, Codex, Hermes & OpenCode |
| [runesleo/claude-video-kit](https://github.com/runesleo/claude-video-kit) | 120 | explainer | safe | TypeScript | MIT | Agent Skill + Remotion pipeline: brief/script → review receipt → narrated 9:16 explainer. RC: video-explainer skill. |
| [Monet-AI-Editor/Monet](https://github.com/Monet-AI-Editor/Monet) | 119 | editing | safe | TypeScript | MIT | Edit Videos and Design Images with Claude code or Codex |
| [0xS4N4TG/kinetograph](https://github.com/0xS4N4TG/kinetograph) | 115 | general | safe | TypeScript | NOASSERTION | A blazingly fast programmatic video engine, made for agents. |
| [louisedesadeleer/cut-video](https://github.com/louisedesadeleer/cut-video) | 114 | editing | safe | Python | MIT | Claude Code skill: tighten long recordings — remove silences, ums, dead air. Preserves laughs and comedic pauses. Fast on Apple Silicon. |
| [kangarooking/promo-creator-skills](https://github.com/kangarooking/promo-creator-skills) | 112 | promo | safe | Python | MIT | 产品宣传视频创作 Skills：从产品判断、分镜、素材、HyperFrames 剪辑到 BGM 设计的完整 Agent 工作流 |
| [leosssvip-dot/remotion-ad-video-skill](https://github.com/leosssvip-dot/remotion-ad-video-skill) | 110 | promo | safe | JavaScript | MIT | Create Remotion ad video projects from a URL with an AI coding agent, no video-generation AI required. |
| [jaxxchen003/book-video-factory](https://github.com/jaxxchen003/book-video-factory) | 110 | shorts | safe | Python | MIT | Portable Codex skill for auditable, rights-aware Chinese book-review short-video workflows. |
| [pycodebr/heditor](https://github.com/pycodebr/heditor) | 100 | shorts | safe | Python | NOASSERTION | Creative vertical video editing skill for AI agents (Claude Code, Codex, OpenCode, Cursor, Hermes): named styles, creative mode, HyperFrames + voice-first mix. |
| [SpaceZephyr/map-motion](https://github.com/SpaceZephyr/map-motion) | 98 | shorts | safe | Python | NOASSERTION | 地图动效视频 Claude Code skill：基于高德地图真实路线/边界/卫星底图，做地球俯冲、vlog 我在这里、徒步海拔、骑行旅程、航线、行政区高亮等地图动画，12 种风格 |
| [AKMessi/vex](https://github.com/AKMessi/vex) | 95 | editing | safe | Python | NOASSERTION | claude code for video editing |
| [kingselyjoe/video-shotcraft-dsh](https://github.com/kingselyjoe/video-shotcraft-dsh) | 91 | promo | safe | TypeScript | Apache-2.0 | 面向 DeepSeek Harness 的电影感产品视频 Agent Skill，包含 152 张镜头配方卡、Remotion 模板、代码组件和音频资产。 |
| [0xS4N4TG/papercut-explainer](https://github.com/0xS4N4TG/papercut-explainer) | 91 | shorts | safe | TypeScript | MIT | Short vertical cut-paper explainer reels, made by your coding agent. Nine Agent Skills plus a Remotion library. Works with Claude Code, OpenCode, Codex, Gemini CLI, Cursor, Copilot and more. |
| [Xiangyu-CAS/rea-capcut](https://github.com/Xiangyu-CAS/rea-capcut) | 89 | editing | safe | TypeScript | MIT | ReaCut — REA × CapCut: an independent, open-source video editor rebuilt from REA-assisted, authorized local behavior and format research. No copied CapCut proprietary code or assets. Offline desktop app + Codex plugin. |
| [JUNKDOGE-JOE/after-effects-mcp](https://github.com/JUNKDOGE-JOE/after-effects-mcp) | 87 | editing | safe | JavaScript | MIT | AI-powered Adobe After Effects automation via MCP. Chat in a CEP panel or connect external agents, with 16 tools for editing, previews, recovery, reusable scripts, and multi-instance workflows. |
| [JuneYaooo/cheap-claude-video](https://github.com/JuneYaooo/cheap-claude-video) | 86 | general | safe | Python | Apache-2.0 | Claude Opus 视频效果的稳定平替：用 Claude 创作出片一次，蒸馏成视频配方，其他模型按配方复刻出类似效果。 |
| [kurbaitaev/ghost-editor](https://github.com/kurbaitaev/ghost-editor) | 81 | editing | safe | Python | MIT | AI video editor for talking-head reels: 7 styles, face-safe captions, motion scenes, reverse-engineer any reference edit. A Claude Code / agent skill on HyperFrames. |
| [Hype-yolo/claude-motion-skill](https://github.com/Hype-yolo/claude-motion-skill) | 80 | general | safe | JavaScript | MIT | Source-assisted animation Skill: 47 open-source cases, editable scenes and MP4 export. Public beta. |
| [Avijit07x/cursorcam](https://github.com/Avijit07x/cursorcam) | 79 | promo | safe | TypeScript | Apache-2.0 | Ask your AI agent for a demo video. It clicks through your web app and hands back a polished MP4 that zooms in on each click. Works with any AI agent, or on its own as a CLI. |
| [derek-zhuolin/motion-skill](https://github.com/derek-zhuolin/motion-skill) | 79 | explainer | safe | TypeScript | NOASSERTION | 不出镜，用文案或录音把想法做成视频。Script- or voice-driven motion graphics with Codex and Remotion: real footage, captions, editable projects and 96 motion studies. |
| [holy-templar/vox-animated-ad-mcp](https://github.com/holy-templar/vox-animated-ad-mcp) | 75 | explainer | safe | — | AGPL-3.0 | Vox-style paper-collage animated video skill for AI agents — Claude + MaxFusion AI MCP (Google Omni). One sentence in, a full explainer or ad out, entirely from chat. |
| [ymybxx/SoloMCN](https://github.com/ymybxx/SoloMCN) | 72 | shorts | safe | JavaScript | MIT | 把 Claude Code 变成你的短视频团队：不搭工作流，直接让 Claude 干活，从国内热榜、推特热帖到抖音、小红书、B站、YouTube、推特发布。Turn Claude Code into your short-video team — no workflows, from trends and X posts to publishing. |
| [adityaarsharma/youtube-marketing-skills](https://github.com/adityaarsharma/youtube-marketing-skills) | 71 | craft | safe | JavaScript | MIT | Open-source agent-agnostic YouTube growth toolkit — 21 AI commands + live channel MCP. Works with Claude Code, Cursor, Codex, Windsurf, Gemini CLI & any MCP client. SEO, scripts, Shorts, thumbnails, analytics, WordPress publishing. Free VidIQ/TubeBuddy alternative. |
| [ledbetterljoshua/functional-emotions-video](https://github.com/ledbetterljoshua/functional-emotions-video) | 71 | music | safe | JavaScript | MIT | A painted music video for "Functional Emotions", made by Claude Opus 5.5: custom GPU brushstroke renderer, storyboard, and 7 parallel chapter agents. |
| [talsiach/take-one](https://github.com/talsiach/take-one) | 67 | promo | safe | HTML | MIT | Claude Code skill that makes launch videos, ads and music videos from one line: idea, script, visuals, voice, music, edit and review, then a review room where your notes make the next take |
| [dev-isaacmello/extreme-motion-design](https://github.com/dev-isaacmello/extreme-motion-design) | 66 | general | safe | Python | MIT | Skill de motion design por código (Remotion) com trilha, narração e QA automático. Para Claude, Claude Code e Codex. |
| [Mort1d/motion-graphics-skills](https://github.com/Mort1d/motion-graphics-skills) | 65 | promo | safe | JavaScript | NOASSERTION | Showreel-grade motion graphics from code, with an original soundtrack for every video. An Agent Skill for Claude Code, Codex, Cursor and other agents. |
| [norahe0304-art/30x-video](https://github.com/norahe0304-art/30x-video) | 64 | promo | safe | TypeScript | MIT | One URL in, an agency-grade launch video out. A Claude Code skill (Remotion + React) with a 16-law taste codex — 12 brands, 12 worlds, zero templates. |
| [sunxiayi/make-blender-education-video-skill](https://github.com/sunxiayi/make-blender-education-video-skill) | 62 | explainer | safe | Python | MIT | A Codex and Claude Code skill for creating fact-checked cinematic educational videos with Blender |
| [Maartenlouis/remotion-ads](https://github.com/Maartenlouis/remotion-ads) | 62 | shorts | safe | JavaScript | MIT | Claude Code skill for creating Instagram Reels & Carousel ads with Remotion |
| [noamdorr/saas-product-demo-video](https://github.com/noamdorr/saas-product-demo-video) | 60 | promo | safe | TypeScript | MIT | Skill for shipping a 20-45s SaaS product demo video in Remotion. Built and used in production with Claude Code; methodology is LLM-agnostic. |
| [Changroro/code-video](https://github.com/Changroro/code-video) | 59 | general | safe | Python | MIT | Agent skill: research a topic and render a hand-drawn + 8-bit promo video (MP4) |
| [revfactory/showreel](https://github.com/revfactory/showreel) | 58 | promo | safe | Python | MIT | Claude Code skill that produces Korean motion-graphics videos end to end: brief, research, script, ElevenLabs narration, music, images, deterministic HTML render, mix and automated QA. |
| [iart-ai/explainer-video-skills](https://github.com/iart-ai/explainer-video-skills) | 58 | explainer | safe | HTML | MIT | Explainer video skills for Claude Code: script, storyboard, and render narrated explainers, year-in-review recaps, and animated diagrams. |
| [pyang5166/gbro-collage-info](https://github.com/pyang5166/gbro-collage-info) | 55 | shorts | safe | CSS | MIT | 半调纸拼贴风信息动画 Agent Skill · Halftone paper-collage info-graphic animations from voiceover scripts (HyperFrames, no image models) |
| [LordVayron/video-factory](https://github.com/LordVayron/video-factory) | 53 | general | safe | JavaScript | MIT | An AI video studio your coding agent installs and runs: UGC ads, micro-dramas, sitcoms, launch films, fantasy shorts, hour-long meditations. Claude Code / Codex + APIMart + Kie.ai + ElevenLabs. |
| [znyupup/knowledge-explainer-skill](https://github.com/znyupup/knowledge-explainer-skill) | 53 | explainer | safe | JavaScript | MIT | 把一份 markdown 文稿，自动生成讲解动画视频 — Powered by Remotion + AI Agent |
| [liangdabiao/story-handdrawn-video](https://github.com/liangdabiao/story-handdrawn-video) | 52 | shorts | safe | Python | — | 把一段 中文/英文 故事文本变成 9:16 竖屏（720×1280）手绘蜡笔风短视频。Remotion 技术的视频 Skill。基于 Agnes Video V2.0（纯文生视频，当前 $0/秒）+ edge-tts（免费旁白）的全免费视频制作方案。 |
| [krusemediallc/video-editor-agent](https://github.com/krusemediallc/video-editor-agent) | 50 | editing | safe | Python | — | Claude Code skill pack: edit short-form videos end to end — style-clone a reference reel, build a branded motion-graphics edit, ElevenLabs sound design, frame-level QA, timeline-comment review loop |
| [JJenglert1/remotion-claude-video](https://github.com/JJenglert1/remotion-claude-video) | 49 | general | safe | Python | — |  |
| [hahadu4520/vlog-cut](https://github.com/hahadu4520/vlog-cut) | 49 | editing | safe | Python | NOASSERTION | Narration-driven video editing pipeline for Claude Code · 给 Claude Code 用的「按文案剪辑」流水线 |
| [trunghaiy/appshot](https://github.com/trunghaiy/appshot) | 48 | promo | safe | TypeScript | MIT | Generate App Store & Google Play preview videos and screenshots from a simple TypeScript config. Built on Remotion + React + Tailwind. Ships with AI agent skills for Claude Code — scan your app, direct the creative, render to MP4. |
| [buildwithhanif/claude-animation-skill](https://github.com/buildwithhanif/claude-animation-skill) | 48 | story | safe | JavaScript | MIT | Hand-drawn 2D animation written as code: detail bible, a detailed ant rig, brush/watercolour pen, frame-matched recreation loop, synthesised sound. Node canvas + ffmpeg. |
| [Zane-0x5a/remotion-director](https://github.com/Zane-0x5a/remotion-director) | 47 | promo | safe | JavaScript | MIT | A Claude Code plugin that designs and builds a finished motion piece from a one-line brief — via the 甲乙环 (critic-loop) pipeline, judged on its real rendered frames. |
| [alisadiq-ai/favstash-ai-video-editing-skills](https://github.com/alisadiq-ai/favstash-ai-video-editing-skills) | 46 | editing | safe | JavaScript | Apache-2.0 | Create Instagram Reels, TikToks and YouTube Shorts with adaptive AI editing, captions and motion graphics. Connect FavStash for planning, publishing and analytics. |
| [Chamanrajragu/purffle-shorts](https://github.com/Chamanrajragu/purffle-shorts) | 46 | shorts | safe | Python | MIT | Free open-source AI YouTube Shorts generator and faceless video maker in Python. Any LLM (GPT, Claude, Gemini, Ollama) writes scripts; text-to-speech, stock footage or AI images, word-synced captions, text-message chat and Reddit story videos, podcast and long video clipping, YouTube auto-upload and scheduling. TikTok and Reels ready. MCP server. |
| [threeaus/creator-style-kit](https://github.com/threeaus/creator-style-kit) | 46 | shorts | safe | Python | MIT | 18 套 9:16 HyperFrames 视频风格（字体/配色/动效/音效/构图），Claude Code skill |
| [zz41354899/mografy](https://github.com/zz41354899/mografy) | 46 | motion | caution | TypeScript | MIT | Create, preview, and export Canvas motion graphics with Claude Code or Codex. Local-first and open source, with MP4, Lottie, and SVGA export. |
| [lintsinghua/paint-mv-skills](https://github.com/lintsinghua/paint-mv-skills) | 46 | music | safe | JavaScript | NOASSERTION | Agent skills that turn a song and its lyrics into a hand-painted watercolour music video (p5.brush + headless Chrome + ffmpeg), following PDoomVideo's pipeline |
| [ops120/video-recap-skills-plus](https://github.com/ops120/video-recap-skills-plus) | 45 | editing | caution | Python | MIT | Clip any video into a narration recap with claude code skill｜用claude code skill把任何视频剪辑成中文解说视频，支持剪映导出 |
| [Johnson-Jia/video-clipforge](https://github.com/Johnson-Jia/video-clipforge) | 44 | general | safe | Python | Apache-2.0 |  AI 驱动的短视频制作系统。给它一个想法，它帮你写稿、配音、做画面、出成片。9 阶段 DAG 管线 + 自进化评分，支持每日自动执行。基于 Claude Code + HyperFrames。 |
| [BayramAnnakov/remotion-video-director](https://github.com/BayramAnnakov/remotion-video-director) | 44 | promo | safe | — | — | Interactive Claude Code skill for creating Remotion videos through expert-guided deliberation |
| [wocha-xiaoli/video-shot-analysis-feishu](https://github.com/wocha-xiaoli/video-shot-analysis-feishu) | 43 | craft | safe | Python | MIT | 视频拉片到飞书 Base：自动抽帧、逐镜头分析、倒推分镜与视频生成提示词的 Codex/OpenCode skill |
| [zhouyuechuan2025-ui/ai-self-media-video-packaging-skill](https://github.com/zhouyuechuan2025-ui/ai-self-media-video-packaging-skill) | 42 | general | safe | TypeScript | MIT | Open Agent Skill for packaging talking-head videos with Remotion, optional HyperFrames, seek-safe motion, and line illustrations. |
| [yammaku/typewriter-video](https://github.com/yammaku/typewriter-video) | 42 | shorts | safe | TypeScript | NOASSERTION | VS Code-style typewriter animation video skill for AI agents (agentskills.io format) |
| [erduo1998-cell/vidmuse-video-creator](https://github.com/erduo1998-cell/vidmuse-video-creator) | 41 | general | safe | Python | MIT | Open-source Agent Skill for VidMuse images, videos, and SRT B-roll from first authorization to delivery |
| [AmitSubhash/3brown1blue](https://github.com/AmitSubhash/3brown1blue) | 41 | explainer | safe | Python | MIT | First-principles Manim skill for Claude Code — mathematical animation from scratch, paper-explainer patterns, 21 rule files. |
| [Win-Hao/music-mv](https://github.com/Win-Hao/music-mv) | 41 | music | safe | Python | MIT | 用 AI 写代码给一首歌做音乐 MV 的 agent skill：听懂歌 → 定画风 → 画角色 → 剧本分镜 → 先出草图再做整片 → 看片修改。每一帧都是代码画的，不是视频生成；不限模型。 |
| [motalim-ai/motion-starter-kit](https://github.com/motalim-ai/motion-starter-kit) | 40 | general | safe | TypeScript | MIT | اصنع فيديو موشن جرافيك بـ Remotion من أي فيديو مرجعي. نفس القواعد لـ Claude Code وCodex وAntigravity. |
| [darrenli6/JJKoubo](https://github.com/darrenli6/JJKoubo) | 40 | editing | safe | JavaScript | NOASSERTION | JJ Koubo (JJ口播) is a local-first collection of video editing Skills for talking-head, interview, knowledge-sharing, and other videos centered on spoken content. The Skills can be used through Codex or other compatible Agent environments. |
| [new-xp/ultrademo](https://github.com/new-xp/ultrademo) | 39 | promo | safe | JavaScript | Apache-2.0 | Ultrademo is an automated demo video generator for any websites, SaaS applications, etc. - executed conveniently through a Claude Code skill. |
| [liangdabiao/story-handdrawn-remotion](https://github.com/liangdabiao/story-handdrawn-remotion) | 38 | story | safe | Python | — | 视频SKILL: 把一段 中文/英文 故事文本变成  「手绘日记漫画风」竖屏视频。  核心方法论：**不是把一张漂亮图配文字朗读，而是把每句故事拆成「文字 → 黑白画稿 → 彩色插画」三阶段横向擦除揭示，让一句话被画出三次。 |
| [imMamdouhaboammar/motion-graphics-skills](https://github.com/imMamdouhaboammar/motion-graphics-skills) | 37 | promo | safe | JavaScript | NOASSERTION | Motion graphics for AI agents. Creative direction + production skills for Claude Code & Codex, powered by HyperFrames. Build brand films, kinetic type, 2D/3D and programmatic video. Every frame is code |
| [vibe-motion/remotion-code-motion-explainer](https://github.com/vibe-motion/remotion-code-motion-explainer) | 37 | explainer | safe | TypeScript | MIT | AI Agent skill for continuous, editable Remotion explainers — created by Bingo |
| [crimeacs/product-demo-director](https://github.com/crimeacs/product-demo-director) | 36 | promo | safe | Python | MIT | Turn product evidence and raw footage into narration-paced, source-bound demo films—with Codex or Claude Code directing cuts, zooms, render, and QA. |
| [mattivilola/brag-codex](https://github.com/mattivilola/brag-codex) | 36 | promo | safe | Python | MIT | Codex-compatible clone of latent-spaces/brag for Hyperframes launch videos |
| [8533502-dev/deepseek-opus-motion-skill](https://github.com/8533502-dev/deepseek-opus-motion-skill) | 35 | general | safe | TypeScript | MIT | 让 DeepSeek 用和 Claude Opus 5.5 同样的提示词做出接近的 Remotion 动效视频 · A skill for Opus-parity motion design |
| [Kappaemme-git/codex-video-short-maker-skill](https://github.com/Kappaemme-git/codex-video-short-maker-skill) | 35 | editing | safe | Python | MIT |  |
| [TryWorld2026/paper-algorithm](https://github.com/TryWorld2026/paper-algorithm) | 33 | shorts | safe | Python | CC-BY-SA-4.0 | Paper Algorithm - An AI-agent skill set that turns one sentence into a finished voiceover video. \| 纸上算法 · 一句话到成片的口播视频生产线：选题、写稿、出片、发布计划。 |
| [Mr-funny/hbg-douyin-code-explainer-video](https://github.com/Mr-funny/hbg-douyin-code-explainer-video) | 32 | explainer | safe | Shell | — | HBG Codex skill for deterministic Chinese 9:16 HyperFrames explainer videos with dialogue TTS, global Whisper sync, stable BGM and final visual QA. |
| [santmun/video-vox](https://github.com/santmun/video-vox) | 32 | explainer | safe | JavaScript | — | Crea shorts verticales animados estilo Vox sobre cualquier tema con Claude Code + Remotion (cartoon + voz + música + SFX) |
| [klsoen/opus-js-animations](https://github.com/klsoen/opus-js-animations) | 32 | shorts | safe | Python | MIT | Claude Opus 5.5 directs and renders films in JavaScript: brief → sound (file, YouTube, generated, or AI voiceover) → director's treatment → frame-exact MP4. A Claude Code skill + plugin. |
| [AllenAI2014/remotion-guofeng-starter](https://github.com/AllenAI2014/remotion-guofeng-starter) | 32 | story | safe | TypeScript | NOASSERTION | 国风纸片动画：Remotion 风格资产库 + 制作流程 Skill。给 AI 一首诗/成语，照这套国风纸片拼贴风格从零出片，全程代码渲染不进剪辑。 |
| [axtonliu/video-illustrator](https://github.com/axtonliu/video-illustrator) | 31 | promo | safe | JavaScript | MIT | Your voice, your assets, any look you choose — an Agent Skill that turns your narration and real assets into a short film. |
| [gorkem-bwl/onboarding-video-generator](https://github.com/gorkem-bwl/onboarding-video-generator) | 31 | promo | safe | TypeScript | MIT | Claude Code skill for column-sized web app onboarding videos in Remotion |
| [JimLiu/taohuayuan](https://github.com/JimLiu/taohuayuan) | 31 | story | safe | JavaScript | — | 桃花源记 3D 网页 |
| [vakovalskii/nd-video-studio](https://github.com/vakovalskii/nd-video-studio) | 30 | explainer | safe | Python | MIT | Explainer videos from HTML (HyperFrames → MP4) with AI narrator, music and mixing — Claude Code / Codex skills on NeuralDeep models |
| [AgriciDaniel/claude-gif](https://github.com/AgriciDaniel/claude-gif) | 30 | motion | safe | Python | MIT | Ultimate GIF creator skill for Claude Code. 6 generation pipelines: Remotion, Veo 3.1, SVG-to-transparent-GIF, FFmpeg, image sequences, AI frames. Palette optimization, perfect loops, platform sizing. |
| [GordenSun/react-bits-video](https://github.com/GordenSun/react-bits-video) | 29 | general | safe | JavaScript | NOASSERTION | 结合react-bits、remotion、hyperframe整合的Skill，可以生成华丽的视频。 |
| [Dancan254/shenghua](https://github.com/Dancan254/shenghua) | 29 | shorts | caution | Python | MIT | AI-agent skill: turn a voice note, a talking-head or green-screen video, or a written topic into an animated, sound-designed Short or YouTube video |
| [AsadMoulviDev/reel-video](https://github.com/AsadMoulviDev/reel-video) | 29 | story | safe | TypeScript | MIT | Create AI videos with Claude Code, Codex, Cursor, and Grok Build, with no extra generation fees beyond the plans you already pay for |
| [bbylw/hyperframes-cn](https://github.com/bbylw/hyperframes-cn) | 28 | general | safe | TypeScript | MIT | HyperFrames 是一个开源框架，可将 HTML、CSS、媒体与可定位（seekable）动画转化为确定性的 MP4 视频。你可以在本地通过 CLI 使用它，让 AI 编程智能体借助 skills 使用它，或者把它作为托管创作工作流背后的渲染核心。 |
| [Lanqiao-Luca/ai-video-marker](https://github.com/Lanqiao-Luca/ai-video-marker) | 27 | story | safe | Python | MIT | 🎬 AI 视频创作工作流｜🧩 Graph · Loop · Workflow · Agent · Skill · Standards 六层协作｜8条流程覆盖故事、像素、诗词、定格、MV、绘画世界与AI漫画剪映｜📦 3个可运行像素场景、漫画成片、原创配乐与素材库｜🔄 跨窗口接续方法 |
| [Arman-Luthra/aftr](https://github.com/Arman-Luthra/aftr) | 26 | promo | safe | JavaScript | MIT | Puppeteer for After Effects. Use AE with Claude Code to make production-ready videos. |
| [memex-lab/product-launch-video-skill](https://github.com/memex-lab/product-launch-video-skill) | 25 | promo | safe | JavaScript | MIT | AI agent skill for creating cinematic product launch videos with Remotion |
| [Anil-matcha/zack-d-films-ai-video-generator](https://github.com/Anil-matcha/zack-d-films-ai-video-generator) | 25 | explainer | safe | Python | — | 🎬 Turn any topic into a finished Zack D Films-style 3D animated short — curiosity-loop script, character sheets, 3D keyframes, Veo 3.1 motion, voiceover, captions & punchy edits, all automated. An agent skill for Claude Code, Codex & other coding agents. Powered by MuAPI + ffmpeg. |
| [liuliu-66-create/ll-video-decomposer](https://github.com/liuliu-66-create/ll-video-decomposer) | 25 | craft | safe | Python | MIT | 专业的五层视频拆解 Codex Skill：支持视频链接、本地媒体和逐字稿。 |
| [dlazy-ai/ai-product-video](https://github.com/dlazy-ai/ai-product-video) | 24 | promo | safe | TypeScript | Apache-2.0 | AI agent skill (Claude Code / Codex) that turns your product into a cinematic promo video: 152 shot recipe cards, 209 style variants, tuned Remotion implementations, 149 SFX, a validated 10-shot template, plus music, voiceover, captions and B-roll via the dLazy CLI. |
| [Agentchengfeng/chengfeng-video-styles](https://github.com/Agentchengfeng/chengfeng-video-styles) | 24 | explainer | safe | HTML | Apache-2.0 | 15 种代码视频风格 Skill：让 Claude Code / Codex 用纯代码做 10 秒知识讲解短片 |
| [Phantomlau3674/voxstylehub-steven](https://github.com/Phantomlau3674/voxstylehub-steven) | 24 | explainer | safe | Python | MIT | Standalone Codex skill for Vox-inspired editorial collage knowledge videos with deterministic Remotion assembly and optional generated motion. |
| [smwbev/framewright](https://github.com/smwbev/framewright) | 23 | general | safe | HTML | MIT | Agent skill + template: short videos made entirely from code. One HTML file, every frame a pure function of (frame, seed, width). Claude Code, Codex, Gemini CLI, Cursor. |
| [ahmdd4vd/MotionCraft](https://github.com/ahmdd4vd/MotionCraft) | 23 | explainer | safe | JavaScript | MIT | Clean, high-taste motion graphics videos made by your AI agent with Remotion. One-command Agent Skill: npx skills add ahmdd4vd/motioncraft |
| [abhinavshrivastava950/Montara](https://github.com/abhinavshrivastava950/Montara) | 22 | general | safe | Python | AGPL-3.0 | Montara is an AI-native, renderer-agnostic autonomous video production operating system that understands content before it edits it, plans visual storytelling through a Timeline IR, and orchestrates specialized renderers, skills, and agents to produce production-quality videos with minimal human intervention. |
| [AgriciDaniel/lipnardo](https://github.com/AgriciDaniel/lipnardo) | 22 | avatar | safe | Python | NOASSERTION | HeyGen avatar video generation skill for Claude Code. Batch, templates, translation, photo avatars, TTS. Stdlib-only Python. |
| [ZLHad/OpenVideoHarness](https://github.com/ZLHad/OpenVideoHarness) | 21 | general | safe | Python | MIT | A video harness work platform for Agents such as Claude or Codex: the agent writes the code that renders the film, checks its own frames, storyboard and cut.  给 Claude Code 和 Codex 用的视频Harness工作台：agent 写代码渲染成片、自己检查分镜，立意、成片由你拍板；众多风格预设、引擎丰富，配音配乐也用代码做。 |
| [every-app/video-skills](https://github.com/every-app/video-skills) | 21 | promo | safe | Python | MIT | Make great videos about your product 10x faster. |
| [Mng-dev-ai/explainer-video](https://github.com/Mng-dev-ai/explainer-video) | 21 | explainer | safe | JavaScript | MIT | AI skill that turns any topic into an animated narrated explainer video. Runs locally, free, works with Claude Code / Codex / Cursor. |
| [skailabinfo/claude-video-harness](https://github.com/skailabinfo/claude-video-harness) | 21 | editing | safe | TypeScript | MIT | Монтаж вертикальных роликов «говорящая голова» в Claude Code + Remotion: клонируй, ответь на вопросы — пайплайн настроится под тебя |
| [AmazingAng/ccvideo](https://github.com/AmazingAng/ccvideo) | 20 | promo | safe | TypeScript | MIT | Claude-style promo videos from ground-truth captures — a Claude Code skill (Remotion) |
| [derrickgong87/demo-video-creation-skill](https://github.com/derrickgong87/demo-video-creation-skill) | 20 | promo | safe | TypeScript | MIT | A reusable Codex skill for creating polished SaaS and product demo videos from a company URL, screenshots, and product flows. |
| [aijiduonadegou/Paper-Cut](https://github.com/aijiduonadegou/Paper-Cut) | 20 | explainer | safe | Python | MIT | 无需视频模型，即可制作拼贴动画的skill。用图像模型定稿、原图拆层与 HyperFrames 代码动画制作可编辑 Paper Cut / Vox 风格纸拼贴科普视频；支持三阶段审批、旁白、音效、文字动效与确定性 QA。 |
| [iart-ai/map-animation-skills](https://github.com/iart-ai/map-animation-skills) | 20 | explainer | safe | HTML | MIT | Animated map skills for AI coding agents — draw routes, zoom-to-location, and data-driven map animations rendered to video. |
| [toufuim/personal-ip-brand-intro-skill](https://github.com/toufuim/personal-ip-brand-intro-skill) | 20 | motion | safe | JavaScript | MIT | 開源 Codex Skill：用文字、原創插圖或使用者圖片製作個人 IP 品牌開場，支援上傳音樂對拍與無音樂自主節拍。 |
| [iart-ai/ad-video-skills](https://github.com/iart-ai/ad-video-skills) | 19 | promo | safe | HTML | MIT | Claude Code skills for ad / advertising video — turn one motion-graphics template into batches of on-brand, A/B-testable ad creative, launch films, and testimonial clips. |
| [124zhe/xigua-msvideo-skill](https://github.com/124zhe/xigua-msvideo-skill) | 19 | explainer | safe | Python | MIT | Codex skills for YouTube curiosity stories, MS Paint storyboards, narration, and video assembly. |
| [IdkwhatImD0ing/claude-music-videos](https://github.com/IdkwhatImD0ing/claude-music-videos) | 19 | music | safe | Python | MIT | Music videos made with Claude in Claude Code, every frame rendered from code: a Blender tabletop miniature (P(doom)) and a League of Legends montage (PARANOIA) |
| [IskakovDamir/apple-motion](https://github.com/IskakovDamir/apple-motion) | 18 | promo | safe | Python | MIT | Claude skill for Apple-style kinetic typography recaps in Remotion, built from frame-accurate measurements of Apple recap videos |
| [chacha95/demo-video-creator](https://github.com/chacha95/demo-video-creator) | 18 | promo | safe | Python | MIT | Agent skill: product URL → 16:9 launch film (EN/KO). Real UI captures, one-take VO, beat-locked motion, −14 LUFS mp4. |
| [chenyuxiaojin/cyxj-hyperframes](https://github.com/chenyuxiaojin/cyxj-hyperframes) | 18 | explainer | safe | HTML | MIT | Open-source HTML+GSAP video projects & reusable toolkit for Claude Code tutorial videos, powered by HeyGen HyperFrames. |
| [reedodev/explainer-studio](https://github.com/reedodev/explainer-studio) | 18 | explainer | safe | TypeScript | MIT | One line in, an explainer video out, in eight hand-drawn styles (Claude Code skill) |
| [iart-ai/tiktok-video-skills](https://github.com/iart-ai/tiktok-video-skills) | 18 | shorts | safe | HTML | MIT | Short-form video skills for Claude Code — engineer Reels, TikToks, and YouTube Shorts that survive the swipe. |
| [kelano86/what-if-animation](https://github.com/kelano86/what-if-animation) | 18 | story | safe | TypeScript | MIT | A free Claude Code skill: one topic line in, a finished 15-25 s What If animated episode out. Every frame drawn in code (SVG + Remotion); voice and sound from ElevenLabs. |
| [dillydog580/animate-roblox-characters](https://github.com/dillydog580/animate-roblox-characters) | 18 | motion | safe | Python | MIT | AI-driven Roblox R6/R15 animation skill for Codex and Claude, using authentic Blender rigs, audited keyframing, Roblox export, and verified GIF previews. |
| [Aaryan-Kapoor/video-production-skill](https://github.com/Aaryan-Kapoor/video-production-skill) | 17 | general | safe | Python | MIT | Turn an AI agent into a full-stack educational video producer: research, storyboard, narration, animated visuals, render, QC, archive, and delivery. |
| [WhiteTowerAI/cut-as-code](https://github.com/WhiteTowerAI/cut-as-code) | 17 | general | safe | JavaScript | MIT | Open-source agentic video editing skills — an AI-powered alternative to Opus Clip and CapCut |
| [doublesq97-ui/su-card-to-video](https://github.com/doublesq97-ui/su-card-to-video) | 17 | general | safe | Shell | MIT | Minimal HTML/CSS card-to-video starter skill using HyperFrames, GSAP, and FFmpeg. |
| [gloweaseco-leo/hyperdirector](https://github.com/gloweaseco-leo/hyperdirector) | 17 | general | safe | JavaScript | Apache-2.0 | Hermes Skill Pack for structured AI video production on HyperFrames — brief → storyboard → HTML → lint/render. |
| [smit-vanani/claude-promo-video](https://github.com/smit-vanani/claude-promo-video) | 17 | promo | safe | JavaScript | MIT | Turn a website URL into a premium, beat-locked product promo video with Remotion — original design system every time, music-onset-locked scenes, self-verifying renders, 16:9 + 9:16 + thumbnails. |
| [cclank/lanshu-html2video-skill](https://github.com/cclank/lanshu-html2video-skill) | 17 | explainer | safe | TypeScript | MIT | Turn web articles into polished 1080p videos with Codex and Remotion |
| [ww-w-ai/super-video-agent](https://github.com/ww-w-ai/super-video-agent) | 17 | shorts | safe | JavaScript | NOASSERTION | Free, source-available Claude Code skill that draws narrated videos in code: YouTube Shorts, explainers, 3D flights. Install in two commands. |
| [yuwenbin121/book-video-production-skill](https://github.com/yuwenbin121/book-video-production-skill) | 17 | shorts | safe | Python | MIT | A Codex skill for producing fact-based vertical book videos |
| [garycaogan/pixel-diary](https://github.com/garycaogan/pixel-diary) | 17 | story | safe | Python | MIT | 用代码复刻“像素角色 + 对话框”类爆款视频：分集 JSON → HyperFrames 出片，附 Claude Code 复刻方法 skill |
| [Neelshah1810/Claude-Video-Skills](https://github.com/Neelshah1810/Claude-Video-Skills) | 16 | general | safe | HTML | MIT | 11 Claude skills that make professional motion-graphics videos (launch films, explainers, social ads, data stories) as HTML + MP4. Works with Claude and Claude Code. |
| [crawfordxx/xiaoma-studio](https://github.com/crawfordxx/xiaoma-studio) | 16 | general | safe | JavaScript | MIT | 用代码做 Opus 5.5 爆款级动效视频与 HTML 幻灯片的 skill：30 条高赞视频拆解出的工作流、审美规则与确定性渲染引擎，任何模型/agent 可用 \| Code-driven motion video & HTML slides skill for any coding agent |
| [Matteoikarieth96/whiteboard-video-skill](https://github.com/Matteoikarieth96/whiteboard-video-skill) | 16 | explainer | safe | HTML | MIT | Claude Code skill: turn any topic into a hand-drawn whiteboard explainer video with voice-over and captions |
| [indieshade/video-director-workflow](https://github.com/indieshade/video-director-workflow) | 16 | explainer | safe | HTML | MIT | Design-led AI video workflow: director skill, aesthetic guidance, blind comparisons and a reproducible HyperFrames collage sample |
| [iart-ai/kinetic-typography-skills](https://github.com/iart-ai/kinetic-typography-skills) | 16 | motion | safe | HTML | MIT | Kinetic typography skills for AI coding agents — turn text into beat-synced, animated type for social, titles, and lyric videos. |
| [iart-ai/data-animation-skills](https://github.com/iart-ai/data-animation-skills) | 15 | general | safe | HTML | MIT | Data video / animated infographic skills for Claude Code — turn a CSV into a chart that moves, with exact, accurate numbers on every frame. |
| [cblain100-prog/motion-design-claude-code](https://github.com/cblain100-prog/motion-design-claude-code) | 15 | promo | safe | HTML | MIT | Motion design de lancement façon agence avec Claude Code + HyperFrames (HeyGen), sans savoir coder : la méthode en skill, les patterns de 56 films, les modèles et 3 exemples. |
| [timscheuerai/launch-video-kit](https://github.com/timscheuerai/launch-video-kit) | 15 | promo | safe | Python | MIT | Turn your website into an Apple-style launch video in your own brand. Brand kit from your URL, a hook-plus-body film template, 10 agent skills for Claude Code and Codex, and a free audio path. MIT. |
| [intelligent-iterations/ii-content-engine](https://github.com/intelligent-iterations/ii-content-engine) | 15 | shorts | safe | HTML | MIT | AI-powered content generation and auto-posting engine built for Claude Code and Codex. Generate viral videos, carousels, and reels for TikTok, Instagram, and X using Grok. No API keys needed — uses browser cookies for both generation and posting. Template-first, agent-driven, free and open source. |
| [ouerf-man/kinetic-editorial-skill](https://github.com/ouerf-man/kinetic-editorial-skill) | 15 | shorts | safe | Python | — | Claude Code skill: word-by-word kinetic caption reels (9:16) built with HyperFrames + Three.js — storyboard, asset prep, offline SFX, build & preview. |
| [kiki-lgtm-dot/cinematic-product-promo-skill](https://github.com/kiki-lgtm-dot/cinematic-product-promo-skill) | 14 | promo | safe | TypeScript | Apache-2.0 | Remotion 产品宣传片 Agent Skill：150+ 镜头配方、200+ 动效预览、2.5D 运镜、节奏剪辑与声音设计。 |
| [welingtondesosa/video-motion](https://github.com/welingtondesosa/video-motion) | 14 | promo | safe | Python | NOASSERTION | Skill do Claude Code: vídeos de motion graphics feitos em código para qualquer marca (kit de marca, trilha sintetizada, MP4 revisado). |
| [chenjun198711/book-video-generator](https://github.com/chenjun198711/book-video-generator) | 14 | explainer | safe | Python | MIT | 《三分钟精读一本书》视频生成器 - Agent Skills 跨平台技能 \| 从扣子工作流移植，支持 WorkBuddy、OpenClaw、Codex、TRAE Work、Claude等。 |
| [ZiadAbdelkarim/beat-synced-edit](https://github.com/ZiadAbdelkarim/beat-synced-edit) | 14 | editing | safe | Python | MIT | Automatic beat-synced video editing: feed it a song and raw footage, it analyzes beats, energy, and scenes, then cuts a ready-to-post edit. Pure Python/ffmpeg CLI, plus a bundled Claude Code skill for natural-language editing. |
| [jincheng2026/jc-remotion-skills](https://github.com/jincheng2026/jc-remotion-skills) | 14 | editing | safe | TypeScript | NOASSERTION | Remotion Koubo Skill — 口播视频的 AI 成片工作流（Codex / Claude Code 双端）：粗剪+SRT 进，带 MG 包装、音效、母带的成片出 |
| [isaachang/motion-video-workflow](https://github.com/isaachang/motion-video-workflow) | 14 | shorts | safe | JavaScript | MIT | 一段配音，变成一支逐帧代码渲染的口播动效片 —— Claude Code / Codex Skill（9:16 · 16:9 · 60fps · 封面） |
| [erduo1998-cell/video-script-builder](https://github.com/erduo1998-cell/video-script-builder) | 14 | craft | safe | Python | MIT | 把中文视频逐字稿转换为可执行 HyperFrames 分镜规格的 AI Agent Skill |
| [AmirSbss/spotlight](https://github.com/AmirSbss/spotlight) | 13 | promo | safe | JavaScript | MIT | Open-source Agent Skill that turns a website, phone clips or a CSV into a finished short video: launch videos, promo reels and narrated explainers, made by Claude Code, Codex CLI or Hermes Agent. With music, captions in any language (Arabic and Persian too) and built-in quality checks. |
| [BusyBee3333/animated-explainer-skills](https://github.com/BusyBee3333/animated-explainer-skills) | 13 | explainer | safe | HTML | MIT | Build Kurzgesagt-quality animated explainer videos as one self-contained HTML file — GSAP choreography patterns, YouTube-retention method, overlap auditor, ElevenLabs VO pipeline. Claude Code skills + standalone tools. MIT. |
| [Science-Prof-Robot/recursive-math-animator](https://github.com/Science-Prof-Robot/recursive-math-animator) | 13 | explainer | safe | Python | MIT | Cursor and Claude Code skill: Manim animations, manim-voiceover, git-based scene versioning, optional Gemini TTS |
| [kakaxi12/vox-director-codex](https://github.com/kakaxi12/vox-director-codex) | 13 | explainer | safe | Python | — | Codex skill for creating Vox-style editorial paper-collage videos with ImageGen, HyperFrames, and MiniMax narration. |
| [fernandokaraka/remotion-motion-graphics-skill](https://github.com/fernandokaraka/remotion-motion-graphics-skill) | 13 | motion | safe | — | MIT | Claude Code skill: build animated motion graphics as real video with Remotion — title cards, lower thirds, overlays with real alpha for Premiere/FCP/CapCut. |
| [zhenwusw/orca-motion-skill](https://github.com/zhenwusw/orca-motion-skill) | 13 | motion | safe | JavaScript | MIT | Agent skill for in-scene motion graphics built from abstract, design-token-driven shapes; pairs with orca-transition-skill. |
| [PHANTOMTONYT/motion-design-skills](https://github.com/PHANTOMTONYT/motion-design-skills) | 12 | general | safe | — | MIT | Five animation styles as Claude skills: 3D toy, kinetic type, hand-drawn cartoon, system diagrams, data charts. Describe an idea, get an MP4. |
| [magichourhq/skills](https://github.com/magichourhq/skills) | 12 | general | safe | Python | MIT | AI media generation skills for Codex, Claude Code and other agents. Images, video and audio workflows using Magic Hour MCP or API. |
| [mrieck/demoday-claude-plugin](https://github.com/mrieck/demoday-claude-plugin) | 12 | promo | safe | JavaScript | MIT | Claude Code plugin that creates demo videos for your projects.  Claude is given tools to drive the browser/cli, make recordings, and uses FAL.ai, ElevenLabs, and remotion to build a demo video with scene transitions.    |
| [opus-pro/opusclip-video-tools](https://github.com/opus-pro/opusclip-video-tools) | 12 | promo | safe | TypeScript | — | OpusClip Video Tools for Codex and Claude Code: editable launch video templates and managed media generation. |
| [PoetCoderJun/MotionTalk](https://github.com/PoetCoderJun/MotionTalk) | 12 | editing | safe | Python | NOASSERTION | Speak your ideas. Turn a recording into a concise, polished video with your Agent. 保留原声，让好想法变成好视频。 |
| [cocolayuan/videoclip-AI-skill](https://github.com/cocolayuan/videoclip-AI-skill) | 12 | editing | safe | Python | MIT | Claude Skill-powered video editing toolkit - 视频全自动剪辑 |
| [onuryildriim/claude-kurgu-atolyesi](https://github.com/onuryildriim/claude-kurgu-atolyesi) | 12 | editing | safe | JavaScript | MIT | Claude Code ile konuşan-kafa YouTube videolarını kodla kurgulama atölyesi — Remotion + FFmpeg, kendi marka kimliğinle. |
| [chenmisss/laoxu-video-script](https://github.com/chenmisss/laoxu-video-script) | 12 | craft | safe | — | MIT | 老徐·自媒体起号方法论 skill：定位/选题/破立合成稿/五维诊断/去车轱辘话/112条母题库。零基础起号，Claude Code/Codex skill。 |
| [Kimeur/motion-launch-videos](https://github.com/Kimeur/motion-launch-videos) | 11 | general | safe | HTML | MIT | Claude Code plugin + skill: turn a short brief into a looping kinetic-typography launch video. One self-contained HTML file with a pure seek(t), closed-form springs, per-glyph motion, real motion blur and a pixel-exact loop, rendered to MP4 with Playwright + ffmpeg. |
| [suntay44/motionable](https://github.com/suntay44/motionable) | 11 | promo | safe | Swift | NOASSERTION | The latest and updated Claude Code plugin that turns your app, website or project folder into a hype video. AI plans a beat-synced motion-graphics launch video, composes original music and renders a 9:16 MP4 for TikTok, Reels and Shorts. Free, open source, macOS-only Swift engine: nothing to install. Also works with Codex via Agent Skills. |
| [liuliu-66-create/ll-video-edit](https://github.com/liuliu-66-create/ll-video-edit) | 11 | explainer | safe | TypeScript | — | Codex + Remotion 六步自动剪辑 Skill 与视频动效素材库 |
| [davidbfrogman/davecto-cutroom](https://github.com/davidbfrogman/davecto-cutroom) | 11 | editing | safe | JavaScript | — | Edit your talking-head videos with Claude: a local editor, a notes loop, captions and animations in your own brand. Claude Code plugin for macOS. |
| [misbahsy/tiktok-ig-shorts](https://github.com/misbahsy/tiktok-ig-shorts) | 11 | editing | safe | Python | — | Generate Viral Tiktok and Instagram Shorts using Hyperframes with Claude and Codex |
| [LetMeHappyCode/auto-video-agent](https://github.com/LetMeHappyCode/auto-video-agent) | 11 | story | safe | Python | — | 自动流水线化生成成套「分镜插画提示词 → AI 生图 → 拼接成片」的视频，基于 Agent / Skill / Workflow 编排。 |
| [diko0071/remotion-video-skills](https://github.com/diko0071/remotion-video-skills) | 10 | general | safe | TypeScript | MIT | Remotion video factory: deterministic product videos as code, with a reusable motion kit and measured-replica tooling |
| [JohnChow90/video-shotcraft](https://github.com/JohnChow90/video-shotcraft) | 10 | promo | safe | TypeScript | Apache-2.0 | Cinema-grade product video skill: 157 shot recipe cards + Remotion templates + 2.5D camera moves + sound design (originally by Vincentwei1021) |
| [Karnaf0katom/taskfilm](https://github.com/Karnaf0katom/taskfilm) | 10 | promo | caution | Python | MIT | Make short videos that teach people how to use your software. Local CLI and agent skill for feature walkthroughs and tool demos. |
| [syklayou-glitch/video-shotcraft](https://github.com/syklayou-glitch/video-shotcraft) | 10 | promo | safe | TypeScript | Apache-2.0 | An AI agent skill for crafting cinematic product videos with Remotion: 152 shot recipe cards, 2.5D camera moves, beat-synced cuts, and film-grade SFX. |
| [fltman/infofilm](https://github.com/fltman/infofilm) | 10 | explainer | safe | TypeScript | MIT | Claude Code skill: narrated infographic explainer films — 1950s newsreel cards, ElevenLabs narrator, Suno jingles, Gemini art, Remotion scenes cued to the spoken word |
| [jaredcassoutt/vox-editorial](https://github.com/jaredcassoutt/vox-editorial) | 10 | explainer | safe | Python | — | A Claude Code skill that turns one researched topic into a finished vertical explainer film: cut-paper artwork animated across depth planes, free local narration, assembled with ffmpeg. |
| [Fagan1024/smart-video-editor](https://github.com/Fagan1024/smart-video-editor) | 10 | editing | safe | Python | MIT | AI video editing skill that watches your footage before cutting. Vision model analyzes every frame, scores usability, drops bad takes, reorders by narrative, then renders with ffmpeg. For Claude Code, Cola, OpenClaw. 会看画面的 AI 自动剪辑 Skill——先用视觉模型看懂素材再决定剪哪几秒。 |
| [nomicsadmin/editor-video-ia](https://github.com/nomicsadmin/editor-video-ia) | 10 | editing | safe | JavaScript | NOASSERTION | Editor de vídeo dentro do Claude Code: manda o vídeo, o briefing e a referência, recebe o reels editado. Corte pela fala, legenda palavra a palavra, telas na palavra certa. Roda no seu Mac. |
| [qingyunAGI/qingyun-cine-skill](https://github.com/qingyunAGI/qingyun-cine-skill) | 10 | editing | safe | Python | — | qingyun-cine-skill: PLAN-FIRST cinematic video editing Codex skill for trailers, beat-sync edits, promos, and highlights. |
| [RahmanBuildsAI/ravyn-reel-kit](https://github.com/RahmanBuildsAI/ravyn-reel-kit) | 10 | shorts | safe | JavaScript | NOASSERTION | Script, edit, caption, cover and post short-form reels with Claude Code. ravyn (a browser video editor), 5 skills, 11 styles, a worked example. Free to use, not for resale. |
| [Samin12/samin-reel-engine-plugin](https://github.com/Samin12/samin-reel-engine-plugin) | 10 | shorts | safe | Python | NOASSERTION | A modular Codex plugin for source-backed reels: research, scripting, real B-roll, fast edits, quality review and giveaway handoffs. |
| [adriiita/vertical-video-editing-skill](https://github.com/adriiita/vertical-video-editing-skill) | 10 | shorts | safe | HTML | MIT | Claude skill: turn a script + talking-head media into a polished, creator-grade vertical 9:16 short with HyperFrames — editorial look, A-roll/B-roll cutting, eased camera moves, themed motion graphics, SFX, and a render verification gate. |
| [generalizingai/shadowcast](https://github.com/generalizingai/shadowcast) | 10 | shorts | safe | Python | MIT | Paste a YouTube channel link, get your own original channel in that format, produced and scheduled on autopilot. A Claude Code plugin. |
| [acelera-agency/html-animation](https://github.com/acelera-agency/html-animation) | 10 | motion | safe | JavaScript | MIT | Motion graphics as code. An agent skill for building frame-exact animated logos, kinetic typography and brand videos as a single HTML file, then exporting them to MP4 in 1:1, 4:5 and 9:16. |
| [Vandit1604/vawe](https://github.com/Vandit1604/vawe) | 9 | general | safe | JavaScript | Apache-2.0 | Open-source HTML to video / programmatic video framework for AI agents. Free Remotion alternative. |
| [VasiniDevi/motion-skills](https://github.com/VasiniDevi/motion-skills) | 9 | general | safe | Python | — | Programmatic motion design skills for AI agents — motion graphics, kinetic typography, data viz, slide decks in Python |
| [molkex/mcp-flow-google](https://github.com/molkex/mcp-flow-google) | 9 | general | safe | — | NOASSERTION | MCP server for Google Flow — Veo video and Nano Banana images inside Claude Code, Cursor or any MCP client. Runs on your own Google account: images free, video on your credits. |
| [resemble-ai/remotion-resemble-skill](https://github.com/resemble-ai/remotion-resemble-skill) | 9 | general | safe | — | MIT |  |
| [ssrajadh/paperview](https://github.com/ssrajadh/paperview) | 9 | explainer | safe | Python | Apache-2.0 | Turn research papers, codebases, etc. into explainer videos, local rendering + TTS, Claude Code plugin. |
| [mcdenil-skills/AutoMontage-Agent](https://github.com/mcdenil-skills/AutoMontage-Agent) | 9 | shorts | safe | JavaScript | MIT | Движок автомонтажа видео для ИИ-агента (Claude Code/Codex): субтитры, плашки, картинки под смысл, музыка, вертикаль. Локально, Remotion. |
| [zlong6943-commits/football-prediction-avatar-video](https://github.com/zlong6943-commits/football-prediction-avatar-video) | 9 | avatar | safe | Python | — | Codex skill for verified Chinese football prediction avatar videos with official logos, captions, motion graphics, and approval gates. |
| [taylorzhou16/video-gen](https://github.com/taylorzhou16/video-gen) | 9 | story | safe | Python | — | AI Video Editing Skill for Claude Code |
| [awesome-skills/manim-skill](https://github.com/awesome-skills/manim-skill) | 9 | motion | safe | Python | MIT | Manim animation skill for Claude Code - create programmatic animations for technical blogs and educational content |
| [sharon-laicc/viral-video-decomposer](https://github.com/sharon-laicc/viral-video-decomposer) | 9 | craft | safe | Python | MIT | 拆解爆款短视频，生成镜头级拉片、爆款机制、AI 生产蓝图和视频生成 brief 的 Codex/Claude skill。｜A Codex/Claude skill for decomposing hot short videos into shot-by-shot analysis, reusable production blueprints, and AI-ready video generation briefs. |
| [latent-variable/deep-learning-birthday](https://github.com/latent-variable/deep-learning-birthday) | 9 | music | safe | JavaScript | MIT | Happy Birthday to Me (Self-Supervised): a riso-print music video for AlexNet's 14th birthday, made with local open models and Claude |
| [chenyuxiaojin/cyxj-remotion-starter](https://github.com/chenyuxiaojin/cyxj-remotion-starter) | 8 | general | safe | TypeScript | MIT | 用 Claude Code 指挥 Remotion 出视频的工作台脚手架:三层架构 + hooks 守规矩 + skills 编排流程 \| by 陈与小金 |
| [davidcervinka/vibe-editing](https://github.com/davidcervinka/vibe-editing) | 8 | general | safe | Python | MIT | Vibe coding, but for video. Launch films, aftermovies & reels — cut, scored and shipped by talking to Claude Code. Full stack + flow. |
| [patraxo/ltx2-vidgen-skill](https://github.com/patraxo/ltx2-vidgen-skill) | 8 | general | safe | Python | NOASSERTION | Own your AI video pipeline. LTX-2.3 (22B) self-hosted on your Modal GPU via a Claude Code skill — t2v, i2v, keyframes, v2v + synced audio. ~$0.02 per 5s clip, idle = $0. |
| [jammaru/launch-video-craft](https://github.com/jammaru/launch-video-craft) | 8 | promo | safe | TypeScript | MIT | Agent Skill for creating polished product launch videos with real UI using Remotion. For Cursor, Claude Code, Codex, and Copilot. |
| [0x68616F4C/auto-popsci](https://github.com/0x68616F4C/auto-popsci) | 8 | explainer | safe | Go Template | — | Agent Skill for generating math/science popularization videos with Manim + AI narration |
| [dy-hashiqi/VibeDS-skill](https://github.com/dy-hashiqi/VibeDS-skill) | 8 | explainer | safe | — | Apache-2.0 |  |
| [gquthier/autonomous-sop-video](https://github.com/gquthier/autonomous-sop-video) | 8 | explainer | safe | Python | MIT | CREATIVE · SOP VIDEO — Turn any workflow into a narrated tutorial. Agent skill, screenshots, button zooms, checklists and chapters. |
| [undsky/doudou-remotion-whiteboard-skill](https://github.com/undsky/doudou-remotion-whiteboard-skill) | 8 | explainer | safe | TypeScript | NOASSERTION | 专为 Remotion 打造的通用手绘白板、纸质媒介与草图动效（Whiteboard & Hand-Drawn Animation）引擎。 |
| [rahlquist/hermes-video-toolchain](https://github.com/rahlquist/hermes-video-toolchain) | 8 | editing | caution | Shell | NOASSERTION | Complete free/open-source video production pipeline for Hermes Agent: OpenScreen capture + HyperFrames authoring + headless post-processing, wired into one skill. |
| [riffkit/skill](https://github.com/riffkit/skill) | 8 | shorts | safe | — | MIT | Official Riffkit skill — riff a winning TikTok into your own short video from your AI agent (Claude Code, Cursor) or the browser. Riff the formula, not the video. |
| [xinyiqin/X2V-AI-Images-Videos-skill](https://github.com/xinyiqin/X2V-AI-Images-Videos-skill) | 8 | avatar | safe | Shell | — | [LightX2V](https://x2v.light-ai.top) integration for [OpenClaw](https://openclaw.ai) — image generation (t2i/i2i), video (t2v/i2v/s2v), TTS, and voice clone via cloud API. |
| [neelnanda-io/dont-go-quiet-on-me](https://github.com/neelnanda-io/dont-go-quiet-on-me) | 8 | music | safe | JavaScript | NOASSERTION | Don't Go Quiet On Me: a song and hand-drawn-in-code music video about the history of mech interp, made with Claude Opus 5.5 and Suno v6 |
| [argus-metis/hermes-video-editing](https://github.com/argus-metis/hermes-video-editing) | 7 | general | caution | Python | MIT | Professional video editing skill for Hermes Agent — AI captioning, silence removal, scene detection, stabilisation, smart reframing, and more. All free and open source. |
| [hraness/slopcamera](https://github.com/hraness/slopcamera) | 7 | general | safe | TypeScript | MIT | SlopCamera lets your coding agent make images, diagrams, animation, 3D scenes, and video from source files it can keep revising. |
| [vibe-motion/remotion-starter](https://github.com/vibe-motion/remotion-starter) | 7 | general | safe | TypeScript | MIT | 用 Claude Code 指挥 Remotion 出视频的工作台脚手架:三层架构 + hooks 守规矩 + skills 编排流程 \| maintained by 陈与小金 |
| [Anshgrover23/product-demo-playbook](https://github.com/Anshgrover23/product-demo-playbook) | 7 | promo | safe | HTML | — | Showreel: your coding agent makes your product demo video. Agent skill, templates, and the full manual. |
| [holy-templar/static-to-video-ad](https://github.com/holy-templar/static-to-video-ad) | 7 | promo | safe | — | AGPL-3.0 | Turn any static ad into a finished video ad — two treatments, one skill: a real hand builds your ad on camera, or live footage leads up to your photo and freezes on it. Free Claude skill for the MaxFusion AI MCP. |
| [cymcymcymcym/notes-to-video](https://github.com/cymcymcymcym/notes-to-video) | 7 | explainer | safe | Python | MIT | Turn notes into animated explainer videos in the style popularized by 3Blue1Brown. Claude Code skill + Manim + TTS + ffmpeg. |
| [jinjin1/scenewright](https://github.com/jinjin1/scenewright) | 7 | explainer | safe | TypeScript | MIT | Claude Code-native pipeline: a source text → narrated, Remotion-rendered YouTube explainer video. Any topic, ~$0/video, local Korean TTS (Supertonic), JSON-as-contract stages. |
| [pjecuacion/script-to-video-skill](https://github.com/pjecuacion/script-to-video-skill) | 7 | explainer | safe | — | MIT | Public script-to-video skill for HyperFrames workflows with sanitized paths and secret-free examples. |
| [sukai213/bilibili-ai-video](https://github.com/sukai213/bilibili-ai-video) | 7 | explainer | safe | Python | — | AI 科技测评视频制作与 B 站自动发布技能（Codex Skill）：原创文案 / Qwen3-TTS 配音 / 全息幻灯片 / ffmpeg 剪辑 / 投稿 API 自动发布 |
| [griffinwooldridge/talking-head-editor](https://github.com/griffinwooldridge/talking-head-editor) | 7 | editing | safe | JavaScript | NOASSERTION | A Claude skill that edits raw talking-head footage: tight cuts, clean voice, captions and motion graphics as code |
| [marketcalls/story-telling](https://github.com/marketcalls/story-telling) | 7 | shorts | safe | TypeScript | MIT | Turn a context into a narrated story video: Sarvam AI voiceover, FLUX 2 artwork, Remotion render. Ships Claude Code skills. |
| [wendymardigian-prog/Editor-IA-Pro](https://github.com/wendymardigian-prog/Editor-IA-Pro) | 7 | shorts | safe | TypeScript | NOASSERTION | AI-powered video editor built with Remotion and Claude Code |
| [zoglmk/codex-video-pipeline](https://github.com/zoglmk/codex-video-pipeline) | 7 | shorts | safe | Python | MIT | 从选题、脚本和素材到成片、封面与发布包的 AI 视频生产流水线 Skill |
| [sudadom/arabic-motion-graphics-skill](https://github.com/sudadom/arabic-motion-graphics-skill) | 7 | motion | safe | TypeScript | MIT | Agent skill for Arabic RTL motion graphics — reels, explainers, logo reveals with Remotion |
| [BuzyOwl/song-video-director](https://github.com/BuzyOwl/song-video-director) | 7 | music | safe | Python | MIT | Claude Code skill: direct and build code-rendered music videos (Remotion, p5.js, three.js) |
| [bishpls/animation-pipeline](https://github.com/bishpls/animation-pipeline) | 7 | music | safe | Python | — | Films made with Claude Code: songs, cut-paper riso animation, kinetic type, all in code. First work: OPEN ALL NIGHT (a love song for the ads). |
| [kmilo93sd/agent-video-editor](https://github.com/kmilo93sd/agent-video-editor) | 6 | general | safe | HTML | MIT | Editing judgment for AI agents: pacing, cuts, captions, audio, color and thumbnails for any video type, plus 128 CC0 sounds, 26 CC0 graphics and a meme rights reference. Agent Skills format. |
| [liaojianquan9-source/ljq-broll-video-generator](https://github.com/liaojianquan9-source/ljq-broll-video-generator) | 6 | general | safe | JavaScript | MIT | 基于参考素材打点、抽帧、拆解并用 Remotion 生成可编辑 B-roll 的 Codex Skill |
| [ChenShuo2004/cs-shotcraft-skill](https://github.com/ChenShuo2004/cs-shotcraft-skill) | 6 | promo | safe | TypeScript | Apache-2.0 | CS Skills 产品宣传片 skill：真实产品界面、镜头表运镜、152 张镜头配方卡与 Ink Press 模板。Remotion 出片，配乐默认可选。 |
| [Hainrixz/motion-architect](https://github.com/Hainrixz/motion-architect) | 6 | promo | safe | Python | MIT | Agente de motion graphics para Claude Code. Le das videos, imágenes y tu marca; él diseña, anima y renderiza en 9:16, 16:9, 1:1 y 4:5. Todo local. |
| [farming-labs/product-film-skill](https://github.com/farming-labs/product-film-skill) | 6 | promo | safe | — | MIT | Claude skill for making product launch videos as one HTML file, exported to MP4 |
| [hamen/app-promo-reel](https://github.com/hamen/app-promo-reel) | 6 | promo | safe | Python | MIT | Claude Code skill: beat-synced 9:16 app promo reels with HyperFrames |
| [haxzie/genmotion](https://github.com/haxzie/genmotion) | 6 | promo | safe | TypeScript | — | Create motion graphics and launch videos using claude code |
| [hub-ki/video-generation](https://github.com/hub-ki/video-generation) | 6 | promo | safe | JavaScript | MIT | Agent skills that turn an app or a public website into a designed demo video: concept and script first, then capture, narration, verification and a 4K render. |
| [muayadjamal/flow-video-recorder](https://github.com/muayadjamal/flow-video-recorder) | 6 | promo | safe | JavaScript | MIT | Agent skill: record narrated, captioned demo videos of web (Playwright) and native iOS (Maestro) apps, with ElevenLabs voice-over and motion graphics |
| [Ali-Aria/aria-animator](https://github.com/Ali-Aria/aria-animator) | 6 | explainer | safe | TypeScript | MIT | 阿梨动画机（Aria Animator）：使用 Remotion、豆包配音和霞鹜文楷制作可编辑动画讲解视频的 AI Agent Skill。 |
| [Lunamos/paper2video](https://github.com/Lunamos/paper2video) | 6 | explainer | safe | Python | MIT | Claude Code skill: turn a research paper or project into a 3Blue1Brown-style explainer video with Remotion (script, narration, real-data animation, captions, covers, YouTube/Bilibili copy) |
| [Nealsun1993/talk-workbench](https://github.com/Nealsun1993/talk-workbench) | 6 | explainer | safe | TypeScript | MIT | 口播工作台：配音驱动的竖屏讲解视频 Skill、Remotion 组件库与动效规范。 |
| [centraltowerlabs/diagram-tour](https://github.com/centraltowerlabs/diagram-tour) | 6 | explainer | safe | Python | MIT | This Claude Skill makes it easy to generate explainer video tours of your codebase (or any other complex system) |
| [isabellagreco1997/script-to-video](https://github.com/isabellagreco1997/script-to-video) | 6 | explainer | safe | Python | MIT | Turn a script into a narrated, word-synced explainer video (16:9 YouTube or 9:16 Reels/Shorts). Free local Kokoro voice, licensed web images, clips, browser-rendered motion. Python CLI + Claude Code skill. |
| [mathruffian-dot/claude-motion-explainer-images](https://github.com/mathruffian-dot/claude-motion-explainer-images) | 6 | explainer | safe | Python | MIT | 圖片加強版：用一句話讓 coding agent 做出有授權照片與 AI 示意圖的原理解說動畫（HyperFrames + 中文旁白）｜Explainer animations with a licensed-photo and generated-image layer |
| [brookejlacey/boring-tools](https://github.com/brookejlacey/boring-tools) | 6 | editing | safe | Python | MIT | Agent-driven video editing where the edit is text. No timeline, no project file, nothing uploaded. ffmpeg + whisper + python, and a SKILL.md your agent reads. |
| [hahadu4520/newcut-video-clipping-skill](https://github.com/hahadu4520/newcut-video-clipping-skill) | 6 | editing | safe | JavaScript | MIT | Open-source Codex skill for semantic Chinese video clipping with hooks, captions, cleanup, and FFmpeg rendering |
| [aaksa/facecammaxxing](https://github.com/aaksa/facecammaxxing) | 6 | shorts | safe | TypeScript | NOASSERTION | Claude Code skill: pro motion-graphics effects on your own talking-head clip - Remotion + GSAP + Three.js over a clean cut-out (text behind your head, 3D card rings, tier lists, kinetic captions, punch-ins) |
| [kvrancic/oneshot-video](https://github.com/kvrancic/oneshot-video) | 6 | shorts | safe | Python | MIT | One-shot AI video studio for Claude Code, Codex and Cursor. Raw footage in: a finished edit, captioned clips, or a promo from a brief. Local on a Mac; hands the timeline back to Premiere, Final Cut and Resolve. |
| [mattypark/Motion-graphics-skill](https://github.com/mattypark/Motion-graphics-skill) | 6 | shorts | safe | — | — | Best Motion graphics and video editing skill for Claude Code, Codex and Cursor. Plan, animate, edit, caption and sound-design complete videos, rendered locally without After Effects. |
| [mikelix/gstack-tutorial-5-agi-race-v2](https://github.com/mikelix/gstack-tutorial-5-agi-race-v2) | 6 | shorts | safe | JavaScript | MIT | gstack Tutorial #5: from YouTube analytics to a re-engineered Short with CSR-RAP, Claude Opus 5.5 and HyperFrames (EN/ZH) |
| [ninestonelee/youtube-shorts-factory](https://github.com/ninestonelee/youtube-shorts-factory) | 6 | shorts | safe | — | MIT | Claude Code skill — YouTube reference video analysis → benchmarking → 30%+ adaptation → AI Shorts production workflow |
| [puntorigen/avatar-skills](https://github.com/puntorigen/avatar-skills) | 6 | avatar | safe | Python | MIT | Cloud-based agent skills for creating AI avatar talking-head videos and short-form reels (skills.sh format) |
| [hacimertgokhan/geomotion](https://github.com/hacimertgokhan/geomotion) | 6 | motion | safe | JavaScript | MIT | Hand-drawn morphing line animations from simple shapes. Zero-dependency JS engine + live studio, CLI, MCP server & Claude skill. Export animated SVG, Lottie, HTML or a shareable link. |
| [rock3r/cav](https://github.com/rock3r/cav) | 6 | motion | safe | Go | MIT | Build, check and render Cavalry motion graphics from the terminal: the cav CLI and an agent skill |
| [MakeMyClip/editor](https://github.com/MakeMyClip/editor) | 5 | general | safe | TypeScript | MIT | AI-native video editor — talk to make video. Trim, zoom, caption, and assemble clips from any AI agent (Claude, Cursor, Copilot). Delivered as an MCP server + agent skill https://www.npmjs.com/package/@makemyclip/editor. |
| [iart-ai/text-message-video-skills](https://github.com/iart-ai/text-message-video-skills) | 5 | general | safe | HTML | MIT | Text-message animation skills for AI coding agents — turn a chat script into a scroll-stopping iMessage/WhatsApp/SMS conversation video, rendered from a messages array. |
| [oxygenframe/oxframe-sdk](https://github.com/oxygenframe/oxframe-sdk) | 5 | general | safe | Rust | Apache-2.0 | Motion graphics as Rust code, made for AI agents. A scene is an async fn; oxframe check returns a storyboard and a report that points to the line to fix. Native, deterministic rendering to MP4, WebM, ProRes or PNG — no browser, no Node. Text, code, charts, matrices, formulas, 3D, sound. |
| [siegerts/skill-canvas-video](https://github.com/siegerts/skill-canvas-video) | 5 | general | safe | JavaScript | MIT | Agent skill for creating animated videos using HTML5 Canvas and JavaScript, exportable to MP4. |
| [Teseife/diffusion-studio-codex](https://github.com/Teseife/diffusion-studio-codex) | 5 | promo | safe | Python | MIT | Independent Codex and Claude Code plugin for local video editing with Diffusion Studio. |
| [patryk-penczek/ship-video](https://github.com/patryk-penczek/ship-video) | 5 | promo | safe | JavaScript | MIT | Turn your app into a music-synced launch film, as code. An agent skill for Claude Code and Codex: real UI capture, cuts on the beat, frame-exact MP4 with Playwright, GSAP and ffmpeg. |
| [sub-level/marketing-videos](https://github.com/sub-level/marketing-videos) | 5 | promo | safe | TypeScript | MIT | Make marketing videos with a coding agent: Remotion + Claude Code skills + the AI footage pipeline behind our launch films |
| [surajshetty3416/demo-video-skill](https://github.com/surajshetty3416/demo-video-skill) | 5 | promo | safe | JavaScript | MIT | Claude Code skill: polished 60fps ScreenStudio-style demo videos of any web app via Playwright + Pillow + ffmpeg (no screen recorder) |
| [xsourabhsharma/remotion-marketing-video-skill](https://github.com/xsourabhsharma/remotion-marketing-video-skill) | 5 | promo | safe | PowerShell | MIT | Agent skill for creating premium Remotion marketing videos product demos, SaaS promos, motion graphics, UI walkthroughs, social ads, and explainers using React and code-first animation. |
| [GODGOD126/opus-5-5-86-styles-in-motion](https://github.com/GODGOD126/opus-5-5-86-styles-in-motion) | 5 | explainer | safe | JavaScript | — | 86 visual styles for code-driven explainer animations. A Codex skill: choose a style, preview 20 seconds, then create the full HTML video. |
| [OnurGumus/codebase-video](https://github.com/OnurGumus/codebase-video) | 5 | explainer | safe | JavaScript | MIT | Turn a codebase into a fact-checked narrated video. A Claude Code plugin: one agent reads the code, fresh agents verify every claim, then the video is made locally. |
| [arthurkatcher/explainer-video](https://github.com/arthurkatcher/explainer-video) | 5 | explainer | safe | TypeScript | — | Agent skill: motion-graphics explainer videos made entirely in code (Remotion + numpy music) |
| [beriki770-ship-it/code-animations](https://github.com/beriki770-ship-it/code-animations) | 5 | explainer | safe | HTML | MIT | Claude Code skill: videos where a program draws every frame (Canvas 2D + Web Audio), rendered headless to MP4 in 16:9 / 9:16 / 4:5 |
| [coding-ax/docvideoer](https://github.com/coding-ax/docvideoer) | 5 | explainer | safe | JavaScript | MIT | 文档转视频 skills。用于把文档 URL、网页文章、Markdown 或粘贴文本转换成带中文旁白的 Remotion 讲解视频，包含正文提取、分镜生成、免费 TTS、视频项目搭建和 MP4 渲染。 |
| [crawfordxx/paper-cut-video-skill](https://github.com/crawfordxx/paper-cut-video-skill) | 5 | explainer | safe | TypeScript | MIT | Script-driven Chinese paper-cut explainer videos with content-matched characters, Remotion, Doubao TTS, and semantic captions |
| [jackjls/jtxvideo-skill](https://github.com/jackjls/jtxvideo-skill) | 5 | editing | safe | Python | MIT | Codex + HyperFrames 口播视频制作 Skill：按 SRT、结构、分镜、设计、渲染的审核流程生成 AI 金融短视频 |
| [vincentventalon/claude-code-video-editing-skill](https://github.com/vincentventalon/claude-code-video-editing-skill) | 5 | editing | safe | Python | MIT | Les skills Claude Code que j'utilise vraiment sur mes sites et mes vidéos, une nouvelle chaque mardi (montage vidéo, SEO Search Console…) |
| [beatra-ai/viral-video-remake-skill](https://github.com/beatra-ai/viral-video-remake-skill) | 5 | shorts | safe | Python | MIT-0 | Viral video remake agent skill for Claude Code, Cursor, Codex, Windsurf, Cline & OpenClaw: break a short video that worked into its hook, beats, and call to action, then rebuild it around your own subject as a shot list, beat frames, narration, and a vertical clip or segmented sources. 爆款视频拆解 · 短视频复刻 |
| [icytear-svg/podcast-to-video](https://github.com/icytear-svg/podcast-to-video) | 5 | shorts | safe | Python | — | Codex skill for turning podcast episodes into platform-ready captioned videos |
| [inematds/skill-video-plan-editor](https://github.com/inematds/skill-video-plan-editor) | 5 | shorts | safe | HTML | — | Skill que transforma assunto/link em plano profissional de edição de vídeo (renderer-agnóstico) + render opcional via FFmpeg/Auto-Editor/HyperFrames |
| [liangdabiao/wechat-article-remotion](https://github.com/liangdabiao/wechat-article-remotion) | 5 | shorts | safe | Python | — | 动画skill:  把任意一篇微信公众号文章 转成 Studio 风格的 Remotion 视频 —— 暖白画布 + 上下镜像透视格子 + 顶部章节进度 + 公众号原文图完整保留 |
| [beatra-ai/talking-avatar-video-skill](https://github.com/beatra-ai/talking-avatar-video-skill) | 5 | avatar | safe | Python | MIT-0 | Talking avatar video agent skill for Claude Code, Cursor, Codex, Windsurf, Cline & OpenClaw: turn one portrait and a script or voice track into an AI presenter video for explainers, lessons, and product messages. 数字人口播视频 · 照片说话 |
| [i-afaqrashid/claude-animation-studio](https://github.com/i-afaqrashid/claude-animation-studio) | 5 | story | safe | JavaScript | NOASSERTION | Claude Code plugin: animated short films where every frame is drawn in code and the music is synthesized in code, perfectly synced. |
| [CJ10110425/logo-intro](https://github.com/CJ10110425/logo-intro) | 5 | motion | safe | JavaScript | MIT | WebGL logo intros from any SVG logo: lens-warped close-up, wordmark reveal, gradient bloom, lock-in ring and shimmer. JS module, single-file HTML, video export and a Claude skill. |
| [iart-ai/youtube-video-skills](https://github.com/iart-ai/youtube-video-skills) | 5 | motion | safe | HTML | MIT | Audiogram and YouTube intro/outro skills for Claude Code — brand your channel and repurpose every episode into motion graphics. |
| [liuliu-66-create/ll-video-motion](https://github.com/liuliu-66-create/ll-video-motion) | 5 | motion | safe | TypeScript | — | Turn Chinese copy into local Remotion motion graphics with a Codex Skill |
| [10291124jcjc-dotcom/xp-desktop-mv](https://github.com/10291124jcjc-dotcom/xp-desktop-mv) | 5 | music | caution | JavaScript | — | Claude Code skill: turn a music-video clip into a beat-synced Windows XP desktop screen recording (example: APT. -> Pinkdows XP) |
| [lumian2015/opus-5-5-video-templates](https://github.com/lumian2015/opus-5-5-video-templates) | 3 | promo | safe | HTML | MIT | 30 free motion-graphics templates remade from viral Claude Opus 5.5 videos, plus a Claude Code skill that rebrands one and renders it to MP4 with HyperFrames. |
| [samuelstroschein/opus-video-generator](https://github.com/samuelstroschein/opus-video-generator) | 2 | promo | safe | TypeScript | MIT | Make launch videos with Claude Opus 5.5, in your browser, on your own Claude subscription. npx opus-video-generator |
| [RohanRatwani/explainer-video-opus-5.5](https://github.com/RohanRatwani/explainer-video-opus-5.5) | 1 | explainer | safe | JavaScript | MIT | Claude Code skill: turn a topic into a motion-design explainer video (YouTube 16:9 or Shorts) with an AI voice or your own, every animation timed to the narration. Built with Claude Opus 5.5. |
| [xdis/opus-video-studio](https://github.com/xdis/opus-video-studio) | 1 | shorts | safe | Python | NOASSERTION | A short-video studio where Claude Opus 5.5 writes the script and the code, and your Mac renders the video: Canvas/Three.js/Blender code animation plus local FLUX + Wan2.2 pipelines. |

## How a repo gets on the list

From the list's README, which states the rules for the maintainer:

1. It makes or edits video or motion graphics. A 3D web page or a prompt collection does not count.
2. An agent operates it: a skill, a plugin, an MCP server, or a toolkit written for the agent.
3. It has a README. Without one it cannot be graded.
4. At 50 stars or more it is listed on topic alone. Under 50 it must also clear a README
   quality bar and have 5 stars unless it names Claude Opus 5.5.

The answers are produced by a decision model reading each README, not by hand — so a repo near
a cut-off can land on either side.

## Overlap with what our research already knows

| Repo in our docs | Stars | In this list? |
|---|---:|---|
| `heygen-com/hyperframes` | 73,569 | yes — general, safe |
| `lemomo-ai/lemo-opuscar` | 8,439 | yes — story, safe |
| `JohnHeibel/PDoomVideo` | 2,642 | yes — music, safe |
| `buildwithhanif/claude-animation-skill` | 48 | yes — story, safe |
| `Vincentwei1021/mg-styles-15` | 2,413 | **no** — cited in synthesis.md |
| `LottieFiles/motion-design-skill` | 2,553 | **no** — cited in synthesis.md |
| `mexicat/pdoom-video` | 3,558 | **no** — cited in synthesis.md |
| `nateherkai/herk-edit-kit` | 2,270 | **no** — cited in competitors.md |

Four repos we already use are **not** in the catalogue: it is a wide net, not a superset.
Its inclusion rules lean on the README mentioning the agent/video topic, so a strong repo can
fall outside it.

## Caveats

- The catalogue is **generated**, not curated by hand: `data/skills.json` carries a
  `generated` stamp and names the live page as its source. The four inclusion rules are
  applied by a decision model reading each README, so the list is a reasonable filter, not
  an authority. The maintainer says so: *"A repo near a cut-off can land on either side."*
- The measured test covers **16 of 362**. They were picked for the test, not at random, and
  the brief is one promo with fixed copy — the hardest question the test asks is whether a
  line of copy survives, not whether the film is good.
- *"How good a film looks is not scored."* The look column is the maintainer's eye. The
  video files and prompts are on the evidence pages, linked from each row.
- The test ran on **one machine** and one driver (Claude Code, Opus 5.5), with no API keys
  for video, image or voice models — so a skill that needs a paid model was scored on a
  degraded path, not on its best path.
- Star counts and security grades are as of the `generated` stamp and drift.
- The star counts in the measured-results table are the **test snapshot**, read at source on
  2026-10-11, and are older than the catalogue's. HyperFrames alone reads 60,291 in the test
  and 73,569 in the catalogue — the same repo, two dates. Do not compare the two columns
  directly.
