# Speaker-tag fixture dumps

Each file is one moment of a meeting app as Accessibility shows it: a JSON
array of the app's windows, each an `AxNode` tree (`src/tree.rs`). The
adapters' tests (`tests/adapters.rs`) and the end-to-end timeline test
(`deino-core`'s `tests/speaker_tags_fixtures.rs`: dumps through the
watcher, then aligned with segments) read them; nothing at runtime does.

```json
[{ "role": "AXWindow", "title": "Zoom Meeting", "children": [
  { "role": "AXTabGroup", "description": "Bea Ruiz, Computer audio unmuted",
    "frame": { "x": 0, "y": 93, "width": 1261, "height": 709 },
    "children": [{ "role": "AXImage" }, { "role": "AXButton", "description": "Bea Ruiz" }] }
]}]
```

Keys are the attributes' short names (`AXDescription` is `description`,
Chromium's `AXDOMClassList` is `domClasses`, a window's `AXDocument` is
`document`, `AXPosition` and `AXSize` together are `frame`); absent
attributes are left out, so dumps made before `frame` existed still read.

| Folder | App | Cases |
|---|---|---|
| `zoom/` | Zoom Workplace (`us.zoom.xos`) | **real captures** (`real-*.json`): Speaker View with the remote talking, Gallery View with the remote unmuted, everyone muted; hand-modelled: an unknown layout, the home window only, and `gallery-alex-talking`/`screen-share-no-indicator`, kept only for deino-consent's chat tests (no tile Zoom really shows) |
| `teams/` | new Teams (`com.microsoft.teams2`) | a call with Alex or Bea speaking, crosstalk, the user speaking; the chat window only |
| `meet/` | Meet in Chrome (`com.google.Chrome`) | a call with Alex or Cleo speaking, silent, the user speaking; a presentation (no indicator); the lobby; an unknown layout |

The names are fictional. The Zoom `real-*` dumps are anonymised captures
of Zoom Workplace on macOS (October 2026) from Deino's own Accessibility
grant: the user is "Alex Chen" (signed in, shown by the Zoom Workplace
window's account button), the one remote participant "Bea Ruiz"; the
meeting address is `https://example.zoom.us/j/00000000000`, IDs and
passcodes are zeros, and the calendar sidebar's content was removed. The
capture had no frames, so the Speaker View dump's two tile frames were
added by hand from the screen-reading tool (the remote 1261×709, the
user's thumbnail 92×52); the other two have none. The rest was modelled on
each app's call window (its tile and roster labels, Meet's classes) to pin the adapters'
rules; **replace them with real captures** as described below whenever an
app changes, and during CAP-05's manual QA. The easiest way is to let Deino
save the window during a real call (next section).

## What real Zoom exposes (October 2026)

- The call window is titled "Zoom Meeting". Each video tile is an
  `AXTabGroup` described "Name, Computer audio muted|unmuted[, Video off]",
  holding an `AXImage` and an `AXButton` described with the name.
- **There is no talking indicator**, in Speaker View or Gallery View. In
  Speaker View the active speaker gets the large tile; the adapter reads
  tile frames to find it, else takes the one unmuted remote participant.
- No tile marks the user. The "Zoom Workplace" window's account button is
  "Zoom, Own Name, In a Zoom Meeting, Pro account"; an open Participants
  panel lists "Own Name (Host, me)".

## Capturing a new dump with Deino's launch flag

Deino can dump a running app's windows with its own grant:

```bash
open -n -a /Applications/Deino.app --args --dump-call-window zoom /tmp/zoom.json
```

(`zoom`, `teams` or `meet`; the flag is `frontend/src-tauri/src/call_window_dump.rs`
on main.) Then anonymise the file and add it as in steps 4 and 5 of the
terminal method below.

## Capturing a new dump from Deino (no terminal grant)

Deino already has Accessibility, so it can save the window itself:

1. Turn on Settings → Advanced → Developer → Diagnostics → "Save the call
   window when speaker names can't be read" (`developer.captureCallWindows`).
2. Record a real call with at least two other people. When Deino cannot
   read the app's call window (a layout its adapter does not know, or no
   call window 20 s into the recording), it saves that app's windows once
   per meeting to
   `~/Library/Logs/com.xenshreyas.deino/call-window-<app>-<meeting id>-<UTC time>.json`,
   in this folder's format with every attribute, and logs the file name.
3. Turn the setting off again; the files hold participants' real names.
4. Anonymise the file and add it here as in steps 4 and 5 below.

## Capturing a new dump from a terminal

1. Grant Accessibility to your terminal: System Settings → Privacy &
   Security → Accessibility. The tool checks without prompting and stops if
   it is missing.
2. Join a call in the app with at least two other people, in the layout you
   want to capture, and have someone speak.
3. Dump the app's windows (`zoom`, `teams` or `meet`; for Meet, the call
   must be the active tab of its Chrome window):

   ```bash
   DEINO_AX_APP=zoom DEINO_AX_OUT=/tmp/zoom.json \
     cargo test -p deino-speaker-tags --test capture -- --ignored --nocapture
   ```

4. **Anonymise it before committing**: replace every real name, email,
   meeting title, room code and URL with fictional ones (keep the Meet
   address shape `https://meet.google.com/abc-defg-hij`), and delete windows
   that have nothing to do with the call (keep one non-call window, such as
   Gmail beside Meet, to show it is ignored).
5. Save it as `<app>/<layout>-<who>-<state>.json`, for example
   `zoom/speaker-view-alex-talking.json`, and add a test in
   `tests/adapters.rs` with what the window showed.

## When a test fails on a new dump

The app changed its UI. Find the participant tiles and the speaking
indicator in the dump and update the adapter's vocabulary: Zoom's tile
role and states in `src/adapters/zoom/tiles.rs`, the `GRAMMAR` words in
`teams.rs`, the `CLASSES` in
`src/adapters/meet.rs`, or a call-window rule. Keep the old dumps: the
adapters should read both versions while users update the app.

## Chat dumps (CAP-13)

`deino-consent` reads these dumps too, to find each app's meeting chat
(`crates/deino-consent/tests/chat_views.rs` and `tests/notice.rs`). The
call dumps above show the chat **closed** (its "Chat" button beside the
call's leave button); `chat-*.json` add the chat panel:

| Dump | What it shows |
|---|---|
| `<app>/chat-open-empty.json` | the call with the chat panel open: an empty composer and its send button. Zoom's also has a Team Chat composer in its home window, and Teams' its main chat window: neither is a call, so neither may be used |
| `<app>/chat-open-drafting.json` | the same, with the user's own text in the composer |

They were modelled on each app's chat panel (Zoom's "Type message here..."
and "Send", Teams' "Type a message" and "Send", Meet's "Send a message"
field and button). To capture real ones, follow the steps above with the
call's chat panel open:

1. Open the meeting chat (Zoom: Chat; Teams: Chat in the meeting controls;
   Meet: Chat with everyone) and leave the composer empty, or type a
   draft for a `chat-open-drafting` dump. **Do not send anything.**
2. Run the capture tool as above. It records the composer's text, so
   replace a real draft with fictional text when anonymising.
3. Save it as `<app>/chat-open-<state>.json` and run
   `cargo test -p deino-consent`. When a test fails, update the app's
   `ChatVocabulary` in `crates/deino-consent/src/apps.rs` (the composer's
   label, the send and chat buttons, the leave button) and keep the old dump.
