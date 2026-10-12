# Chat notice through Accessibility, guarded against the user's typing

- **Status:** Accepted
- **Date:** 2026-10-12
- **Card:** CAP-13 (Consent and transparency tools)

## Context

Granola posts a notice in the meeting chat once it transcribes and has
heard speech (`docs/research/granola.md` §10.3): through Accessibility in
Zoom and Teams, and through its Chrome extension in Meet. The card asks
for the same in Zoom, Teams and Meet in Chrome, off by default with a
switch per app, focusing the composer only when it is empty. Writing into
another app's chat is the first time Deino acts on another app rather than
reading it, so the rules that keep it from getting in the user's way must
be explicit: never while the user types, never into their draft, never
twice, and never in an ordinary chat.

CAP-05 (ADR 0045) already reads the three apps' call windows through
Accessibility into `AxNode` snapshots, with one `SpeakerAdapter` per app
and fixture dumps.

## Decision

- **A crate of its own, `deino-consent`**, over `deino-speaker-tags`. It
  reuses CAP-05's `Platform` (permission, running apps, `WindowReader`),
  `AxNode` and each app's `SpeakerAdapter` for "which window is a call",
  so the notice reads the same trees as speaker tags. Speaker tags keep
  their invariant (they only read); everything that writes lives here.
- **One `ChatAdapter` per app = the speaker adapter + a `ChatVocabulary`**
  (the composer's label, the send button, the chat button, the leave
  button). `view` is pure over a snapshot: not in a call, no chat, closed
  (with the chat button found beside the call's leave button, so Teams'
  app-bar "Chat" is never pressed), or open (composer, its send button
  within three levels, and whether it holds a draft). A composer counts
  only inside a window the speaker adapter reads as a call.
- **Act by path, re-found and re-checked.** An element is addressed by its
  window (title and address) and its child-index path in the snapshot;
  the reader keeps children in order and only drops trailing ones, so the
  path is the live one. The driver walks it again before every action and
  checks the role, failing with `Moved` instead of acting on something else.
- **Accessibility writes only, no synthetic keys.** Focus is `AXFocused`,
  the text is `AXValue`, sending is `AXPress` on the send button. Posting
  key events would type into whatever has focus if the user clicks away
  (and `CGEventPostToPid` is not bound safely in the crates we use, whose
  lints deny `unsafe`). A missing send button means waiting, not pressing
  Return.
- **The typing guard is the HID state's last key press**
  (`CGEventSourceSecondsSinceLastEventType(HIDSystemState, KeyDown)`):
  physical keys only, readable without a permission. Nothing is touched
  unless the last key press is at least 3 s old, checked before deciding,
  before focusing, before writing and before pressing Send. Unknown counts
  as typing.
- **Undo before give up.** A check that fails after the notice is written
  takes it back out, but only if the composer holds exactly the notice,
  then waits. Any other text is the user's: left alone, and the meeting is
  given up. Send is confirmed by the composer emptying; if it did not,
  the notice is taken back out and the meeting is given up.
- **Once per meeting, in memory.** A `Monitor` follows the recorder's
  playhead every second and keeps one `Session` per meeting; a posted or
  given-up meeting is remembered for the app's lifetime, so pauses,
  resumes and later recordings of it never post again. Driver failures
  retry up to three times. Speech means the meeting has a saved segment.
- **Degrade without a sound.** No permission, no platform, a failing read
  or write: typed `ChatError`s, logged once per meeting at debug level.
- **The reminder card is UI only**, under the record pill in CAP-10's
  panel, shaped like auto-stop's split card. The panel's window may call
  the `consent` plugin's `reminder` and `stop_reminding`, not the whole
  settings document.

## Alternatives

- **A Chrome extension for Meet** (Granola): a second codebase and an
  install step; Chromium's Accessibility, already enabled for speaker
  tags, covers the composer and its button.
- **Typing with `CGEventPostToPid` or the pasteboard and ⌘V:** targets
  focus instead of an element, can land in the wrong field, and the
  pasteboard would overwrite the user's clipboard.
- **Persisting "sent" per meeting in the database:** would survive a
  restart mid-meeting, at the cost of a table for one flag; noted as a
  follow-up if restarts during calls turn out to matter.
- **Watching key events with an event tap:** needs Input Monitoring; the
  last-key-press time needs nothing.

## Consequences

- The fixture dumps now also pin the chat (`chat-open-empty`,
  `chat-open-drafting` per app); they were modelled, not captured, and
  must be replaced by real captures during the owner's manual QA.
- Zoom remembers the chat's last recipient; the notice goes wherever the
  composer points (checked in manual QA).
- The watermark camera (a CoreMediaIO system extension) is not built: it
  needs a system extension and signing, and waits for the owner.
