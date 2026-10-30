# NOT-streamed-notes · Enhanced notes stream in line by line

## Progress log (PARITY_PLAN §7.1)
Enhanced notes stream into the Notes tab as they are written, a finished line at a time, with Stop ([ADR](../adr/2026-10-30-streamed-notes.md)): `Gateway::structured_streamed` + `AnswerSink` (deino-llm); `enhance::partial::complete_lines`, `LiveDraft`, `assemble::preview`, `NotesProgress`/`NotesDraft`/`NotesPhase`/`NotesRunEnd`, `EnhanceService::{draft, stop, with_progress}`, `EnhanceError::Stopped`, `STOPPED` (deino-core); `NotesRun` event (`plugin:enhance:notes-run`) and commands `draft`, `stop` (app); `DocView`, `DraftNotes`, `RunStatus`, `useLiveNotes` (UI). `MockProvider::{reply_in_chunks, stall}`.

## Follow-ups (PARITY_PLAN §7.2)
- Owner QA: regenerate a real meeting on Sonnet 5.5 and on the built-in model; check the swap from the draft to the saved notes shows no jump, and Stop mid-way leaves the previous version.
- NOT-02: a model that writes a section's blocks before its heading shows that section only when it closes; a schema-ordered model (all current providers) is unaffected.

## Architecture (ARCHITECTURE.md)
- deino-core `enhance` row: add "streamed notes: `partial.rs` (the finished lines of an answer still being written), `live.rs` (`LiveDraft`, published through `NotesProgress`), `runs.rs` (one run per meeting, its latest draft, Stop)".
- deino-llm row: `Gateway` also has `structured_streamed` (an `AnswerSink` gets the raw JSON as it arrives).

## CLAUDE.md
- deino-llm row: `Gateway` (`stream`, `complete`, `structured::<T>()`, `structured_streamed::<T>()`, `with_tools`, `embed`).
