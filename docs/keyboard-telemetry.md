# Keyboard telemetry (`telemetry/keys.jsonl`), version 1

The contract between the AeroShoot recorder (writer) and AeroEdits (reader) for keyboard
activity in an `.aero` recording. It sits next to the mouse stream (`telemetry/events.jsonl`)
and uses the same clock.

**What the editor does with it:** typing and shortcuts count as screen activity, so the
webcam no longer fills the frame while you type with a resting mouse, and short scenes keep
the screen. Typing stretches become zoom targets. Shortcuts can show as a small badge
("Ctrl + Shift + P").

**Privacy rule:** the file never holds typed text. Typing is stored as stretches with a key
count only. Only shortcuts name a key, and only keys pressed with Ctrl, Alt or Win/Cmd held,
or F-keys.

## File

- Path: `<project>.aero/telemetry/keys.jsonl`. UTF-8, one JSON object per line, `\n` endings.
- Written incrementally and flushed on the same cadence as `events.jsonl`, so a crash loses at
  most the last flush. The editor ignores a truncated final line.
- No file means "keyboard not recorded": the editor falls back to mouse-only behaviour.
  The recorder writes no file when keyboard recording is switched off.
- Typical size: a few KB per hour.

## Common fields

Every record has:

| Field | Type | Meaning |
|---|---|---|
| `version` | `1` | Format version. The editor ignores the whole file for any other value. |
| `seq` | u64 | 0 for the first record, then +1 per record. Its own counter, separate from `events.jsonl`. |
| `t_us` | u64 | Microseconds on the recording clock: the same clock as `events.jsonl` `t_us` (0 = recording start after the countdown; paused time is included and written as a `gap`). |
| `kind` | string | One of the kinds below. The editor skips kinds it does not know. |

## Kinds

### `capture` (exactly one, the first record)

What this recorder captures. Written at recording start (`t_us` 0, `seq` 0).

```json
{"version":1,"seq":0,"t_us":0,"kind":"capture","platform":"windows","shortcuts":true,"typing":true,"caret":true}
```

| Field | Type | Meaning |
|---|---|---|
| `platform` | `"windows"` \| `"macos"` | Lets the editor draw Win vs ⌘ and Alt vs ⌥. |
| `shortcuts` | bool | `shortcut` records are written. |
| `typing` | bool | `typing` records are written. |
| `caret` | bool | The recorder tries to report caret positions. Some apps never expose one, so `true` still allows records without them. |

### `shortcut`

One shortcut press, at its key-down time.

```json
{"version":1,"seq":12,"t_us":8412000,"kind":"shortcut","modifiers":["ctrl","shift"],"code":"KeyP","label":"P","focus_inside_source":true}
```

| Field | Type | Meaning |
|---|---|---|
| `modifiers` | string[] | Held modifiers, in the order `ctrl`, `alt`, `shift`, `meta` (Win on Windows, Cmd on macOS). Left/right not distinguished. |
| `code` | string \| null | The physical key as a W3C `KeyboardEvent.code` (`KeyP`, `Digit1`, `Slash`, `Enter`, `F5`, `ArrowUp`, `Escape`, …). `null` for a modifier tapped alone. |
| `label` | string | What the key shows in the active layout, ready to draw: letters in upper case (`P`), symbols as typed without Shift (`/`), named keys in English (`Enter`, `Esc`, `Tab`, `Space`, `Backspace`, `Delete`, `Up`, `Down`, `Left`, `Right`, `Home`, `End`, `PgUp`, `PgDn`, `F5`). Empty for a modifier tapped alone. |
| `focus_inside_source` | bool, optional | The window receiving the key is in the recorded source: on the recorded display, or the recorded window itself. Omit if unknown. The editor ignores shortcuts with `false`. |

A key-down is written as a `shortcut` when, and only when:

1. a non-modifier key goes down while `ctrl`, `alt` or `meta` is held (Shift may be held too), or
2. F1–F24 go down (alone or with modifiers), or
3. `meta` is pressed and released with no other key in between, within 1 s (opening the
   Start menu or Spotlight). It is written at the release with `modifiers: ["meta"]`,
   `code: null`, `label: ""`.

Never a `shortcut`:

- **AltGr.** Windows reports AltGr as LeftCtrl plus RightAlt; keys typed with it are typing.
  This matters on Greek, German and other layouts.
- Shift plus a character (typing).
- Auto-repeat: a held key writes one `shortcut` at its first down only.
- The recorder's own hotkeys (start, stop, pause): nothing is written for them.
- Anything during the countdown or while paused.

### `typing`

A run of typing, written once the run ends.

```json
{"version":1,"seq":13,"t_us":15230000,"kind":"typing","start_us":12010000,"end_us":15230000,"keys":38,"caret_start":{"geometry_id":"g1","norm_x":0.412,"norm_y":0.633},"caret_end":{"geometry_id":"g1","norm_x":0.598,"norm_y":0.633},"focus_inside_source":true}
```

| Field | Type | Meaning |
|---|---|---|
| `start_us` | u64 | Key-down time of the run's first key. |
| `end_us` | u64 | Key-down time of the run's last key (not the idle timeout after it). Equals `t_us`. |
| `keys` | u32 | Key-downs in the run, auto-repeat excluded. Never the keys themselves. |
| `caret_start`, `caret_end` | object, optional | The text caret at the first and the last key, when the OS reports one. `geometry_id`, `norm_x` and `norm_y` mean exactly what they mean in `events.jsonl` (normalized to the recorded source under that geometry). Omit when the caret is unknown or outside the source. |
| `focus_inside_source` | bool, optional | As for `shortcut`, taken at the first key. |

Typing keys: characters (with or without Shift or AltGr), `Space`, `Enter`, `Tab`,
`Backspace`, `Delete`, arrows, `Home`, `End`, `PageUp`, `PageDown` and `Insert`, pressed
without Ctrl, Alt or Win/Cmd.

- A run ends when no typing key goes down for **500 ms**, or at pause, stop, or a `gap`.
  The editor joins runs itself where it wants longer ones, so keep this short.
- A `shortcut` in the middle of a run does not end the run.
- A run with a single key is still written (`keys: 1`).
- Records stay in `seq` order. A `typing` record's `t_us` is its `end_us`, so a `shortcut`
  pressed during the run can have a later `seq` but an earlier `t_us` than the run. The
  editor sorts by time.

### `gap`

A stretch where the keyboard state is unknown. The editor treats it as "might be typing",
never as "no typing". The exception is `hook_unavailable`: there the editor uses the mouse
alone, as if the keyboard had not been recorded, so a missing permission never blocks the
webcam for a whole recording.

```json
{"version":1,"seq":20,"t_us":61000000,"kind":"gap","reason":"recording_paused","start_us":42000000,"end_us":61000000,"dropped_events":0}
```

| `reason` | When |
|---|---|
| `recording_paused` | The recording was paused (written on resume or at stop). |
| `hook_unavailable` | The keyboard hook or event tap could not be installed or was removed. Written from that time to the end (or to when it came back). On macOS this includes missing Input Monitoring permission. |
| `queue_overflow` | Events were dropped between the hook and the writer. `dropped_events` is how many. |
| `secure_input` | macOS secure event input was on (password fields), so keys were not seen. |

## Platform notes for the writer

- **Windows:** `WH_KEYBOARD_LL` on the same kind of hook thread as the mouse hook
  (`capture/windows/record/mouse.rs`), with the same `RecordingClock`, pause flag and
  dropped-event counter pattern. `LLKHF_INJECTED` keys are recorded like any other key.
  Caret: `GetGUIThreadInfo` on the foreground thread (`rcCaret`, converted to screen
  coordinates, then normalized like the mouse). Many apps (Chromium, Electron) report no
  caret; then omit it.
- **macOS:** a listen-only `CGEventTap` for `keyDown` and `flagsChanged`, beside the mouse tap
  in `MouseHookMac.swift`. Caret: the focused element's `AXSelectedTextRange` bounds through
  Accessibility, when permitted; otherwise omit it.
- Labels come from the active layout at the time of the press (`ToUnicodeEx` with modifiers
  cleared on Windows, `UCKeyTranslate` on macOS).

## Versioning

New optional fields may be added to version 1; readers ignore fields they do not know.
A change to the meaning of an existing field, or to a rule above, needs `version: 2`.
