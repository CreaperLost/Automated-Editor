# Performance and correctness review — branch `optimizations-bug-fixes`

Written while fixing. Each issue says what was wrong, how it was found or measured, and what was done. Status is one of **fixed**, **open** (with the reason) or **not a bug**. Astra's reports in this folder (`performance-review-2026-10-04.md`, `recording-overlap-investigation-2026-10-04.md`) are answered item by item in the second section.

Measurements come from `src-tauri/src/perf.rs` on a real 31 s 1080p60 recording (screen, camera, microphone, system audio), with and without 25 jump cuts, and from the browser preview with a made-up project of 280 clips and 69 cuts (dev build). This machine is noisy, so timings vary between runs; the trends hold.

## Issues found in this pass

### R01 — Long recordings lost all their video and sound ("Overlapping segment") — fixed

The recorder writes a long recording in ~55 s segments. Each one starts a moment before the previous ends: 4 µs for the screen, about 2 ms for sound, up to a frame (17 ms) for the camera. The reader took any overlap as a conflict and marked **both** files unavailable, so an 11-minute recording lost every screen, camera and microphone segment. It wasn't new code; this was the first long recording.

Fix (`project/reader.rs`): an overlap of up to 250 ms ends the earlier segment where the next one begins, so nothing plays twice. Each file keeps its own time mapping. A longer overlap is still a conflict. A missing file takes no part in overlap checks, so it neither disables nor shortens its neighbours; the extended test found that case. On the real recording, all 48 segments are available and there are no notes.

### R02 — Playback and export stalled at every segment switch — fixed

Lookahead decoders were started only at clip edges. Inside a long clip, moving to the next segment file needed a new FFmpeg process at about 400 ms, once every 55 s. Lookahead now covers segment boundaries too. Playing across the 55 s boundary of the real recording showed no slow frames.

### R03 — The decoder pipe was the slowest part of decoding — fixed

A 1080p BGRA frame is 8 MB, and Windows pipes carry about 0.6–1 GB/s. Piping raw output alone took 13.9 ms a frame as BGRA, against 3.3 ms as NV12. Recording video now crosses the pipe as NV12 (a third of the bytes), and the GPU compositor converts it in its shader. The conversion was checked against FFmpeg's own BGRA (mean difference under 3 levels; `gpu_nv12_layers_match_bgra`). Imported files stay BGRA because they may carry transparency, and the CPU compositor converts NV12 itself.

### R04 — Export decoded every source frame — fixed

Export decoded with no frame-rate cap, so a 60 fps recording in a 30 fps export piped every other frame through for nothing (12.8 ms a frame against 8.1 ms with the cap). Export now decodes at its own frame rate.

### R05 — Export ran every stage one after another — fixed

The loop went decode, composite, encoder write, audio, all on one thread. Encoder writes and audio now run on a writer thread with a two-frame queue. Export also starts decoders ahead of cuts, and checks the finished file with a software decoder, which starts far sooner than a GPU one. Overall, 1080p30 export went from 1.09× to 1.96× real time, and with jump cuts from 1.01× to 1.54×.

### R06 — Timeline interactions re-rendered every clip — fixed

Selecting, dragging or editing re-rendered all 280 clips. Clips are now memoized components with their handlers behind a ref. An edit keeps the objects of clips it didn't change, and cut marks and camera clips are worked out once per revision.

| Interaction | Before | After |
| --- | --- | --- |
| Select a clip | 119 ms | 21 ms |
| Deselect | 217 ms | 18 ms |
| Each drag move | 75–117 ms | 18–34 ms |
| Apply an edit | 97 ms | 35 ms |

### R07 — Frames copied their pixels whenever they were passed on — fixed

`VideoFrame` held its pixels in a `Vec<u8>`, so every clone copied them: a cached still each frame (33 MB for a 4K still), the wallpaper each frame, caption frames, and every decoded frame handed out by the decoder. Pixels now live in a shared, copy-on-write `PixelBuffer`, and decoder buffers go back to the reader only when nothing holds them. Stills are also kept at the size the preview needs. Combined with R03–R05, 1080p30 export is now at **2.62× real time** without cuts and **1.76×** with jump cuts (it started at 1.09× and 1.01×). A preview frame averages 6.5 ms.

### R08 — Dragging hit-tested every clip on every pointer move — fixed

Each move asked the page which elements were under the pointer, to find the target track. With 8,000 clips, 97 elements overlapped at that point, and the call took 57 ms. The row now comes from the few row boxes, read when the drag starts and every 250 ms after. Together with P05, a pointer move dropped from 42–58 ms to under 1 ms.

### R09 — The first waveform of a long recording took over a second per sound — fixed

This was profiled on the real 11-minute recording. Building a waveform did two 128-bit divisions per sample, to turn a frame number into a time and the time into a bucket, and the file fingerprint fed FNV one byte at a time. The loop now works out where each bucket ends once and runs through its samples, and the fingerprint hashes eight bytes at a time. Microphone: 988 → 290 ms. System audio: 1,300 → 407 ms. `buckets_match_the_per_sample_formula` checks the new loop against the old formula at 44.1, 48 and 22.05 kHz. Waveforms already cached on disk are rebuilt once, because the fingerprint changed.

### R10 — The timeline drew every clip, even far off screen — fixed for zoomed-in views (was O01)

Each track now draws only the clips within half a view or more of the visible stretch. The window moves in half-view steps, so scrolling doesn't redraw the timeline every frame. With 8,000 clips at 16× zoom, 1,052 are drawn, and an edit that shifts them all takes 215 ms instead of 1,459 ms (dev build). With the whole timeline in view, every clip is still drawn; see O01.

### R11 — The transcript redrew every word several times a second while playing — fixed

The transcript panel re-rendered all its words whenever the playing word changed (several times a second during playback) or the selection changed. Each word also searched its line to find the next one. Lines are now memoized components, given only what concerns them. With 5,000 words (dev build), clicking a word went from 74 ms to 4 ms, and the playing word moving on went from 72–116 ms to 12–48 ms. Most of what remains is scrolling the playing word into view.

### R12 — Every edit reloaded the transcript, captions and zoom suggestions — fixed

The transcript panel, the caption track and zoom suggestions refetched on every project revision. Dragging a layout slider makes many revisions, and each one reloaded the whole transcript (then redrew it), rebuilt the caption cues, and ran zoom detection in the backend while holding the project lock. They now reload only when what they depend on changes: the sequence (unchanged edits keep its object; see R06), the transcript, or the caption or zoom settings. Checked in the browser: a layout edit triggers no reload, and a cut triggers one of each.

### R13 — Switching an audio effect on held playback up for over a second — fixed

The playback worker builds its audio mixer after every edit. The first build with loudness, noise reduction or ducking switched on analyzed every sound file of the recording one after another: 1.24 s on the 11-minute recording, with playback held up. The files the enabled effects need are now analyzed side by side first, and the plan is built from the cache: 122 ms. A warm build takes 13 ms.

### R14 — Reading a long transcript blocked the window — fixed

Tauri runs commands that aren't `async` on the main thread, which also drives the webview. `transcript_get`, `transcript_suggestions` and `project_caption_cues` read and placed the whole transcript there. They now run on the blocking pool, as the waveform and pause commands already did. Edits (about 12 ms) still run on the main thread; they're short.

### R15 — Waveform segments were built one after another — fixed

A long recording is many segment files, and each is independent. They're now built side by side. On the 11-minute recording, the first waveform for the microphone went from 290 to 155 ms, and for system audio from 407 to 220 ms; before R09 these were 988 and 1,300 ms.

### R16 — Restoring jump cuts left gaps around recorder pauses — fixed

A recording paused in the recorder starts as one clip per stretch between pauses, side by side. Restore treated those pause joins as cuts, and restoring a cut that crossed a pause stopped at the pause: the left clip could only be lengthened up to it, so the recorded time after the pause stayed cut and the restore marker stayed. Now a cut is the recorded time missing between two clips (a pause alone is none, in the backend and the timeline's restore markers), and restoring puts each recorded stretch back as its own piece, so the timeline returns to exactly how it started. Covered by `restoring_cuts_across_recorder_pauses_leaves_no_gap` (restore all and one by one, cuts across one and two pauses), which fails on the old code. `probe_restore_all_cuts` cuts every detected pause of a real recording and restores them, all at once or one by one; both 4 Oct recordings come back as one clip per track.

### R17 — Export converted colours on the CPU — fixed

Export already decoded (NVDEC), composited (wgpu) and encoded (NVENC) on the GPU, but each composited frame was read back as BGRA (8 MB at 1080p) and the encoder process converted it to YUV 4:2:0 in software. The compositor now converts on the GPU (`render/to_yuv.wgsl`, the inverse of the decode shader) and reads back NV12 (3 MB); FFmpeg passes it to the encoder unconverted. The macOS writer and CPU compositing keep BGRA.

- Speed (1080p30, RTX 3060, NVENC): straight export 70–72 → 81–90 fps; with jump cuts 37–45 → 46–49 fps (decoder start-up at each cut limits that case).
- CPU: the same export took 195–199 CPU-seconds instead of 249–289.
- Colour: against FFmpeg's conversion with a box filter, every plane is within 1 level (`gpu_nv12_output_matches_ffmpeg_conversion`). Against the old export path on the probe recording (`probe_export_nv12_against_bgra`), brightness is identical and colour differs by 0.04 levels on average, up to 12 at a few sharp colour edges, where FFmpeg's bicubic filter shrinks colour slightly differently.

### R18 — Jump cuts missed an imported video's speech — fixed

Jump cuts scan one sound, and picked the first speech on the timeline: with a recording and an imported video, the recording's microphone. The video's pauses were never found, however its sound was marked. A sound marked as speech on its track's header didn't count as speech at all. Now, with speech from more than one source on the timeline, jump cuts scan all of it by default ("All speech on the timeline"): a pause is where every speech playing there is silent, so one pause can run from the end of the recording into the video. Muted tracks are left out, and a single sound can still be chosen. Covered by `common_pauses_need_every_speech_playing_to_be_silent`; `probe_imported_video_jump_cuts` puts a video with three pauses after the probe recording and finds all 13 pauses (10 in the recording, 3 in the video), where the recording's microphone alone finds none in the video.

### R19 — Audio clips showed a name and an icon instead of a useful waveform — changed

The timeline asked for 512 bars across a whole sound: on the 11-minute recording, one bar per 1.3 s, too coarse to show a pause, drawn dim under the clip's icon and name. A new command, `project_waveform_overview`, returns the whole sound at a bar every 20 ms (36 ms for a two-hour sound) from the waveform summaries already cached on disk, packed as one byte per bar for peak and RMS: 33,000 bars and about 200 KB for the 11-minute recording, 98–137 ms the first time and 9 ms after. Audio clips draw it as a static Premiere-style waveform (peak and a solid RMS core, mirrored, on a dB scale so pauses read as a flat line and quiet noise stays visible), only over the part of the clip near the view so it stays sharp when zoomed in, with the name as a small tag in the corner. It no longer redraws as the playhead moves. Covered by `overview_shows_pauses_bar_by_bar`.

### R20 — Selecting several clips — added

Dragging over empty timeline space draws a box and selects every clip it touches, with what they are linked to (Alt: just those clips; Shift/Ctrl: added to the selection). Clicking a track's name selects its clips (Shift+click adds or removes them). Selected clips have a white border, an accent ring and a brighter fill. A click on a clip's edge now selects it too: on a narrow clip the trim edges cover nearly all of it, so clicks there used to do nothing.

### The 11-minute recording: everyday operations (release build)

| Operation | Time |
| --- | --- |
| Open the project | 52 ms |
| First waveform, mic / system (after R09 and R15; was 988 / 1,300 ms) | 155 / 220 ms |
| Waveform again | 8 ms |
| Find pauses (179 found) | 106 ms |
| Zoom suggestions | 25 ms |
| Cut all 179 pauses (720 clips after) | 8 ms |
| An edit / undo | 12 ms |
| Rebuild the preview after an edit | 7 ms typical |
| Build the audio mixer with every effect on, first time / again (R13) | 122 / 13 ms |
| Seek | 65 ms typical, 170 ms slow end (p95) |

## Astra's performance review (`performance-review-2026-10-04.md`)

Each item was measured with Astra's probes (`performance-probes-2026-10-04.rs`) before and after.

| Item | Status | What was done | Before → after |
| --- | --- | --- | --- |
| P01 4K stills copied every frame | fixed | Shared `PixelBuffer`; stills decoded no larger than the decode limit | 4 × 4K stills in a 720p preview: 33.6 ms → 0.27 ms per scene |
| P02 Noise reduction analyzes unused segments | fixed | Only segments that intersect a clip's source range are analyzed | 1 s clip of a 120 s recording: 109 ms → 0.13 ms |
| P03 Analysis cache clears at 256 files | fixed | Least-recently-used eviction within 128 MB; the cache is never cleared wholesale | 257 files, second pass: 257 recomputed, 148 ms → 0 recomputed, 10 ms |
| P04 Clock building quadratic, repeated per zoom | fixed | Binary search with a local merge; one clock per recording for zooms and shorts | 8,000 cut clips: 115 ms → 3 ms; `zooms_with_ranges` (4,000 cuts, 30 zooms): 827 ms → 1.7 ms |
| P05 Multi-clip drag math quadratic | fixed | Clip index map, selection set, snap edges per drag, one redraw per frame, plus R08 | 8,000 clips / 4,000 selected: 42–58 ms → 0.3–0.7 ms per move (whole handler, in the browser) |
| P06 Waveform canvases as wide as the zoomed clip | fixed (capped) | Backing canvas capped at 4,096 px across (512 bars per sound, so no visible loss) | 64× zoom: 169 MB → under 5 MB per clip |
| P07 JPEG encoder can stall the playback worker | fixed (stall); open (encode cost) | The worker skips drawing while the encoder is busy and never blocks on handover, so audio keeps being fed | In release builds a 720p encode takes 6–13 ms, inside a 33 ms frame. A detailed 1080p frame takes about 30 ms, which is borderline; see O02 |
| Decoder warm-up unbounded | fixed | At most 4 lookahead decoders start at once; an edge turned away is retried | Not benchmarked (no multi-track fixture with many sources) |

Astra's follow-up fixes of 2026-10-05 (decoder cache that keeps active decoders apart from prefetched ones and drops stale ones on an edit, retained noise analyses, still images read without a copy, a handover audio test) were reviewed and committed as `4dbb5a0`.

## Astra's overlap investigation (`recording-overlap-investigation-2026-10-04.md`)

Agreed and fixed as R01. Astra's recommended policy matches what was built: a deterministic handover with each file's own time mapping untouched, real overlaps still reported, and recording files never rewritten. The suggested regression cases are covered by `rolling_segments_hand_over_and_real_overlaps_are_flagged`: a 4 µs overlap, a 2 ms overlap, a 0.5 s conflict, exact adjacency, a gap, and a missing file. Astra also suggests checking the recorder's boundary accounting. That code lives in the separate AeroShoot recorder and is not changed here; see O03.

## Open

- **O01 — With the whole timeline in view, every clip is drawn.** R10 bounds drawing when zoomed in, but at full view thousands of clips under a pixel wide are still separate elements: an edit that shifts 8,000 of them takes about 1.5 s in the dev build. Drawing such dense stretches as one strip per track (they can't be clicked individually at that size anyway) would fix this.
- **O02 — Full-resolution webview preview is near the JPEG budget.** A detailed 1080p frame takes about 30 ms to encode in release. Two encoder threads taking turns would double throughput at the cost of one frame of latency, and frames would need ordering on presentation. The default preview (720p) is fine.
- **O03 — The recorder's segment boundaries overlap by up to a frame.** The editor now tolerates this, but the recorder could write exact boundaries. That's in the recorder repository.
- **O04 — System audio has a 10 ms gap at every segment boundary of a long recording.** It plays as a moment of silence. The gap is in the recording (as written by the recorder), not in the editor.
