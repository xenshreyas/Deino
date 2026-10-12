# Cloud transcription as an option: ElevenLabs Scribe after the meeting, on the user's own key

- **Status:** Accepted (owner decision, 2026-10-12)
- **Date:** 2026-10-12
- **Card:** CAP-14; extends CAP-08's post-meeting re-pass
  (`final_transcription`, ADR 0037), and builds on ADR 0013 (keys in the
  Keychain), ADR 0019 (network egress) and ADR 0020 (local diarization)

## Context

Deino transcribes on the Mac: Parakeet live, and optionally a
high-accuracy re-pass after the meeting (CAP-08). The owner wants a more
accurate transcript for some calls and is willing to send the audio to
ElevenLabs for it, with their own account. PARITY_PLAN §1 says data leaves
the Mac only to a service the user connects; this is such a service, but
it is the first one that receives audio.

The questions: what is sent, when, by which path through the code; what
happens to the live transcript; how speakers survive; what a failure does;
and what the user sees.

The ElevenLabs contract was checked against the official docs on
2026-10-12:

- `POST https://api.elevenlabs.io/v1/speech-to-text`,
  `multipart/form-data`, key in `xi-api-key`. Fields used: `model_id`,
  `file`, `language_code`, `diarize`, `timestamps_granularity=word`,
  `tag_audio_events`.
- The answer: `language_code`, `text`, `words[]` with `text`, `type`
  (`word`, `spacing`, `audio_event`), `start`/`end` in seconds,
  `speaker_id` (`speaker_0`, ...), `logprob`.
- Limits: files up to 3 GB on the overview (5 GB in the API reference),
  up to 10 hours, 32 speakers.
- Errors: `{"detail": {"type", "code", "message"}}`; 401
  `invalid_api_key`, 402 `insufficient_credits`, 403
  `insufficient_permissions`, 429 `rate_limit_exceeded` /
  `concurrent_limit_exceeded` / `system_busy`, 5xx; 422 validation lists.
- Models: `scribe_v2` is current. **`scribe_v1` was deprecated with removal
  announced for 2026-07-09** (changelog of 2026-06-08), before this card.
  It is offered because the owner asked for it; if ElevenLabs refuses it,
  the run fails as "rejected" and falls back to the Mac.
- `GET /v1/user` returns the account and costs nothing: the key check.
  A key restricted to speech to text answers 403 there (no `user_read`),
  which still proves ElevenLabs knows it; only 401 means a wrong key.

## Decision

1. **Local by default; ElevenLabs is opt-in and after the meeting only.**
   Live transcription stays Parakeet on the Mac. With a key saved and
   Settings → Advanced → ElevenLabs → "Transcribe meetings with ElevenLabs after
   they end" on (`transcription.cloudProvider = "elevenlabs"`, model in
   `transcription.cloudModel`, default `scribe_v2`), each meeting is
   re-transcribed by ElevenLabs once it ends, and that transcript replaces
   the live one. Scribe v2 Realtime for live captions is out of scope.
2. **The existing re-pass path, not a parallel one.** The
   `final_transcription` stage asks a new `CloudChoice` port per run; with
   a cloud service the core's `FinalTranscriptionService` takes its cloud
   branch (`cloud_run.rs`) over the `CloudTranscriber` port instead of the
   `TrackTranscriber`. The stages after it (diarize, names, notes, action
   items, ...) run on the new transcript because they always run after it.
3. **One upload per channel.** `mic.m4a` is sent as the user (no
   diarization); `system.m4a` (or the mixed `audio.mp4` of an older
   recording) with `diarize=true`, so a call with several people keeps
   them apart. Channel attribution therefore never depends on the cloud.
4. **Speakers the way diarization makes them.** Scribe's voices on a track
   become "Speaker 1..N" in order of first appearance, skipping the user's
   label, coloured as `diarize` colours them, and replace the meeting's
   other speakers. They carry no voice embedding: the local `diarize`
   stage still runs next, measures voices on the Mac, merges known
   voiceprints and re-attributes (and `identify_speakers` names them), so
   voice profiles keep working. When the local models are missing, the
   cloud's speakers stay.
5. **The live transcript is never lost.** The cloud branch writes nothing
   until every track came back. Before replacing, it keeps the transcript
   in place in `original_transcripts` (one per meeting, the first one, so
   the one made on the Mac; lines without speakers). "Restore local
   transcript" puts it back with speakers carried over by time overlap and
   re-runs the speaker step and every later step. Retention deletes the
   kept copy with the transcript (a trigger on `transcript_deleted_at`);
   deleting the meeting cascades.
6. **Failure falls back to the Mac.** 429, 5xx and network failures are
   retried by the pipeline's retry policy (3 attempts with backoff). Any
   other failure, or the last attempt, runs the Mac's own rules (a meeting
   without a transcript is transcribed locally; a live one stays) and skips
   the step with "ElevenLabs transcription failed: <reason>." The meeting
   page shows that line with Retry. Every failure is a typed `CloudError`
   mapped to an `ErrorCode` (a refused key is `permission_denied`).
7. **Edits win unless the user asks.** An automatic run leaves a
   transcript the user edited alone. "Re-transcribe with ElevenLabs" on a
   meeting asks explicitly: it replaces even an edited transcript (the
   original is kept) and works with the setting off. The request is held
   in memory until the step finishes; quitting mid-run falls back to the
   setting.
8. **Network and keys as everywhere else.** `ServiceId::ElevenLabs`
   (stored as `elevenlabs`, host `api.elevenlabs.io` only, off by default)
   gets a switch on Settings → Privacy → Network and egress-log entries;
   the egress log's CHECK list is rebuilt to accept it. Saving the key
   switches the service on, forgetting it switches it off, as CAP-12's
   import keys do; offline mode blocks it. The key lives only in the
   Keychain (`SecretName::ElevenLabsApiKey`); the UI can save, check and
   forget it, never read it. The plugin is guarded by the app lock. Logs
   carry meeting IDs, byte sizes and line counts, never the key, audio,
   paths or text.
9. **Provider code in its own crate.** `deino-cloud-transcription` holds
   the wire format, the multipart body, the word-to-utterance grouping and
   the `ElevenLabs` service; `deino-core` holds the rules and never sees a
   provider's JSON. Its tests replay recorded fixtures; nothing calls
   ElevenLabs.

## Consequences

- With the option on, every meeting's audio leaves the Mac after it ends.
  The settings card says so in one line, the Network page names ElevenLabs
  as receiving meeting audio, and About no longer claims audio never
  leaves.
- A meeting costs ElevenLabs credits for both channels (the microphone is
  sent separately from system audio). Sending one stereo file with
  `use_multi_channel` would bill both channels anyway.
- Notes are written on the cloud transcript, so they wait for the upload
  and the transcription (minutes for a long meeting); a failure costs the
  retries' backoff before the Mac's transcript is used.
- The kept original roughly doubles a meeting's transcript storage until
  it is restored or deleted.

## Follow-ups

- Scribe v2 Realtime for live captions (out of scope here).
- Drop Scribe v1 from the model list once the owner confirms ElevenLabs
  refuses it.
- `keyterms` from the vocabulary (TRN-05) would bias Scribe toward names;
  it costs 20% more, so it needs a decision.
- Persist one-off "Re-transcribe with ElevenLabs" requests if quitting
  mid-run turns out to matter.
- `enable_logging=false` (zero retention) is enterprise-only; offer it if
  the owner's plan allows.
