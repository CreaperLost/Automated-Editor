# AI Motion Graphics & Video Agent Research Synthesis

This directory contains in-depth research reports analyzing cutting-edge open-source tools, agent skills, review pipelines, creator workflows, and motion graphics engines. The overarching objective is to identify the best architectural patterns and integrate them into **AeroEdits** to build the most capable AI-native video editor.

---

## 1. Research Catalog

| Document | Subject | Author / Org | Core Strength |
| :--- | :--- | :--- | :--- |
| [**`framejam.md`**](framejam.md) | **FrameJam** | Moritz Kremb | Frame-accurate spatial pin review (`pin: {x,y}`), multi-round iteration loop, playbooks. |
| [**`hyperframes.md`**](hyperframes.md) | **HyperFrames** | HeyGen | Deterministic HTML/CSS/GSAP $\to$ video engine, automated contrast checks (`check`), alpha channels. |
| [**`video-use.md`**](video-use.md) | **Video-Use** | Browser Use Team | Audio-first transcript reasoning, 30ms anti-pop cut crossfades, segment grading. |
| [**`broll-mcp.md`**](broll-mcp.md) | **Broll-MCP** | Edigeh | Remote scene detection (PySceneDetect), Archive.org extraction, verified legal provenance JSON. |
| [**`ffmpeg-skill.md`**](ffmpeg-skill.md) | **FFmpeg-Skill** | Kajisho5 | 42 strongly-typed tools, `doctor` environment probing, machine-readable contracts (`contract --json`). |
| [**`kitcut.md`**](kitcut.md) | **KitCut** | KitCut HQ | Local timeline-free MCP pipeline, multicam audio sync, automated PII/secret blurring, `journal.md`. |
| [**`video-shotcraft.md`**](video-shotcraft.md) | **Video-Shotcraft** | Vincent Wei | 120+ pre-engineered 2.5D Shot Recipe Cards, beat synchronization, integrated SFX sound cues. |
| [**`impeccable.md`**](impeccable.md) | **Impeccable** | Paul Bakaus | 60+ mechanical anti-pattern linters (no-raw-black, contrast gate, no-nested-boxes), design commands. |
| [**`ui-ux-pro-max-skill.md`**](ui-ux-pro-max-skill.md) | **UI-UX-Pro-Max** | NextLevelBuilder | 200+ product-specific palettes, font pairings, and Bento Grid layout grammars in a searchable DB. |
| [**`lemo-opuscar.md`**](lemo-opuscar.md) | **Lemo-Opuscar** | Lemomo AI | 100% code-driven animation (Canvas/WebGL/Three.js + FFmpeg), procedural artistic styles (ink wash, whiteboard). |
| [**`diagram-design.md`**](diagram-design.md) | **Diagram-Design** | Cathryn Lavery | 45 editorial diagram archetypes (architectures, pipelines, 2x2 grids), "anti-Mermaid", orthogonal routing. |
| [**`higgsfield-skills.md`**](higgsfield-skills.md) | **Higgsfield Skills** | Higgsfield AI | Consistent character IDs (`Soul ID`), native NLE extensions, "taste and assets travel together" (bundled SFX). |
| [**`riverside-mcp.md`**](riverside-mcp.md) | **Riverside MCP** | Riverside.fm | 60+ MCP methods, cross-archive transcript search, transactional batch editing (`editing_batch`), social publish. |
| [**`zane-hoyer-automated-editing.md`**](zane-hoyer-automated-editing.md) | **Zane Hoyer Agency Workflow** | Zane Hoyer | $50k/mo agency assembly line (ChatGPT cuts + Claude Code Remotion motion + OpenArt B-roll). |
| [**`jake-moran-hyperframes-peter-yang.md`**](jake-moran-hyperframes-peter-yang.md) | **Jake Moran & Peter Yang Masterclass** | Jake Moran (HeyGen) & Peter Yang | Co-creator of HyperFrames reveals 5-step playbook (`frame.md`, `storyboard.md`, keyframe gate, component library). |
| [**`matt-loui-capcut-automation.md`**](matt-loui-capcut-automation.md) | **Matt Loui CapCut Automation** | Matt Loui (CapCut Lab) | Programmatic timeline generation via `draft_content.json` reverse-engineering; moodboard-driven aesthetics. |
| [**`sirio-berati-youtube-ai-system.md`**](sirio-berati-youtube-ai-system.md) | **Sirio Berati YouTube AI System** | Sirio Berati | Open-source pipeline with teleprompter metadata logging, script-guided retake cutting, and visual boards. |
| [**`jad-mh-ai-editing-skill.md`**](jad-mh-ai-editing-skill.md) | **NextPhase Editor V1 (Jad M.H)** | Jad M.H (NextPhase) | Empirical documentary edit: 0.6–1.5s picture changes, 18 named devices (`tile_reveal`, `wall3d`, `pop_collage`), procedural sound synthesis (`synth-sfx.py`), punchline cuts. |
| [**`higgsfield-gpt6-astra-davinci.md`**](higgsfield-gpt6-astra-davinci.md) | **GPT-6 Astra + DaVinci Resolve** | Higgsfield AI | Autonomous multimodal agent driving DaVinci Resolve via native MCP, automated node grading, and Seedance B-roll. |
| [**`claude-video-2-jack-roberts.md`**](claude-video-2-jack-roberts.md) | **Claude Video 2.0** | Jack Roberts | HyperFrames motion graphics framework: 21 visual styles, 5 hacks, universal ban list, 3-still review gate, automated pacing & loop linters. |
| [**`jason-cooperson-claude-video-editor.md`**](jason-cooperson-claude-video-editor.md) | **Claude Video Editor & Blueprint** | Jason Cooperson & Lana Epifanova | Local studio architecture (86k LOC, 20+ skills, Claude artifact `BYH6Mcma5to3kt2KogkEzB`), `presets/` taste compounding, empirical silence/retake recipes, TikTok UI safe zones, 73x token reduction. |

---

## 2. Master Comparative Matrix

| Product / Tool | Primary Engine | Agent Control Interface | Review / Quality Feedback Loop | Motion / Visual Philosophy | Audio & Sound Design | Key AeroEdits Takeaway |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **FrameJam** | Hyperframes / Remotion | MCP Server (:2400 web companion) | Spatial Pin Drops `(x, y)` + frame JPEGs | Playbook narrative structures | External TTS (Kokoro) | Native canvas pin feedback (`NOTE-1`) |
| **HyperFrames** | Headless Chrome + FFmpeg | Non-interactive CLI & skills | AST linter + Headless WCAG contrast gate | Seekable GSAP / Web Animations | Web audio element | Contrast & parameter schema in `graphic_check` |
| **Video-Use** | FFmpeg filtergraphs | `SKILL.md` for Claude Code | LLM self-evaluation of rendered frames | Remotion / Manim / PIL sub-agents | 30ms anti-pop audio crossfades | Anti-pop crossfades on all timeline cuts |
| **Broll-MCP** | Stream cuts + PySceneDetect | MCP tools (`search`, `cut`) | Keyframe images returned via MCP | Public domain documentary / historical | Source audio extraction | Video-only stream placement (`FTG-2`) + CC license sidecars |
| **FFmpeg-Skill** | 42 Python standard lib tools | MCP stdio + `SKILL.md` | `doctor --json` environment verification | Basic reframing (`fit.py`, `blur_bg.py`) | 2-pass LUFS, sidechain ducking | Machine-readable `contract --json` schemas |
| **KitCut** | GPU hardware acceleration | MCP Server (Claude/ChatGPT) | `project.json` + `journal.md` audit | Burned text cards, vertical crop | Audio-driven multicam switching | Automated secret/PII redaction using window telemetry |
| **Video-Shotcraft**| Remotion (React + TSX) | Skill linked to agent directory | Vite-powered Motion Workbench | 120+ 2.5D Shot Recipe Cards | Beat-sync cuts + SFX cues | 2.5D perspective card templates in HyperFrames |
| **Impeccable** | HTML / CSS / React | 24 design vocabulary commands | 60+ mechanical AST anti-pattern linters | Anti-slop: clean hierarchy, no nested boxes | N/A (Frontend UI) | Design taste rules in `graphic_check` (no-raw-black, tabular nums) |
| **UI-UX-Pro-Max** | Web components / Tailwind | Searchable Python DB utility | Pre-flight design contracts (`MASTER.md`) | Bento Grids, domain palettes | N/A (Frontend UI) | Bento Grid explainer templates & industry style presets |
| **Lemo-Opuscar** | Canvas 2D + WebGL + Three.js| Claude Opus directing skill | Headless frame capture review | Procedural shaders, whiteboard explainers | Local TTS + music sync | Procedural whiteboard explainer & Three.js 3D templates |
| **Diagram-Design**| SVG + Semantic HTML | Agent skill (`SKILL.md`) | Visual SVG inspection | 45 editorial diagrams, orthogonal routing | N/A (Static diagrams) | Animated 3-stage pipeline & architectural flow templates |
| **Higgsfield** | Diffusion models + After Effects| MCP bridge into Adobe NLE | Review within Premiere / AE timeline | Cinematic light-sweeps, camera motion | "Taste and assets travel together" (bundled SFX) | Bundle royalty-free SFX with every graphic template |
| **Riverside MCP** | Cloud studio backend | 60+ tools across 5 namespaces | Cloud versioning | Automated vertical shorts layouts | Remote track sync | Transactional atomic batch editing (`editing_batch`) |
| **Zane Hoyer Agency** | DaVinci Resolve + Remotion + OpenArt | Fragmented (ChatGPT + Terminal + Browser) | Manual visual checks in Resolve | Code-driven kinetic text, 2.5D UI cards | Basic BGM tracks | Unified desktop app replacing fragmented 4-app agency stack; 3-hook variation testing |
| **Jake Moran Masterclass** | HyperFrames + Claude Code | Standalone CLI + prompt contracts | Keyframe contact sheet gate | Reusable component primitives (2.5D cards, terminals) | Voiceover track sync | Enforce `frame.md` & `storyboard.md` contracts; bundle 4 core motion primitives |
| **Matt Loui CapCut Automation** | CapCut Desktop (`draft_content.json`) | Claude Desktop with CapCut skill | Visual check inside CapCut app | Pinterest moodboard-inspired text & drift keyframes | CapCut audio ducking | Export to CapCut Draft (`DLV-5`); moodboard image inputs for style generation |
| **Sirio Berati YouTube System** | Python CLI + Claude Code | Terminal script automation | Script-to-audio verification | Visual Boards (lower-thirds, bullet cards, code blocks) | Whisper timestamped cuts | Script-guided take assembly (`FTG-4`); Visual Board templates; YouTube metadata packaging |
| **NextPhase Editor V1 (Jad M.H)** | HyperFrames + Python + FFmpeg | Claude Code skill (`editor-v1`) | Pre-render plan approval + clipsheet | Empirical documentary: 0.6–1.5s picture, 18 named devices (`tile_reveal`, `wall3d`, `pop_collage`, `cutout_world`) | Synthesized procedural audio (`synth-sfx.py`), click rotation, punchline bed cuts | Adopt NextPhase devices in HyperFrames; procedural SFX synthesis; structural punchline music cuts |
| **GPT-6 Astra + DaVinci** | DaVinci Resolve Studio + Seedance | Multimodal agent via Resolve MCP | Human editor review in Resolve GUI | Automated Fusion titles + node grading | Native Fairlight mix | Typed MCP tools over computer use; automated camera color grading & LUT passes |
| **Claude Video 2.0 (Jack Roberts)** | HyperFrames + GSAP 3.15 + FFmpeg | Claude Code skill (`claude-video-2`) | 3-still review gate at $t = \text{trans} + 0.6\text{s}$ + automated pacing check | 21 distinct visual styles; continuous secondary motion; universal ban list (no raw black, no linear easing) | External voiceover + SFX cues | Pre-render 3-still contact sheet gate; automated `pacing_check.py` flagging holds $>1.5\text{s}$; 21 curated style recipes |
| **Jason Cooperson Claude Editor** | HyperFrames + Python + FFmpeg (Drop NLE) | Local Claude Code studio (`CLAUDE.md` + 20 skills) | Binarized pixel difference verification (`lut=y`) + text alpha linters | Hand-crafted presets (Georgia Bold, zero-blur hard knockout, no panels) | -35 dB RMS silence cut, speech restoration, dialogue ducking | Formalized `presets/` manifest; compounding `bake_preset` loop; empirical silence/retake recipes; TikTok UI safe zones (`y: 0.25`, `y: 1260`); 73x session token control |

---

## 3. The Definitive AeroEdits Architectural Synthesis

From 21 foundational tools, frameworks, and creator workflows, the strategic direction for **AeroEdits** is crystal clear:

1. **NextPhase High-Velocity Documentary Devices:**
   - Incorporate the top reverse-engineered devices into AeroEdits’ HyperFrames catalog: `tile_reveal` (9-pane rule-of-thirds entrance), `pop_collage` (staggered word-by-word cards), `wall3d` (perspective sweeps), and `page_card` with animated highlighter lines.
2. **Procedurally Synthesized Sound Engine (`synth-sfx.py`):**
   - Synthesize sound effects mathematically in code. This provides a zero-copyright, zero-asset-bloat library of clicks, risers, and impact cues tuned directly to motion velocities.
3. **Structural Music Bed Cuts ("Punchline Cuts"):**
   - Support structural bed muting: dropping background music completely under designated punchlines and slamming it back on the next phrase.
4. **Typed MCP API Over Computer Use:**
   - Both Higgsfield's DaVinci demo and our own research prove that screen-driving "computer use" is slow and error-prone. AeroEdits’ typed Rust MCP tools provide deterministic, millisecond-fast execution.
5. **Spring Physics & High-End Motion Curves:**
   - Ban linear transitions. Provide calibrated physics spring tokens (`--look-spring-overshoot`, `--look-spring-smooth`) for all HyperFrames templates.
6. **Script-Guided Take Assembly (`FTG-4`):**
   - Incorporate script-matching logic: mapping lines to spoken takes, keeping the final take and archiving retakes onto disabled lanes.
7. **CapCut Draft Export as an Ecosystem Bridge (`DLV-5`):**
   - Provide 1-click export to CapCut drafts so creators who rely on CapCut’s mobile app or trending filters can easily transfer their AeroEdits projects without lock-in.
8. **The Planning Contract (`frame.md` + `storyboard.md`):**
   - Mandate that agents establish design constraints (`frame.md`) and temporal structure (`storyboard.md`) before generating animation code.
9. **Zero-Ambiguity Canvas Feedback:**
   - Interactive spatial pin drops `(x, y)` on the native desktop preview player, giving the agent exact coordinate guidance and cropped snapshots.
10. **Telemetry-Driven Automation:**
   - Real mouse dwell, click clusters, active window bounds, and typing carets driving zooms and camera framing.
11. **Pre-Render 3-Still Review Gate & Automated Linters:**
   - Implement Jack Roberts' lightweight review protocol: capturing stills at $t = \text{transition} + 0.6\text{s}$ to confirm settled layouts before full renders, paired with automated checks (`pacing_check.py` flagging static holds $>1.5\text{s}$ or cuts $<0.3\text{s}$).
12. **21-Style Curated Motion Library:**
   - Expand AeroEdits' built-in HyperFrames templates to include Jack Roberts' 21 validated visual styles (Swiss Brutalism, Apple Minimal, Cyberpunk Terminal, Claymorphism, Blueprint, Isometric Architecture).
13. **Mathematical Presets & Compounding Taste Loop (`bake_preset`):**
   - Eliminate generic AI slop by storing aesthetic tokens in `presets/`. Allow users to tweak an edit and run `bake_preset`, training the agent on their exact taste delta for future one-shot executions.
14. **Empirical Post-Production Discipline:**
   - Enforce calibrated silence cutting (-35 dB RMS, 20ms envelope, 0.02s lead-in, 0.04s tail), adjacent quiet speech restoration (`restore_speech.py`), standing retake pruning (last take keeper via difflib $\ge 0.72$), TikTok vertical UI safe zones (captions at $y=0.25$, photo center at $y=1260$), zero-blur hard knockout edges for 9x faster renders, and strict session token hygiene.
