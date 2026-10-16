# FIX-offline-echo · Echo removed from the after-meeting transcript

## Progress log (PARITY_PLAN §7.1)
Recorded on laptop speakers with live transcription off, the
`final_transcription` stage transcribed the raw `mic.m4a`, which holds the
other side of the call, so Me repeated Them's lines under the user's name.
The after-meeting path now has both of the live path's defences:

- **Echo cancellation before transcribing the microphone.** The service
  gives the microphone track the system track as `TrackRequest::echo_reference`.
  `OfflineTranscriber` decodes both to 16 kHz and runs the decoded
  microphone copy through `SonoraAec` (AEC3) with the system track as the
  far end, 10 ms frame by frame (`deino_recording::offline::cancel_echo`).
  The stored file is never changed. No offset is applied: both tracks are
  anchored to the recording epoch with leading silence and decoded with the
  AAC priming trimmed, so they are sample-aligned (`TrackTimeline`,
  finalize). Skipped when "I use headphones" is on (`HeadphonesSetting`,
  wired to `SettingsService` in `composition/transcript.rs`). The recording
  does not store its output route, so otherwise it runs, treating the route
  as speakers like `EchoMode::choose`. If the system track can't be read,
  the microphone is transcribed without cancellation (logged). Cost: about
  0.01× real time in the test profile (10 s in under 0.1 s), so about 40 s
  for a 72-minute meeting. It takes the first 10% (`ECHO_SHARE`) of the Me
  track's progress and stops on cancellation.
- **Text de-duplication after both tracks are transcribed**, for every
  engine and the ElevenLabs branch (`deino_core::final_transcription::without_echo`,
  `echo_text.rs`). First, words: when the engine timed them, Me words are
  aligned with Them words starting within ±1.5 s of the segment (an LCS on
  normalised words, each pair within 1.5 s). Each run of at least 3 matched
  words, allowing one misheard word between matches, is cut, and the rest
  stays. Then, segments: what is left is dropped when it matches Them's
  text from the same moment (`overlap::overlaps` > 0.6 against up to 3
  adjacent Them segments merged, starting within 6 s), Them was speaking
  within 0.5 s of it, and more than 60% of its own words are in that text
  (so an untimed line with the user's words plus echo is kept). A segment
  left empty is dropped. Audio correlation never drops anything.
- `bleed/overlap.rs` moved to `deino-core` (`final_transcription::overlap`,
  plus `coverage`); `deino_recording::bleed::overlap` re-exports it.

## Follow-ups (PARITY_PLAN §7.2)
- CAP-02/CAP-08: store the probed output route in `recording.json` so the
  offline pass can skip cancellation on a headphones recording even when
  the setting is off.
- Owner QA: re-run the 72-minute laptop-speaker meeting's transcript
  (Processing details → retry Transcript) and check Me no longer repeats
  Them.

## Architecture (ARCHITECTURE.md)
- Post-meeting transcription (CAP-08): the microphone track is
  echo-cancelled against the system track before the batch engine hears it
  (unless on headphones), and after both tracks are transcribed, Me words
  and lines that repeat Them at the same moment are removed
  (`final_transcription::echo_text`, all engines and ElevenLabs).

## CLAUDE.md
- `crates/deino-recording` row: "the after-meeting `OfflineTranscriber`
  cancels the microphone's echo against the system track (`offline/echo.rs`)".
