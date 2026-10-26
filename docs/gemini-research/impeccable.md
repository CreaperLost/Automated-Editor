# Impeccable Research Report

- **Source:** [github.com/pbakaus/impeccable](https://github.com/pbakaus/impeccable) | [impeccable.style](https://impeccable.style)
- **Author:** Paul Bakaus (Renaissance Geek, backed by a16z)
- **License:** Open Source
- **Category:** AI Design Director, Frontend Anti-Pattern Linter & Design System Language
- **Analyzed for:** AeroEdits AI Agent & Motion Graphics Architecture

---

## 1. Executive Summary

**Impeccable** (`pbakaus/impeccable`) is an open-source design engineering skill and linter suite created by Paul Bakaus (creator of jQuery UI and AMP lead at Google). It functions as a **"Design Director" for AI coding assistants** (Claude Code, Cursor, Codex, Copilot).

Its core thesis is that generative AI code models produce **"AI slop"**: generic centered cards, overused purple/cyan gradients, low-contrast gray text on tinted backgrounds, uncalibrated Inter/Arial fonts, and dated spring animations. Impeccable fixes this not through vague prompt reminders, but through **60+ deterministic, mechanical anti-pattern linters** and a shared design vocabulary (`/impeccable craft`, `/impeccable polish`, `/impeccable typeset`) backed by durable project truth files (`PRODUCT.md` and `DESIGN.md`).

---

## 2. Core Architecture & Workflow

```
┌──────────────────────────────────────────────────────────────┐
│                    Coding Agent (Claude / Cursor)            │
└──────────────────────────────┬───────────────────────────────┘
                               │
       ┌───────────────────────┼───────────────────────┐
       ▼                       ▼                       ▼
1. Persistent Context   2. Design Commands      3. Deterministic Linter
   `PRODUCT.md` (truth)    `/impeccable craft`     60+ Mechanical
   `DESIGN.md` (tokens)    `/impeccable polish`    Anti-Pattern Rules
       │                   `/impeccable typeset`   (Zero-token AST check)
       │                       │                       │
       └───────────────────────┼───────────────────────┘
                               ▼
┌──────────────────────────────────────────────────────────────┐
│                   High-End UI / Motion Code                  │
│       • No nested card borders  • Tinted contextual neutrals │
│       • Calibrated typography   • Accessible WCAG contrast   │
└──────────────────────────────────────────────────────────────┘
```

### 2.1 The 24 Design Vocabulary Commands

Impeccable gives the agent structured commands rather than freeform instructions:

| Command | Purpose |
| :--- | :--- |
| `/impeccable init` | Scans the codebase, interviews the developer about core product audience, and generates `PRODUCT.md` and `DESIGN.md`. |
| `/impeccable craft` | Complete shape-then-build flow: establishes spatial hierarchy before writing component details. |
| `/impeccable critique` | Senior design director review evaluating visual hierarchy, clarity, tone, and breathing room. |
| `/impeccable audit` | Technical verification: accessibility (contrast, focus states), responsive behavior, performance. |
| `/impeccable polish` | Micro-interaction and visual cleanup pass before shipping (spacing rhythm, subtle borders, subpixel rendering). |
| `/impeccable typeset` | Enforces typographic scale, optical sizing, line-height ratios, and tabular numerals. |
| `/impeccable distill` | Simplifies overly complex layouts; removes gratuitous containers, dividers, and decorative noise. |
| `/impeccable harden` | Stress-tests UI against long text strings, localization (i18n), small screen widths, and empty states. |

---

## 3. Deep Dive: Key Technical Capabilities

### 3.1 60+ Mechanical Anti-Pattern Detectors (Zero Token Cost)
Instead of asking an LLM to "critique itself" (which models do poorly), Impeccable runs deterministic static code analyzers across HTML/CSS/JSX:

| Anti-Pattern Rule | Description & Enforcement |
| :--- | :--- |
| `no-raw-black` | Flags `#000000`. Enforces tinted dark neutrals (e.g. `hsl(220, 15%, 8%)`) to prevent harsh, unrefined contrast. |
| `no-uncalibrated-gray` | Prohibits default cold gray text `#888888` on warm or colored cards. Enforces contextual tinting. |
| `no-nested-boxes` | Detects cards inside cards wrapped in explicit borders (`border: 1px solid`). Enforces grouping via whitespace or background contrast instead of lines. |
| `no-cliche-gradients` | Detects the ubiquitous AI diagonal purple-to-pink or blue-to-cyan radial gradient. |
| `wcag-contrast-gate` | Mechanically calculates luminance ratios; rejects any text under 4.5:1 (or 3:1 for large display titles). |
| `no-bouncy-easing` | Prohibits cartoonish, excessive spring overshoot in serious utility tools. Enforces crisp, intentional eases (`cubic-bezier(0.16, 1, 0.3, 1)`). |

### 3.2 Durable Design Truth: `PRODUCT.md` and `DESIGN.md`
AI agents forget context across long chat sessions. Impeccable anchors the agent to two compact markdown files:
- **`PRODUCT.md`:** Target audience, tone of voice, what the product is *not*, and essential constraints.
- **`DESIGN.md`:** Typography tokens, color palette formulas, corner radii, elevation/shadow scale, and layout density rules.

Whenever the agent is asked to create or modify UI, it reads these files to ensure visual coherence.

---

## 4. AeroEdits vs. Impeccable Comparison

| Capability | Impeccable | AeroEdits |
| :--- | :--- | :--- |
| **Domain** | Web/App Frontend UI (HTML, React, CSS). | Video Editor: Motion Graphics, Title Cards, Explainer Overlays, Captions. |
| **Quality Gate** | 60+ static anti-pattern linters + headless audit. | `graphic_check` (validates GSAP timeline, offline assets, durations). |
| **Design System** | `DESIGN.md` (tokens, colors, typography, elevations). | Style System (`--brand-*`, `--look-*` tokens in `styles/`). |
| **Motion Philosophy** | Crisp, purposeful micro-interactions; no cartoon bounce. | GSAP 3.15 timelines, hard cuts for full-screen (`data-entrance="cut"`), easing for camera overlays. |
| **Agent Steering** | Structured `/impeccable` command vocabulary. | MCP tools (`graphic_add`, `graphic_write_file`, `graphic_check`, `frame_render`). |

---

## 5. Actionable Takeaways for AeroEdits

### 🟢 What We Should Adopt

1. **Mechanical Design Anti-Pattern Linter in `graphic_check`:**
   - Upgrade AeroEdits' `graphic_check` tool from just checking technical syntax (offline GSAP, `data-duration`) to checking **design taste**:
     - *No raw `#000` or `#fff`:* Enforce brand-tinted backgrounds and darks.
     - *No box-in-box border spam:* Disallow nested bordered containers in card graphics.
     - *Contrast verification:* Calculate WCAG contrast between text elements and the graphic background.
     - *Font token enforcement:* Flag hardcoded `font-family: Arial` or `Inter`; require `var(--brand-font)` and `var(--brand-body-font)`.
2. **Adopt the `/graphic distill` and `/graphic polish` Agent Commands:**
   - Add specialized agent prompts/sub-skills to `mcp/skill/graphics.md`:
     - **Distill:** Strips clutter from a graphic. If an agent created a card with 5 lines of text, an icon, a badge, and 2 containers, `distill` cuts it to the single punchy claim and the metric.
     - **Polish:** Tightens padding rhythm, enforces 8px grid spacing, and aligns typography baselines.
3. **Tabular Numerals for Metric Graphics:**
   - Enforce `font-variant-numeric: tabular-nums` on all counter and metric graphics so numbers don't jitter horizontally as they count up.

### 🔴 What We Refuse / Avoid

1. **Do NOT run heavy full-stack web design frameworks inside the video graphics engine:**
   - Impeccable is built for web apps (React/Tailwind/Next.js). In AeroEdits, graphics must remain ultra-lightweight, self-contained HyperFrames compositions rendered directly in headless Chrome.
2. **Do NOT allow soft, washed-out low-contrast aesthetics:**
   - Web apps can sometimes afford subtle low-contrast gray text; video cannot. Video is watched on mobile screens under glare and compressed via H.264/AV1. AeroEdits graphics must maintain bold, high-contrast legibility at all times.
