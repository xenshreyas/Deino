# CAP-14 · ElevenLabs Scribe transcription as an option (user's own API key)

## Progress log (PARITY_PLAN §7.1)
CAP-14 · Opt-in ElevenLabs Scribe re-transcription after each meeting on the
user's own key; live transcription stays local (Parakeet).
ADR `2026-10-12-cloud-transcription-option`. New crate
`deino-cloud-transcription` (`ScribeApi`, `ElevenLabsTranscriber`,
`ElevenLabs` service). `deino_core::final_transcription` gains the
`CloudTranscriber`/`CloudChoice` ports, `CloudError`, the cloud branch
(`cloud_run.rs`), `CloudVoices`, `OriginalTranscripts` (+ SQLite) and
`OriginalTranscriptService` (restore). `ServiceId::ElevenLabs`
(`api.elevenlabs.io`), `SecretName::ElevenLabsApiKey`, settings
`transcription.cloudProvider` (`none` | `elevenlabs`) and
`transcription.cloudModel` (`scribe_v2` | `scribe_v1`). Migration
`20261012173514_cap14_cloud_transcription.sql` (egress log CHECK +
`original_transcripts` + retention trigger). `PipelineService::rerun_from`.
Plugin `cloud-transcription`: `key_status`, `save_key`, `check_key`,
`forget_key`, `retranscribe`, `has_original`, `restore_original`. UI:
Settings → Advanced → "ElevenLabs (optional)" (section `elevenlabs`,
order 25, registered from `features/cloud-transcription/settings-page.ts`);
the meeting ⋯ menu's "Re-transcribe with ElevenLabs" and "Restore local
transcript" (`CloudTranscriptMenuItems`, next to "Re-transcribe…" in
`recording-menu-items.tsx`); the header's status line shows
"ElevenLabs transcription failed: …" with Retry (`PipelineStatus`).

## Follow-ups (PARITY_PLAN §7.2)
- New card: Scribe v2 Realtime for live captions (out of scope by owner decision).
- Owner QA: try a real key on an old meeting (⋯ → Re-transcribe with
  ElevenLabs); confirm whether ElevenLabs still accepts `scribe_v1` (its
  changelog announced removal on 2026-07-09). If it refuses it, drop the
  option from `CloudTranscriptionModel`.
- TRN-05: send the vocabulary as Scribe `keyterms` (20% surcharge; needs a decision).
- CAP-14: persist one-off "Re-transcribe with ElevenLabs" requests (held in
  memory today; quitting mid-run falls back to the setting).

## Architecture (ARCHITECTURE.md)
- Rust workspace table: add `crates/deino-cloud-transcription` — opt-in
  cloud transcription after the meeting (CAP-14, ADR
  2026-10-12-cloud-transcription-option): `ScribeApi` (`POST
  /v1/speech-to-text` multipart, `GET /v1/user` key check, statuses →
  `CloudError`), the multipart body, Scribe words → utterances,
  `ElevenLabsTranscriber` (the core's `CloudTranscriber`) and `ElevenLabs`
  (Keychain key save/check/forget, `ServiceId::ElevenLabs` switch, the
  core's `CloudChoice` with one-off per-meeting requests); recorded
  fixtures; over `deino-net`.
- `deino-core` row: `final_transcription` also holds the cloud branch
  (CAP-14): the `CloudTranscriber` and `CloudChoice` ports, the cloud's
  voices as "Speaker N" (`CloudVoices`), and the kept original transcript
  (`original_transcripts`, `OriginalTranscriptService`).
- `deino-pipeline` row: `final_transcription` asks `CloudChoice`; a cloud
  failure retries (429, 5xx, network), then falls back to the Mac's rules
  and skips with "ElevenLabs transcription failed: …". `PipelineService`
  gains `rerun_from(meeting, stage)` (a stage and every later one).
- "What Deino is": transcription is local by default; the user may opt in
  to ElevenLabs re-transcribing each finished meeting with their own key.

## CLAUDE.md
- "What Deino is": replace "transcription (always local)" with
  "transcription (local; ElevenLabs after the meeting only if the user
  opts in with their own key, CAP-14)".
- Rust workspace table: the `deino-cloud-transcription` row above.

## Plan updates
- Add CAP-14 to the card list as done, with the follow-ups above.
