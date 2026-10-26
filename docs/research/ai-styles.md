# Styles an agent proposes: how others do it (GFX-5 research, 24 Oct)

George asked (23 Oct) for research before building AI-proposed styles: how do people who get
good graphics from agents set the agent up? Two web searches, 24 Oct. Most sources are
community skills and product docs, not measured results.

## What they do

1. **A brand intake writes one file every later step reads.** The most complete community pack
   ([motion-graphics-skills](https://github.com/charlie947/motion-graphics-skills), 13 Claude
   Code skills) starts with a brand-intake skill that writes `brand.md` and `MOTION.md`; every
   other skill reads them first. [brand-motion-design-skill](https://github.com/ouerf-man/brand-motion-design-skill)
   works without a brand book: it asks a few questions (colours, type, tone, audience), or
   reads a website URL and pulls colour and type tokens from it.
2. **Style frames before motion.** The same skill shows its picks as still style frames for the
   user to choose from before anything animates. This is our `library` `kind: style`
   `op: preview` already.
3. **References are decomposed, not copied.** The pack's reference-to-motion skill breaks down a
   video the user admires into its moves, takes the look from a site or moodboard, and writes a
   scene-by-scene prompt from that. Moodboards alone are not resolvable; concrete assets are
   (see [creator-workflows.md](creator-workflows.md), point 5).
4. **Products keep a saved kit plus references plus a brief.** HeyGen's
   [brand kits](https://developers.heygen.com/docs/brand-kits) (colours, fonts, logos, applied
   to backgrounds, text and logo placement), Typeface's image agent (reference images for
   lighting and composition), [OpenArt's brand kit](https://openart.ai/features/ai-brand-kit/)
   (a "reference focus": style, product, logo or palette, and it says what it dropped) and
   Lovart (brief plus kit in one agent context) all converge on the same three inputs.
5. **Tight motion rules travel with the style.** Prompt write-ups
   ([Claude motion graphics prompts](https://roo.beehiiv.com/p/claude-motion-graphics-prompts))
   pin duration, exact copy, timed beats, palette, font and motion rules, and animate transforms
   and opacity only. A Remotion skill ([claude-remotion-skill](https://github.com/haidrrrry/claude-remotion-skill))
   ships a theme template with colours, easings and spring presets.

## What it means for GFX-5

We already have most of the parts: the Style (brand, graphics look tokens, captions, frame),
`op: preview` (style frames on the video's own graphics) and `op: set`. What is missing is the
intake and the choice:

- **Intake, in one call:** a brand in words, optionally a URL or a logo file; the agent turns
  it into two or three Style JSONs that differ in a named way (calm / bold / playful), never
  three shades of one.
- **Choice on the video's own graphics:** `preview` each, one sheet per style, the user picks;
  only the pick is saved (`set` into a copy). That keeps the 99 %/10 % cost rule: three
  stills sheets, no renders of motion.
- **Motion rules in the style, not the prompt:** our graphics look already carries speed and
  easing tokens; the intake should set them, so the look holds across graphics made later.
- **Reference videos** (point 3) belong to STY-2: decompose a reference into tokens and moves.

Not built yet: George asked for research first. Proposed next step: a `library` `kind: style`
`op: propose` taking `brief` (+ `url`, `logo`), answering with up to three previews, and the
Style editor showing them side by side. Decide that before building.
