# 05 — A style is data plus a guide

- **Source:** [github.com/moritzkremb/framejam](https://github.com/moritzkremb/framejam) ·
  [framejam.ai](https://www.framejam.ai)
- **Licence:** MIT · captured 25 Oct 2026 · 59★ · last push **2026-10-25** (the day of capture)
- **Raw capture:** [evidence/2026-10-25/moritzkremb-framejam.md](evidence/2026-10-25/moritzkremb-framejam.md)
- **Evidence grade:** vendor claim, read from the source. Not tested in AeroEdits.

## What it is

A local app and an MCP server that put a **code-rendered video** behind a review loop. The agent
renders with any tool (HyperFrames, Remotion, Motion Canvas, ffmpeg), then `open_review` puts the
**render** in a browser page where the user pauses, **clicks the frame** to drop a pin, types, or
drags a range on the filmstrip. `wait_for_feedback` blocks until the user presses *Finish review*,
then hands the agent every comment with its timestamp, **the frame with the pin drawn on it**, and
a JSON body.

14 MCP tools, confirmed by name in the README: `open_review`, `wait_for_feedback`, `get_feedback`,
`list_reviews`, `add_version`, `resolve_comments`, `list_presets`, `get_preset`, `list_playbooks`,
`get_playbook`, `open_picker`, `wait_for_pick`, `get_selected_preset`, `get_selected_playbook`.

## Three things worth having

### 1. Styles as data, with a guide

30 presets, each `style.json` + a HyperFrames `composition/` + `preview.mp4`. The shape of one
`style.json`:

```json
{
  "mood": ["...", "...", "..."],
  "pacing": "fast",
  "format": "16:9",
  "palette": { "background": "#050806", "surface": "...", "primary": "...",
               "text": "...", "muted": "...", "warning": "...", "border": "..." },
  "fonts":   { "mono": { "family": "JetBrains Mono", "weights": [...],
                        "source": "Google Fonts",
                        "usage": "Everything. Terminal text 34px, logo 150px weight 800" } },
  "easing": { "typing": "steps(n) where n = character count", "enter": "power2.out",
              "logo": "expo.out", "chip": "back.out(2)", "glitch": "none (0.04s yoyo jitters)" },
  "transitions": [...],
  "textAnimations": [...],
  "rhythm": { "averageShotSeconds": 3.7,
              "notes": "Output lines every 0.3–0.4s, like a real CLI. Hold the final URL line ~1s…" },
  "guide": "- Look only: build the user's own subject in this visual language; don't reuse the example"
}
```

Four parts of this AeroEdits' Style does not have:

- **a prose `guide`** — look rules in words, so an agent can build *in* a look rather than imitate
  one it saw;
- **a per-element easing map** — `typing`, `enter`, `logo`, `chip`, `glitch` — where AeroEdits' `Look`
  has one global `ease`;
- **a `rhythm`** — average seconds per shot plus prose notes;
- **the font's usage rules** — "terminal text 34px, logo 150px weight 800", not just a family name.

Verified in this repo: AeroEdits' `Look` (`src-tauri/src/graphics/look.rs`) already carries size,
roundness, speed, energy and density, plus named `ease` / `duration` / `gsap-ease` tokens — so the
raw motion tokens exist. What has **no home** is the prose guide, the rhythm, and the per-element
map. That is the gap, not "add more tokens".

The `guide` line is also a licence and ethics guardrail, and an art-direction one: *look only, build
the user's own subject in this visual language; don't reuse the example*.

### 2. Playbooks as methods

18 folders, each a `SKILL.md` + `references/` + `scripts/`: what you bring, the steps, the tools,
the quality bar, as defaults a request can override. Four are ours by name: **Explainer**,
**How-to**, **Founder Launch** (talking head plus screen recordings) and **YouTube Hook** (the
20–60 second opening).

### 3. Spatial pins

A normalised `(x, y)` on the frame plus the exact timestamp. It turns "the card overlaps the badge"
into a coordinate. AeroEdits' NOTE-1 record type already anticipates this — it specifies "a
normalised canvas rectangle" — so the design already had the primitive; what is missing is the
interaction that fills it and the agent's answer arriving as a revision.

## What we refuse

- **The separate web server and browser pane.** FrameJam is a Node app on `:2400` because it has no
  GUI and targets CLI agents. AeroEdits is a native app; the review loop belongs in the Agent
  panel, and the agent proposes rather than polls.
- **Remotion as a second engine.** Already decided
  (`docs/decisions.md`, *Not doing*: two style systems, two failure modes).
- **Downloading playbook updates from the vendor at runtime.** Our gate allows no network.
- **Its own styles outside HyperFrames.** FrameJam's presets are HyperFrames compositions, so the
  30 of them are directly in our runtime — and MIT, with original copy and no brand assets.

## One more thing it gets right that we should copy

> **you always review the render**

Every comment points at a frame of the rendered output, never at the source. AeroEdits' graphics
are code, so the source *is* legible — which makes it tempting to review the source and skip the
render. The rule says: review what the viewer will see.

## Caveats

- 59★ and pushed the day of capture. Early, and moving fast — the playbooks are updated from
  framejam.ai at runtime, so the repo and the served content can differ.
- The styles are curated for cinematic product films, not for screen-recorded tutorials.
- "Every one passes `hyperframes lint`" — a vendor claim, though AeroEdits' own gate runs
  `hyperframes check` (lint, runtime errors, layout overflow, contrast at 16:9 and 9:16), which is
  stricter.
