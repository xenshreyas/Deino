# deino-detect

## Role

Meeting detection (CAP-06, [ADR 0023](../../docs/adr/0023-meeting-detection.md)):
notice that a call started on this Mac, decide whether it is a meeting, and
drive the prompt that offers to take notes, including the auto-start
countdown.

## Owns

- The OS signals, behind ports in `ports.rs` and macOS adapters in `macos/`:
  Core Audio process objects (`kAudioHardwarePropertyProcessObjectList`,
  `IsRunningInput`/`IsRunningOutput`/`BundleID`) with a listener per process
  object, the default input's `DeviceIsRunningSomewhere`, CoreMediaIO
  cameras (`kCMIODevicePropertyDeviceIsRunningSomewhere`, the crate's only
  `unsafe`, in `macos/cmio.rs`), running apps (`NSWorkspace`) and browser
  window URLs (Accessibility `kAXDocumentAttribute`).
- Attribution (`attribution.rs`): the outermost `.app` bundle, the FaceTime
  and iPhone call daemons, Safari's WebKit audio process, unbundled tools.
- Classification (`classify.rs`, `classify/tables.rs`): meeting apps, weak
  apps (Slack, WhatsApp), browsers and the deny list; meeting URL
  normalisation (`url.rs`).
- The decision: `resolver.rs` (Muesli's priority order) and `policy.rs`
  (dwell, cooldown, handled meetings, recording and on/off suppression, over
  a `Clock`).
- `DetectionService` (the prompt's answers, the auto-start countdown, the
  `DetectionEvent` stream) and the `Detector` thread.
- The ports the app implements (`outputs.rs`, `calendar.rs`) and their fakes
  (`fake.rs`, feature `test-support`). Which event a call belongs to is
  `deino-core`'s rule (`calendar::linking::nearest_event`, ORG-03), applied
  by the app's `CalendarLookup`.

## Does Not Own

- The prompt window, IPC and settings storage: the app's `detection` plugin
  and `deino-core`'s `DetectionSettings`.
- Recording: it asks through `RecordingStarter` (CAP-02 owns recording).
- Calendars: `CalendarLookup` is a port; ORG-01 provides events.
- Auto-stop (CAP-07 builds on `DetectionEvent::MeetingEnded`).

## Invariants

- No dependency on `tauri` or `deino-net`; it reads no audio and sends
  nothing anywhere.
- OS callbacks only wake the detector thread; all state lives on that thread
  or behind the service's mutex. Listeners are removed when their watch drops.
- Nothing logs window titles, URLs or calendar titles: prompt ids and counts only.
- Deino's own process and `com.apple.CoreSpeech` never count as meetings.
- Everything except `macos/` builds and is tested on every platform;
  elsewhere `platform_sensors()` reports `Unsupported`.

## Tests

`cargo test -p deino-detect` runs the table, resolver, policy (with
`FakeClock`), service, reader and detector-thread tests over fakes. On a Mac,
the real sensors are checked by hand:

```bash
cargo test -p deino-detect --test macos_sensors -- --ignored --nocapture
```

## Manual QA (real calls)

Run the app (or a build with another identifier), keep Settings → Meeting
detection on, and check each case. "Within 3 s" is from the moment the call
connects.

- [ ] **Zoom:** join a meeting in the Zoom app → "Meeting detected · zoom.us"
      within about 3 s; leave the call → the prompt closes about 2 s later.
- [ ] **Meet in Chrome** (Accessibility granted to Deino): join a Meet call →
      "Meeting detected · Google Chrome" within about 3 s. Without
      Accessibility, the same call prompts after about 15 s.
- [ ] **Slack huddle:** start or join a huddle → "Huddle detected · Slack".
      Recording a Slack voice clip (microphone only) does not prompt.
- [ ] **FaceTime / iPhone call** → "Call detected".
- [ ] **Take notes** brings Deino forward and starts recording; no new prompt
      for that call while it lasts.
- [ ] **Ignore** closes the prompt; the same app does not prompt again for
      10 minutes. **Always ignore this app** adds it to Settings → Meeting
      detection → Ignored apps, and it never prompts again; removing it there
      restores prompts.
- [ ] **Auto-start:** switch on "Start recording automatically" (10 s) → the
      prompt shows "Recording in 10s", the green line drains, and recording
      starts without taking focus from the call. **Don't record** cancels it.
- [ ] Unanswered prompts close after 30 s. Nothing prompts while Deino is
      recording, or with detection off.
- [ ] The prompt never takes keyboard focus from the call (type in the meeting
      chat while it is showing), appears over a full-screen Zoom window and
      on every Space, and follows light and dark mode.
- [ ] Dictation (Handy, Superwhisper), QuickTime and Siri never prompt.
