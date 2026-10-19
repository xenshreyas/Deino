# deino-speaker-tags

## Role

Speaker tags (CAP-05, [ADR 0045](../../docs/adr/0045-speaker-tags-from-the-meeting-apps-accessibility-tree.md)):
while Deino records, read who the meeting app shows as the active speaker
(Zoom, Teams, Meet in Chrome, Brave, Edge, Arc or Vivaldi) through macOS Accessibility every 250 ms, and
store it as `(start, end, name)` spans on the recording's timeline. The
`identify_speakers` stage (`deino-core::speaker_tags`) turns the spans into
names for the system track's segments before it matches voices.

## Owns

- `AxNode` (`tree.rs`): the snapshot of an app's window every adapter
  reads, live or from a fixture dump.
- The `SpeakerAdapter` trait (`adapter.rs`) and one adapter per app
  (`adapters/`): Zoom and Teams over a description scanner
  (`described.rs`), Meet over a CSS-class scanner (`classed.rs`).
- Labels without decorations (`label.rs`), readings and their typed errors
  (`reading.rs`), the poll-to-span `Timeline` (`timeline.rs`).
- `Watch` (one recording's polls, `watch.rs`), `Watcher` (its thread,
  `watcher.rs`), `SpeakerTags` (the app's handle, `service.rs`) and the
  Settings rows (`status.rs`).
- The ports (`ports.rs`) and their macOS adapters (`macos/`):
  `AXIsProcessTrusted`, `NSRunningApplication`, `AXUIElement` trees,
  Chromium's `AXManualAccessibility` and `AXDOMClassList`, and opening the
  Accessibility page of System Settings.
- The fixture dumps (`fixtures/`, with how to capture new ones), the
  capture tool (`tests/capture.rs`) and the opt-in dump of a call window
  the watch cannot read (`capture.rs`: `WindowCapture`, once per meeting
  and app; the app writes it to the log folder).

## Does Not Own

- Storing spans and using them: `deino-core::speaker_tags` (the
  `speaker_tag_spans` table, alignment, seats, the plan) and
  `deino-core::diarization::identify` (CAP-04's stage).
- When a recording starts and where it is: the app wires a start observer
  and a `RecordingClock` over `deino-recording`'s `Playhead`.
- IPC and the Settings page: the app's `speaker_tags` plugin and
  `frontend/src/features/speaker-tags`.

## Invariants

- No dependency on `tauri`, `deino-net` or any HTTP client; nothing leaves
  the Mac.
- Never prompts for Accessibility and never changes a system setting; the
  only call that touches another app is `AXManualAccessibility` on Chrome,
  which asks Chrome to expose its web content (as a screen reader does).
- Polls run on the watcher's own thread, never on capture, encoder or
  transcription threads; every Accessibility request has a 100 ms timeout
  and a window read is capped at 3,000 elements.
- Every failure degrades to voice matching: a typed `TagError`, logged once
  at debug level; nothing is raised to the recording or the user.
- Nothing logs names, window titles or addresses: meeting IDs and counts only.
  A call-window dump holds names, so it is taken only when the user turned
  the setting on, and handed to the port, never logged.
- Everything except `macos/` builds and is tested on every platform;
  elsewhere `platform()` is `None` and every app is Unsupported.

## Tests

```bash
cargo test -p deino-speaker-tags                          # unit tests, every adapter on the fixture dumps
cargo test -p deino-core --test speaker_tags_fixtures    # dumps → watcher → spans → won segments (> 50 %)
```

The capture tool reads a real app and is ignored by default; see
[`fixtures/README.md`](fixtures/README.md).

## Manual QA (real calls)

Grant Deino Accessibility (Settings → Speaker tags → Open System Settings),
record each call with at least two other people, then open the meeting's
transcript after processing. Count the system-channel segments of speakers
who talked for more than 3 s: at least 90 % must carry the right name.

- [ ] **Settings → Speaker tags** shows Zoom, Teams and Meet as "Needs
      permission" before granting, "Enabled" after (reopen the page), and
      the button opens Privacy & Security → Accessibility without changing
      anything. In light and dark mode.
- [ ] **Zoom desktop**, gallery view: three people take turns → each turn's
      segments carry the speaker's Zoom name; ≥ 90 % right.
- [ ] **Zoom**, speaker view and while someone shares a screen: names
      continue where Zoom shows a speaking tile; stretches without one fall
      back to "Speaker N" (no wrong names).
- [ ] **Meet in Chrome** (the call tab active in its window): same check,
      ≥ 90 % right. Chrome stays responsive (Activity Monitor: Deino's CPU
      during the call is a few percent).
- [ ] **Teams desktop**: same check.
- [ ] A participant who is also a calendar invitee or in People gets that
      person (the name links to their page); an unknown name becomes the
      speaker's label, and renaming it in the Participants panel works.
- [ ] Your own talking never names a remote speaker.
- [ ] Revoke Accessibility mid-call: the recording continues, the rest of
      the meeting is named by voice, nothing errors.
- [ ] Pause and resume mid-call: names stay aligned after the pause.
- [ ] Crosstalk: overlapping speech is not given one person's name.
- [ ] `RUST_LOG=deino_speaker_tags=debug`: a missing permission or an
      unknown layout logs once, with no names in the log.
