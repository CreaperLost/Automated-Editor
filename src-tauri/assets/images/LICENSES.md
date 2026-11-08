# Built-in image library (FCL-5/6)

509 pictures in one folder per look, since a video keeps one look (GFX-5):

| Folder | Pictures | For |
|---|---:|---|
| `broadsheet/` | 245 | Faceless explainers: the Explainer's engravings, its 3D textures, the gap set |
| `light/` | 60 | Styles Light, My library, Word Sync |
| `dark/` | 60 | Style Dark |
| `editorial/` | 60 | Style Hermes Editorial |
| `neon/` | 60 | Styles Neon Pop, Fancy-Short |
| `sourced/` | 24 | Every look: flags, maps, wordmarks, portraits, pages from elsewhere |

A style no look names takes the look nearest its background and accent. `catalog.json` lists
the looks, topics and groups, and per picture its look, what it depicts, where it goes and its
licence. Raster pictures are WebP (quality 80, transparency at 70: 25 MB in all, half of
lossless transparency, the same at 2x zoom), at most 1024 px on the long side (full-bleed
paper grounds 2048, sourced pictures 1440); SVGs unchanged; names the originals'. A graphic
keeps a used picture as `images/<look>-<name>`. Every standing figure (768 × 1024) was moved
on import so its feet stand at y=950. This folder is the only copy the app needs: neither the
Explainer repository nor the delivery folders are (each picture's Flova sheet is named in the
catalog). `scripts/add_images.py` adds a new set; `../image-looks` holds each look's rules and
approved tile.

## Generated for the library (no credit)

The Broadsheet illustrations and the 3D textures and turnarounds were generated for George's
channel with GPT Image 2.5 Sunburst in George's Flova project,
https://www.flova.ai/project/?id=57ec424b12da40e0b8fbe5fe4a02edb7 (the resource of each kit
is in its catalog entry where the original recorded it). `ai-assets/news-subjects` was
generated with GPT Image 2.6 Sunburst through OpenRouter on 7 Nov 2026. They show no real
product, person or logo, and stand for ideas, never as evidence.

Originals are 2048 × 1536; props cut from sheets 1024 × 768; the Operator and Reviewer
cutouts 768 × 1024 and share a foot baseline; the two torn-map halves were split from the
whole map, so stacked at the same place they make it. Repeat textures (512 × 512) tile on
both axes.

## The gap set (added 7 Nov 2026, no credit)

68 more Broadsheet illustrations for subjects AI-news stories need and the library lacked, found
by searching a week of discovered stories: `markets/stocks`, `markets/funding`, `law/courts`,
`politics/elections`, `people/executive` (a new cast member), and `ai-assets/humanoid`,
`deepfakes`, `chips`, `space`, `social`, `products`. Made in the same Flova project with GPT
Image 2.5 Sunburst from 24 sheets, the original references attached; each picture's resource is
in the catalog. Alpha was taken from the ink as for the originals. The executive and humanoid
figures were moved down 28 px on import so every cast member's feet stand at y=950 of the
768 × 1024 canvas, with Operator and Reviewer. The source sheets stay in Flova.

## The style looks (added 8 Nov 2026, no credit)

The same 60 everyday subjects (devices, software, AI, data, security, ideas, money, people) in
four looks, each from a style tile George approved first and attached to every later sheet,
made in the same Flova project with GPT Image 2.5 Sunburst: light (charcoal monoline, pale-blue
fills; 126 listed credits), dark (soft-white monoline, blue tints; 126), editorial (navy and
blue two-colour print with grain; 119), neon (pink and violet stickers with white edges; 126).
Each picture's Flova resource is in the catalog. On import, light's `app-grid` and `checklist`
were cut again from their sheet (`software_symbols`): the maker's grid had split the sheet
inside the checklist, so app-grid carried its top and checklist lost it. The source sheets stay
in Flova; the approved tiles and each look's rules are in `../image-looks`.

## Sourced reference pictures (`sourced/`)

Real material from elsewhere, each with its source in the catalog:

| Pictures | Source | Licence in the catalog |
|---|---|---|
| Country and region maps (`sourced/countries/*-map.svg`) | Natural Earth 5.1.2, 1:50m countries | Public domain (`publicDomain`) |
| Flags (`*-flag.svg`) | Wikimedia Commons | `unknown`: check emblem use before publication |
| Portraits (`sourced/people/*`) | Wikimedia Commons | CC BY / CC BY-SA (`creditRequired`): credit line in `rights` |
| Wordmarks (`sourced/ai-assets/*-wordmark.svg`) | Wikimedia Commons | `unknown`: trademarks, check the brand's terms |
| Product and release pages (`sourced/ai-assets/*` pages) | The publisher's page | `unknown`: clear use and credit the publisher |

They come with every look. `library` marks these with `*`, the skill uses them only at the
user's word, and `check` flags a graphic carrying one.

The institutions' reference list (NIST, CISA, EU AI Act pages to capture) had no pictures and
was not carried over.
