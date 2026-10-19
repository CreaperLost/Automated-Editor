# Export

0. Chapters come right before export, never earlier: they read the final edit's words. Run
   `status`; if chapters were never made (and the user wants them) or it says they are out of
   date, run `chapters` now and show them to the user.
   Then run `check` (it reads the final edit too). Tell the user about any error or warning
   and fix it with their go (`check` with `fix`); an error stops the export.
1. When the edit is done, ask first: "Export now?" with the choices, default first:
   - Resolution: 1080p (default), 720p, 1440p, 4K. The canvas shape in `status` sets the
     orientation: 1080p is 1920x1080 for 16:9, 1080x1920 for 9:16, 1080x1080 for 1:1,
     1440x1080 for 4:3. Scale the others the same way.
   - Where: next to the project (default) or a path the user gives.
   - Quality: high (default, sharp screen text), standard (smaller file), max (near lossless).
   Use the saved export answers when the user said "same as last time".
2. Wait for the user's go. Don't export on your own, even at the end of a template.
3. Run `export` with `settings` `{"width": W, "height": H, "quality": "high"}` and
   `destination` only if the user gave one. It waits and reports progress; a long tutorial
   can take several minutes, so say so.
   If your client gives up on long calls, run `export` with `wait: false` instead and check
   `export_status` every minute or so.
4. Answer with the file path, the resolution and how long it took.
5. If it fails, tell the user the error and what you suggest. Don't retry with other settings
   unless the user agrees.
