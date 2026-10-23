# Export

0. Chapters come right before export: run `status`; if chapters were never made (and the user
   wants them) or it says they are out of date, set them now and show them to the user. Then
   run `check`. Tell the user about any error or warning and fix it with their go (`check` with
   `fix`, whose answer already says what is left); an error stops the export.
1. Ask first: "Export now?" with the choices, default first:
   - Resolution: 1080p (default), 720p, 1440p, 4K. The canvas shape in `status` sets the
     orientation: 1080p is 1920x1080 for 16:9, 1080x1920 for 9:16, 1080x1080 for 1:1,
     1440x1080 for 4:3. Scale the others the same way.
   - Where: next to the project (default) or a path the user gives.
   - Quality: high (default, sharp screen text), standard (smaller file), max (near lossless).
   Use the saved export answers when the user said "same as last time".
2. Wait for the user's go. Don't export on your own, even at the end of a template.
3. Run `export` with `settings` `{"width": W, "height": H, "quality": "high"}` and
   `destination` only if the user gave one. It waits and reports progress; the app shows it in
   its Export window too. If your client gives up on long calls, run `export` with
   `wait: false`, then `export_status` with `waitS: 300`, which answers when the export ends (or
   after 5 minutes): no need to sleep between checks.
4. Answer with the file path, the resolution and how long it took.
5. If it fails, tell the user the error and what you suggest. Don't retry with other settings
   unless the user agrees.
6. Other files, only when the user asks: `export` with `file` `srt` or `vtt` (subtitles),
   `txt` or `md` (the transcript; `md` has the chapters as headings) or `wav` (the sound
   alone), and `short` for a short's. They go next to the project, named after it.
