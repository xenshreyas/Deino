# deino-screen

## Role

Screen capture while Deino records a detected meeting (CAP-09,
[ADR](../../docs/adr/2026-10-12-screen-capture.md)): the meeting window as
an H.264 video and snapshots of what is shared in it, through
ScreenCaptureKit.

## Owns

- `Frame` (owned BGRA) and `FrameRate`; `PHash` (64-bit DCT perceptual
  hash) and `SnapshotSampler` (every 10 s, kept when more than 10 bits
  changed from the last kept frame); PNG encoding.
- `decide`: whether a recording captures (off by default, detected
  meetings only, per-meeting off switch, Screen Recording granted).
- `pick_window`, `stream_size`, `is_visible`; the record panel's picker
  (`pick`: `CaptureTarget`, `Source` = a window or a whole display,
  `choices`, `resolve`) and `picked`, its plan.
- `recognize_text`: Apple Vision OCR of a snapshot (cidre `vn`), for the
  `screen_context` stage (CAP-09 phase B).
- `Capture` (one recording, step by step, on the recorder's timeline),
  `Capturer` (its thread), `ScreenCapture` (the app's handle, `status`).
- The ports `ScreenSource`/`FrameFeed`, `Windows`, `VideoEncoders`,
  `ScreenPermission`, `RecordingClock`, `SnapshotStore`; the macOS
  adapters (cidre: `SCShareableContent`, `SCStream`, `AVAssetWriter`,
  `CGPreflightScreenCaptureAccess`); fakes behind `test-support`.
- The file names `video.part.mp4`, `video.mp4`, `snapshots/<ms>.png`.

## Does not own

- The rows (`deino-core::screen`), the settings (`settings.screen`), the
  audio in the video (`deino_audio::mux`, stage `screen_video`), when a
  recording starts or stops (`deino-recording`).

## Invariants

- Never prompts: ScreenCaptureKit is only touched after
  `ScreenPermission::granted`; `request_screen_recording` is for the user's
  own click only.
- Nothing runs on an audio or runtime thread; failures are logged and never
  stop a recording.
- Frames are dropped while the recording is paused or the window is not on
  screen. Pixels and window titles are never logged; nothing leaves the Mac.
