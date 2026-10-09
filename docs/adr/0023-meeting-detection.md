# 0023. Detect meetings from Core Audio process objects, decided by a pure policy

- **Status:** Accepted
- **Date:** 2026-10-09
- **Card:** CAP-06 (`docs/references/audio-capture.md` §6, §10;
  `docs/references/audio-muesli-openwhispr.md`; `docs/references/audio-anarlog.md` §7)

## Context

Granola and Circleback offer to take notes when a call starts (Granola §2.3:
per-app titles, a 15-minute calendar window; Circleback §3.5: an ignore list
and a countdown auto-start). Deino inherited `audio/system_detector.rs`,
which listened to the default *output* device, attributed nothing reliably,
was never wired up, and violated every §3A rule. The references disagree on
details: OpenWhispr listens to process objects, anarlog polls them every
second with a 15 s hold and a 10-minute cooldown, Muesli combines process
attribution, camera, browser URLs and a calendar under a priority order and a
3 s stability rule. A meeting prompt must appear within about 3 s, must not
fire for dictation apps or a two-second Siri request, and must never steal
focus from the call.

## Decision

- **A crate of its own, `deino-detect`** (§7.0): heavy macOS-native code, no
  `tauri`, no `deino-net` (checked by `scripts/check-dependency-rules.sh`).
- **Signals through ports, listeners first.** `Sensor<T>` (a reading plus a
  `Watch`) for Core Audio process objects (a listener on the process list and
  on each object's `IsRunningInput`/`IsRunningOutput`), the default input's
  `DeviceIsRunningSomewhere`, and CoreMediaIO cameras; `AppDirectory`
  (`NSWorkspace`/`NSRunningApplication`) and `BrowserTabs` (Accessibility
  `kAXDocumentAttribute`, only when granted). OS callbacks only wake one
  detector thread, which re-reads everything (OpenWhispr's reconcile); a
  5 s heartbeat catches a missed notification, and a 1 s poll replaces a
  listener that cannot register (anarlog).
- **CoreMediaIO through `objc2-core-media-io`**, the objc2-family binding at
  the pinned objc2 version (`audio-capture.md` §6 asks to prefer it), wrapped
  by a small module that holds the crate's only `unsafe`.
- **Attribution to the outermost `.app`**, relabelling Apple's call daemons
  (anarlog) and crediting Safari's WebKit audio process to Safari; Deino's
  own pid and `com.apple.CoreSpeech` never count (OpenWhispr).
- **Muesli's resolver order**, with two deliberate changes: a calendar event
  only links to and strengthens a candidate, and weak evidence (a browser
  without a known room, an unknown app) ranks last with a 15 s dwell, so it
  cannot delay a real meeting app. Slack and WhatsApp count only in full
  duplex or as the frontmost app with camera and microphone.
- **A pure policy over `Clock`**: 2.5 s dwell for strong evidence (the card's
  3 s target), 15 s for weak; 2 s end grace; the prompt closes after 30 s;
  "Ignore" cools the app down for 10 minutes; an answered, timed-out or
  recorded meeting stays quiet until it ends; nothing prompts while
  recording or switched off.
- **The prompt is a non-activating `NSPanel`** (`tauri-nspanel`, Handy's
  overlay pattern) at status level on every Space, in its own Vite page with
  its own capability that grants only the `detection` plugin.
- **Recording goes through a `RecordingStarter` port.** Until CAP-02's typed
  `recording.start`, the app adapter asks the main window's shell to start a
  recording (the sidebar/⌘⇧R/tray path); "Take notes" brings the window
  forward, an auto-start does not.

## Alternatives

- **Poll `System::processes()` every second (anarlog).** Simpler, but it
  costs a full enumeration every second all day and adds up to a second of
  latency; listeners plus a heartbeat are both cheaper and faster.
- **`AVCaptureDevice` for the camera.** Its discovery installs main-queue HAL
  listeners that Muesli saw stall on AirPods; the DAL API does not.
- **Hand-written CoreMediaIO `extern` blocks.** Equivalent, but they would
  duplicate signatures the objc2 crate already generates and keeps correct.
- **A plain always-on-top Tauri window.** Showing it can make Deino's window
  key and clicking it activates the app, taking focus from the call.
- **Starting the legacy recorder directly from Rust.** The record screen
  still owns the meeting title, transcript state and the stop-and-save flow,
  so a Rust-started recording would not be saved correctly.

## Consequences

- Detection costs listeners and a 5 s heartbeat at idle, and one snapshot
  (a few ms) per audio change.
- Browser meetings are recognised by URL only with Accessibility granted
  (CAP-05 requests it); without it they prompt after the 15 s hold.
- QuickTime and other recorders are on the deny list: recording a voice memo
  is not a meeting. Users can add any app to the ignore list; there is no
  "include" list yet.
- CAP-07 builds auto-stop on `DetectionEvent::MeetingEnded`; ORG-01 replaces
  `NoCalendar` with an adapter over `calendar_events`; CAP-02 replaces the
  shell recording adapter. CAP-10's record pill can reuse the pinned
  `tauri-nspanel` and the panel pattern in `features/detection/prompt_panel.rs`.
