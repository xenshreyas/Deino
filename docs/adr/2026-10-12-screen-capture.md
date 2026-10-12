# Capture the meeting window with ScreenCaptureKit, and give its video the meeting's mixed audio after the recording

- **Status:** Accepted
- **Date:** 2026-10-12
- **Card:** CAP-09 (screen recording, shared-screen snapshots, screen
  context), phase A; `docs/research/granola.md` §2.10,
  `docs/research/circleback.md` §3.5 and §5, ADR 0026, ADR 0040, ADR 0048

## Context

CAP-09 records the meeting window as a video (`video.mp4`, H.264 at 1 fps
by default) and keeps snapshots of what is shared in it, for the notes
(`screen_context`, phase B). Three constraints shaped it:

- **Playback and clips.** TRN-03 plays `video_path` muted, following the
  audio, and only inside `$APPDATA/recordings/**` (ADR 0040). TRN-04 cuts a
  clip of a meeting with a video as a passthrough cut of that file
  (ADR 0048): a silent video gives silent clips, and a downloaded video
  would be silent too.
- **Audio.** System audio comes from the Core Audio process tap, not
  ScreenCaptureKit (§3C keeps SCK audio as a fallback only); the recorder
  writes crash-safe CMAF tracks and `finalize_audio` writes the mixed
  `audio.mp4` after the stop (ADR 0014, ADR 0026).
- **Permissions.** Touching ScreenCaptureKit without Screen Recording shows
  the macOS prompt. A capture starts from a background start (a detected
  call), where a prompt would be wrong.

## Decision

- **A new crate, `deino-screen`,** with the rules pure and the platform
  behind ports (`ScreenSource`/`FrameFeed`, `Windows`, `VideoEncoders`,
  `ScreenPermission`, `RecordingClock`, `SnapshotStore`) and fakes, like
  `deino-speaker-tags`. macOS adapters use **cidre** (already in the tree
  at rev a9587fa, MIT, with `sc` and `av` built): `SCShareableContent`,
  an `SCStream` of the window alone (`desktopIndependentWindow`, BGRA,
  `minimumFrameInterval` = 1/fps, scaled to fit 1920 px), `AVAssetWriter`
  with a pixel buffer adaptor (H.264, 250 kbps per fps, a key frame every
  10 frames, 10 s movie fragments). `objc2-screen-capture-kit` was not
  added: cidre already binds what we need and shares its build.
- **One frame stream feeds both outputs.** A capture thread steps each
  frame through `Capture::step`: it asks the recorder's playhead where the
  recording is (paused or stopped: drop), checks every 2 s that the window
  is on screen (minimised, hidden or closed: drop, "Paused screen
  capture"), appends the frame to the video at its timeline position, and
  every 10 s hashes it (a 64-bit DCT pHash) and keeps it as a PNG plus a
  `snapshots` row when it differs from the last kept one by more than 10
  bits. Video and snapshots share the recording's timeline, pauses
  skipped, like the transcript.
- **The video is written silent, then given the meeting's mixed audio.**
  The capture writes `video.part.mp4` and renames it `video.mp4` at the
  stop. A new stage, **`screen_video` (order 15)**, runs after
  `finalize_audio` has written `audio.mp4`: it adds that audio track to
  the video with an `AVMutableComposition` and a Passthrough export (no
  re-encode, no FFmpeg, `deino_audio::mux`), then stores `video_path`.
  It is the only writer of a live recording's `video_path`, after
  `finalize_audio`, so the two cannot race. While only the part file
  exists it retries (the capture is closing it); after 20 s untouched it
  adopts it (a crash's leftover, playable up to its last fragment). A
  video that already has sound (an import, a re-run) is kept.
- **When.** Off by default. Settings → Screen capture → "Capture detected
  meetings" turns it on for recordings whose start sees a meeting app in
  meeting detection (Circleback's "on by default for detected meetings",
  opt-in here). "Turn off for this meeting" stops the capture and writes
  `screen_capture_choices`, so a resumed recording stays off. A capture
  starts only when `CGPreflightScreenCaptureAccess` (which never prompts)
  says yes; Settings and onboarding open System Settings, and onboarding's
  row may call `CGRequestScreenCaptureAccess` on the user's click.
- **One start path.** `CaptureOnStart` is a `StartObserver` of the
  recorder, beside CAP-05's `WatchOnStart`; the capture ends when the
  playhead says the recording is over.

## Alternatives

- **Mux the audio live into `video.mp4`.** Needs the recorder's audio on
  the capture thread and duplicates the encoder; a crash would lose the
  file's moov unless it too were fragmented. The post-stop mux reuses the
  finished, mixed `audio.mp4`.
- **ScreenCaptureKit audio in the video.** A second system-audio capture
  next to the tap (§3C), without the microphone, so the video's sound
  would differ from the recording's.
- **Keep the video silent and compose at clip time.** Fixes TRN-04 only;
  a downloaded video stays silent, and every consumer must know to pair it
  with `audio.mp4`.
- **`SCRecordingOutput`** (macOS 15+) writes the file itself but not on our
  macOS 14.2 floor, and gives no frames for snapshots.

## Consequences

- A meeting with screen capture stores its audio twice (in `audio.mp4` and
  in `video.mp4`, about 30 MB an hour) plus about 110 MB of video an hour
  at 1 fps. EXP-02's account export still skips video.
- Clips (TRN-04) of such meetings have sound with no change to the cutter.
- Phase B builds on this: OCR in `screen_context`, NOT-02's time-aligned
  context, the Images strip, the consent reminder and the record panel's
  window or screen picker (a screen target is not modelled yet).
