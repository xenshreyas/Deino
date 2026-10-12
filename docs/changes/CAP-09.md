# CAP-09 · Screen recording, shared-screen snapshots, screen context

## Progress log (PARITY_PLAN §7.1)
| CAP-09 Screen recording, snapshots (phase A of 2) | 🟡 phase A on branch `feat/CAP-09-screen` ([ADR 2026-10-12-screen-capture](../adr/2026-10-12-screen-capture.md); new crate `deino-screen`: pure `PHash` (64-bit DCT) and `SnapshotSampler` (10 s, kept when > 10 bits changed from the last kept frame), `decide` (off by default; "Capture detected meetings"; per-meeting off; Screen Recording granted), `pick_window`/`stream_size`, `Capture::step` (recorder playhead, window visibility every 2 s → "Paused", video + snapshots on the recording's timeline), `Capturer` thread, `ScreenCapture`; macOS adapters on cidre: `SCShareableContent`, an `SCStream` of the window alone, `AVAssetWriter` H.264 at 250 kbps per fps in 10 s fragments, `CGPreflightScreenCaptureAccess`; fakes behind `test-support`. `video.part.mp4` → `video.mp4` at the stop; new stage `screen_video` (order 15) adds the mixed `audio.mp4` with `deino_audio::mux` (`AVMutableComposition`, Passthrough) and stores `video_path`. Migration `cap09_screen`: `snapshots`, `screen_capture_choices`; `deino-core::screen` (`ScreenRepo`, `ScreenService`); `settings.screen` (`detectedMeetings` false, `video`, `videoFps` 1, `snapshots`); `PermissionKind::ScreenRecording` (optional). Plugin `screen`: `status`, `open_screen_recording_settings`, `turn_off_for_meeting`, `snapshots`; `CaptureOnStart` beside `WatchOnStart`. UI: Settings → Screen capture, onboarding's Screen Recording row. Manual check on this Mac (permission already granted): a 1440 × 900 pt window captured at 2 fps for 12 s → H.264 1920 × 1204, 24 frames, a 1920 × 1204 PNG snapshot; mux of a silent H.264 + the one-on-one fixture → video + AAC tracks) | |

## Follow-ups (PARITY_PLAN §7.2)
- CAP-09 phase B: the `screen_context` stage (Vision OCR, optional `vision.describe`), NOT-02's time-aligned context, the Images strip, the 3-time consent reminder, the record panel's window or screen picker; see `docs/handoff/CAP-09.md`.
- CAP-09 phase B: a resumed recording (CAP-07 `Recorder::resume`) does not go through `StartObserver`, so it is not captured; a resumed capture would also overwrite `video.mp4`.
- CAP-09 phase B: Meet in a browser captures the browser's largest window, which may not be the call's tab window; reuse CAP-05's `MeetAdapter` to pick it.
- CAP-09 phase B: a meeting window closed and reopened (a new window id) stays "Paused"; re-pick the app's window.
- Owner QA: a full detected Zoom/Teams call with capture on: video plays in the player, a clip has sound, snapshots appear on disk, minimising the window pauses.
- EXP-02: the account export still skips `video.mp4` and `snapshots/`.

## Architecture (ARCHITECTURE.md)
- Crates: add `deino-screen` (screen capture: `PHash`, `SnapshotSampler`, `decide`, `Capture`/`Capturer`, `ScreenCapture`; ScreenCaptureKit and `AVAssetWriter` through cidre; no network, no tauri).
- Pipeline: stage `screen_video` (order 15) after `finalize_audio`: gives `video.mp4` the mixed audio, stores `video_path`.
- Files: `<recordings>/<meeting>/video.mp4` (with sound after `screen_video`) and `snapshots/<ms>.png`.

## CLAUDE.md
- Rust workspace table: `crates/deino-screen` | Screen capture (CAP-09, ADR 2026-10-12-screen-capture): the meeting window's H.264 video and perceptual-hash snapshots through ScreenCaptureKit (cidre), `Capture` on its own thread following the recorder's playhead, paused while the window is hidden; fakes behind `test-support`; never prompts for Screen Recording.
- `deino-core` row: add `screen` (CAP-09: `snapshots`, `screen_capture_choices`, the per-meeting off switch).
- `deino-audio` row: add `mux` (CAP-09: a silent video given the mixed audio, `AVMutableComposition` Passthrough).
- The architecture sketch's files: `<meeting>/{mic,system}.m4a, audio.mp4, video.mp4, snapshots/`.

## Plan updates
- CAP-09 accept "Turning it off for one meeting works" and "Paused while the window is hidden": done in phase A (unit-tested; manual QA in a real call pending). "A recorded slide deck's text shows up in the enhanced notes" waits for phase B.
- §7.2 TRN-03 row (CAP-09 must write `video_path` inside the recordings folder): closed, `video.mp4` lives in the meeting's recording folder.
- §7.2 TRN-04 row (a clip of a silent screen recording): closed by the `screen_video` stage, which gives the video its sound before any clip.
