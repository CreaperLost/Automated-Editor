# Transcripts — wave 2 reference videos (captions never captured before)

Collected 21 October 2026. YouTube's own auto-generated caption tracks, fetched with `youtube-transcript-api` (falling back to `yt-dlp`'s subtitle extractor) — see [reference-video-scripts/fetch_dossier.py](scripts/fetch_dossier.py). Each paragraph is a merged run of captions; the timestamp is where that paragraph starts.

**On the text itself.** This is each creator's speech as YouTube's speech model heard it. Brand and product names are routinely mangled — Opus appears as *Appus*, HyperFrames as *Hyperframes*, DaVinci Resolve as *Davinchi*. The sense is reliably intact; the spellings are not, so quote the descriptions and links, not these spellings. Bracketed `[music]` markers are the caption track's own.

Videos 1–10 of the original motion-graphics set keep their captions in [transcript.md](transcript.md); this file covers the ten videos that were missing them.

---


## 8. How I make insane motion graphics with Claude Opus 5.5 (my exact workflow)

**—** · — · — · [https://www.youtube.com/watch?v=H6lBZ7AfDDE](https://www.youtube.com/watch?v=H6lBZ7AfDDE)  
_3,246 words · 197 paragraphs · track: en (youtube-transcript-api)_

**`[0:00]`** Claude Opus 5.5 just dropped and it's really good at animations like this.

**`[0:09]`** So in this video, I'm going to teach you exactly how to create animations like this.

**`[0:13]`** I'll show you the exact workflow start to finish that I use to do this and get really

**`[0:17]`** good output. I'm using these a bunch at work right now. So I'm the head of marketing at a biotech startup called Stack Health.

**`[0:23]`** And part of my job is storytelling in science and health.

**`[0:27]`** And these sorts of animated videos are extremely helpful for doing that,

**`[0:31]`** for helping to educate, teach people, get people interested in science and health.

**`[0:36]`** So even though it seems a little bit daunting at first,

**`[0:38]`** I believe that it's a really good skill.

**`[0:41]`** And if you just follow the steps, you can get great output.

**`[0:44]`** That, let's jump into it. So real quick, I want to outline the workflow

**`[0:48]`** and there is five steps, which are these ones here.

**`[0:51]`** So you start with style. This is a reference style, a reference video

**`[0:54]`** that we want to mimic.

**`[0:57]`** Then from there, you decide on the topic that you wanna make a video about. if you don't know, you can literally just ask Claude Opus, so it'll give you a bunch

**`[1:04]`** of good ideas. From there, you generate a little script. Now, you don't necessarily always need

**`[1:08]`** a script, some of the videos that you might want to make aren't like with a script area,

**`[1:12]`** there wouldn't be someone talking, but it is good to think about how a script would

**`[1:16]`** influence what you are animating, if that makes sense. Like, if someone was describing what's

**`[1:21]`** happening on screen, what would, you know, what would they be saying, what would be showing,

**`[1:25]`** basically. Then you iterate. It can sometimes one-shot it. A lot of people on Twitter

**`[1:30]`** are like, oh, I've one-shotted this with like a not even a crazy prompt. It's like,

**`[1:33]`** yeah, I guess, but it will be better if you iterate. So we're gonna do that. And

**`[1:37]`** then render. And with part of rendering, we'll talk about like

**`[1:42]`** adding in sound because that can take it from a pretty good animation

**`[1:47]`** to just beautiful, like just world-class. That's what it looks like. So to dive

**`[1:51]`** into each of these deeper style is the first one finding a reference video where

**`[1:56]`** do you find a reference video well great question there are a few places I

**`[1:59]`** would always start with Twitter right now there are tons of people talking

**`[2:05]`** about Opus 5.5 on Twitter and so you can scroll through literally just like search

**`[2:09]`** Opus 5.5 animations and you'll see a bunch of cool stuff so like this is

**`[2:13]`** one that I really like this guy had this kind of hand-drawn style and then

**`[2:17]`** and it's gonna animate this ball, so it starts moving.

**`[2:20]`** He did, I believe he did this in Blender, is how he got like the 3D effects, so you can do that. We're not gonna cover that, unfortunately, in this video, because that's a little bit different, but everything like up here, we will cover.

**`[2:32]`** So again, just to show you a bunch more, this guy made a video about like Western Civilization.

**`[2:36]`** That's pretty cool. This is all, again, in Claude Opus with JS.

**`[2:41]`** This is another one that I really like, and you were just looking for a style that you find interesting, that you want to mimic.

**`[2:46]`** That's the goal. Now, another place you can find them is YouTube.

**`[2:51]`** Like you, there are certain YouTubers or channels

**`[2:54]`** that have a specific design style.

**`[2:56]`** So you can mimic some of those. If you don't know though, I would literally just ask.

**`[3:01]`** So I'm going to show you guys a prompt that I gave to Claude. This is as I was learning and building stuff out.

**`[3:08]`** As I said, I'm trying to think about animation styles that I can use to create consistent content for Stack Health. So namely animated videos using JavaScript since Opus 5.5 is really good at that.

**`[3:16]`** 3Blue1Brown is one style. A lot of you guys have probably seen those videos.

**`[3:20]`** Kurzgesagt is another, but Opus actually isn't good at mimicking that I tried.

**`[3:24]`** It's like, it's just too complicated.

**`[3:26]`** Kurzgesagt is too good. Data is beautiful is like another channel that does like animated charts and stuff like that. But I said, but there's probably tons more. Can you find and suggest some that I might be able to do?

**`[3:35]`** So then it suggested a bunch.

**`[3:38]`** And one that I really like is Neil Halloran.

**`[3:41]`** And so I'll show you that video. he does these like animated charts to show different data so there's

**`[3:49]`** a bunch more like this so I think this is really cool now if you're on X you can easily

**`[3:52]`** just download a video like if you go down here I guess not all of them but a

**`[3:57]`** lot of them will let you download videos yeah right there but on YouTube I mean

**`[4:02]`** you can also you can use like YouTube to MP4 but what I would do is just

**`[4:05]`** screen record the section that you want because obviously this is like an 18

**`[4:08]`** minute video we can't feed Claude the whole thing or I wouldn't suggest it

**`[4:11]`** But if I just screen record if you do command shift 5 on a Mac

**`[4:15]`** Go down here record the entire screen and then just record the section that you

**`[4:19]`** want real quick Another good place that you can find references is on Pinterest

**`[4:23]`** The thing is Pinterest is much better for images than it is for videos. There's

**`[4:27]`** just like a lot less videos But it can be good and especially if you want to do anything like web based which

**`[4:34]`** I'm going to show in this video There's a really cool video. I made kind of like this. Maybe I've already showed

**`[4:39]`** in the intro I don't know if I'll show it there or not, but you can find a lot of cool website styles and it can do really nice animations of those.

**`[4:46]`** Step two is the topic. So what are you gonna make your video about? I think this one's pretty simple. It's kind of just up to you, but if you're like me and sometimes you're like,

**`[4:53]`** oh, I'd love to make a video, but I don't know what topic to do,

**`[4:56]`** just pick something that you're kind of interested in

**`[4:59]`** or maybe like that your company does or that you're learning about and just ask Claude for ideas about it.

**`[5:04]`** I think that's the best way. Then third is the script or essentially included in the script is the setup of the video, so the outline

**`[5:14]`** of what sorts of animations are going to appear. And I will have Claude generate this together,

**`[5:19]`** but I feel like this sounds kind of vague just talking about it right now, so let's actually

**`[5:23]`** jump into what this looks like. So that style that we just looked at is what I'm going

**`[5:28]`** to be using for this demo. So we're going to go down here, we're going to add files and photos.

**`[5:33]`** I have saved a screen recording you can see of this um this style here

**`[5:37]`** So we're going to attach that and then I'm just gonna talk

**`[5:40]`** So I want to create a really cool animated video using

**`[5:44]`** JavaScript that mimics this style of their the video that I attached for topic.

**`[5:50]`** I'm thinking something about

**`[5:52]`** obesity rates around the world Because a lot of people think like America is the most obese country in the world

**`[5:58]`** It's actually not true. And so I want to kind of show like a comparison of like

**`[6:01]`** all these different countries And then maybe we can talk about a couple of different

**`[6:05]`** like factors and like why this is

**`[6:07]`** or like change over time. I don't know if we really need that, but for right now, I just want basically the start of a video like that.

**`[6:13]`** So think a 15 second teaser that is in this style,

**`[6:16]`** the reference style. But I want you to come up with a shot list for this

**`[6:22]`** or a shot outline of what this is going to look like,

**`[6:26]`** what sort of animations we're gonna have, what sort of data we're gonna have. You will also need to like research the internet

**`[6:31]`** to find the data on on those obesity rate statistics, please do that and then give

**`[6:36]`** me the plan of what what you're visualizing. So it's kind of a long and rambly prompt,

**`[6:41]`** but the reason I always keep those in is because I want you guys to see that the

**`[6:46]`** prompting does not have to be as specific as you think as getting really good at understanding

**`[6:50]`** what you mean. There's some people are like, oh, here's my 12 tips for prompting. It's like, yes,

**`[6:55]`** kind of like this could be improved a little bit, but this this will be good enough. And so I

**`[6:59]`** just want you see the real workflow of what it actually looks like. So now we're just going to

**`[7:03]`** paste that in. Additionally, if you're wondering which level to use, so you should definitely use

**`[7:08]`** Opus 5.5. That's what the whole video is about. But you could use a different effort level.

**`[7:14]`** I would recommend high. You can also use medium. Medium is pretty fine too. Obviously the

**`[7:18]`** ones above will also work. You can use extra high or max. They just use it for credits real quickly,

**`[7:23]`** and I think it is unnecessary for most work. Cool. Now you can see it's working.

**`[7:29]`** This will take a little bit, it'll probably be 10 to 15 minutes. But for the sake of this video,

**`[7:33]`** I've actually already done this, like I've already created the video before. And so I'm just going

**`[7:37]`** to jump over to that and I'll show you guys how it looks. So again, this is a very similar prompt.

**`[7:42]`** It says, I watched the full clip, pulled the latest who data WHO, and then you know, the

**`[7:46]`** context of why this works, that doesn't really matter. It gives me the data gives me a script.

**`[7:50]`** So this is like it's thinking about what would be said, but then here is the shot plan. So this

**`[7:57]`** is the important part to look at. So the assumption is like the camera is tied on one white line

**`[8:01]`** drawing across the smoke, pin pops up 41 in the United States, and framed alone it feels like number

**`[8:06]`** one because it's the only one you see, but then the camera eases out, it starts to rise, longer

**`[8:10]`** line to draw in above the U.S., so you get some of the other countries that are actually higher,

**`[8:15]`** then the upward pan speeds up, camera settles, so this is like it's copying the

**`[8:19]`** style pretty well and I like I like what it's saying so for here I have no notes and then

**`[8:24]`** it's telling me exactly how it'll build it it's got a look with like fonts and

**`[8:28]`** stuff like that that's great export perfect and then I just had some

**`[8:32]`** questions I was asking everybody you can ignore that and then it rendered it so

**`[8:36]`** this whole thing like all of those steps took probably 30 minutes something

**`[8:40]`** like that and then let's look at the finished product and here we go perfect

**`[9:03]`** So I think that's pretty cool, like to me that looks really impressive but it's

**`[9:08]`** not always perfect and one of the things that I don't like is if you're just

**`[9:12]`** seeing this for the first time you don't really have context, you don't

**`[9:14]`** really know what's going on, you're like what are more half of adults, what like

**`[9:18]`** what date are we even looking at. So I would go back and I would say something

**`[9:20]`** like we need a title in there which is actually what I do say here so I'm

**`[9:25]`** thinking at some point we need something like a title so we probably

**`[9:29]`** need something to signify that this is talking about obesity. Again you can

**`[9:33]`** See, I'm just rambling to the AI. So it's white text up there and bigger, but you can't see it right away. It kind of appears like as the camera pulls out.

**`[9:40]`** And so I asked for that. And now I'll look at V2.

**`[9:57]`** And there you see adult obesity. Okay, cool. That's much better. So we skipped a few steps in there, but you saw how we got topic, script,

**`[10:05]`** and then we're at iterating. Now the last part to talk about,

**`[10:09]`** and you might've seen this because it was actually in my previous prompt, is I want to add sound. The best way to add sound is to use ElevenLabs

**`[10:16]`** and they do offer a decent amount of free credits that you can use but

**`[10:21]`** otherwise like you will you will have to pay but it's not too expensive I think

**`[10:26]`** they have like an $8 a month plan and like an $11 or $12 a month plan but the

**`[10:30]`** cool thing is because this is all in code and like you can basically go frame

**`[10:36]`** by frame or the AI can go frame by frame through it ElevenLabs knows the exact

**`[10:40]`** timing of all these things popping up and it can add in sound effects at all

**`[10:45]`** those times. And so I say here, first two things. First, can we use ElevenLabs to add

**`[10:50]`** sound effects to this? There's a ton of motion and a lot of nice sound effects would make this feel

**`[10:54]`** really premium. Think like a professional sound designer or animator. So then it uses ElevenLabs once

**`[10:59]`** it renders that the final product would look like this. So that's

**`[11:06]`** one workflow, but I want to show you guys a couple more just so you get that this is, it's not a one-trick pony, it can

**`[11:11]`** do a lot of different stuff. We're going to go quicker through these because I don't want

**`[11:14]`** to waste your time it's valuable and I think you get it enough to to understand it better. So

**`[11:19]`** the next one is I really like this website design style and I want to do

**`[11:23]`** something with this where it's like a web page and it's kind of animated maybe

**`[11:27]`** a mouse moves over it click something I just I think that would be cool and

**`[11:30]`** recently I was watching the movie Oppenheimer I think that is a great

**`[11:33]`** example of storytelling in science and tech and so I was rewatching it but I

**`[11:39]`** want to do something about like Oppenheimer and so I'll show you this

**`[11:43]`** I gave again this as my reference now

**`[11:45]`** This was just an image, but I described what I wanted to happen

**`[11:48]`** And so I'll walk you through some of the the prompt things

**`[11:51]`** So I want you to create a very short teaser around 10 seconds doesn't actually have

**`[11:55]`** to be a video I basically want to be a website with some motion and movement happening

**`[11:58]`** It can be subtle just some cool effects like that

**`[12:01]`** I'm gonna paste in a website whose style I want you to copy but design it around

**`[12:04]`** Oppenheimer You can be creative with it focus on the man himself rather than the movie although

**`[12:08]`** you can use elements from the movie if want basically treating them as a revolutionary scientist don't use

**`[12:12]`** actual photos use a stylized design this is key or maybe a designer like the

**`[12:16]`** atom bomb or something like that so duration format this one I wanted to be

**`[12:19]`** vertical like for real at work we use a lot of real so it's just good to do

**`[12:23]`** stuff like that and then I wanted to have an interactive feel like you could

**`[12:27]`** have similar simulated cursor moving around and clicking one or two things

**`[12:30]`** make it look super premium similar to the reference website so we send that it

**`[12:35]`** did ask me a couple questions because I said ask questions if needed which

**`[12:39]`** which I always love to add to prompts and just so you can nail stuff on the first try because especially with things like this, it takes a long time to render. So if you wait like 20 minutes and then it missed one thing because you didn't give good instructions and you're like, I have to wait 20 more minutes

**`[12:53]`** for another one of these renders when I just needed you to change one tiny thing.

**`[12:57]`** So add that in where possible.

**`[12:59]`** Now it created it.

**`[13:01]`** So I'm gonna open in the mouse moves. That's not my mouse to be clear. My mouse is over here. So I think this is sweet. Like I know there's like elements that look like a little bit like AI and all of that but like look at like this stuff down here I think that's so

**`[13:13]`** cool but again this doesn't have sound so I just asked hey can we add sound

**`[13:17]`** effects to it with ElevenLabs keep it pretty minimal and subtle tech launch

**`[13:21]`** vibes no heavy risers or drops make it feel alive I don't know if that even

**`[13:24]`** makes sense but that's what I said and then I'll show you guys the the finished

**`[13:28]`** product cool and

**`[13:39]`** then the last one I want to show you guys is this hand drawn

**`[13:42]`** style that I had shown you before so from this catapult here I really

**`[13:56]`** like this look of like the things being highlighted in yellow it kind of has

**`[14:00]`** this hand-drawn paper style all of that I think that's really cool so that's

**`[14:04]`** what I wanted to replicate and so I gave that in as a reference image or

**`[14:08]`** reference video in this chat and then I said hey can you basically make me some

**`[14:13]`** videos that explain like a couple interesting things in in the world of

**`[14:16]`** science and specifically peptides since we also do that at Stack Health so this is

**`[14:21]`** what it came up with. Ever wonder why you can't just take your peptides as a

**`[14:42]`** So if you're like me, that's pretty cool and pretty engaging.

**`[14:47]`** I did use ElevenLabs for the voice here as well as sound effects and stuff like that.

**`[14:51]`** So that's kind of what you need to know. Now, briefly, let's talk about what it's good at and what it is not good at.

**`[14:57]`** Good at motion graphics, charts and data, explainers, UI product animations, yes.

**`[15:02]`** Simple illustrated scenes, people are doing pretty cool simple animation to tell

**`[15:07]`** little scenes like a Pixar story, essentially.

**`[15:10]`** It's not good at real footage, that's not the point of this.

**`[15:13]`** Detailed character acting, not super good.

**`[15:18]`** Long pieces in one shot, I would always recommend building it scene by scene instead.

**`[15:21]`** It people have done like pretty good examples with this but it's complicated and

**`[15:26]`** they also will never tell you the prompts So it's it's hard to replicate and then heavy 3d and like particle stuff. It kind

**`[15:32]`** of just doesn't look Nice. It just doesn't look nice and also judging its own taste

**`[15:38]`** It's yeah, it can't really tell if stuff is super good or not. So that's why you

**`[15:41]`** need the human in the loop So that's the workflow style topic script iterate render if this was useful, please

**`[15:48]`** subscribe. I'll see you next time

---


## 11. Did AI Change Editing Forever? (GPT-6 Astra + DaVinci Resolve)

**Higgsfield AI** · 2 Oct 2026 · 11:36 · [https://www.youtube.com/watch?v=kOoC3yhUyDQ](https://www.youtube.com/watch?v=kOoC3yhUyDQ)  
_1,801 words · 132 paragraphs · track: en (youtube-transcript-api)_

**`[0:00]`** AI can edit videos now. Editing has changed.

**`[0:05]`** Editing has changed.

**`[0:08]`** EDITING HAS CHANGED.

**`[0:13]`** Editing has changed.

**`[0:17]`** [snorts] Yes, it has. Now Astra does the editing

**`[0:20]`** for you. What does that mean? Editing, [music] color grading, motion graphics, sound

**`[0:25]`** design. Astra can handle all of it now.

**`[0:27]`** Hi, I'm [music] Adil and today I'm putting Astra to the ultimate test. We'll create a cinema grade commercial from scratch, rebuild a full YouTube tutorial, and turn its [music] best moments into shorts you'd actually stop

**`[0:38]`** scrolling for. And for the grand finale, I'm going to give Astra our most viral film on Eric, asking [music] for a trailer that literally gives you chills. I want to see how much editing work [music] I can hand over. I'm tracking two things. How long the whole process takes, and how much time I actually spend [music] on prompts, reviews, and final tweaks. Let's make THAT AD.

**`[0:57]`** EDITING HAS CHANGED.

**`[1:03]`** Everyone in this commercial hears the same three words. Editing [music] has

**`[1:07]`** changed. Each person takes it a little

**`[1:09]`** worse than the last. I'm [music] going to give Astro the script. The job is to

**`[1:13]`** generate the characters and scenes in Hixel, then build the commercial in

**`[1:17]`** Resolve. [music] First, we'll need to connect Hixel to Chad GPT. Pause here for instruction or

**`[1:22]`** check the link below. [music]

**`[1:24]`** Next, computer use.

**`[1:27]`** This lets Astra open and operate Resolve

**`[1:30]`** on my Mac. And a quick note before enabling computer use, Astra can make real changes in your apps, [music] and mistakes can happen. Back up your projects, review permissions, and keep

**`[1:39]`** sensitive apps closed. Now, for the

**`[1:41]`** actual edit, I'm going to describe the characters, dialogues, [music] camera moves, and sound cues in as much

**`[1:47]`** detail as I can. Every reaction needs to

**`[1:50]`** get bigger until the final shot brings everything back to [music] the cafe. Let's see what Astra does with it. Astra

**`[1:56]`** is using Kixel's built-in skills to [music] turn the brief into generation prompts. It starts with the characters and locations. And with all the assets ready, Astra moves on to video generation [music] and opens Resolve for the edit. For the screen in the cafe scene, I supplied a screenshot of the Da Vinci [music] timeline. Astra wrote the generation prompt and Hixel generated

**`[2:14]`** the shots. Now, in Resolve, Astra

**`[2:16]`** [music] made the first draft. Let's watch it.

**`[2:20]`** Editing has changed.

**`[2:23]`** Editing has changed.

**`[2:26]`** Editing has changed.

**`[2:33]`** [music] Editing has changed.

**`[2:37]`** [screaming]

**`[2:40]`** Editing has changed.

**`[2:43]`** [screaming]

**`[2:45]`** What happened? Editing has changed.

**`[2:51]`** Editing has changed.

**`[2:55]`** Editing has

**`[2:59]`** changed.

**`[3:02]`** Yes, it has. Now Astra does the editing

**`[3:06]`** for you.

**`[3:09]`** Okay. There's one thing I want to change, though. In the first FBI take, [music] the agent's on the wrong side of the desk. It happens. I'll just submit a note to Astra. And while Astra [music] works on the change, let's check out the sound. Astra pulls a dialogue, grabs

**`[3:22]`** track music from my drive, layers sound effects from our library, and organizes

**`[3:26]`** everything onto separate clean tracks, [music] and the sound is pretty good. Now, turn

**`[3:31]`** up your speakers and listen [music] to this scream in the scene with the mafia bombs. [screaming]

**`[3:37]`** What happened? It carries straight into

**`[3:39]`** the headphones of the FBI agent [music] and stops the exact moment he takes them off. The cuts work, but I think I want it a little faster. So, for the pacing, I decided [music] to make the final changes myself. Look at these tiny gaps

**`[3:51]`** between reactions. I'll get rid of them real quick. I made one pass myself in

**`[3:55]`** Resolve, trimming those micropauses from the [music] edit Astro Build. Astro's version was about 49 seconds, minus 37.

**`[4:03]`** Same shots in sequence, just stripped of

**`[4:05]`** all that air.

**`[4:08]`** Editing has changed.

**`[4:11]`** EDITING HAS CHANGED.

**`[4:13]`** EDITING HAS CHANGED.

**`[4:19]`** Editing has changed.

**`[4:22]`** EDITING HAS CHANGED.

**`[4:26]`** What happened? Editing has changed.

**`[4:30]`** Editing has changed.

**`[4:33]`** Editing [music] has changed.

**`[4:38]`** Yes, it has. Now Astra does the editing

**`[4:41]`** for you. I spent 30 minutes actually working [music] on this, including generation time and rendering. The whole thing took about 2 hours. And now, let me show you

**`[4:51]`** something way bigger. Dumping [music] 3

**`[4:53]`** days worth of raw files from a 20-minute

**`[4:56]`** YouTube tutorial directly into Astro.

**`[4:58]`** And it's handling [music] the sound design, motion graphics, and color

**`[5:02]`** grading, too. Let's see how it handles

**`[5:05]`** the pressure. Look at all these files. The original episode took 3 days to edit. I'm giving Astra the same material. Everything is on this SSD. I'm

**`[5:14]`** also attaching the script and our custom YouTube editor skill right here. Let's

**`[5:18]`** start the clock. Here's the secret to making this work. [music] The context you give it. If you want the output to match your personal style, you have to feed it references. I handed over our past After Effects projects, fonts, [music] and audio packs. The script

**`[5:32]`** tells it what we're explaining. Our YouTube editor skill tells it how we

**`[5:36]`** actually work. I'm dropping that exact skill file and our sound library in the description below so you can use it too. Here are the folders Astra is creating.

**`[5:45]`** And now the timeline is filling up.

**`[5:48]`** Okay, that is surprisingly accurate for

**`[5:50]`** first pass. A few cuts dropped in slightly [music] offbeat, so I sent Astra a quick list of precise time codes to adjust. Here's that exact same sequence after Astra executed those

**`[6:00]`** fixes. Now that's much smoother. Moving

**`[6:03]`** on. Next up, matching our signature

**`[6:05]`** color look. Astro gives each adjustment

**`[6:08]`** its own node, and those nodes are saved with the project. So, if I need bigger changes later, I can just jump in and tweak them [music] myself. It nailed the contrast and my skin tone. Switch

**`[6:18]`** angles, and the color still matches. I

**`[6:21]`** think I'm pretty hyped with how that looks. Now, [music] for the screen recordings and assets, these have to lock in seamlessly with a voice over. Watch here. I name the setting, and the

**`[6:30]`** matching action appears on screen. The

**`[6:32]`** audio is still in sync. All the supporting mural is landing right where it belongs. I already have these screen recordings on the drive, but a quick tip, if you need them, you can also ask Astra to record the demonstrations first. Just give it the project and ask us for that step. Now, for the hardest part of the whole build, motion graphics. We're covering just the basics here. For a full breakdown, check out this video on our channel. Astro has

**`[6:54]`** access to our past projects and the exact timing of the edit. Let's see what

**`[6:58]`** it builds from scratch. [music] It's writing and executing a custom script directly inside After Effects to build

**`[7:04]`** the entire graphic. Here's the first

**`[7:07]`** render. [music] Visually, the style is right on point. And watch what Astron

**`[7:11]`** does right after rendering. It automatically exports the animation with an alpha channel or a transparent background, brings it into Resolve, and

**`[7:19]`** places it over the footage. And there it is, built, rendered, and composited

**`[7:24]`** without me touching a single key frame.

**`[7:26]`** So, we started with a bunch of loose files on an SSD, and now we have a Final

**`[7:30]`** Cut and an editable Resolve project file. I'd say Astro got about 80% of it

**`[7:35]`** right. A few things still aren't quite how I wanted them, so I'll make those final tweaks myself. [music] In Resolve, my hands-on time so far comes to about

**`[7:43]`** 20 minutes. Getting to this version [music] took about 2 hours in total,

**`[7:47]`** including processing and rendering. But we're not done yet. There's way more value we can squeeze out of this exact timeline.

**`[7:57]`** Moments like this are scroll [music] stoppers and so is this one. Let's pull

**`[8:01]`** these out and turn them into high converting social clips. So short clips

**`[8:06]`** are the ultimate hook for reaching new people. One good clip can make them want

**`[8:10]`** to watch the full episode. Astra already

**`[8:13]`** has all of our raw footage and the script. Okay, let's ask Astra to make us viral short video using that scene from the release with a crazy camera move.

**`[8:21]`** That's our request. Let's get it started. And you could scale this instantly. Ask for 10, ask for 20, then

**`[8:27]`** just review the full badge. Every single

**`[8:30]`** camera move landed exactly where I set it. Same workflow. I'm just asking Claude for the Moodle. Like I'm talking to a real GOP. Three scenes, one [music]

**`[8:38]`** continuous orbit.

**`[8:43]`** [music]

**`[8:46]`** Now, let's take things a step further.

**`[8:48]`** I'm going to turn a full film into

**`[8:50]`** 60-second cinematic [music] trailer. This is Oneric, our most viral film with

**`[8:55]`** over 2 million views on YouTube. Yes,

**`[8:57]`** I'm giving Astro the full film, and [music] my brief is to catch your attention immediately, build tension,

**`[9:03]`** and leave you wanting to see what happened next. Okay, the overall structure works, but there are three things I want to change. First, this

**`[9:10]`** moment doesn't carry through the cut.

**`[9:13]`** Then, these three shots pass way too

**`[9:15]`** [music] quickly. I'd also hold each shot a little longer so the action reads clearly. And now listen closely to this

**`[9:22]`** audio cut to spend one more day

**`[9:26]`** with you. Hear that pop? Terrible. I know. I'm

**`[9:31]`** sending those notes together. Fix the movement across the cuts. Hold those shots longer and clean up the audio transition. Oh yeah, I'm pretty happy with that. The whole thing flows much better now. Let's check out the full trailer.

**`[9:46]`** If you could go literally anywhere, [music] any place, any world,

**`[9:51]`** somewhere with no age, no war, no

**`[9:54]`** sickness, [music]

**`[10:07]`** [music]

**`[10:14]`** [music]

**`[10:18]`** I think I just asked them [music] to send me to today

**`[10:22]`** right here to spend one more day

**`[10:27]`** with you. I got you, brother. All good. Come on.

**`[10:33]`** What's he seeing [music] in there?

**`[10:41]`** And there we go. The trailer done. Now,

**`[10:43]`** remember that other video I said Astro edited? You've seen Astro rebuilt an

**`[10:48]`** entire YouTube episode from scratch. Now, look at this timeline. This exact

**`[10:52]`** video you're watching right now, Astro edited [music] it, too. Even the part where I was literally explaining how it works. We've come a full circle. The

**`[11:01]`** biggest thing I'm learning is how much the input matters. Give [music] Astra the script, source files, references,

**`[11:07]`** examples of the style you want. And one, let's look at the numbers. [music] You can give Astra all the repetitive tasks, which gives you more time to take on bigger projects, grow your business, and

**`[11:17]`** [music] finally create something of your own. The skills in the SFX library I'm

**`[11:20]`** using are linked below, so you can try this on your own footage. And as always,

**`[11:25]`** if you found this video helpful, hit the like button, subscribe, and I'll see you guys in the next one.

---


## 12. How To Make Motion Graphics With AI (After Effects Full Tutorial)

**Rourke** · 28 Sep 2026 · 15:53 · [https://www.youtube.com/watch?v=mcpAbTHFIRE](https://www.youtube.com/watch?v=mcpAbTHFIRE)  
_3,155 words · 192 paragraphs · track: en (youtube-transcript-api)_

**`[0:00]`** In today's video, I'm going to teach you how you can use AI to create personalized motion graphics for your videos, where [music] every layer is

**`[0:07]`** editable inside of After Effects. We'll

**`[0:10]`** even go through every single cost breakdown, [music] so we leave nothing unturned. And before we get into this, no, motion designers are [music] not cooked. In today's video, we'll show you how you can create commercial grade motion graphics, SaaS animations, and

**`[0:24]`** YouTube graphics for your videos. So, step one for you to go and create these motion [music] graphics for yourself, you need to install the ChatGPT desktop

**`[0:32]`** app. And I'll leave a link to it in the description, so you can go and check that out. Here, you can see there is a download for Mac option, which you can simply click on here, and you'll see right at the bottom there is an option for Windows, as well.

**`[0:43]`** Now, when you're in here, we want to go and click on the installer, and we're just going to want to install this by

**`[0:49]`** double-clicking. So, now you're in ChatGPT, you want to make sure at the top we're on the work section. And then,

**`[0:54]`** we want to click on the drop-down down here, and make sure you have GPT Astra

**`[0:59]`** selected as your model. There'll be a couple in here, make sure we have Astra, which is the new model, selected. And you can see here we've got medium, high, extra high, and this is going to be the highest usage of your credits. I'm in no

**`[1:11]`** rush, so I'm just going to leave it on medium. So, how much is this going to cost you? It's important that we talk about the cost implications before we dive into this, so you know whether or not this is for you. And you've got a few options. One is the $20 a month

**`[1:24]`** plan, and the other one is the $100 a

**`[1:27]`** month plan. For every project that you

**`[1:29]`** ask it to do, usually it takes about 10

**`[1:32]`** minutes to execute any degree of motion

**`[1:34]`** graphics. Now, 10 minutes of ChatGPT

**`[1:38]`** usually is around 2% of your usage, if

**`[1:41]`** you pay for the $100 a month subscription. So, we just want to be transparent around that.

**`[1:47]`** Next up, we need to open After Effects.

**`[1:50]`** Start a brand new project, new composition, and you can keep the aspect ratio. Let's say we're going going a classic 1920

**`[1:57]`** by 1080 and we've now got our composition in here. So with this blank composition, this is the perfect starting block that we need in order for Astra to start doing some work. And just a reminder that every single one of these motion graphics examples that we're showing you today started off with a blank canvas inside

**`[2:15]`** After Effects. So stay tuned and we'll show you the entire process. Now in order to connect Chat GPT Astra to After

**`[2:22]`** Effects, we have to do one step. So we

**`[2:24]`** can go into plugins here and then we can

**`[2:26]`** search for computer.

**`[2:29]`** And you're going to want to go into computer use. This is basically going to

**`[2:33]`** allow Chat GPT to control your computer.

**`[2:36]`** So we're going to flick on this enable

**`[2:39]`** MCP button down here. Now on your

**`[2:41]`** desktop system, you're going to need to give Codex permission to use screen

**`[2:46]`** record and accessibility. So in order to

**`[2:49]`** do that, we need to go and search for

**`[2:52]`** screen record. Go into privacy and security.

**`[2:56]`** Now in here, we want to go into accessibility and you can see here we

**`[3:01]`** have Codex computer use. You want to

**`[3:04]`** make sure this is enabled and then you can enter in your password and this will allow Codex, aka Chat GPT, to have full

**`[3:11]`** control over your After Effects projects. Now step three is that we need to prepare the scene. If you have existing footage that you want to give to After Effects, you can go ahead and upload it

**`[3:22]`** just like we do in here. Or alternatively, if you're building a project from scratch and you want it to do custom motion design, then you can

**`[3:30]`** leave a project blank like we have right

**`[3:32]`** here. The best motion graphics artists

**`[3:35]`** are going to be the ones who can articulate exactly what they want. So

**`[3:39]`** here is our first example and this is a HSBC advert that we created using AI

**`[3:45]`** from scratch. And you can see here the before and after where Astra has added

**`[3:49]`** in these motion graphics onto the screen for us and tracked it in in one single

**`[3:54]`** generation. It took [music] about 10 minutes to do, which is about 2% of our ChatGPT usage. It added in the text that

**`[4:00]`** you can see at the end here, and it also added in the logo. But what's really

**`[4:04]`** interesting is that it did add in all of

**`[4:06]`** these awesome motion graphics and track it onto our screen for us, which is

**`[4:11]`** something that would have been quite fidgety to do in the past with camera tracking, but now it can do all of this for us without us even to need to lift a

**`[4:18]`** finger. So this is one simple use case

**`[4:21]`** that you can use it for. Onto the next one. So everything that you can see here is entirely edited by Astra. All of this

**`[4:27]`** website and SaaS product that is our own

**`[4:30]`** SaaS product, it has gone through and animated everything that we want, and this is the first attempt that it did.

**`[4:36]`** Now it's not bad, but we can improve this a lot, and a traditional motion graphics artist could do a much better

**`[4:42]`** job of this. However, I just want to show you this is what our actual website

**`[4:47]`** looks like, and all we had to do to get these results was take a couple of screenshots. So here you can see we added in four simple screenshots of the

**`[4:56]`** user interface of our website, and we gave ChatGPT a prompt like this. Using

**`[5:00]`** the screenshots provided, I would like you to build a 15-second SaaS animation

**`[5:04]`** promo video for this website. Use the

**`[5:06]`** current After Effects composition I currently have open, and all we did was open a blank one. We should start with the GenHQ logo, which is the name of our company, in the center of the black frame. Then have the mouse cursor click

**`[5:19]`** on the logo. It should expand into the website. Smooth key framing, editable layers,

**`[5:25]`** motion blur. A prompt as simple as that

**`[5:28]`** outputted these results. And honestly,

**`[5:31]`** it's super impressive what it was able to create. The fact that it can click into these, it has all the context, all

**`[5:38]`** the correct fonts, the positioning of of

**`[5:40]`** the text here could do with a bit of work, but overall, it's a really great

**`[5:44]`** starting block. And we didn't even give it a reference of our actual company logo, which you can do and it will then insert that into the projects. Cuz right now, it's like kind of off on the logo.

**`[5:54]`** It's not 100% correct, but you can just

**`[5:57]`** upload your own references in there and it will completely custom animate anything you want. So, the first option

**`[6:03]`** you have is to give screenshots. Describe the animation style that you want and Astra can go in and do that for

**`[6:09]`** you. The second option that you have is

**`[6:12]`** to upload a video reference. So, there's

**`[6:14]`** a couple of different options here when it comes to videos. So, one is to take a

**`[6:18]`** quick screen recording, just like this, where we go through the website and we click through, so it has all the context of what it needs. And you can see here I'm clicking the dials, dragging them along, so it has full understanding of the responsiveness of the site. And when we click search, it then goes to this page here. Then, when we click on something, it takes us to another page and then we click on apply now. And I

**`[6:40]`** want to create an animation that is

**`[6:42]`** smooth transitions between each of those, going through the user journey on

**`[6:46]`** a website. Now, take a look at this. So,

**`[6:49]`** this is our animation. We've already got a satisfying opening here and you can

**`[6:53]`** see here that it's clicking through the same things that I did, using the slider scales, searching. We've got smooth keyframes and animations in here, loading in all the pages just as we

**`[7:03]`** want, with a clean ending, smooth

**`[7:06]`** transitions between whatever we asked for. This took about 10 minutes, which would have taken me a very long time.

**`[7:12]`** And what's interesting about this is you can also give it references of your

**`[7:16]`** Figma files, if you've designed specific

**`[7:19]`** elements that you want to show up in your animations. And you can simply describe the effects that you want and

**`[7:25]`** it will go and do it for you. It is

**`[7:27]`** mind-blowingly impressive. Before we move on, I will show you the prompt that I used for this video, which is, "I want

**`[7:34]`** you to build me a 15-second SaaS animated promo video for the new GenHQ

**`[7:39]`** marketplace platform. Build this inside

**`[7:41]`** of After Effects in the composition I currently have open. I want you to recreate the flow that is shown in the screen recording provided. So, we're telling it the screen recording isn't there. We're then instructing that we want you to animate the smooth process

**`[7:54]`** that the user will do on the website for searching for a job. Each element of the

**`[7:59]`** website should have its own animation, and we should have smooth zooms that follow the mouse around the screen. Keeping whatever the viewer should be looking at in the dead center of the frame. That's really key, because as a motion graphics artist, we know that you

**`[8:13]`** typically want to keep the viewer's attention in the center of the frame.

**`[8:17]`** And what you can also add is, say, add

**`[8:19]`** subtle motion blur to the video, making

**`[8:21]`** sure that there's always the same motion in the animation. The other video I have

**`[8:25]`** provided can be used as a reference to

**`[8:28]`** look at the movement of the animation.

**`[8:30]`** So, this is the reference video that we provided to Astra to say, "This is the

**`[8:34]`** style of motion graphic that we want to have in our video." So, we had created

**`[8:39]`** this already, and this took us about 2 weeks of motion graphics work to create

**`[8:42]`** a product of this caliber. Now,

**`[8:45]`** obviously, we have an a motion graphics

**`[8:47]`** artist who has plenty of experience creating all of these transitions, and he built everything in Figma in here to create all of these awesome effects. And

**`[8:55]`** this is for our AI platform that allows you to hire AI talent, whether you are a

**`[9:00]`** motion graphics artist, whether you are a video editor, or if you have AI image and video skills. We built this platform

**`[9:07]`** to enable people to go and get full-time

**`[9:09]`** jobs and find high-paying positions,

**`[9:12]`** salaries, and also just marketplace freelance work. So, it's a centralized place for people with AI skills. And this was the reference video that we'd created. What's also interesting is we actually fed this as a video to Astra,

**`[9:24]`** and we asked it to recreate it. Now,

**`[9:27]`** take a look at this. This is it recreating those animations itself

**`[9:31]`** totally from scratch. So, here is the

**`[9:34]`** side-by-side comparison, so you can see just how insane this is. It's obviously

**`[9:38]`** not as good as the original. However,

**`[9:41]`** this is the first output that we've got,

**`[9:43]`** you know, without any tweaks to it. Now, you may be saying to yourself, this is concerning, and I totally agree to you, because you're probably thinking, well, now people can copy my motion graphics work totally for free. And truth be told

**`[9:56]`** is that people could already do that anyway if they have the skills. You but it's never going to be as good as the original. Now, the reason I wanted to show you this is not so you can go out and copy. It's so that you can go out and find specific references of styles

**`[10:09]`** of motion graphics that you like, so that you can guide that as a style reference. We're not trying to copy and steal, we're just trying to use references to help us get better results. And in our case here, we used

**`[10:20]`** our own work as a reference to see

**`[10:23]`** whether or not we could guide it to get us similar results, and it ended up doing a pretty good job. So, now you have two options. One is that you can go and design your own custom UI elements

**`[10:32]`** directly inside of Chat GPT by simply

**`[10:35]`** describing it and prompting to get the exact UI components that you want. For

**`[10:40]`** example, I have this one copied out here, and I'm just going to say, I want you to go away and create create a green

**`[10:48]`** screen background with this prompt and

**`[10:51]`** use Chat GPT's image model to design

**`[10:55]`** these motion graphic UI. And now it's

**`[10:59]`** going to go away and use Chat GPT's image model on a green screen background, cuz then you're going to get a much cleaner result in After Effects.

**`[11:06]`** But it's then going to go away and redesign whatever it is that you've described for your UI element directly

**`[11:12]`** inside Chat GPT, so you don't even need

**`[11:14]`** to pay for any other aggregator platform. You can also go and find your

**`[11:19]`** own custom UI directly on Pinterest as a

**`[11:21]`** reference as well. So, here is one

**`[11:24]`** example, and then here is what Chat GPT

**`[11:26]`** has created for us. And you can see here it's even put it on a PNG background,

**`[11:31]`** because Chat GPT can actually create

**`[11:33]`** PNGs. So, you can either use a green

**`[11:36]`** screen background or just ask it to

**`[11:38]`** create it as a PNG format. It's given us

**`[11:40]`** both options. Using the After Effects

**`[11:44]`** composition that I have open, I would

**`[11:47]`** like you to animate this prompt bar with

**`[11:51]`** a slick UI animation. And now we want to

**`[11:54]`** go in and we need to describe what those

**`[11:56]`** slick animations are. And if you work in

**`[11:58]`** motion graphics, you're going to have that key understanding of all the rich

**`[12:02]`** terminology that you want in order to

**`[12:05]`** actually achieve the results that you have in your head. So, we can go in and

**`[12:09]`** basically prompt whatever we need for these effects. So, the way that I'm going to do it is I'm going to order it. So, I'm going to order it things from like one and I'm going to just like kind of go through all the steps of the animation. So, the first thing that I

**`[12:22]`** would like to happen is a cursor to come

**`[12:24]`** in, click on the prompt bar, and write into the prompt bar saying, "Subscribe

**`[12:29]`** to the Rock Heath AI education page."

**`[12:32]`** The cursor then clicks onto the deep

**`[12:36]`** think button and opens up all the

**`[12:38]`** different options. Every element is

**`[12:40]`** animated when the mouse hovers over it

**`[12:43]`** with a satisfying animation. The cursor

**`[12:46]`** then clicks on deep think, colored

**`[12:48]`** splats come out in a nice modern UI.

**`[12:52]`** I just have to go in and upload the reference of the transition that I want it to go into. And just so you have context of the animation, this is the style of animation that I would like it to go into, which is a previous example of our own animations that we've done.

**`[13:05]`** So, now we can go ahead and start

**`[13:08]`** running that. You see down here it says, "Allow Chat GPT to use After Effects."

**`[13:12]`** You can click on always allow and just

**`[13:14]`** click allow in here and make sure you allow everything that it requests. So,

**`[13:19]`** what you can see right now is I'm actually not touching my screen. Astra

**`[13:23]`** is now completely editing everything and

**`[13:26]`** you can see it's added in this second cursor in here.

**`[13:32]`** So, now ChatGPT has completed the animation, let's take a look. You can just click on this here, a watch 23-second animation, and it will show it to you in ChatGPT, which is just crazy.

**`[13:42]`** So, here we can press play, and you can see we have everything that we have

**`[13:46]`** asked for. And you can go into more detail and be descriptive about the changes that you want as well now. But, it has literally edited everything that

**`[13:54]`** we asked for. It is color changed

**`[13:56]`** everything, kept the animation the same,

**`[13:58]`** changed everything in our original animation. It has done everything that

**`[14:02]`** we asked for to insanely high spec. So,

**`[14:06]`** this is a very, very interesting, powerful tool, loads of really great

**`[14:11]`** stuff that we can use this [music] for.

**`[14:15]`** I'm going to leave in the description some template prompts, so you can use this as your guide to start actually

**`[14:21]`** creating great motion graphics directly

**`[14:24]`** inside of ChatGPT. So, if you are a

**`[14:26]`** motion graphics artist, or if you have AI image and video skills, or maybe even just a video editor who's interested in working in the AI space, we built this

**`[14:35]`** platform for you. So, you can go on here

**`[14:38]`** and find full-time roles and positions,

**`[14:40]`** so you can get hired and get paid premium salaries. And we've also built a marketplace, so you can do freelance work. So, if you're interested in building your career as a creative in the AI space, then you can join Gen HQ.

**`[14:51]`** It'll be the first link in the description, and it's totally free to sign up. [music]

**`[14:57]`** What are my actual takes on this new AI release? Where I think that this plays into a very effective tool for creatives

**`[15:03]`** is if you already have motion graphics experience and you're deep into that space, you can use this to absolutely

**`[15:09]`** supercharge your work. You can use this to turn over projects significantly quicker, and definitely make a lot more

**`[15:16]`** money in the process. Now, yes, it is a

**`[15:18]`** great place to build a starting block, but I don't believe it produces anything

**`[15:22]`** that is to a standard that professional

**`[15:25]`** motion graphics artists would be happy

**`[15:27]`** with. The creative space requires us to constantly be evolving at this point in time, and I think that rings true now

**`[15:33]`** more than ever. So, whether you're a freelancer or looking for a full-time position in the AI space, you can click

**`[15:39]`** the link in the description to join Gen HQ totally for free and build your portfolio. We try to be very honest with

**`[15:45]`** our takes here about this technology cuz I think it's important to approach it from both sides. [music] So, I'd love to hear your opinions down in the comments, and I'll see you guys in the next one.

---
