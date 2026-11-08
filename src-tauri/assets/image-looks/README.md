# Making more library images

The image library (`../images`, built into the app) has one set per look; a video keeps one
look. This folder keeps what an image editor needs to add pictures that match: each look's
rules and its approved style tile (not built into the app). New sets are made in George's Flova
project (https://www.flova.ai/project/?id=57ec424b12da40e0b8fbe5fe4a02edb7) with GPT Image 2.5
Sunburst, every generation with the look's reference images attached, then added with
`python scripts/add_images.py <look>=<drop folder>` (from `src-tauri`; its header gives the
drop format).

## Rules for every look

- No words, letters, numbers, labels, logos or brand marks; screens and papers show abstract
  marks only. No real people or products; figures are fictional.
- One subject per image, centred, filling about 75% of the canvas with even margin, 3/4 or
  front view, lit from the upper left. Transparent PNG: objects 1024 × 768, standing figures
  768 × 1024 (the library moves their feet to y=950).
- States of one object (open/closed, on/off, rising/falling) are generated together on one
  sheet and cut at the same size and place, so they can be stacked and swapped on a cue.
- Generate on a 2048 × 1536 sheet with clear gaps between subjects; cut each one inside its own
  gap (a grid cut through a subject is how light's checklist lost its top on 8 Nov).
- Look at every image on the look's ground before delivering it.
- Deliver groups.json and a catalog.json per group: `depicts` (exactly what is drawn),
  `placement` (how a graphic uses it, what it pairs with) and `flova_resource` (its sheet).

## The looks

### broadsheet (faceless explainers) — ground #F0E9DA
A 19th-century newspaper engraving: one object in near-black sepia ink #1C1A16 with line
hatching, cross-hatching and stippling; no colour, grey fills, gradients, shadows, floor or
frame. Transparency is taken from the ink (white fully clear, hatching partly see-through),
never a hard cut-out. References, from the library: `broadsheet/ai-assets/costs/receipt-long`,
`broadsheet/ai-assets/news-subjects/gpu-card`, `broadsheet/ai-assets/compute/server-rack-open`,
`broadsheet/workplace-assets/hands/hand-point`, and `broadsheet/people/operator/operator-front`
for figures (`library` `op: show` writes any of them out as a file).

### light (styles Light, My library, Word Sync) — ground #FFFFFF, tile `light-tile.webp`
Calm drawn product sketches with fine, even charcoal #1D1D1F monoline contours and rounded
joins. Flat pale-blue fills #E6F1FC / #CCE3F9 and at most one #0071E3 detail per subject. No
realistic screenshots, detailed faces, shading or texture.

### dark (style Dark) — ground #111114, tile `dark-tile.webp`
The light drawings in soft-white #F5F5F7 monoline with rounded caps and joins: same designs,
angles, character and poses, attach the matching light image as geometry. Flat dark-blue
#102843 tint, at most one small solid #0A84FF accent. No glow, neon, shadows, gradients or grain.

### editorial (style Hermes Editorial) — ground #F7F8FC, tile `editorial-tile.webp`
Bold two-colour risograph / screen-print illustrations from flat filled silhouettes: navy
#111827 and blue #246BFE ink only, fine print grain, slight misregistration. Negative spaces
transparent; no outlines, gradients, shadows or gloss. People have blue face and hand shapes
with minimal features.

### neon (styles Neon Pop, Fancy-Short) — ground #12062E, tile `neon-tile.webp`
Chunky playful stickers with thick #FFF5FB outer edges and white inner separators. Hot pink
#FF2E93 and violet #5B21FF dominate; yellow #FFD60A only as a tiny accent (under 5% of the
colour). Flat fills with one crisp upper-left white gloss mark; no dark contours, gradients,
glow or shadows. Minimal faces.

## A new look

Write its rules as above, make a style tile of six subjects (laptop, chat window, padlock,
rising chart, rocket, standing person) and have George approve it before anything else, then
make the same 60 subjects as the other style looks so any style can show any idea (the list is
the light set's catalog), and add the look to `LOOKS` in `scripts/add_images.py`.
