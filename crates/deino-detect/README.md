# deino-detect

## Role

Meeting detection (CAP-06, [ADR 0021](../../docs/adr/0021-meeting-detection.md)):
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
  (`fake.rs`, feature `test-support`).

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
