# Graphics

Motion graphics are HyperFrames compositions the app renders and plays full screen over
everything (screen, camera, captions); in a 9:16 short the same graphic renders at 9:16, or at
its part of the split. Cuts never go through a graphic, so add them after the cuts are done and
checked, before chapters.

A graphic that fills the whole screen never eases in or out: it appears and disappears with a
hard cut. Only graphics laid over the full-screen camera (a lower third or callout during a
camera-only section) ease. The app decides this from where each graphic plays and sets
`data-entrance` (`cut` or `ease`) on its root when it renders; `graphic_render` says which each
got. Never add your own fade to a full-screen graphic. In a composition you write, show the card
whole from the first frame to the last and fade or slide it only when
`root.dataset.entrance === "ease"`; mark one that covers only part of the frame with
`data-overlay` on the root (without it the app treats it as full screen and always cuts it).

Motion: GSAP 3.15 (HyperFrames' own) works offline: load it by file name, `<script
src="gsap.min.js">` (a plugin as `SplitText.min.js`, `CustomEase.min.js`...; a CDN link to GSAP is
turned into that name), build one paused timeline as long as the clip and register it as
`window.__timelines["graphic"]`. `blank` starts that way. The Web Animations API works too, with
`data-no-timeline` on the root. Nothing else may reach the network.

Look first at what exists: the list below has the bundled templates and the user's library,
each library item with when to use it; pick one that fits and fill in its variables. Write a composition of your own (`graphic_add` with `blank`, then
`graphic_write_file`) only when nothing there does the job. Ask before `graphics_install`.
After adding, `graphic_render`, then look at its motion in one `frame_render` with `graphic`
(its id: a sheet of frames across it; `short` for a short's), never a call per frame.

Each graphic carries the sentence it sits on: the claim, the number, the step or the names said
there, in the speaker's words, shortened. Read the words around its `atWord` first. A title
that only names the topic ("Memory", "Tools") or a line any video could use is not a graphic:
show what this sentence says ("Reset the chat. Keep the plan."), the numbers it gives, the
three things it lists, the before and after it describes. If a sentence has nothing to show,
it gets no graphic. Propose graphics as a table, one row each: start, end, template (or
library item), what it shows in one line, and the words it sits on.

When the user likes a graphic you wrote, offer to keep it. On a yes: `graphic_templatize` (its
source and the rules: text as variables, the style's colours, timing from `data-duration`, a
portrait block, a cut unless `data-entrance` is `ease`, no network), rewrite the files, then
`graphic_check` until it passes. Then `library_save` with a `label`, a one-line `useWhen` and
`lengthUs`. If it answers with a close library item or template, ask whether to update that
item (`update`) or keep both (`asNew`). Never save without the user's yes; save a failing
graphic (`asIs`) only at their word.

Every graphic in a video wears the project's style (`library` `kind: style`): its brand
(colours, title and body fonts, logo) and its graphics look (type sizes, corners, spacing and
motion as `--look-*` tokens), bigger and livelier at 9:16. Changing the style restyles every
graphic in every project wearing it, so ask first; for a change in words ("calmer", "rounder"),
`op: preview` with `set`, show the user, and `op: set` on their yes. A graphic may wear another
style or set tokens of its own (`graphic_set` `style`, `look`). The style fills the
`background`, `text` and `accent` variables a graphic has not set, so leave those unset; a
composition of your own takes its sizes, corners and motion from the tokens (`blank` shows how)
and can use `var(--brand-accent)`, `var(--brand-background)`, `var(--brand-text)`,
`var(--brand-muted)`, `var(--brand-font)`, `var(--brand-body-font)` and `var(--brand-logo)`;
`graphic_check` warns about values it fixes itself. `graphic_add` with `library`
copies a library item in. Items marked "as is" fail the check: look at one with `frame_render`
before relying on it.

<!-- shorts -->
In a short, a graphic shows one of three ways: `full` (the whole frame), `screen` (split: the
face above, the graphic in the screen's part, footage or not) or `overFace` (the face fills the
frame and the graphic is drawn over it, see-through); also `camera` and `overScreen`. Pick it
when the graphic is added (`mode` in `short_footage` items) so it renders once; `graphic_set`
with `short` and `slot` changes it later. The graphic's root gets `data-mode` (`full`, `split`
or `over`): a composition of your own for `over` leaves its background out. A split graphic
renders into its own box, the screen's part: the reply that places it says the box's size (`a
1080x1248 box (portrait)`), and the root's `data-width`/`data-height` hold it. The user can move
the split, so the box can turn about square or landscape; it renders again at the new size. Up to
3:2 wide the box takes your `@media (orientation: portrait)` rules, as a part of a phone frame;
wider, your landscape ones. Write for the box you are told, in vw/vh/vmin, never for the 9:16
frame.
<!-- /shorts -->
