# PIPE-progress · Live progress for the after-meeting transcription

## Progress log (PARITY_PLAN §7.1)
The `final_transcription` stage reports how far it is, and the meeting
page's status line reads "Transcribing… · 40%" instead of a bare
"Transcribing…". Key types: `deino_core::final_transcription::TrackProgress`
(a callback on `TrackRequest::progress` and `FinalPass::progress`, 0.0–1.0),
`deino_pipeline::StageFraction` (throttles to one event per whole percent),
`PipelineProgress::fraction: Option<f32>` in the pipeline crate and
`percent: number | null` (whole percent 0–100, rounded down) on the app's
`PipelineProgress` event. No new commands.

How the fraction is computed:
- **Per track** (`OfflineTranscriber`, every local engine: Parakeet Unified,
  Parakeet TDT and the Whisper re-pass): after each utterance, the end of
  that utterance over the track's length; 1.0 when the track is finished
  (at once for a silent track).
- **Per run** (`FinalTranscriptionService`): (tracks done + the current
  track's fraction) / tracks. Tracks weigh the same, which matches duration
  because the microphone and system tracks both span the meeting.
- **Cloud** (ElevenLabs): each channel's share moves to done when its upload
  comes back (0.5, then 1.0 for two tracks).
- **Stage**: emitted through `ctx.events` as `Running` with a fraction, only
  when the whole percent changes. Plain status events carry `None`.

UI: `usePipelineProgress` writes a percent event into the query cache
(`pipelineKeys.percent(meeting, stage)`, read by `useStagePercent`, a
`skipToken` query) without refetching; a status event clears that stage's
percent and refetches the status as before. `dev:mock`: Retranscribe
(ElevenLabs) or re-running `final_transcription` walks the transcript from
0% to 100% before the notes.

## Architecture (ARCHITECTURE.md)
- Pipeline progress: besides status changes, a long stage may report a
  fraction through `StageFraction` (one event per whole percent); the app's
  `PipelineProgress` event carries it as `percent` (0–100, null otherwise),
  and the meeting header shows it after the running line.

## CLAUDE.md
- `crates/deino-pipeline` row: add "`StageFraction` (a running stage's
  percent through `ProgressSink`)".
