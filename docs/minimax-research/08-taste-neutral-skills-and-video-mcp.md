# 08 — Taste-neutral skills, and video over MCP

- **Sources:** [Remotion's agent skills](https://www.remotion.dev/docs/ai/skills),
  [Remotion Skills 2.0 announcement](https://x.com/Remotion/status/2089295038932996194),
  [Motion's MCP guide](https://docs.motion.so/guides/mcp),
  [Motion AI Kit](https://motion.dev/ai-kit), [motionvid.ai](https://motionvid.ai/)
- **Evidence grade:** **vendor claims, read from public docs and announcements.** Not captured with
  the intake tool, not tested in AeroEdits. Read the pages before relying on any of it.

## 1. Remotion went taste-neutral — deliberately

From the Remotion Skills 2.0 announcement (17 Aug 2026):

> **Better skills for better videos! Remotion Skills 2.0: Sub-skills · Interactive · Taste-neutral**
>
> `/remotion-best-practices` is now just a router for other skills. This ensures that only relevant
> info gets loaded into context. … **We de-bloated our skills and removed all design instructions.**
> More context for your vision, fewer clashes with defaults.

Two ideas, and they point in opposite directions from AeroEdits.

**Sub-skills and a router.** The top-level skill is a router; each sub-skill loads only when
relevant. AeroEdits has the same problem from the other end — the standing preference is that
everything an agent reads in a whole edit stays around 100k tokens or less (AI-1), and the measured
cost is dominated by a few large answers. A router that loads only the relevant guidance is the
same trick, applied to knowledge instead of to tools.

**Taste-neutral, design instructions removed.** Remotion decided the *library* should not carry
design opinions, so the model brings its own taste and the skill only prevents API mistakes. That
is the **opposite** of AeroEdits' settled decision — "one look for every graphic in a video, very
important", decided on 22 Oct, with the look in the style beside the brand kit.

Both positions can be right, because they hold the taste in different places. Remotion keeps it
out of the skill so the model is free; AeroEdits keeps it in the app so every graphic in a video
matches. But the question becomes: **is the look a property of the video, or of the agent?** If it
is the video's, it belongs in the style and the skill should stay taste-neutral — and Remotion's
move is evidence that a well-organised skill does not need to carry design at all.

That is a decision for George, and it is the sharpest thing in this folder.

## 2. Motion's MCP takes a *reference*

[Motion's MCP guide](https://docs.motion.so/guides/mcp) documents `create_video`, which accepts
alongside `prompt`, `engine`, `effort`, `template` and `aspect_ratio`:

- **`design_system_id`** — a named design system,
- **`design_md`** — a design document,
- **`style_reference_url`** — **a YouTube URL**,
- and up to 100 `attachments`.

The `style_reference_url` is the important one: someone has shipped, as a product feature, the
idea that **you point at a video and the system derives the look from it**. That is the shape of
the research conclusion already in AeroEdits' corpus — "a reference is mandatory, and it should be
small and real" (creator-workflows.md, finding 5) — turned into an API parameter.

Motion also splits profiles by host (`/mcp`, `/mcp/openai`, `/mcp/muse`, `/mcp/anthropic`) and
guards the server with OAuth 2.1, and ships a live video card widget in supported hosts. Note the
Claude profile is explicitly **code-based motion graphics with no generative-media or billing
tools** — the same division of labour AeroEdits already has: the agent writes code, something else
renders.

## 3. Two more products in the same space

**[Motion AI Kit](https://motion.dev/ai-kit)** — the Motion team's own skill and MCP for agents,
with 510+ examples, a "Performance Doctor" and a MotionScore. It is packaged as a Cursor plugin
and, per a third-party catalogue, ships **no licence file**.

**[motionvid.ai](https://motionvid.ai/)** — an MCP server and a REST API over a multi-scene engine.
Its pitch is the agent-facing loop: create, watch the render, edit one scene, and **stop for
approval before spending on footage**. It ships an MCP connector on every paid plan.

## Why it matters here

- The **style reference** parameter is a concrete external validation of the style-trial idea
  (PLAN-1) — and a warning: if a commercial API can take a reference, users will expect AeroEdits
  to as well.
- The **router** pattern is directly applicable to AI-1's token budget, and it is cheap: our
  `SKILL.md` ships to the agent as one file with graphics, rules, export and presets in it.
- The **taste-neutral** argument is the live design question in this folder. Everything else here
  is a technique; this one is a decision about where the taste lives.

## Caveats

- All four are vendor claims from public documentation. None was installed, trialed or benchmarked.
- Motion, motionvid and the Remotion ecosystem are all **commercial** and **cloud** — the opposite
  of AeroEdits' local-first, own-subscription stance. Their relevance is in the *interface
  patterns*, not in the products.
- Remotion's licence bars us from shipping anything Remotion-derived anyway (see
  [06](06-licence-landscape.md)). The 2.0 announcement is still worth reading for the design
  reasoning alone.
