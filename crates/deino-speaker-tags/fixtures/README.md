# Speaker-tag fixture dumps

Each file is one moment of a meeting app as Accessibility shows it: a JSON
array of the app's windows, each an `AxNode` tree (`src/tree.rs`). The
adapters' tests (`tests/adapters.rs`) and the end-to-end timeline test
(`deino-core`'s `tests/speaker_tags_fixtures.rs`: dumps through the
watcher, then aligned with segments) read them; nothing at runtime does.

```json
[{ "role": "AXWindow", "title": "Zoom Meeting", "children": [
  { "role": "AXGroup", "description": "Alex Chen, Computer audio unmuted, Video on, Talking" }
]}]
```

Keys are the attributes' short names (`AXDescription` is `description`,
Chromium's `AXDOMClassList` is `domClasses`, a window's `AXDocument` is
`document`); absent attributes are left out.

| Folder | App | Cases |
|---|---|---|
| `zoom/` | Zoom Workplace (`us.zoom.xos`) | gallery with Alex, Bea or the user talking, silent; screen share (no indicator); an unknown layout; the home window only |
| `teams/` | new Teams (`com.microsoft.teams2`) | a call with Alex or Bea speaking, crosstalk, the user speaking; the chat window only |
| `meet/` | Meet in Chrome (`com.google.Chrome`) | a call with Alex or Cleo speaking, silent, the user speaking; a presentation (no indicator); the lobby; an unknown layout |

The names are fictional. The first set was modelled on each app's call
window (its tile and roster labels, Meet's classes) to pin the adapters'
rules; **replace them with real captures** as described below whenever an
app changes, and during CAP-05's manual QA.

## Capturing a new dump

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
indicator in the dump and update the adapter's vocabulary: the `GRAMMAR`
words in `src/adapters/zoom.rs` and `teams.rs`, the `CLASSES` in
`src/adapters/meet.rs`, or a call-window rule. Keep the old dumps: the
adapters should read both versions while users update the app.
