# deino-consent

## Role

The consent tools' automatic chat message (CAP-13,
[ADR](../../docs/adr/2026-10-12-chat-notice-through-accessibility.md)):
once Deino is transcribing a meeting and has heard speech, post the user's
message in the meeting chat of Zoom, Teams or Meet in Chrome, once per
meeting, through macOS Accessibility. Off by default, one switch per app.

## Owns

- `NodePath` (`path.rs`): where an element sits under its window.
- The `ChatAdapter` trait (`chat.rs`) and one adapter per app (`apps.rs`),
  each CAP-05's `SpeakerAdapter` plus the app's chat words: what a call
  window shows of the chat (`ChatView`: not in a call, hidden, closed with
  its toggle, open with its composer and send button).
- When to post (`gate.rs`, pure): transcribing, speech heard, an empty
  composer, and no key press for `QUIET` (3 s).
- One post (`attempt.rs`): the typing guard re-checked between every step,
  taking the notice back out when it cannot be sent.
- Once per meeting (`session.rs`), following the recorder (`monitor.rs`),
  and the polling thread (`ChatNotice`, every second).
- The ports (`ports.rs`) and their macOS adapters (`macos/`): the chat
  driver (`AXFocused`, `AXValue`, `AXPress`) and the keyboard's last key
  press (`CGEventSourceSecondsSinceLastEventType`, HID state).
- Typed errors (`ChatError`), each logged once per meeting (`LoggedOnce`).

## Does Not Own

- Reading windows, the Accessibility permission and running apps:
  `deino-speaker-tags` (CAP-05). This crate reuses its `Platform`,
  `WindowReader`, `AxNode`, `SpeakerAdapter`s and fixture dumps.
- The settings (`deino-core::settings::ConsentSettings`), the recorder and
  its segments: the app wires them (`composition/consent.rs`).
- The consent reminder card and Settings → Consent:
  `frontend/src/features/consent` and the app's `consent` plugin.

## Invariants

- Never posts while the user is typing: no key press for 3 s before the
  composer is focused, before the notice is written and before Send is
  pressed; unknown key activity counts as typing.
- Never writes into a composer that holds text, and never leaves its own
  text behind: when a check fails after writing, the notice is taken back
  out (only if the composer holds exactly the notice). Anything else in
  the composer is the user's and is left alone.
- One send per meeting: a posted (or given-up) meeting is never posted to
  again, through pauses, resumes and later recordings of the same meeting
  while the app runs.
- Only a composer inside a **call** window is used, never an app's
  ordinary chats (Zoom's Team Chat, Teams' chat window).
- No key events are posted: text goes in through `AXValue`, out through
  the send button's `AXPress`. The only other write is pressing the call's
  own "Chat" button to open the panel.
- Never prompts for Accessibility and never changes a system setting.
- No dependency on `tauri`, `deino-net` or any HTTP client. Nothing logs
  the message, names, window titles or addresses.
- Everything except `macos/` builds and is tested on every platform;
  elsewhere `platform()` is `None` and nothing is posted.

## Tests

```bash
cargo test -p deino-consent   # gate rules; adapters on fixture dumps; sessions over fakes
```

`tests/chat_views.rs` reads CAP-05's dumps plus the chat ones
(`../deino-speaker-tags/fixtures/<app>/chat-*.json`); `tests/notice.rs`
runs the monitor over a fake Mac showing those dumps, a fake chat driver
and a scripted keyboard. Nothing in a test touches a real app. To capture
real chat dumps, see
[`fixtures/README.md`](../deino-speaker-tags/fixtures/README.md#chat-dumps-cap-13).

## Manual QA (real calls; owner)

Grant Deino Accessibility, turn the app on in Settings → Consent, and join
a test call with a second account or person.

- [ ] **Zoom desktop**: start recording, say something; within a few
      seconds of the first transcript line the message appears in the
      meeting chat, sent to Everyone, once. The chat panel opens if it was
      closed.
- [ ] **Meet in Chrome** (the call tab active): same check; the In-call
      messages panel opens if closed; the message is posted once.
- [ ] **Teams desktop**: same check.
- [ ] Type continuously in another app (or in Deino's notes) while speech
      starts: nothing is posted until you stop for 3 s.
- [ ] Start writing a chat message yourself first: Deino waits and never
      adds to or sends your draft.
- [ ] Pause and resume, or stop and resume the meeting: no second message.
- [ ] Every app off (the default): nothing is posted.
- [ ] Revoke Accessibility: nothing is posted, the recording is unaffected.
- [ ] `RUST_LOG=deino_consent=debug`: a missing permission or a failed post
      logs once, with no message text or names.
- [ ] Zoom's chat recipient: check the message goes to Everyone when the
      chat was last used for a direct message (see Follow-ups).
