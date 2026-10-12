# Raycast and Apple Shortcuts: drive Deino from anywhere

Deino answers `deino://` links and has system-wide hotkeys (DEV-05). With
them, Raycast script commands and Apple Shortcuts can start and stop
recording, open a new note, search, and jump to a meeting, while the
`deino` command reads your meetings for them. There are no native App
Intents; everything below goes through links and the command-line tool.

## Hotkeys (no setup)

Deino → **Settings → Shortcuts**. They work in every app, even when Deino is
in the background:

| Hotkey | Default | What it does |
|---|---|---|
| Start or stop recording | ⌃⌥⌘R | Starts a recording, or stops the one running |
| Quick note | ⌃⌥⌘N | Brings Deino forward on a new note |
| Ask | ⌃⌥⌘J | Opens Ask on the open meeting, or brings Deino forward |

**Change**, then press the new combination (Esc cancels). A hotkey needs two
modifier keys, one of them ⌘, ⌃ or ⌥ (F1–F20 need none), so it never takes
a plain key from other apps. Deino refuses macOS's own shortcuts, another
Deino hotkey's combination, and one another app already holds; the old
hotkey keeps working and the reason shows under the row.

## The links

| Link | What it does |
|---|---|
| `deino://meeting/<id>` | Opens the meeting |
| `deino://meeting/<id>?t=<ms>` | Opens the meeting's transcript at that moment (milliseconds from the start) |
| `deino://new-note` | A new note |
| `deino://new-note?event=<calendar event id>` | The calendar event's note, created and linked if it is new |
| `deino://record/start` | Starts recording a new meeting |
| `deino://record/stop` | Stops the recording; its notes are written as usual |
| `deino://search?q=<words>` | Searches your library (URL-encode the words) |
| `deino://recipe?data=<recipe>` | Imports a shared recipe. Recipes is not built yet: Deino says so |

- Exports, the `deino` command, Claude (MCP) and transcript **Copy link**
  write `deino://meeting/<id>`. Older exports wrote `deino://meetings/<id>`;
  Deino opens those too.
- **App lock:** while Deino is locked, recording links still work, but a link
  that would show your library waits on the lock screen and opens as soon as
  you unlock.
- A link Deino cannot read (a typo, a missing `q`) shows "Deino could not
  open that link" and does nothing else.

Try one from Terminal:

```sh
open "deino://search?q=release%20plan"
```

The links need the installed app (`/Applications/Deino.app`); macOS learns
the scheme from it. A `pnpm run tauri:dev` build does not register it.

## Raycast script commands

Raycast → **Settings → Extensions → Script Commands → Add Directories**, pick
a folder (for example `~/raycast-scripts`), and save each script below into
it with `chmod +x`. Raycast reads the `@raycast` comments.

**Start or stop recording** (`deino-record.sh`):

```sh
#!/bin/bash
# @raycast.schemaVersion 1
# @raycast.title Deino: Start Recording
# @raycast.mode silent
# @raycast.icon 🎙️
open "deino://record/start"
```

Copy it as `deino-stop.sh` with the title `Deino: Stop Recording` and
`deino://record/stop`.

**Search Deino** (`deino-search.sh`), with an argument:

```sh
#!/bin/bash
# @raycast.schemaVersion 1
# @raycast.title Deino: Search
# @raycast.mode silent
# @raycast.icon 🔎
# @raycast.argument1 { "type": "text", "placeholder": "words" }
query=$(python3 -c 'import sys, urllib.parse; print(urllib.parse.quote(sys.argv[1]))' "$1")
open "deino://search?q=${query}"
```

**Open the latest meeting** (`deino-latest.sh`), through the `deino` command
(install it from **Settings → Developer**; see [claude-code.md](claude-code.md)):

```sh
#!/bin/bash
# @raycast.schemaVersion 1
# @raycast.title Deino: Open Latest Meeting
# @raycast.mode silent
export PATH="$HOME/.local/bin:/usr/local/bin:$PATH"
link=$(deino --json meetings -n 1 | python3 -c 'import json, sys; print(json.load(sys.stdin)["meetings"][0]["link"])')
open "$link"
```

**Today's action items** (`deino-todo.sh`), shown in Raycast:

```sh
#!/bin/bash
# @raycast.schemaVersion 1
# @raycast.title Deino: My Open Action Items
# @raycast.mode fullOutput
export PATH="$HOME/.local/bin:/usr/local/bin:$PATH"
deino action-items --assignee me --status open
```

## Apple Shortcuts

Links open from the **Open URLs** action; the `deino` command runs from **Run
Shell Script**.

**"Record this meeting"** (good for a Stream Deck button or Siri):

1. Shortcuts → **New Shortcut**, name it *Record this meeting*.
2. Add **URL** with `deino://record/start`.
3. Add **Open URLs**.

Make *Stop recording* the same way with `deino://record/stop`.

**"Search Deino"**:

1. Add **Ask for Input** (Text, prompt "Search Deino for…").
2. Add **URL Encode** on *Provided Input*.
3. Add **Text** with `deino://search?q=` followed by the *URL Encoded Text*
   variable.
4. Add **Open URLs**.

**"Open my last meeting"**:

1. Add **Run Shell Script** (shell `zsh`, input *nothing*) with:

   ```sh
   export PATH="$HOME/.local/bin:/usr/local/bin:$PATH"
   deino --json meetings -n 1 | python3 -c 'import json, sys; print(json.load(sys.stdin)["meetings"][0]["link"])'
   ```

2. Add **Open URLs** on the *Shell Script Result*.

Give any of them a keyboard shortcut in the shortcut's **Details**, or add
them to the menu bar. The first run asks to allow opening Deino; allow it.

## Troubleshooting

- **Nothing happens on a link:** open Deino once from `/Applications` so
  macOS registers `deino://`, then try `open "deino://new-note"` in Terminal.
- **A hotkey does nothing:** Settings → Shortcuts shows whether each one is
  on, and why not (usually another app holds the combination; pick another).
- **The `deino` command is not found** in Raycast or Shortcuts: they do not
  read your shell profile, so the scripts above set `PATH` themselves.
