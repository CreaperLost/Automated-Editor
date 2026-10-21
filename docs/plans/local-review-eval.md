# Local vs OpenRouter filler and retake review (task 3)

Measured 21 Oct 2026 on George's PC (RTX 3060 12 GB, Ryzen 5 3600), branch `expansion`.

## What was compared

What `apply: true` would cut: the rules' suggestions plus each engine's spans, merged by
`edit::suggestions`, as the app does.

- **rules**: no model (the baseline both engines build on).
- **local**: Qwen3.5 4B Q4_K_M (`local/qwen3.5-4b-q4_k_m`), llama.cpp on Vulkan. It reads only
  the candidate windows (`ai/candidates.rs`), with the flags as hints.
- **cloud**: the OpenRouter review in George's settings (`openai/gpt-6-luna`). It reads every
  word in 300-word chunks.

The probe is `src-tauri/src/ai/review_probe.rs`, an ignored test like the perf and parity
probes. The labelled cases are in [review-eval](review-eval/).

```
AEROEDITS_REVIEW_CASE=docs/plans/review-eval/en-b.json cargo test --release --lib --features local-llm review_probe -- --ignored --nocapture
```

Build it in the environment `scripts/build-windows.ps1` sets up (Vulkan SDK, VS developer
shell, Ninja, `CARGO_TARGET_DIR=C:\aeb`). The English cases point at transcripts in George's
Documents folder.

### Cases and labels

The labels were written blind, from the numbered words, before any engine ran. **Must** spans
are clear cuts. **Optional** spans are judgement calls, mostly a sentence-opening "So" or
"And", or "pretty, pretty" and "very very": cutting one is not a wrong cut and keeping it is
not a miss.

A must span counts as missed unless most of its words are cut. A cut counts as wrong when
fewer than half of its words are labelled (must or optional).

| Case | Source | Words | Must (fillers / retakes) | Optional |
|---|---|---|---|---|
| EN-A | "Agent test", Scribe v2, first 1200 words | 1200 | 5 (4 / 1) | 26 |
| EN-B | "19 Oct 2" Arkus video, Parakeet TDT v3, whole | 1044 | 8 (4 / 4) | 30 |
| EN-C | "19 Oct Review Edit" Hermes video, Parakeet TDT v3, first 1000 | 1000 | 5 (3 / 2) | 24 |
| EL-A | Scripted Greek tutorial, Scribe-style words | 374 | 19 (14 / 5) | 3 |

No Greek recording exists yet, so EL-A is a script (`review-eval/make-greek-case.py`). It
plants the fillers from the Greek prompt, plus traps where the same words carry meaning:
"ας πούμε ότι", "ξέρεις τι", "δεν ξέρετε", "τα βασικά", "είναι εντάξει". It tests the review,
not Scribe, and should be redone on a real Greek recording.

## Results

Misses are must spans left in. "Wrong cuts" counts cut spans (wrong words in brackets);
"by AI" excludes the wrong cuts the rules made, which every engine inherits.

| Case | Engine | Missed | Wrong cuts (words) | Wrong cuts by AI | Load + review |
|---|---|---|---|---|---|
| EN-A | rules | 5 / 5 | 1 (4) | — | — |
| | local | 4 / 5 | 7 (18) | 6 | 2.6 + 11.7 s |
| | cloud | 2 / 5 | 1 (4) | 0 | 27.8 s |
| EN-B | rules | 3 / 8 | 1 (4) | — | — |
| | local | 2 / 8 | 4 (30) | 3 | 2.3 + 8.6 s |
| | cloud | 2 / 8 | 1 (4) | 0 | 21.7–26.0 s (2 runs, same score) |
| EN-C | rules | 3 / 5 | 0 | — | — |
| | local | 3 / 5 | 1 (5) | 1 | 2.2 + 2.8 s |
| | cloud | 1 / 5 | 0 | 0 | 37.8 s |
| EL-A | rules | 15 / 19 | 0 | — | — |
| | local | 10 / 19 | 1 (2) | 1 | 2.9 + 4.0 s |
| | cloud | 1 / 19 | 0 | 0 | 12.3 s |
| **English total** | rules | 11 / 18 | 2 (8) | — | |
| | local | 9 / 18 | 12 (53) | 10 | |
| | cloud | 5 / 18 | 2 (8) | 0 | |

**Where it ran.** Local is on the GPU. llama.cpp lists `Vulkan NVIDIA GeForce RTX 3060` and
the CPU; VRAM rose about 3.4 GB (1.4 → 4.9 GB) and GPU use reached 100%. Cloud is remote.

**Speed.** On these 1000–1200-word cases, local took 2–3 s to load and 3–12 s to review, and
read only 378–774 words (the windows). The first run after a build took 9.2 + 55.6 s while
Vulkan compiled its pipelines and the 2.7 GB file came off disk. A 43 s Greek run was
GPU contention with another run, not Greek tokens; run alone it is 4.0–4.5 s. Cloud took
12–38 s over every word.

### What the local model gets wrong

- **It approves flags instead of judging them.** All 10 of its wrong cuts are candidate flags
  accepted as they were, and none was its own idea:
  - 6 `Retake` flags on phrases said twice, given the same reason, "false start repeated".
    Examples: "Select the same pair on both for example BTC and BTC" (11 words), "side you
    will have Arcus on the other", "whether you like the generation,", "going to be fully
    populated.".
  - 4 `Filler` flags on "so" meaning "therefore" ("so you can use it", "so you don't have to
    do regenerations"), given as "connective 'so' is redundant here".
- **Greek: it declines most Greek filler flags.** Λοιπόν, Εντάξει, Βασικά, ξέρεις and ας πούμε
  were flagged and shown, but left in. It cut only those near the end of the page (δηλαδή,
  Οκ, ξέρετε, Τέλος πάντων, ας πούμε). Its one wrong cut takes "το αρχείο" out of the
  sentence that follows the retake ("Το αρχείο αυτό | Το requirements").
- **The candidate pass never shows it some retakes.** Single-word repeats ("volume volume"),
  cut-off words ("comp… um Complementary", "see set"), and restarts in other words ("they
  there is", "Θα πατήσουμε στο | Θα κάνουμε κλικ", "Και τρέχουμε | Και το τρέχουμε") open no
  window. The cloud review caught 3 of these 6 by reading every word.

### What the cloud review gets wrong

- No wrong cuts of its own in 3,618 words.
- Two partial cuts: "know," out of "you know," (leaving "you") and "ενεργο-" out of "το
  ενεργο- | το ενεργοποιούμε" (leaving "το το"). They count as misses above.
- It cuts most sentence-opening "So" (labelled optional), which suits a tutorial edit.

### What the rules get wrong (all engines inherit it)

Two retake cuts of deliberate repetition: "this is what we call human in the loop. Human in
the loop essentially means…" and a list, "you can select Purple, you can select Nado,
Extended…". `apply: true` makes these cuts whichever engine runs.

## Recommendation

**Decided 21 Oct (George): point 1 is done.** MCP `review: auto` and the app's Find with AI
(saved default `cloud`) use OpenRouter/OpenAI when a key is set, and the local model only
without one.

1. **`review: auto` should prefer the OpenRouter review when a key is set, and use the local
   model only without a key.** In English, local caught 2 more must spans than the rules
   alone (9 missed against 11) but added 10 wrong cuts (45 words), against 0 for cloud. Its
   wrong cuts are whole clauses, which is worse than a missed "um". Without a key it is
   still better than the rules, if its cuts are reviewed.
2. **Do not make `apply: true` the default for the local review.** Of its 31 English AI
   cuts, 19 were judgement calls (mostly an opening "So"); of the other 12, 10 were wrong and
   2 right. For cloud: 35 AI cuts, 27 judgement calls, 8 right, 0 wrong.
3. **Not yet for the cloud review either; it is close.** Its own cuts were clean, but
   `apply: true` also applies the rules' two wrong retakes, and it made two partial filler
   cuts. Once the rules stop cutting deliberate repetition, `apply: true` for cloud fillers
   only (not retakes) would be safe on this evidence. 18 English must spans is a small
   sample: label one more real recording before switching.
4. **Greek: local is not usable yet** (10 of 19 missed; cloud 1 of 19). Greek is online-only
   for transcription anyway, so a Greek user has a key; `auto` should go to cloud for Greek
   even if local is preferred for English. Gemma 4 E4B in the same slot is the next test
   (below).

### Fixes that would help the local review (not done; defaults unchanged)

- Stop the model approving `Retake` flags blindly: only cut a flagged retake when the span is
  the first attempt *and* the second attempt follows within a pause, or leave local retakes
  as suggestions and never apply them.
- Drop "so" from the local `Filler` flags unless it opens a sentence or comes before a pause.
- Flag single-word repeats, cut-off words (ending in "-" or a stub followed by its longer
  form) and short restarts. Both engines missed "volume volume" and "to the lower pairs → to
  the lower perpetuals".
- Rules: don't treat "X. X essentially means" or list items ("you can select A, you can
  select B") as retakes.

### Gemma 4 E4B

Not run yet. Our llama.cpp (llama-cpp-2 0.1.160) has the `gemma4` architecture but not its chat
template, so the probe needs Gemma's turn format written out, as `local.rs` does for Qwen.
The file is `unsloth/gemma-4-E4B-it-GGUF`, `gemma-4-E4B-it-Q4_K_M.gguf`, 4.98 GB,
Apache-2.0. George chose to skip it for now (21 Oct).
