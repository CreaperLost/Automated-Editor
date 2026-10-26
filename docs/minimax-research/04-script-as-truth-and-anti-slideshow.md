# 04 — The script as truth, and the anti-slideshow position

- **Source:** [github.com/Vincentwei1021/video-talkcraft](https://github.com/Vincentwei1021/video-talkcraft)
- **Licence:** **PolyForm Noncommercial 1.0.0** — noncommercial use is free, any commercial use
  needs the author's permission · captured 25 Oct 2026 · 1,759★ · last push 2026-10-12
- **Raw capture:** [evidence/2026-10-25/vincentwei1021-video-talkcraft.md](evidence/2026-10-25/vincentwei1021-video-talkcraft.md)
- **Evidence grade:** vendor claim, read from the source. Not tested in AeroEdits.

> **Licence first.** AeroEdits' rule allows MIT, Apache-2.0, GPL-3.0 (as a separate component) and
> OpenRAIL, and refuses non-commercial, research-only and revenue-capped licences. This repo is
> PolyForm Noncommercial, so it is **not usable**. Read the method, take the idea, copy nothing —
> no file, no card, no text. Its sibling `video-shotcraft` is Apache-2.0 and is freely adaptable
> (see [03](03-shot-recipe-card-format.md)). This is not legal advice; it is the project's own
> rule applied to a licence file we read.

## What it is

The talking-head companion to `video-shotcraft`: a skill that takes a **script and a finished voice
track**, aligns **word-level timestamps** on the machine, writes each semantic beat into a
**SHOTBOOK** storyboard, and renders with Remotion — kinetic type cards, evidence screenshots,
camera moves, captions and sound, all locked to the voice.

108 motion recipe cards, a seven-layer "anti-slideshow" camera system, and a triple acceptance
check. Six ideas from it are worth having.

## 1. The script is the truth when trimming the voice

Their `voice_trim.py` pre-cuts the recording **against the script**:

- only cut segments **that are not in the script** — fillers, stutters, over-long pauses;
- a word the ASR **misheard is never cut**, because the script says it is there;
- the word list can come from FireRed ASR, a per-character SRT, or word-level JSON;
- cuts snap to the **frame grid**, and the same edit list can trim the picture so audio and video
  stay the same length.

This is a direct answer to a gap AeroEdits has already written down. `docs/gaps.md` records that
the filler/retake rules cut a deliberate repeat — "human in the loop. Human in the loop essentially
means…" — and that every review engine inherits that wrong cut. Their approach does not try to
detect a retake better; it **removes the ambiguity** by holding a second source of truth. Where
AeroEdits has a script, that is the better oracle. Where it does not, the gap stays open.

## 2. Subtraction as a motion policy

> 每场景只保留一条极缓推拉相机曲线 + 让位；idle 微动 / 环境呼吸 / 脉冲全部默认关闭。
> *Each scene keeps one very slow push/pull camera curve and yields the rest; idle micro-motion,
> ambient breathing and pulses are all off by default.*

AeroEdits' research already flagged the same risk in HyperFrames' own creative defaults — ambient
motion everywhere, layered backgrounds — and warned they "can make busy work". A second
independent source reaching the same conclusion is a signal, and the concrete form is useful: not
"less motion" but **one camera move per scene, and everything else off**.

## 3. A text-only shot must have a companion drawing

> 纯文字镜必配一张图标 + 方框 + 箭头的线稿陪衬图，文字不裸放，每句至少一个可见变化，缺陪衬行
> preflight 报 WARN。
> *A pure-text shot must have a schematic companion — an icon, a box, an arrow. Text is never left
> naked. Every sentence gets at least one visible change. A missing companion line is a preflight
> WARN.*

This is the clearest rule we have found for **explainer graphics**, and it is checkable without a
model: a shot whose only content is text fails a preflight check. It is also a direct answer to a
pattern in the research — "presenter-only moments are a deliberate choice" and "a visual needs an
explanatory job" are good principles, but neither tells an agent what to put on screen. This does.

## 4. Judge the domain first, then reskin — never regenerate

> 开工先由口播稿判定领域、派生风格档写进 SHOTBOOK G0；库里的卡是中性 UI，进片按风格档改皮不改
> 运动命门，每镜写蒙皮行。
> *Before work starts, judge the domain from the script, derive a style tier and write it into the
> shotbook. The library cards are neutral UI; in the film you reskin them and do not change the
> card's motion vital principle. Each shot records its skinning line.*

Two ideas: a **style tier decided once at the start** (which is AeroEdits' "one look for every
graphic in a video", reached from the other direction), and **reskin without regenerating** — the
colour changes, the motion's principle does not. That is the same instinct as GFX-4's revision
scope, where "a colour fix never regenerates a sequence".

## 5. Per-shot pipeline state, visible

Their workbench shows, per shot: *placeholder / implemented / rendered / passed the gate / stale /
red dot*, driven by what is on disk, with a script that only adds the human's judgement. AeroEdits'
proposals have a status, but a shot-level board of "built, rendered, passed, stale" is a different
thing — it is the state of the **work**, not of a **proposal**.

## 6. Do the first shot first, and confirm it

> ⑤ 实现改为先搭全合成骨架、其余镜头占位，只做样板镜并给出一条有声单镜预览给用户确认，确认后再做其余镜头。

Build the whole skeleton with placeholders, implement **one** sample shot, show it with sound, get
it confirmed, then do the rest. AeroEdits' PLAN-1 proposes a style trial on a 5–10 s range, which
is the same idea at video scale; their version applies it per shot.

## Caveats

- **Non-commercial licence.** Everything above is a method, not a file to copy.
- It is **Chinese-language and Remotion-based**; the shotbook's structure is the transferable part.
- The word-level sync assumes a **finished voice track** — the opposite of AeroEdits, where the
  recording *is* the source and the cleanup happens before the edit.
- Their pipeline is a skill plus a Remotion workbench, not an editor. AeroEdits has the editor;
  what is missing here is the editor part.
