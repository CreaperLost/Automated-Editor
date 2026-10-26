# How to Make Motion Graphics With Claude Opus 5.5 Without the AI-Slop Look - Artimind

- **Source:** https://aiartimind.com/how-to-make-motion-graphics-with-claude-opus-5-5/
- **Kind:** web
- **Captured:** 2026-10-25T17:40:38
- **Evidence grade:** vendor or creator claim. Not tested in this repo. Quotes from a caption are what a creator *said*, not a measurement.

**Title:** How to Make Motion Graphics With Claude Opus 5.5 Without the AI-Slop Look - Artimind

**Meta description:** A step-by-step Claude Opus 5.5 motion graphics workflow, shown with real renders: pick a deterministic engine, add taste rules, brief with exclusions, lock tokens, plan beats in frames, build in four passes, inspect boundary frames and revise by frame range.

## Page text

How to Make Motion Graphics With Claude Opus 5.5 Without the AI-Slop Look - Artimind 

 Skip to main content 

 Hit enter to search or ESC to close 

 Close Search 

 Artimind 

 search 
 
 0 

 Menu 

 Home 

 AI Art 

 AI Prompt 

 Blog 

 Contact Us 

 twitter pinterest instagram 

 search 

 0 

 was successfully added to your cart. 

 Menu 

 Artimind

 AI Coding AI Tools Claude Code for Video & Design 
 How to Make Motion Graphics With Claude Opus 5.5 Without the AI-Slop Look

 By hello.artimind@gmail.com September 29, 2026 October 22nd, 2026 No Comments 30 min read 

 Claude Opus 5.5 does not generate video frames. It writes a program, usually HTML, CSS and JavaScript or a React component, in which every pixel is a function of time. A headless browser then captures each frame and ffmpeg encodes the MP4. That is why its motion graphics look sharper than diffusion video: the type is real type, the shapes are real vectors, and every frame renders the same way twice.

 It is also why most first attempts look the same. Ask for “a cool animated intro” and the model fills every open decision with its defaults: a shifting purple-to-cyan gradient, a frosted glass card, glowing particles, words bouncing in together, and a tagline like “AI-powered. Next-gen. Seamless.” The engine is not the problem. The brief is.

 This guide walks through the workflow step by step, using one real 10-second, 1080×1080 piece built for this article. Every still and clip below comes from that project, rendered with Remotion, so you can see what each step changes.

 Step 
 What you decide 
 Slop it prevents 

 Choose a render engine 
 Remotion, HyperFrames or a single HTML file 
 Nondeterministic, unrepeatable renders 

 Install skills and taste rules 
 Framework best practices plus your own bans 
 Default gradients, glass and glow 

 Write a brief with exclusions 
 Audience, message, format, what must not appear 
 A flashy reel with nothing to say 

 Lock design tokens 
 Palette, type, spacing, safe areas 
 Colors and sizes drifting between scenes 

 Plan beats in frames 
 Purpose and frame range for every beat 
 Random pacing and everything moving at once 

 Build in four passes 
 Layout, type, motion, then color 
 Motion hiding weak composition 

 Render and inspect frames 
 Boundary stills before the full render 
 One-frame flashes and clipped text 

 Direct revisions by frame 
 Notes tied to exact frame ranges 
 Vague “make it pop” loops 

 1. What AI Slop Looks Like in Motion

 Slop is not a style. It is what appears when nobody made a decision. The clip on the left was produced by asking for “an exciting animated intro for an innovative AI product” with no other constraints. The clip on the right is the finished piece from this guide.

 Left: an unconstrained “exciting AI intro” prompt. Right: the directed piece built in this guide. Same engine, same length. 
 The tells are consistent across almost every unconstrained generation:

 Rainbow or purple-cyan gradients that shift hue constantly, so the background competes with the content.

 Glassmorphism : translucent cards with blur, glow borders and inner shadows.

 Decorative particles drifting upward with no connection to the message.

 Everything animating at once , usually fade plus scale on a linear curve, with nothing establishing hierarchy.

 Generic copy : “Unleash”, “the future of”, “seamless”, “next-gen”.

 Constant motion : bouncing, orbiting or 3D tilting that never settles.

 No hold at the end , so the viewer never gets to read the resolved frame.

 The fix for each one is a decision made before code exists. The rest of this guide is about making those decisions in the right order.

 2. Choose a Render Engine

 Opus 5.5 will pick the simplest route unless told otherwise. Independent tests found it defaults to a single HTML file with a seek() function, captured by Playwright and encoded by ffmpeg, even when a framework like Remotion is installed. Name the engine in the prompt.

 Engine 
 How it works 
 Best for 

 Remotion 
 React components; every value derived from useCurrentFrame() 
 Reusable systems, data-driven video, long projects 

 HyperFrames 
 HTML, CSS and GSAP compositions rendered by headless Chrome and FFmpeg 
 Web designers, GSAP users, Claude Design handoff 

 Single HTML + seek(t) 
 One file exposing window.seek(t) ; Playwright captures, ffmpeg encodes 
 One-off launch clips, quick social pieces 

 Whichever you choose, the same contract applies: every visible state must be a pure function of time. No Date.now() , no requestAnimationFrame loops, no CSS transitions, no timers and no unseeded Math.random() . Browser animations that run on wall-clock time render differently on every capture. Frame-derived animations render the same result every time, which is what makes precise revision possible.

 The demo in this guide uses Remotion . Create a project and install its agent skills:

 Install 

 npx create-video@latest
npx skills add remotion-dev/skills 

 For the HTML route, HyperFrames is HeyGen’s open-source renderer: the agent writes HTML plus seekable GSAP timelines, previews in a browser and renders a deterministic MP4. It ships as a Claude Code plugin and a Claude connector.

 HyperFrames: HTML and GSAP compositions rendered to deterministic MP4. Source: github.com/heygen-com/hyperframes 
 3. Install Skills and Taste Rules

 Framework skills teach Claude how to use the tool correctly. Remotion’s official set covers project creation, markup, captions, rendering and a /remotion-best-practices fallback. HyperFrames ships skills for its full loop: plan, write HTML, wire seekable animations, lint, preview and render.

 Remotion’s official agent skills, installed with npx skills add remotion-dev/skills. Source: remotion.dev 
 Those skills make the code correct. They do not give the output taste. Add a short rules file of your own, either as a project CLAUDE.md section or as a custom skill, that loads before any code is written. Write it as bans and defaults, not adjectives:

 ## Motion rules (apply to every composition)

Never:
- gradients that shift hue, glassmorphism, glow, bloom, lens flares
- decorative particles, orbiting shapes, 3D card tilt
- linear easing on anything a viewer tracks
- more than one dominant action on screen at a time
- placeholder or marketing copy ("unleash", "seamless", "next-gen")

Always:
- import colors, type sizes and spacing from tokens.ts
- enter with ease-out (cubic-bezier(0.16, 1, 0.3, 1)); exits faster than entrances
- stagger related elements 3-6 frames; stagger nothing unrelated
- hold the resolved final frame for at least 1 second
- keep text inside the safe area at the output resolution 
 Bans work better than descriptions. “No neon glow” removes a default. “Make it premium” leaves the default in place and asks the model to guess what you meant.

 4. Write a Brief With Exclusions

 A production brief separates what Claude must preserve from where it may explore. Lock the format, the audience, one message and one visual idea, then list exclusions. Ask for a plan before any code.

 This is the brief used for the demo:

 ▣ Claude Code ⌁ project ⑂ main ✦ 

 You are the motion director for a 10-second square social loop.

 Deliverable: 1080×1080, 30 fps, exactly 300 frames, built in Remotion as editable React components.

Audience: product designers and founders.

Message: a rough idea only becomes a shippable interface once you add a system and commit to one direction.

Visual idea: one rounded rectangle is the hero. It starts as a hand-drawn sketch, splits into four competing variants, snaps to a grid, merges into a single “Ship it” button, and expands into a finished checklist panel.

Look: warm off-white paper, near-black ink, one violet accent, Inter for UI text, Playfair Display italic for the closing line, flat fills, 3px ink outlines.

Motion: restrained and physical, ease-out entrances, one dominant action per beat.

Avoid: gradients, glass, glow, particles, 3D tilt, bouncing, linear easing, marketing copy.

 Before coding, return:

1. a beat sheet with exact frame ranges and the purpose of each beat;

2. the continuity device that links adjacent beats;

3. design tokens;

4. the three riskiest implementation assumptions.

Do not write code yet. ↵ 

 Auto ＋ ◉ Claude Code Standard 

 The recurring shape is the most important line in the brief. It gives the viewer something to track across every cut, and it gives Claude a concrete constraint for every transition. Without it, each beat becomes a separate slide.

 4.1 Find a reference before you write

 A brief is easier to write when you know what kind of piece you are asking for. The community-maintained awesome-opus-5-5-videos list collects source-linked Opus 5.5 videos and sorts them by production path: code-drawn 2D, Manim explainers, WebGL scenes, captures of real apps, and pipelines that hand off to external video models. It also documents a 53-palette color study and several reusable open-source starter systems.

 A community index of Opus 5.5 videos classified by production path. Source: github.com/athemeroy/awesome-opus-5-5-videos 
 Pick one example whose production path matches what you want, then describe that path in words inside your brief. “Code-drawn 2D, flat fills, one recurring shape” gives Claude something it can build. “Make it look like this video” asks it to guess which parts of the video you liked.

 5. Lock Design Tokens

 What a token is. A design token is a named value, such as violet or safe , that every scene reads from one shared file instead of typing the raw value ( #7C3AED , 96 ) wherever it is needed.

 Why it matters for slop. In a long build, Claude writes dozens of components across several sessions. Without a single source of values, it drifts: one scene gets a slightly different purple, another a 60px title instead of 56px, a third a new gray nobody chose. Each change is small. Together they are a large part of what makes AI output look assembled rather than designed. Tokens make drift impossible to hide, because any value that is not a token stands out in the code.

 What to ask Claude. The brief already asked for tokens as part of the plan. If you skipped that, ask for them on their own, and ask Claude to justify each value against the brief:

 ▣ Claude Code ⌁ project ⑂ main ✦ 

 Propose design tokens for this piece as a single TypeScript object.

Limits: one accent color, at most three type sizes, one spacing unit, one corner radius, one outline width, and a safe-area margin.

For each value, give one sentence explaining how it serves the brief.

Then add a rule to CLAUDE.md: scene components must import every color, font, size and spacing value from tokens.ts; no literal values. ↵ 

 Auto ＋ ◉ Claude Code Standard 

 The limits matter as much as the values. A palette with one accent forces Claude to decide where emphasis goes. A palette with five accents lets it put emphasis everywhere, which means nowhere.

 What the demo uses. 

 Ts 

 export const TOKENS = {
 paper: '#F2EFEC',
 card: '#FFFFFF',
 ink: '#17131C',
 violet: '#7C3AED',
 muted: '#6D6475',
 rule: '#D8D1CB',
 sans: 'Inter, Helvetica, Arial, sans-serif',
 serif: '"Playfair Display", Georgia, serif',
 safe: 96,
} as const; 

 Each value has a job. The background is a warm off-white rather than pure white, so the frame reads as paper, not a blank screen. Ink is near-black rather than #000 , which looks harsh against warm paper. Violet is the only accent, reserved for the moment of decision. The safe margin of 96px (about 9% of the frame) keeps text clear of the areas that social apps cover with buttons and captions.

 How to check it. Search the scene files for raw color values. Any hit outside the tokens file is drift:

 Bash 

 grep -rnE "#[0-9a-fA-F]{3,6}\b" src --include=*.tsx | grep -v tokens 

 Tokens also make the next steps possible. The grayscale review passes below are not a separate build: they use a second token set with the same names and gray values, so the whole film switches palette by changing one setting.

 6. Plan Beats in Frames

 A beat is one unit of the story: a moment with one purpose and one main action. The beat sheet lists every beat with its exact start and end frame.

 Why frames, not seconds or adjectives. The render engine counts frames, not feelings. At 30 frames per second, one second is 30 frames and the whole 10-second demo is 300. If the brief says “hold briefly,” Claude picks a number, and it will pick a different number in the next revision. A frame range is a decision you can review, discuss and change precisely.

 How to size the beats. Start from what the viewer needs to understand, not from the total length. These are the working rules used for the demo:

 A short on-screen title needs at least a second (30 frames) of stillness to be read, on top of the time it takes to arrive.

 A transition between two states reads best at around 15–25 frames. Faster reads as a cut; slower drags.

 The final frame should hold for at least a second, so the viewer sees the resolved result.

 If the beats do not fit the total length, cut a beat. Do not squeeze them all.

 Give every beat three things: a purpose (what the viewer should understand), one dominant action (what moves), and a link to the next beat. In the demo, the link is always the same rounded rectangle, which is why the piece reads as one continuous idea instead of five slides.

 This is the demo’s beat sheet:

 Beat 
 Frames 
 Purpose 
 Dominant action 

 Rough idea 
 0–44 
 Establish the raw starting point 
 Hand-drawn outline draws itself on 

 Too many directions 
 45–99 
 Show indecision 
 Shape splits into four tilted variants 

 Add a system 
 100–159 
 Introduce rules 
 Grid appears first, then variants snap to it 

 Commit to one 
 160–219 
 Show the decision 
 Variants merge into a “Ship it” button, which gets pressed 

 Ship the interface 
 220–267 
 Show the result 
 Button expands into a checklist panel; rows assemble 

 Resolve 
 268–299 
 Let the viewer read it 
 Closing line lands; everything holds 

 Look at the order inside “Add a system.” The grid appears at frame 94, before the cards start moving at frame 100. If they moved at the same time, the viewer would see cards moving and a grid appearing, two unrelated events. Because the grid comes first, the viewer sees a rule, then things obeying it. Six frames of order turns decoration into meaning.

 How to check it. Read only the Purpose column, top to bottom. It should tell the whole story in one breath: rough idea, too many directions, add a system, commit, ship, resolve. If the purposes do not make sense as text, fix the plan before any code exists. Animation will not rescue a story that does not work on paper.

 7. Build in Four Passes

 Asking for a finished piece in one step is where most slop comes from. When layout, type, motion and color all arrive at once, the glossy parts hide the weak parts, and your feedback mixes four kinds of problem. Claude then changes several things per revision, and fixes start breaking each other.

 Build in four passes instead. Each pass answers one question, and you approve it before starting the next:

 Pass 
 Question it answers 
 Deliberately left out 

 1. Blocking 
 Does the composition work? 
 Real text, easing, color 

 2. Typography 
 Can the words be read at the right time? 
 Easing, color 

 3. Motion 
 Do the transitions carry the story? 
 Color, texture, sound 

 4. Finish 
 Does color reinforce the hierarchy? 
 Nothing 

 How to set it up. Ask Claude to build one composition with a pass setting, rather than four separate projects. Every pass then renders from the same code and the same timing, so what you approve in one pass cannot silently change in the next.

 ▣ Claude Code ⌁ project ⑂ main ✦ 

 Build the piece as one Remotion component that takes a pass prop (1-4).

Pass 1: gray token set, gray bars instead of text, every change is a hard cut at its keyframe, show a frame counter.

Pass 2: same as pass 1 but with final copy.

Pass 3: same as pass 2 with eased transitions between the same keyframes.

Pass 4: full token set, color and texture, no frame counter.

Register one composition per pass so each can be rendered separately. ↵ 

 Auto ＋ ◉ Claude Code Standard 

 You do not need to write this code yourself, but it helps to see what the pass setting actually controls. The core of the demo’s setup is one small function that returns how far an animation has progressed at a given frame:

 const EASE = Easing.bezier(0.16, 1, 0.3, 1);

const progress = (frame: number, start: number, pass: Pass, dur = 22) => {
 if (pass < 3) return frame >= start ? 1 : 0;
 return interpolate(frame, [start, start + dur], [0, 1], {
 easing: EASE,
 extrapolateLeft: 'clamp',
 extrapolateRight: 'clamp',
 });
}; 
 In passes one and two it answers “has this started yet?” with a plain yes or no, so every change is an instant cut. From pass three it returns a smooth value from 0 to 1 over 22 frames, shaped by an ease-out curve. The keyframes are identical in every pass. Only the way the film gets between them changes.

 7.1 Pass 1: Block the composition in grayscale

 Goal: judge the composition before anything makes it look good. The frame shows boxes, gray bars where text will go and alignment guides. There is no color, no easing and no real copy. The frame counter in the corner makes it easy to refer to exact moments in feedback.

 Pass 1 at frames 20, 130, 200 and 290: gray bars stand in for text, and every change is a hard cut. 
 Pass 1 in motion. Crude on purpose: this is where you judge composition, not polish. 
 Check: 

 Is the dominant action of each beat obvious without color?

 Is anything too small, too large or crowded against an edge?

 Does everything stay inside the safe area?

 If you cannot tell what matters in grayscale, color will not fix it.

 7.2 Pass 2: Lock typography and reading time

 Goal: make every word readable, at the right size, for long enough. Replace the gray bars with final copy at the real output size, still in grayscale and still with hard cuts.

 Check: 

 Does each title stay on screen long enough to read comfortably, twice?

 Do line breaks fall in sensible places at 1080 pixels wide?

 Is there a clear hierarchy, or are several lines competing for attention?

 Let a short headline arrive as one unit. Staggering words one by one is only worth it when their order carries meaning. The demo uses three type levels: a small numbered label, a 56px beat title and the closing serif line. Anything more at this size starts to compete.

 Pass 2 at the same frames: real copy at output size, still grayscale, still hard cuts. 
 7.3 Pass 3: Add motion and continuity

 Goal: turn cuts into movement that explains the change. Three principles did most of the work in the demo:

 Ease-out for entrances. An element starts fast and settles slowly, the way a physical object stops. Linear movement, constant speed from start to finish, is the most common tell of generated motion.

 Main element first, the rest respond. The hero shape moves first; everything else reacts to it. That is how the viewer knows where to look.

 Small staggers group things. The four variants split 4 frames apart, so the split reads as one gesture instead of four separate events. The checklist rows enter 6 frames apart for the same reason. Staggers above about 8 frames start to read as separate actions.

 The one playful touch, the button press, is a 7% scale dip over ten frames: just enough to read as a click, not enough to bounce.

 Check: watch the pass-2 and pass-3 renders side by side. The layout should be identical. If anything moved position, pass 3 changed more than the transitions.

 Left: pass 2, hard cuts. Right: pass 3, the same keyframes joined with ease-out and staggered entrances. 
 7.4 Pass 4: Add color, texture and sound last

 Goal: use color to reinforce a hierarchy that already works. Swap in the full token set. In the demo, violet appears only where the decision happens: the “Ship it” button, the completed checks and the arrow in the closing line. Everything else stays ink on paper. A 7% film-grain overlay takes the digital flatness off the background without adding a visual idea of its own.

 Pass 4, the finished 10-second loop: violet only where the decision happens, light grain, a held final frame. 
 Check: if you squint at any frame, does the violet land on the thing that matters most in that beat? If color draws the eye somewhere else, the color is wrong, not the layout.

 Add music and sound effects only after the picture is locked, and cut them to visible events. Sound added earlier makes weak timing feel acceptable, and you stop seeing the problems.

 8. Render and Inspect Frames

 Watching the full video is the wrong first check. At 30 frames per second, a problem that lasts five frames is gone before you notice it. Start with still images at the frames most likely to go wrong.

 Which frames. Defects cluster where one beat hands over to the next, because that is where two sets of elements briefly coexist. Take every start frame from your beat sheet, the frame just before it, and the final frame. For the demo that is 0, 44, 45, 99, 100, 159, 160, 219, 220 and 299.

 How to render them. remotion still renders one frame to an image; remotion render renders the full video. The composition name is whatever your project registers, here the pass-4 composition:

 Install 

 npx remotion still src/index.ts OpusGuidePass4 out/f044.png --frame=44
npx remotion still src/index.ts OpusGuidePass4 out/f100.png --frame=100
npx remotion still src/index.ts OpusGuidePass4 out/f160.png --frame=160
npx remotion render src/index.ts OpusGuidePass4 out/final.mp4 --concurrency=1 

 Put the stills together in one contact sheet, a single image with all the frames in a grid, and look for duplicated elements, one-frame flashes, clipped text and sudden background changes. Review them side by side.

 Boundary stills at frames 44, 45, 100, 160, 220 and 299. Check handoffs before watching the full render. 
 This check caught a real defect in the demo. At frame 165, where the four variants merge into the button, the first pass-3 render showed three translucent ghost cards on top of a half-formed “Ship it” label. The timing was fine. The problem was drawing order: the hero shape was drawn first, so the fading variants sat on top of it. The fix was small: draw the hero last, fade the other variants out over 8 frames instead of 22, and show the button label only once the shape is 70% formed. At full speed the glitch lasts under half a second. A still at the exact frame exposes it immediately.

 Frame 165 before and after the fix. The defect lasts under half a second and is easy to miss at full speed. 
 Then watch it at full speed. Only after the stills are clean, watch the video once without pausing. Write down what you perceive, not what you think the code should do: “the grid arrives after the cards start moving,” “the final frame disappears too quickly.” You will turn those notes into instructions in the next step.

 Make Claude check its own work. Opus 5.5 can look at images it renders, so it can do the first round of inspection before you see anything. End every build prompt with a verification requirement:

 ▣ Claude Code ⌁ project ⑂ main ✦ 

 After building, render stills at frames 0, 44, 45, 99, 100, 159, 160, 219, 220 and 299 and inspect each one.

Report any clipped text, overlapping elements, off-token colors, or elements outside the 96px safe area.

Fix what you find, re-render the stills, then render the final MP4 with –concurrency=1 and report the output path. ↵ 

 Auto ＋ ◉ Claude Code Standard 

 9. Direct Revisions by Frame

 Revision is where most time goes, and where vague feedback costs the most. “Make it pop” or “it feels off” gives Claude nothing to change, so it changes something at random, often something you had already approved.

 A good revision note has three parts: what you perceived , where it happens (a frame range, read from the frame counter or the beat sheet), and what should be true instead . The last part is a constraint Claude can meet and you can check, not a description of the code.

 These are examples of translating reactions into notes, using moments from the demo:

 What you perceived 
 Where 
 What should be true instead 

 “The cards jump before the grid makes sense” 
 Frames 94–112 
 The grid finishes drawing before any card starts to move 

 “The split feels like a jump cut” 
 Frames 45–70 
 The four variants start 4 frames apart, in reading order 

 “The ending is cut off” 
 Frames 268–299 
 All motion ends by frame 286; the final frame holds to 299 

 Send the notes in one small batch rather than one at a time, so Claude can see how they interact. Always state what must not change, and ask for proof that nothing else moved:

 ▣ Claude Code ⌁ project ⑂ main ✦ 

 Revise only the following. Keep the copy, tokens, component structure and total duration unchanged.

 – Frames 94-112: the grid must finish drawing before any card starts to move.

– Frames 45-70: stagger the four variants 4 frames apart in reading order.

– Frames 268-299: end all motion by frame 286 and hold the final frame to 299.

 Re-render the stills at 45, 100 and 299 and list every file you changed. ↵ 

 Auto ＋ ◉ Claude Code Standard 

 Asking for the list of changed files is the safety check. If a note about the ending changed the file for the opening beat, you know to look there before you approve anything.

 When to stop. Three to five notes per round keeps each change traceable. Stop when your notes shift from “the viewer will not understand this” to “I would prefer that.” The first kind is a defect. The second is taste, and you can keep adjusting taste forever.

 10. Cost and Limits

 Rendering runs on your own machine and costs nothing beyond compute time. The model cost depends on how much Claude reads and writes. One published breakdown of an automated launch-video pipeline measured about 90,000 input and 15,000 output tokens per film, which is under a dollar at standard Opus 5.5 pricing, before voice, assets and retries. A hands-on project with several revision rounds uses more, because every round re-reads code and images.

 The limits matter more than the cost:

 It can drift toward animated slides. Without a continuity device, like the recurring shape in the demo, each beat becomes a separate slide with a transition.

 Long sessions can reintroduce timers. Late edits sometimes add setTimeout , CSS transitions or requestAnimationFrame , which break deterministic rendering. Search for them before the final render.

 Facts are not checked by default. Supply the copy and numbers yourself, and require Claude to check every on-screen claim against the brief.

 Organic illustration is hard. Type, charts, interfaces and geometric shapes work well. Hand-drawn characters and detailed illustration are better made elsewhere and imported as assets.

 What stays your job is judgment: the brief, the references, the hierarchy, the final copy, licensing and the decision to stop. What Claude gives you is an editable system rather than a locked clip. After the first build, a palette change is one token edit, a pacing note is one frame range, and every render reproduces everything else exactly.

 Tags:
 AI Video Claude Code Claude Opus 5.5 Motion Graphics Remotion 

 Love 0 Share Share Share Pin 

 Previous Post OpenAI Dots vs Meta Muse: Which Always-On AI Agent Should You Use

 Next Post Turn Claude Into a Multi-Model Image and Video Studio With One Skill

 Recommended For You

 AI Tools AI Video & Music AI Video Models 
 Wan 3.0 Guide: What It Costs, What It Does Well and How to Use It 

 Wan 3.0 Guide: What It Costs, What It Does Well and How to Use It

 hello.artimind@gmail.com October 22, 2026 

 AI Coding AI Tools Claude Code Skills & MCP 
 Claude Skills Best Practices: 10 Rules From Anthropic’s Authoring Guide 

 Claude Skills Best Practices: 10 Rules From Anthropic’s Authoring Guide

 hello.artimind@gmail.com October 21, 2026 

 AI Coding AI Tools Claude Code for Video & Design 
 How to Make Motion Graphics in Any Style With Claude Code and fal.ai 

 How to Make Motion Graphics in Any Style With Claude Code and fal.ai

 hello.artimind@gmail.com October 19, 2026 

 Leave a Reply Cancel Reply 
 My comment is.. 

 Name * 

 Email * 

 Website 

 Save my name, email, and website in this browser for the next time I comment. 

 Δ 

 Subscribe and never miss out

 Add your form shortcode here

 Home 

 About 

 More Demos 

 Artimind Studio · 

 Artimind Studio · 

 Artimind Studio · 

 Artimind Studio · 

 Artimind Studio · 

 Close Menu 

 Home 

 AI Art 

 AI Prompt 

 Blog 

 Contact Us 

 About aiartimind
 Hands-on guides to the AI tools creators actually use: Claude Code and coding agents, AI video and motion graphics, and AI agents that automate your work. Every workflow is tested before we publish it.

 Explore
 
 AI Coding Agents 

 AI Video & Visual Creation 

 AI Agents & Automation 

 Shop 

 Contact us 

 E: hello.artimind@gmail.com

## Links

- https://ogp.me/ns#
- https://aiartimind.com/how-to-make-motion-graphics-with-claude-opus-5-5/
- https://aiartimind.com/feed/
- https://aiartimind.com/comments/feed/
- https://aiartimind.com/how-to-make-motion-graphics-with-claude-opus-5-5/feed/
- https://aiartimind.com/wp-json/oembed/1.0/embed?url=https%3A%2F%2Faiartimind.com%2Fhow-to-make-motion-graphics-with-claude-opus-5-5%2F
- https://aiartimind.com/wp-json/oembed/1.0/embed?url=https%3A%2F%2Faiartimind.com%2Fhow-to-make-motion-graphics-with-claude-opus-5-5%2F&#038;format=xml
- https://www.monsterinsights.com/
- https://aiartimind.com/wp-content/plugins/salient-social/css/style.css?ver=1.2.5
- https://aiartimind.com/wp-content/plugins/woocommerce/assets/css/woocommerce-layout.css?ver=11.1.2
- https://aiartimind.com/wp-content/plugins/woocommerce/assets/css/woocommerce-smallscreen.css?ver=11.1.2
- https://aiartimind.com/wp-content/plugins/woocommerce/assets/css/woocommerce.css?ver=11.1.2
- https://aiartimind.com/wp-content/plugins/omnisend/styles/site-styles.css?1792769281&#038;ver=1.9.1
- https://aiartimind.com/wp-content/uploads/uag-plugin/assets/9000/uag-css-8731.css?ver=1792768983
- https://aiartimind.com/wp-content/themes/salient/css/font-awesome-legacy.min.css?ver=4.7.1
- https://aiartimind.com/wp-content/themes/salient/css/build/grid-system.css?ver=17.0.2
- https://aiartimind.com/wp-content/themes/salient/css/build/style.css?ver=17.0.2
- https://aiartimind.com/wp-content/themes/salient/css/build/header/header-layout-centered-logo-between-menu.css?ver=17.0.2
- https://aiartimind.com/wp-content/themes/salient/css/build/single.css?ver=17.0.2
- https://aiartimind.com/wp-content/themes/salient/css/build/elements/element-scrolling-text.css?ver=17.0.2
- https://aiartimind.com/wp-content/themes/salient/css/build/elements/element-recent-posts.css?ver=17.0.2
- https://aiartimind.com/wp-content/themes/salient/css/build/third-party/wpforms.css?ver=17.0.2
- https://fonts.googleapis.com/css?family=Open+Sans%3A300%2C400%2C600%2C700&#038;subset=latin%2Clatin-ext
- https://aiartimind.com/wp-content/themes/salient/css/build/responsive.css?ver=17.0.2
- https://aiartimind.com/wp-content/themes/salient/css/build/third-party/woocommerce/product-style-classic.css?ver=17.0.2
- https://aiartimind.com/wp-content/themes/salient/css/build/woocommerce.css?ver=17.0.2
- https://aiartimind.com/wp-content/themes/salient/css/build/skin-material.css?ver=17.0.2
- https://aiartimind.com/wp-content/uploads/salient/menu-dynamic.css?ver=3623
- https://aiartimind.com/wp-content/themes/salient/css/build/elements/widget-nectar-posts.css?ver=17.0.2
- https://aiartimind.com/wp-content/themes/salient/css/salient-dynamic-styles.css?ver=95361
- https://fonts.googleapis.com/css?family=Space+Grotesk:700%7CInter:700,400,400italic,500%7CMontserrat:500&#038;subset=latin&#038;ver=7.1.3
- https://aiartimind.com/wp-content/plugins/js_composer_salient/assets/css/js_composer.min.css?ver=7.8.1
- https://aiartimind.com/wp-content/plugins/js_composer_salient/assets/lib/vendor/node_modules/animate.css/animate.min.css?ver=7.8.1
- https://aiartimind.com/wp-content/themes/salient/css/build/elements/element-fancy-box.css?ver=17.0.2
- https://aiartimind.com/wp-content/themes/salient/css/build/elements/element-post-grid.css?ver=17.0.2
- https://aiartimind.com/wp-includes/js/jquery/jquery.min.js?ver=3.7.1
- https://aiartimind.com/wp-includes/js/jquery/jquery-migrate.min.js?ver=3.4.1
- https://aiartimind.com/wp-content/plugins/woocommerce/assets/js/jquery-blockui/jquery.blockUI.min.js?ver=2.7.0-wc.11.1.2
- https://aiartimind.com/cart/
- https://aiartimind.com/wp-content/plugins/woocommerce/assets/js/frontend/add-to-cart.min.js?ver=11.1.2
- https://aiartimind.com/wp-content/plugins/woocommerce/assets/js/js-cookie/js.cookie.min.js?ver=2.1.4-wc.11.1.2
- https://aiartimind.com/wp-content/plugins/woocommerce/assets/js/frontend/woocommerce.min.js?ver=11.1.2
- https://www.googletagmanager.com/gtag/js?id=GT-PL9LJ5KB
- https://api.w.org/
- https://aiartimind.com/wp-json/
- https://aiartimind.com/wp-json/wp/v2/posts/8731
- https://aiartimind.com/xmlrpc.php?rsd
- https://aiartimind.com/?p=8731
- https://aiartimind.com/llms.txt
- https://aiartimind.com/wp-content/uploads/2026/09/featured-preview-claude-opus-5-5-motion-graphics-1024x576.png
- https://fonts.googleapis.com/css2?family=Space+Grotesk:wght@300;400;500;600;700&family=Inter:wght@300;400;500;600;700&display=swap
- https://aiartimind.com/wp-content/plugins/visa-acceptance-solutions/includes/../public/css/visa-acceptance-payment-gateway-blocks.css?ver=ef46e0384bee5dc89309a9391d24027b
- https://aiartimind.com/wp-content/themes/salient/css/build/style-non-critical.css?ver=17.0.2
- https://aiartimind.com/wp-content/themes/salient/css/build/third-party/woocommerce/woocommerce-non-critical.css?ver=17.0.2
- https://aiartimind.com/wp-content/themes/salient/css/build/plugins/magnific.css?ver=8.6.0
- https://aiartimind.com/wp-content/themes/salient/css/build/off-canvas/core.css?ver=17.0.2
- https://aiartimind.com/wp-content/themes/salient/css/build/off-canvas/slide-out-right-material.css?ver=17.0.2
- https://aiartimind.com/wp-content/themes/salient/css/build/off-canvas/slide-out-right-hover.css?ver=17.0.2
- https://aiartimind.com/
- https://aiartimind.com
- https://aiartimind.com/ai-art/
- https://aiartimind.com/shop/
- https://aiartimind.com/blog-posts/
- https://aiartimind.com/contact/
- https://aiartimind.com/wp-content/uploads/2026/09/featured-preview-claude-opus-5-5-motion-graphics.png
- https://aiartimind.com/category/ai-tools/ai-coding/
- https://aiartimind.com/category/ai-tools/
- https://aiartimind.com/category/ai-tools/ai-coding/claude-code-video-design/
- https://aiartimind.com/author/hello-artimindgmail-com/
- https://aiartimind.com/how-to-make-motion-graphics-with-claude-opus-5-5/#respond
- https://aiartimind.com/wp-content/uploads/2026/09/opus55-mg-slop-vs-directed.mp4
- https://aiartimind.com/wp-content/uploads/2026/09/opus55-mg-slop-vs-directed-poster.jpg
- https://www.remotion.dev/
- https://github.com/heygen-com/hyperframes
- https://github.com/anthropics/claude-code
- https://aiartimind.com/wp-content/uploads/2026/09/opus55-mg-hyperframes-github.png
- https://aiartimind.com/wp-content/uploads/2026/09/opus55-mg-remotion-agent-skills.png
- https://aiartimind.com/wp-content/uploads/2026/09/opus55-mg-awesome-opus-5-5-videos-github.png
- https://aiartimind.com/wp-content/uploads/2026/09/opus55-mg-pass1-blocking-strip.png
- https://aiartimind.com/wp-content/uploads/2026/09/opus55-mg-pass1-blocking.mp4
- https://aiartimind.com/wp-content/uploads/2026/09/opus55-mg-pass1-blocking-poster.jpg
- https://aiartimind.com/wp-content/uploads/2026/09/opus55-mg-pass2-typography-strip.png
- https://aiartimind.com/wp-content/uploads/2026/09/opus55-mg-pass2-vs-pass3-motion.mp4
- https://aiartimind.com/wp-content/uploads/2026/09/opus55-mg-pass2-vs-pass3-motion-poster.jpg
- https://aiartimind.com/wp-content/uploads/2026/09/opus55-mg-final-motion-reel.mp4
- https://aiartimind.com/wp-content/uploads/2026/09/opus55-mg-final-motion-reel-poster.jpg
- https://aiartimind.com/wp-content/uploads/2026/09/opus55-mg-boundary-frame-contact-sheet.png
- https://aiartimind.com/wp-content/uploads/2026/09/opus55-mg-merge-defect-before-after.png
- https://aiartimind.com/tag/ai-video/
- https://aiartimind.com/tag/claude-code/
- https://aiartimind.com/tag/claude-opus-5-5/
- https://aiartimind.com/tag/motion-graphics/
- https://aiartimind.com/tag/remotion/
- https://aiartimind.com/wp-content/uploads/2026/10/featured-preview-openai-dots-vs-meta-muse.png
- https://aiartimind.com/openai-dots-vs-meta-muse/
- http://www.w3.org/2000/svg
- http://www.w3.org/1999/xlink
- https://aiartimind.com/wp-content/uploads/2026/09/featured-preview-claude-generate-skill-multi-model-image-video.png
- https://aiartimind.com/claude-generate-skill-multi-model-image-video/
- https://aiartimind.com/wan-3-0-guide/
- https://aiartimind.com/wp-content/uploads/2026/10/featured-preview-wan-3-0-600x403.png
- https://aiartimind.com/wp-content/uploads/2026/10/featured-preview-wan-3-0-900x604.png
- https://aiartimind.com/wp-content/uploads/2026/10/featured-preview-wan-3-0-400x269.png
- https://aiartimind.com/category/ai-tools/ai-video-music/
- https://aiartimind.com/category/ai-tools/ai-video-music/ai-video-models/
- https://secure.gravatar.com/avatar/876d6ecd88b307784957842342f79afcc70f30e0b77b959a4e889061b225430f?s=70&#038;d=mm&#038;r=g
- https://secure.gravatar.com/avatar/876d6ecd88b307784957842342f79afcc70f30e0b77b959a4e889061b225430f?s=140&#038;d=mm&#038;r=g
- https://aiartimind.com/claude-skills-best-practices/
- https://aiartimind.com/wp-content/uploads/2026/10/featured-preview-claude-skills-best-practices-600x403.png
- https://aiartimind.com/wp-content/uploads/2026/10/featured-preview-claude-skills-best-practices-900x604.png
- https://aiartimind.com/wp-content/uploads/2026/10/featured-preview-claude-skills-best-practices-400x269.png
- https://aiartimind.com/category/ai-tools/ai-coding/claude-code-skills-mcp/
- https://aiartimind.com/claude-code-motion-graphics-any-style/
- https://aiartimind.com/wp-content/uploads/2026/10/featured-preview-claude-code-motion-graphics-600x403.png
- https://aiartimind.com/wp-content/uploads/2026/10/featured-preview-claude-code-motion-graphics-900x604.png
- https://aiartimind.com/wp-content/uploads/2026/10/featured-preview-claude-code-motion-graphics-400x269.png
- https://aiartimind.com/wp-comments-post.php
- https://aiartimind.com/claude-code/
- https://aiartimind.com/ai-video/
- https://aiartimind.com/ai-agents/
- https://s.pinimg.com/ct/core.js
- https://aiartimind.com/wp-admin/admin-ajax.php
- https://aiartimind.com/wp-content/plugins/salient-social/js/salient-social.js?ver=1.2.5
- https://aiartimind.com/wp-includes/js/dist/dom-ready.min.js?ver=3fe927cab37bf38d6a23
- https://aiartimind.com/wp-content/plugins/astra-sites/inc/lib/onboarding/assets/dist/template-preview/main.js?ver=089983451e6a251a0ab9
- https://aiartimind.com/wp-content/themes/salient/js/build/third-party/jquery.easing.min.js?ver=1.3
- https://aiartimind.com/wp-content/themes/salient/js/build/third-party/jquery.mousewheel.min.js?ver=3.1.13
- https://aiartimind.com/wp-content/themes/salient/js/build/priority.js?ver=17.0.2
- https://aiartimind.com/wp-content/themes/salient/js/build/third-party/transit.min.js?ver=0.9.9
- https://aiartimind.com/wp-content/themes/salient/js/build/third-party/waypoints.js?ver=4.0.2
- https://aiartimind.com/wp-content/plugins/salient-portfolio/js/third-party/imagesLoaded.min.js?ver=4.1.4
- https://aiartimind.com/wp-content/themes/salient/js/build/third-party/hoverintent.min.js?ver=1.9
- https://aiartimind.com/wp-content/themes/salient/js/build/third-party/magnific.js?ver=7.0.1
- https://aiartimind.com/wp-content/themes/salient/js/build/third-party/anime.min.js?ver=4.5.1
- https://aiartimind.com/wp-content/themes/salient/js/build/elements/nectar-animated-gradient.js?ver=17.0.2
- https://aiartimind.com/wp-content/themes/salient/js/build/third-party/stickkit.js?ver=1.0
- https://aiartimind.com/wp-content/themes/salient/js/build/third-party/superfish.js?ver=1.5.8
- https://aiartimind.com/wp-content/themes/salient/js/build/init.js?ver=17.0.2
- https://aiartimind.com/wp-content/plugins/salient-core/js/third-party/touchswipe.min.js?ver=1.0
- https://aiartimind.com/wp-includes/js/comment-reply.min.js?ver=7.1.3
- https://aiartimind.com/wp-content/plugins/woocommerce/assets/js/sourcebuster/sourcebuster.min.js?ver=11.1.2
- https://aiartimind.com/wp-content/plugins/woocommerce/assets/js/frontend/order-attribution.min.js?ver=11.1.2
- https://aiartimind.com/wp-content/plugins/js_composer_salient/assets/js/dist/js_composer_front.min.js?ver=7.8.1
- https://aiartimind.com/wp-content/plugins/woocommerce/assets/js/frontend/cart-fragments.min.js?ver=11.1.2
- https://s.w.org/images/core/emoji/17.0.2/72x72/
- https://s.w.org/images/core/emoji/17.0.2/svg/
- https://aiartimind.com/wp-includes/js/wp-emoji-release.min.js?ver=7.1.3
- https://aiartimind.com/wp-includes/js/wp-emoji-loader.min.js
