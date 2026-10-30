# PERF-transcribe-during-meeting · The after-meeting wait, done during the meeting

## Progress log (PARITY_PLAN §7.1)
With live transcription off, a 72-minute call took 465 s of
`final_transcription` and `diarize` after it ended (Whisper large-v3-turbo,
M4 Max). It now takes about 12 s, with the same transcript, speakers and
word times as the stages give without following (checked field by field on
that call, `deino --final-bench … --follow <dir>`).

- **During the meeting** (`deino_recording::FollowAlong`, wired in
  `composition/follow.rs`): every 30 s while recording, with the live
  transcript off, a pass snapshots each track being recorded (an APFS clone
  of `audio/<track>.001.m4a.part`, cut to its last whole fragment by
  `deino_audio::fmp4::truncate_to_playable`) into `recordings/.following/`
  and runs the stage's own transcriber over it, then the word aligner on
  the call's lines and, every fourth pass, the diarizer's scan.
- **Memories keyed by the exact audio**: `PassMemory` (the engine's answer
  per clip and model; the microphone's echo canceller and cancelled
  samples, resumed when the start of the finished track fingerprints the
  same), `deino_diarize::ScanCache` (segmentation and embeddings per batch
  of windows) and `deino_diarize::align::LineMemory` (each line's word
  times). A finished file decodes to the same samples as the file being
  recorded except its last second, so everything but the end is reused;
  anything that differs is a new key and is computed again. The voice check
  is not remembered (DeepFilterNet carries state from line to line) and
  runs only after the meeting. A new recording clears the memories.
- **Also faster without following**: both tracks transcribe at once
  (progress is the mean of the tracks); microphone utterances without a
  voice are dropped before Whisper; an engine that times no words is never
  asked for halves (they could never win); the voice check runs on up to 6
  threads; the word aligner runs 3 sessions; the diarizer segments the next
  batch while the current one is embedded. 465 s → about 320 s.
- **Whisper is seeded** (`WHISPER_SEED`), so the same audio gives the same
  transcript, which the memories rely on.

## Follow-ups
- Follow with live transcription on (the live lanes share the GPU).
- A recording resumed or restarted mid-meeting (`*.002.m4a.part`) is not
  followed.
- Meetings over 2.5 hours change the diarizer's window step, so its scan
  is redone after the meeting.
- The word aligner's and echo canceller's output can shift slightly between
  builds (float differences); each build is deterministic.
