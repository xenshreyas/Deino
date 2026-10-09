# 0010. Transcribe locally; write notes with the user's own LLM

- **Status:** Accepted
- **Date:** 2026-10-08
- **Card:** FND-13 (records PARITY_PLAN §1, §8 D2/D3)

## Context

Deino must match Granola and Circleback on note quality while staying private
(§1): no Deino server, no telemetry, no meeting bot. Audio is the most
sensitive data and the largest; transcripts and notes are smaller and are
what an LLM needs. Local speech-to-text on Apple Silicon (Parakeet, Whisper on
Metal and CoreML) is fast and accurate in English. Local LLMs that run well on
a laptop write noticeably weaker notes than frontier models.

## Decision

- **Transcription, embeddings and search are always local.** Audio never
  leaves the Mac. `deino-stt` will not even depend on the network crate
  (FND-12 enforces it at compile time).
- **Notes, chat and extraction use the LLM the user connects**, with their own
  API key, called directly from the Mac: Claude by default
  (`claude-sonnet-5-5` for notes and chat, `claude-haiku-5-5` for extraction
  and titles), or OpenAI and compatible providers.
- **Without a key, everything falls back to the built-in local model**
  (llama-helper), so the app works fully offline.
- All LLM calls go through one gateway with task routes (FND-09) and all
  traffic through one network client with an egress log (FND-12).

## Alternatives

- **Everything local:** maximum privacy, weaker notes; kept as the fallback.
- **Cloud transcription:** better diarization in some services, but sends
  audio off the Mac and adds per-minute cost.
- **A Deino relay server:** would hold user data and keys; ruled out by §1.

## Consequences

- Transcripts and notes are sent to the provider the user chose, and the
  Network page (FND-12) shows exactly what left the Mac.
- Keys live in the Keychain (FND-06), never in SQLite.
- Note quality depends on the user's provider and plan; the local fallback is
  always available.
