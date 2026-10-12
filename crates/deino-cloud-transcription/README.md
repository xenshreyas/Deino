# deino-cloud-transcription

## Role

Opt-in transcription after the meeting by a cloud service on the user's own
API key (CAP-14, `docs/adr/2026-10-12-cloud-transcription-option.md`).
Today that is ElevenLabs Scribe. Live transcription never comes here: it
stays on the Mac (`deino-stt`). The crate exists so that `deino-core` holds
the rules (the `final_transcription` stage's cloud branch, the kept
original) and no provider's wire format.

## Owns

- `api.rs`: `ScribeApi`, the two ElevenLabs calls Deino makes:
  `POST /v1/speech-to-text` (multipart, `xi-api-key`) and `GET /v1/user`
  (the key check), with HTTP statuses and `detail.code`s mapped to
  `deino_core::final_transcription::CloudError`.
- `multipart.rs`: the `multipart/form-data` body (`model_id`,
  `language_code=en`, `diarize`, `timestamps_granularity=word`,
  `tag_audio_events=false`, `file`).
- `wire.rs`: the response shapes, as the API reference documents them.
- `utterances.rs`: Scribe's words grouped into utterances (a new one at a
  speaker change, a pause, or a long run that reaches a sentence end).
- `transcriber.rs`: `ElevenLabsTranscriber`, the `CloudTranscriber`: one
  track per request, the size limit checked before anything is read.
- `service.rs`: `ElevenLabs`, the Settings card's service (key status,
  save, check, forget; saving switches `ServiceId::ElevenLabs` on) and the
  `CloudChoice` the stage asks, with one-off requests from the meeting's
  "Re-transcribe with ElevenLabs".
- `fixtures/`: responses written from the API reference, which the tests
  replay. Tests never call ElevenLabs.

## Does Not Own

- When the cloud runs, what it replaces, the fallback to the Mac, the kept
  original and the speakers: `deino_core::final_transcription` and the
  `final_transcription` stage in `deino-pipeline`.
- The HTTP client, the host list, the switch, offline mode and the egress
  log: `deino-net` (`ServiceId::ElevenLabs`, host `api.elevenlabs.io`).
- The key's storage: `deino-secrets` (`SecretName::ElevenLabsApiKey`).

## Invariants

- Every request goes through the `ScopedClient` of `ServiceId::ElevenLabs`.
- The key is read from the Keychain per request and never logged, put in
  an error or returned to the UI.
- Nothing logs audio, transcript text or file paths; logs carry meeting IDs,
  sizes and counts.
- `language_code` is always `en` (Deino is English only).
