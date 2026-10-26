# I Replaced 8 Hours of Video Editing with This (Open Source)

- **Source:** https://www.youtube.com/watch?v=dXz1qkLc9PQ
- **Kind:** youtube
- **Captured:** 2026-10-25T16:50:04
- **Evidence grade:** vendor or creator claim. Not tested in this repo. Quotes from a caption are what a creator *said*, not a measurement.

## Metadata

- **Title:** I Replaced 8 Hours of Video Editing with This (Open Source)
- **Channel:** Sirio
- **Published:** 21 10 2026
- **Runtime:** 20:17
- **Views:** 19036
- **Likes:** 411

## Description (verbatim)

```
Join the Tein Waitlist (free), the cheapest official AI models, open source workflows and more: https://tein.ai

Join PublicAI Community (free): https://www.skool.com/publicai

This entire video was edited by my own AI video editing system, from the teleprompter where I record to the final export with motion graphics. I walk you through all seven steps: recording on my teleprompter, cutting the takes with Claude Code, planning and designing the boards with Claude Design, animating them to my voice and rendering the final video. A 30-minute video now takes me about one hour from start to finish, where a human editor needs six to eight hours, and I break down what that changes in time, money and role for creators and editors. The whole system is free on GitHub, so by the end you can run it on your next video.

You will learn⤵️

1. How a teleprompter can record the data an AI editor needs
2. How Claude Code picks the best take for every line and cuts the video
3. How Whisper checks and cleans the cut word by word
4. How to plan boards a viewer understands just by listening
5. How to design the boards from your own design system with Claude Design
6. How to animate every board so it lands on the word you say
7. How to render the final video and write the description automatically
8. What this changes in time, money and role for creators and editors

This workflow⤵️
The AI video editing system in this video (free): https://github.com/sirioberati/YouTube-AI-Video-Editing-System


Free resources⤵️
PublicAI Community: https://www.skool.com/publicai
Tein, the cheapest official AI models, open source workflows and more: https://tein.ai

My other open source workflows⤵️
Genjutsu open source workflow: https://github.com/sirioberati/Genjustsu-Open-Source-Workflow
Emotion control workflow: https://github.com/sirioberati/Emotion-Control-Open-Source-Workflow

Connect with me on:
https://instagram.com/heysirio
https://instagram.com/saysirio

Chapters
Intro
00:00 – 00:28 My AI Editing System
00:28 – 00:56 The Free Package
00:56 – 01:31 AI vs Human Editor
01:31 – 02:14 Who I Am
02:14 – 02:59 What We'll Cover
02:59 – 04:48 How It Started
The 7 Steps
04:48 – 07:47 Step 1: Record
07:47 – 10:09 Step 2: Cut
10:09 – 11:04 Step 3: Plan Boards
11:04 – 11:48 Step 4: Design Boards
11:48 – 12:24 Step 5: Animate
12:24 – 13:01 Step 6: Render
13:01 – 13:11 Step 7: Description
What Changes for You
13:11 – 14:08 The Math
14:08 – 14:47 My Own Editor
14:47 – 15:30 Shift 1: Time
15:30 – 17:08 Shift 2: Money
17:08 – 18:24 Shift 3: Role
18:24 – 19:04 Don't Wait
19:04 – 19:36 How to Run It
19:36 – 20:09 Join PublicAI
```

## Links from the description

- https://tein.ai
- https://www.skool.com/publicai
- https://github.com/sirioberati/YouTube-AI-Video-Editing-System
- https://github.com/sirioberati/Genjustsu-Open-Source-Workflow
- https://github.com/sirioberati/Emotion-Control-Open-Source-Workflow
- https://instagram.com/heysirio
- https://instagram.com/saysirio

## Captions

573 snippets. ASR output; brand names are mangled.

- [00:00] This entire pipeline that you're seeing
- [00:01] on screen is my AI video editing system.
- [00:03] 90% of it is handled by AI and I built
- [00:07] it from scratch for me and my editor so
- [00:09] we can scale our content production,
- [00:10] which means that I can start posting
- [00:12] long-form content every single day. This
- [00:14] is what I'm using from now on and today
- [00:16] I'm going to show you what is happening
- [00:18] behind the system and why it changes
- [00:20] three things for you whether you are a
- [00:21] creator or an editor. It changes your
- [00:23] time, your money, and your role and I
- [00:26] call these three shifts. Here's
- [00:28] everything that's inside the free
- [00:30] package for you to download today.
- [00:32] First, a teleprompter that follows your
- [00:33] voice, records your mic, and keeps track
- [00:35] of every take and retake, every mistake.
- [00:38] An editing app that reviews and cuts
- [00:40] your footage. You also get the agent
- [00:42] instructions and the tools that
- [00:43] transcribe your recordings, that compare
- [00:45] your takes, and check the results for
- [00:47] you. Then, finally, the design templates
- [00:50] for Claw Design, the animations and the
- [00:52] motion graphics so that you can build
- [00:54] visuals around your video like I'm doing
- [00:55] right now. And yes, this video that
- [00:57] you're watching right now is edited
- [00:59] completely with this pipeline and the
- [01:01] human input was only 10% of the work.
- [01:04] So, let's put things in perspective for
- [01:06] a second. 15 minutes of motion graphics
- [01:08] is done in an hour and a half with this
- [01:10] pipeline, which costs, believe it or
- [01:12] not, about 40 cents in AI tokens. Now, a
- [01:15] normal human editor would charge me
- [01:17] anywhere from $750 to $900 for the same
- [01:20] thing and it would take anywhere from 6
- [01:22] to 8 hours to deliver it. Now, my system
- [01:25] though works a bit differently from what
- [01:27] you've seen online because it all starts
- [01:29] with
- [01:30] my teleprompter.
- [01:39] Hey friend, welcome back. If you're new
- [01:41] here, my name is Serio. I architect AI
- [01:43] workflows and systems. I deploy APIs and
- [01:46] build AI businesses in the media space
- [01:49] and last year we made about $6 million
- [01:51] doing just that. And just so we're on
- [01:53] the same page, everything that I'm about
- [01:55] to show you today is free on my GitHub.
- [01:57] You don't have to worry about how to
- [01:59] recreate this prompt by prompt because
- [02:01] you can copy everything right now and
- [02:04] the link is in the description. So,
- [02:05] instead, I'll just show you what I build
- [02:08] and why. And I really want you to pay
- [02:10] attention to the thinking behind it
- [02:12] because in a world where everyone can
- [02:14] use AI to do pretty much anything, what
- [02:16] really stands out is the way that you
- [02:18] think and how you turn your knowledge
- [02:20] into a product. So, here's what we're
- [02:22] going to cover. First, I'll show you the
- [02:24] whole system in seven steps. That
- [02:26] includes the recording on my
- [02:28] teleprompter, turning my teleprompter
- [02:30] into a data collector, sending that data
- [02:32] into an editing app that I built to sync
- [02:34] with the teleprompter, and seeing how AI
- [02:36] agents understand my video project and
- [02:38] cut it like a human would with only 3%
- [02:41] errors in assembling and clipping the
- [02:43] long-form video together. Then we'll
- [02:45] talk actual numbers. How long does it
- [02:47] take? What does it cost? What does it
- [02:49] save? And how can the system scale
- [02:51] beyond just me as a creator who uses
- [02:53] social media to to build AI businesses.
- [02:55] What can we do with this? Then we'll go
- [02:57] through the three shifts that I
- [02:58] mentioned in the beginning of this video
- [02:59] one by one. So, by the end, you will
- [03:02] have my exact system. You can just copy
- [03:04] it just like that. I'm not asking
- [03:06] literally for anything back in return.
- [03:07] So, here's how the whole thing started,
- [03:10] okay? I was sitting here and recording
- [03:13] looking at my teleprompter like I'm
- [03:14] doing right now, and I was using an app
- [03:16] that's called SpeakFlow, which basically
- [03:18] turns your iPad into a prompter.
- [03:20] Um and I was like, "W- Wait a second.
- [03:22] Like this thing already knows my script.
- [03:24] And it knows exactly which line I'm
- [03:26] reading and how I'm reading it. Not not
- [03:28] my words only, but how fast I talk, how
- [03:30] how loud I am, like every time that I
- [03:32] mess up, every time every time that I
- [03:33] start a line over, and the exact second
- [03:35] that I would say each word, and even
- [03:37] when I go off script, right? So, this is
- [03:38] data that's being collected. But is it
- [03:40] actually being collected? This data is
- [03:41] available to be collected. So, I
- [03:43] thought, "Okay. So, why don't I just
- [03:44] build my own teleprompter that can save
- [03:46] all of that and send all the data that
- [03:48] it logs straight into an editing app
- [03:51] that I'm going to build, too, syncs to
- [03:53] the prompter that's going to cut my
- [03:55] video ba- based on everything that knows
- [03:57] so far about my script and how I speak.
- [03:59] Um so I wanted to test that out um
- [04:01] because in theory like that would make
- [04:03] so much sense. So I built both.
- [04:05] With Claude, I built a teleprompter that
- [04:07] saves everything while I record and an
- [04:09] editing app that reads all of the data
- [04:11] and cuts the video and adds all the
- [04:13] motion graphics and syncs everything to
- [04:15] my voice like you're seeing right now.
- [04:16] Whatever's happening right now is the
- [04:18] product Is that how I say that? Is the
- [04:21] fruit of my labor. Now the whole point
- [04:22] of this is to scale my content
- [04:23] production, right? So I can even give
- [04:25] the AI editing app that I built to my
- [04:27] editor so he will not have to like do
- [04:29] all the repetitive stuff himself so we
- [04:31] can move fast. So the moment that I stop
- [04:33] recording, like if I do this right this
- [04:35] second, my editing app knows exactly
- [04:38] where to cut. This is basically this is
- [04:39] the whole trick because it brings the
- [04:41] error rate of like the clipping and
- [04:43] assembly from like traditional ways of
- [04:44] clipping your content through AI down to
- [04:47] around 3%. That's normally 20 to 30% if
- [04:49] if you just use AI to clip things and
- [04:52] ask Codex or Claude to take a video and
- [04:55] transcribe it and start clipping. And
- [04:56] here's how the whole thing happens in
- [04:58] seven steps, okay? Step one, I record
- [05:00] myself reading a script on my
- [05:02] teleprompter and my Mac controls it and
- [05:05] my iPad shows me the words so right
- [05:07] right here in front of the camera and
- [05:09] they sync in real time as I read like
- [05:11] like you're seeing right now on screen.
- [05:13] And um look at this. Like what I what I
- [05:15] can do is that I can change the text
- [05:17] size. I did on my computer it's going to
- [05:20] change on my iPad. I can change the
- [05:22] column width and then I can even scroll
- [05:24] myself um and make sure that I can sync
- [05:27] to whatever line that I'm reading and it
- [05:28] syncs perfectly to my iPad. But normally
- [05:31] I do not touch this because it listens
- [05:33] through my mic and moves along with me
- [05:35] as I talk. Um and while it does so, it's
- [05:37] also recording my voice at the same time
- [05:39] straight from my mic while my camera
- [05:41] records the video um on its own. So
- [05:43] camera is not really connected to the
- [05:44] setup but we are going to have to sync
- [05:46] the audio later with a script. Under the
- [05:49] hood, this is very simple. It's a little
- [05:51] Python server that's running on my Mac,
- [05:53] and the teleprompter itself is just like
- [05:54] a web page on Chrome, and it follows my
- [05:56] voice with a speech recognition that's
- [05:58] already built into Chrome, so there's
- [05:59] nothing extra for us to install. And the
- [06:01] iPad just opens the same localhost page,
- [06:04] which is how they sync because they go
- [06:06] through the same Wi-Fi and the same
- [06:07] router and the same server. Now, when
- [06:08] it's done recording, it saves everything
- [06:10] into one folder, and this is really
- [06:13] important because this folder has all
- [06:14] the information that the editor will
- [06:16] need later, whether that's my human
- [06:18] editor, or that's like an external team
- [06:20] member, or the editing app that I built.
- [06:23] So, let's go into that folder and see
- [06:25] what's inside. For every take, there's a
- [06:28] file that's called take.json, okay,
- [06:30] which is basically the diary of of our
- [06:33] takes. So, there's all the data in here
- [06:35] that that we talked about, and this is
- [06:36] what it actually looks like if we open
- [06:38] that. Say that I mess up this line, and
- [06:42] I just started again. Like, let me do
- [06:44] this live. Uh say that I mess up this
- [06:46] line, and I'm going to start this again.
- [06:49] Let me let me do this live.
- [06:51] Okay. So, now you can see on screen
- [06:53] right here, it catches that I repeated
- [06:56] the the line twice, and it marks it as
- [06:58] retake, all by itself. I didn't press
- [07:00] any buttons, and this is saved in the
- [07:02] take.json file, and you can see both the
- [07:04] tries and when each of them started and
- [07:07] stopped. But then there's one more file,
- [07:09] it's called um editplan.json,
- [07:12] which basically is the plan for the cut.
- [07:14] It says which take to use for every
- [07:16] line. Again, it's talking it's
- [07:18] communicating with our editor, not the
- [07:19] human editor in this case, but with our
- [07:21] app, with the editing app that that
- [07:23] we've built. And this part is not AI
- [07:25] because the rule is very simple, it's
- [07:26] deterministic. If I say a line twice,
- [07:29] three times, four times, whatever that
- [07:31] is, it keeps the last one. And then we
- [07:33] have a file, obviously that's called
- [07:35] agents.md, which is just simply a note
- [07:38] for Claude that explains what's in the
- [07:40] folder and how to use it when you set it
- [07:43] up, but also if your human editor or
- [07:45] like your friends are going to set up
- [07:47] the same project. That's what's inside
- [07:48] the folder. So now whoever has access to
- [07:50] my editing app can use this information
- [07:52] to automatically cut and assemble the
- [07:54] footage. So let's talk about cutting the
- [07:56] footage now. Step two is where the
- [07:58] cutting happens in the editing app that
- [08:01] I built. And the editing app is
- [08:02] basically a child of my teleprompter
- [08:04] because I literally use the teleprompter
- [08:06] project to build on top of that. So
- [08:08] that's how the editing app came about.
- [08:11] And here's what happens. Now Claude
- [08:14] reads the agents.md file first if you
- [08:16] remember. The agents.md file which lives
- [08:18] in the folder that the teleprompter
- [08:19] produces, then it syncs my audio from
- [08:22] the prompter which we recorded to my
- [08:24] camera footage. Then it pulls up the
- [08:26] edit plan, the the edit.json file, and
- [08:29] follows it to grab the right take for
- [08:31] every line. Then it starts the editing
- [08:33] app on localhost. It all runs local in
- [08:36] Chrome and and it uses FFmpeg which is
- [08:38] free to actually cut and export the
- [08:40] video at the very end. And the reason
- [08:42] that all of these things are saved as
- [08:44] JSON, like all the instructions, is that
- [08:46] the agent can read it exactly with no
- [08:49] guessing. So that means that the cut is
- [08:51] actually coming from predetermined data
- [08:54] instead of an AI watching like an hour
- [08:56] of footage and transcribing it and then
- [08:58] you hope that AI is going to make the
- [09:00] right cut. We we all know that that's
- [09:02] not going to be the case. So the
- [09:04] difference between what they teach you
- [09:05] online and what I'm showing to you today
- [09:07] um is this. And I think that that's a
- [09:09] great hack. If any of you don't have a
- [09:11] teleprompter, you can just use your mic
- [09:13] and build the the same the same
- [09:14] deterministic system.
- [09:16] Okay, once everything is put together,
- [09:18] meaning that the agents have found the
- [09:19] right cuts and and and they've lined
- [09:22] them all up, it uses Whisper, which is
- [09:24] also free, to transcribe the final cut
- [09:27] that the agents put together. So it
- [09:29] double checks that it actually makes
- [09:31] sense and that every word can be heard.
- [09:33] It's not like cut in the middle. Um and
- [09:34] then it finds all the pauses, all the
- [09:36] ums, or and and removes everything that
- [09:39] does not fit my instructions. And he
- [09:41] does so by simply analyzing the audio
- [09:44] waves. And since the as we said, the
- [09:46] audio is synced to the camera footage,
- [09:48] if you make a cut to the audio, that cut
- [09:51] is also going to happen to the video,
- [09:52] too. Everything is linked together. Now,
- [09:54] very important part, it does not just
- [09:57] merge and cut everything into one file.
- [10:00] It puts all the takes on a timeline, so
- [10:03] if something's off, the human editor can
- [10:05] still fix it, um even if it's word by
- [10:07] word, because as you can see right
- [10:08] there, it matches the words to the
- [10:11] footage. And if you highlight and delete
- [10:13] a word, it does the same thing in the
- [10:15] timeline. So, that's step number two.
- [10:17] Step number three is the boards, um
- [10:19] probably the most interesting part of
- [10:21] all of this, because it is saving me
- [10:23] thousands of dollars. The boards are
- [10:25] these graphics that you are seeing
- [10:27] everywhere, like behind me, and I'll
- [10:28] show them to you in a second, um but
- [10:30] first, the boards are planned by a
- [10:33] skill. And the skill reads the the whole
- [10:36] final transcript from Arc Cut,
- [10:38] previously from step number two, line by
- [10:40] line, and figures out which lines need
- [10:42] visuals and what type of visuals, right?
- [10:44] Like, what are we building here? And for
- [10:47] every line that I speak, the skill asks,
- [10:50] um one question, which for me, in this
- [10:52] case, is, "Can you understand this just
- [10:54] by listening, or do you need extra
- [10:56] layers?" And everything that the skill
- [10:59] finds using all our agents, gets saved
- [11:02] as a simple markdown file, like this one
- [11:04] that you're watching right here, which
- [11:05] is our board brief. And it's simply
- [11:07] instructions for the agent to start
- [11:10] designing. It's not the actual thing.
- [11:12] Step four is Claude code that's going to
- [11:15] design every board for my own design
- [11:18] system, which I spent a long time
- [11:20] putting together. Now, there's more than
- [11:21] a hundred boards here that have design
- [11:23] styles, charts, graphs, prompts, and all
- [11:26] the animations. And the more boards or
- [11:28] templates that you have, the easier it
- [11:30] is to turn the new scripts that you are
- [11:32] going to give Claude into graphics. Now,
- [11:36] every board is just an HTML file,
- [11:38] basically it's like a little webpage. It
- [11:40] is a very well-designed web UI. So, how
- [11:43] are the new boards actually designed? I
- [11:45] give Claude the board brief from step
- [11:47] number three and my own design system
- [11:49] with all the templates so that every
- [11:51] board follows a plan and they have the
- [11:53] same design rules and nothing is left to
- [11:55] chance. Step number five, all the boards
- [11:58] now go through another skill that I
- [12:01] built which tells Claude how things
- [12:03] should animate, okay? For example, this
- [12:06] one is one instruction on on how it
- [12:08] should use keyframes. It only knows what
- [12:11] you feed it, right? And then how you
- [12:12] feed things. So, it goes through the
- [12:14] skill and then animates everything and
- [12:16] once that's done,
- [12:17] the animation gets synced to my voice
- [12:20] with a script that happens
- [12:23] in the backend. You don't really see the
- [12:24] process. For example, when you see that
- [12:26] I point up
- [12:27] like this, something shows up right when
- [12:29] I say it, right? So, that's the
- [12:31] audio-to-graphics sync. Step number six,
- [12:33] we have to turn everything into a video
- [12:35] so that we can export it and then then
- [12:36] publish it on on YouTube or any of your
- [12:38] platforms. Now, remember that all the
- [12:40] boards are still HTML, they are code.
- [12:43] So, now what's going to happen is that
- [12:45] Chrome is going to open like each
- [12:48] animation, each board, and it's going to
- [12:49] record it frame by frame and FFmpeg is
- [12:53] going to turn all of it into an MP4
- [12:55] video. Now, on my Mac, a 15-minute video
- [12:57] takes about 20 minutes to render it to
- [13:00] 2K. I have a Mac
- [13:02] M3 and if your computer is actually
- [13:04] slower, you can just rent GPUs on
- [13:06] RunPod. Then I watch the final video and
- [13:08] I approve it. In step seven, a skill
- [13:11] writes a description and the chapters of
- [13:13] the YouTube videos this case using the
- [13:14] transcript that I already approved. And
- [13:17] that's it. But how much does this really
- [13:19] cost? Let's do the math. My Claude plan
- [13:22] is $200 $200 a month, so that's about 27
- [13:26] cents an hour. So this 15-minute video
- [13:29] cost me about 40 cents for for the AI
- [13:33] compute. Now a normal human editor at
- [13:35] $150 an hour for 5 to 6 hours, this is
- [13:39] how much how long this will take, is
- [13:42] about $750 to $900.
- [13:45] And even if you pay your human editor
- [13:47] the same exact rate, like 150, to run
- [13:50] the system that I build or that you are
- [13:52] going to build, that is about $225.
- [13:55] So that is 70 to 75% less than what you
- [13:59] would normally pay. You're saving around
- [14:01] $500 to $700 on a video, which that's
- [14:04] like $25,000
- [14:08] a year
- [14:09] um if you post four videos a month,
- [14:11] let's say. Now if you're looking at
- [14:13] this, you're either very excited or
- [14:15] you're scared. And I know that a lot of
- [14:17] editors think that AI just means that
- [14:19] they are going to get um replaced.
- [14:22] Um and honestly I I I get it. Um she
- [14:25] look at all these headlines.
- [14:27] Uh but but all of them are clickbait or
- [14:29] some of them are sponsored by the
- [14:31] company that's selling a tool that's
- [14:32] supposedly like replacing you. But I
- [14:34] wanted to see what actually happens with
- [14:38] my own human editor. And that's where um
- [14:41] three shifts come from, I think. It it's
- [14:43] his time, his money, and his role.
- [14:45] And I'm saying human editor just because
- [14:47] I'm referring to an other like AI editor
- [14:49] in this video. Like I don't Like it's
- [14:51] it's very weird to refer to people as
- [14:54] human editors, but just to make things
- [14:56] extremely clear. So the first thing
- [14:58] that's changing is time if you're an
- [14:59] editor. An editor that used to take 5 to
- [15:01] 6 hours now takes about like an hour and
- [15:03] a half. So if you look at this graph,
- [15:04] the gray bar is how long one video used
- [15:07] to take, which is 5 to 6 hours, and the
- [15:09] blue bar is now. And that dotted part is
- [15:12] is is the 4 hours that he gets back on
- [15:15] every single video. So now what I can do
- [15:17] is that I can batch record a few videos
- [15:19] in one day, and my editor can finish uh
- [15:21] at least three of them in a normal shift
- [15:24] and and funny enough now the slowest
- [15:26] part of this whole
- [15:28] right the slowest part of this whole
- [15:30] system is me sitting right here and
- [15:31] recording. I'm sorry like if I'm not
- [15:34] extremely clear with with what I say and
- [15:36] how I pronounce things English is not my
- [15:37] first language but let's get back to the
- [15:40] second shift which is money and this one
- [15:43] is interesting because I still pay my my
- [15:45] editor human editor exactly the same as
- [15:48] I did before but he's putting out way
- [15:50] more videos in way less time with way
- [15:53] less effort. If you look at this graph
- [15:55] the gray bar is before when in a normal
- [15:57] 8-hour day my human editor would finish
- [15:59] about one video and the blue bar is now
- [16:03] same exact pay he finishes at least
- [16:06] three videos. So now this gap between
- [16:09] the two bars that's the whole point that
- [16:11] means three times the output for the
- [16:13] exact same money. He's paid the same but
- [16:16] each video takes him a fraction of the
- [16:18] effort so for the time that he's
- [16:20] actually putting in he's earning way
- [16:22] more and I also get more videos for the
- [16:23] same price and honestly we both win. Now
- [16:26] if you're an editor that's a deal for
- [16:27] you because you're going to get paid per
- [16:29] video or per month. If you're going to
- [16:32] get paid by the hour
- [16:34] that just means that a faster edit is
- [16:36] going to be a smaller paycheck for you
- [16:38] and here's how you can actually scale
- [16:39] this say that you charge $700 for a
- [16:42] 30-minute video or like 45-minute video
- [16:44] before you could finish about one in one
- [16:47] day right so like it's going to take you
- [16:48] like six to eight hours. Now you can
- [16:50] finish three so instead of one creator
- [16:52] you can take on three in the same exact
- [16:55] hour and you can start automating the
- [16:56] process and that's $700
- [17:00] that a day that you were turning into
- [17:02] over like $2,000 without having to work
- [17:05] a single extra hour just because you can
- [17:08] handle more output and and let's say
- [17:10] that you're full now you can hand the
- [17:11] same exact system the same exact
- [17:13] workflow to maybe a junior editor and
- [17:15] then teach them what good looks like.
- [17:16] And the third shift, which I think that
- [17:18] matters the most here, is your role.
- [17:21] Before my my editor made everything by
- [17:24] hand, right? Cutting every take,
- [17:26] building every board, keyframing
- [17:27] everything. Now, about 90% of that is
- [17:29] done before he even opens the project.
- [17:32] Um and and the cut cut that that's done
- [17:34] by by the pipeline um
- [17:36] comes in around with like 3% errors. So,
- [17:39] his job
- [17:41] like the last 10% is finding the
- [17:43] mistakes, fixing them, and directing AI
- [17:46] to do a better job. And that's what
- [17:48] takes no more than an hour and a half.
- [17:49] And the great part is that
- [17:51] you don't really have to be technical.
- [17:52] You don't really have to learn how the
- [17:54] AI works. I mean, obviously, if you do
- [17:56] so, that's way better because you're
- [17:57] going to optimize the system, but I
- [17:59] figured the system out for you. I I
- [18:01] figured the system out for him in this
- [18:03] case. So, if you are editing instead of
- [18:05] executing the edits yourself, you're
- [18:06] becoming more of like a creative
- [18:08] director doing quality assurance. Like
- [18:10] your job is spotting the boards if they
- [18:12] land too early, if the cuts feels off,
- [18:14] if the sound sounds flat. All these
- [18:16] minor little details that are extremely
- [18:18] important for creating quality video.
- [18:20] And now, this is what I'm seeing in my
- [18:21] own business. It's not a study, but but
- [18:23] this is very clear to me. And take this
- [18:25] from someone who knows the space. Guys,
- [18:27] I actually want to retire like tomorrow.
- [18:29] I'm very tired. It's It's nuts in here.
- [18:31] Okay, I know that this was a lot. So, if
- [18:33] you made it all the way to here, thank
- [18:36] you. Because here's what I really want
- [18:37] you to take from this. I had a problem.
- [18:39] My editing was way too slow. It was too
- [18:41] expensive. And I just built my own
- [18:43] solution to it. I did not wait for an
- [18:45] app to to add a feature or someone to
- [18:48] like release an MCP cuz everything that
- [18:50] I need was already available right here,
- [18:53] right actually right in front of me. And
- [18:55] what made the difference was how I
- [18:57] architected this entire thing and how
- [18:59] well I understand the tools and how I
- [19:01] put them together into a system that
- [19:03] works for me. It has to work for me. And
- [19:05] if you do not need the system yourself,
- [19:07] you can do it for other people. You can
- [19:08] package it up. You can sell it to
- [19:10] agencies, to other creators, or
- [19:11] students. And to run this. Literally,
- [19:13] you just need a camera, a mic, a tele-
- [19:15] like an iPad for a teleprompter, or like
- [19:16] you can find alternative solutions to
- [19:18] that. Chrome Cloud Code, literally,
- [19:20] that's it. Now, the whole thing is free
- [19:23] on GitHub. The link is below. And to
- [19:25] install it, just like copy the link,
- [19:27] paste it into your Cloud Code, and tell
- [19:28] it this. Read the start here guide in
- [19:30] this repo, install everything it lists,
- [19:33] and run the setup check. So, my friend,
- [19:35] take this system, run it on your next
- [19:36] video, change whatever doesn't fit for
- [19:39] you, and see where that takes you,
- [19:41] because being curious, that's enough to
- [19:43] make it out there. Now, if you want to
- [19:44] build systems like this with other
- [19:46] people, get feedback, which is like hang
- [19:48] out with me, join Public AI. It's my
- [19:50] free community. The community is for
- [19:51] people who want to go further than just
- [19:54] making AI content. It's actually for
- [19:55] people who want to build something, like
- [19:57] a media company, an app, agents,
- [19:59] whatever pushes the boundaries of what
- [20:01] you think is possible. So, my friend, a
- [20:02] piece of advice, do not just watch
- [20:04] tutorials on how to use someone else's
- [20:07] software.
- [20:08] Start building your own solution.
- [20:11] The link is right below. Join me today,
- [20:13] and do not forget, create without
- [20:15] limits. This is Syria.
