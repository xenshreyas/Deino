# CAP-09 handoff: phase B

Branch `feat/CAP-09-screen`. Phase A shipped capture, snapshots, the video
with sound, settings, permissions and the start wiring
([ADR](../adr/2026-10-12-screen-capture.md), `docs/changes/CAP-09.md`).

## Shipped (for orientation)

- `crates/deino-screen`: `phash.rs`, `sampler.rs` (pure), `plan.rs`
  (`decide`), `target.rs`, `capture.rs` (`Capture::step`), `capturer.rs`
  (thread), `service.rs` (`ScreenCapture`), `macos/` (cidre adapters),
  `fake.rs`; `examples/capture_probe.rs` (manual, exits unless already
  granted).
- `deino-audio::mux` + stage `screen_video` (order 15) in `deino-pipeline`.
- `deino-core::screen`: `ScreenRepo` (`snapshots`, `screen_capture_choices`),
  `ScreenService` (`turn_off`, `turned_off`, `snapshots`).
- App: plugin `screen` (`features/screen*`), `composition/screen.rs`,
  `CaptureOnStart` in `composition/recording.rs`'s observers. UI:
  `features/screen` (Settings → Screen capture), onboarding row, mocks.

## 1. Stage `screen_context` (order 55; runs before 50 when enabled)

- New stage file in `deino-pipeline/src/stages/`, `register_stage!`; a port
  in `StagePorts` (e.g. `screen_text: Arc<dyn SnapshotReader>`).
- OCR with Apple Vision: cidre has `vn` (`VNRecognizeTextRequest`) at the
  same rev; run off the runtime (`spawn_blocking`), accurate level, `en`.
- Store text on the rows: a migration adding `ocr_text TEXT` and
  `described TEXT` (NULL = not run) to `snapshots`; skip rows done before.
- Optional `vision.describe`: a new gateway `Route` (Claude, image input)
  behind a setting (off); never send without it.
- "Runs before 50 if enabled": order 55 is after `enhance_notes`(50); either
  give it order 45 or have NOT-02 re-run. Decide with NOT-02's owner.

## 2. NOT-02 time-aligned context

- `EnhanceBasis`/prompt: include each snapshot's OCR text at its `at_ms`
  next to the transcript lines around it; cap the tokens.

## 3. UI

- Images strip on the meeting page (`features/meeting/slots.ts`):
  `snapshotsQuery(meetingId)` (`meta.tables: ['snapshots']`), thumbnails
  through `convertFileSrc` (inside `$APPDATA/recordings/**`), click to
  enlarge, delete (add `ScreenRepo::delete` + file removal).
- Record panel: "Capturing" / "Paused screen capture" from
  `useScreenStatus().capture.state`, "Turn off for this meeting"
  (`useTurnOffForMeeting`), and the window or screen picker (add a
  `CaptureTarget::Display`, a `ScreenSource::open_display`, and a plan
  input for the chosen target; manual starts can then capture).
- Consent reminder the first 3 times: a counter in settings
  (`screen.consentShown`), shown in the panel when a capture starts.

## 4. Known gaps (also in the change note)

- Resume (CAP-07) is not captured; a resumed capture must not overwrite
  `video.mp4` (write `video.2.mp4` and concatenate in `screen_video`).
- Meet in a browser: pick the call's window with CAP-05's `MeetAdapter`.
- A window closed and reopened stays paused: re-pick by bundle in
  `Capture::window_visible` when the id is gone.
- `scale()` defaults to 2.0 when `SCShareableContentInfo` is unavailable.
