# Window telemetry (`telemetry/windows.jsonl`), version 1

The contract between the AeroShoot recorder (writer) and AeroEdits (reader) for the window in
front during an `.aero` recording. It sits next to the mouse stream (`telemetry/events.jsonl`)
and uses the same clock. `telemetry/geometry.jsonl` only describes the recorded display, so
the editor needs this file to know where the window you work in is.

**What the editor does with it:** when the front window is clearly smaller than the recorded
display (a big or second monitor), the screen is cropped to that window with a little room
around it, and the crop glides to the next window when you switch. Quick app switches,
popups, menus and tiny windows never move it. The user can switch it off for the whole video
or for any stretch.

**Privacy rule:** the file never holds window titles or window contents. Only an opaque id,
the executable name, the window's kind and its box.

## File

- Path: `<project>.aero/telemetry/windows.jsonl`. UTF-8, one JSON object per line, `\n`
  endings.
- Written incrementally and flushed on the same cadence as `events.jsonl`. The editor ignores
  a truncated final line.
- No file means "windows not recorded": the editor never crops. The recorder writes no file
  when it records a single window or an area (the crop only makes sense for a display).
- Typical size: a few KB per hour.

## Common fields

| Field | Type | Meaning |
|---|---|---|
| `version` | `1` | Format version. The editor ignores the whole file for any other value. |
| `seq` | u64 | 0 for the first record, then +1 per record. Its own counter. |
| `t_us` | u64 | Microseconds on the recording clock, the same clock as `events.jsonl` `t_us`. |
| `kind` | string | One of the kinds below. The editor skips kinds it does not know. |

## Kinds

### `capture` (exactly one, the first record)

```json
{"version":1,"seq":0,"t_us":0,"kind":"capture","platform":"windows"}
```

### `front`

From `t_us` on, this window is in front. Written at recording start, whenever the foreground
window changes, and when the front window's box changes (after a move or resize settles; at
most 4 per second while it is being dragged).

```json
{"version":1,"seq":7,"t_us":8412000,"kind":"front","window":"0x000A12F4","app":"Code.exe","role":"window","bounds":{"norm_x":0.052,"norm_y":0.041,"norm_w":0.611,"norm_h":0.808},"maximized":false}
```

| Field | Type | Meaning |
|---|---|---|
| `window` | string \| null | An opaque id, stable while the window lives (the HWND in hex on Windows, the CGWindowID on macOS). `null`: no app window in front on the recorded display (the desktop, or the front window is on another display). |
| `app` | string, optional | The executable or bundle id. Informational; the editor does not use it yet. |
| `role` | string | `window` (an ordinary top-level window), `dialog` (an owned window such as Save As), `fullscreen`, or a short-lived one: `popup`, `menu`, `tooltip`. The editor never crops to short-lived ones. |
| `bounds` | object | The window's visible frame (no invisible resize borders or drop shadow) normalized to the recorded source exactly like `norm_x`/`norm_y` in `events.jsonl`: `norm_x`, `norm_y` its top-left, `norm_w`, `norm_h` its size. It may reach past 0..1 where the window is partly off the display; the editor cuts it to the display. |
| `maximized` | bool, optional | The window is maximized (shown whole, never cropped to). |

### `gap`

A stretch where the front window was not seen (the recording was paused, or the hook failed).
The editor keeps the crop it had.

```json
{"version":1,"seq":20,"t_us":61000000,"kind":"gap","reason":"recording_paused","start_us":42000000,"end_us":61000000}
```

## Platform notes for the writer

- **Windows:** `SetWinEventHook(EVENT_SYSTEM_FOREGROUND)` plus `EVENT_OBJECT_LOCATIONCHANGE`
  filtered to the foreground window, on the hook thread beside the mouse hook. Bounds from
  `DwmGetWindowAttribute(DWMWA_EXTENDED_FRAME_BOUNDS)`, converted to the recorded display and
  normalized like the mouse. Role: `menu` for `#32768`, `tooltip` for `tooltips_class32`,
  `popup` for `WS_POPUP` windows without a caption, `dialog` for owned windows,
  `fullscreen` when the box covers the display.
- **macOS:** `NSWorkspace.didActivateApplicationNotification` plus the front window's
  `kAXPositionAttribute`/`kAXSizeAttribute` (or `CGWindowListCopyWindowInfo` layer 0), in the
  display's points converted like the mouse.

## Versioning

New optional fields may be added to version 1; readers ignore fields they do not know.
A change to the meaning of an existing field, or to a rule above, needs `version: 2`.
