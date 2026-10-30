# Enhanced notes stream in line by line

Status: accepted (2026-10-30). Builds on ADR 0030 (enhanced notes) and
ADR 2026-10-29-notes-one-pass-by-length.

**Context.** Notes appeared all at once when generation finished, after
seconds of a skeleton (Sonnet 5.5 thinks first, then writes 1–2k tokens of
JSON). Every provider already streams (`LlmProvider::stream`), but
`Gateway::structured` collected the whole answer before parsing it, and the
answer is an `EnhancedDoc` JSON document, not text a page could show as it
arrives. A prototype compared live text with a caret, line-by-line reveal,
and a template outline filled in; the owner picked line by line.

**Decision.**

- `deino-llm`: `Gateway::structured_streamed::<T>(route, prompt, sink)` is
  `structured` with the raw JSON handed to an `AnswerSink` as it arrives
  (`start` before each attempt, so the repair turn starts over, then
  `piece`). The value is still parsed from the whole answer.
- `deino-core::enhance`: `partial::complete_lines` reads the JSON so far,
  tolerating where it stops, and keeps only what is final: a section once
  its heading is closed, a block once its `origin` is (so who wrote it is
  known; the schema's field order is unchanged), a detail once its text
  is; evidence is left out. A property test proves every prefix of an
  answer shows a prefix of its lines. `LiveDraft` runs those lines through
  `assemble::preview` (rules 1–3 of the saved notes; not rule 4, since a
  user line not written yet is not dropped) and `to_doc`, and publishes a
  `NotesDraft` (phase, TipTap doc, line count, stale) through the
  `NotesProgress` port whenever the count grows. Only the calls whose
  answer becomes the notes stream (one pass, the merge); parts report
  counts. A new attempt (retry, repair, halves after a failed one pass)
  marks the old lines stale until its own first line arrives.
- Runs (`runs.rs`) replace the running set: the latest draft per meeting
  (`EnhanceService::draft`) and Stop (`EnhanceService::stop`, an
  `AbortHandle` over the writing only, never the save). A stopped manual
  run fails with `EnhanceError::Stopped` (`cancelled`); a stopped pipeline
  run is a skip (`STOPPED`). Each run that published reports its end:
  `Written { version, kept }`, `Stopped` or `Failed`.
- App: `TauriNotesProgress` broadcasts `NotesRun`
  (`plugin:enhance:notes-run`), not a per-request channel, because
  pipeline runs have no request. Commands `draft` and `stop`.
- UI: the Enhanced view draws the draft with `DocView`, a pure renderer of
  the same elements and classes the editor uses, so the saved version
  replaces it without a visible change; new lines fade in once. The status
  row says what the run does ("Writing · Discussion") with Stop; editing
  waits for the saved version; on regenerate the old notes stay dimmed
  until the first new line arrives; the page never scrolls on its own
  ("Writing below" offers a jump).

**Consequences.** The saved notes are exactly as before: the draft is
display-only and the save path (`assemble`, `to_doc`, `insert`) is
unchanged. A run interrupted by quitting saves nothing; the pipeline
reruns it on the next launch. Drafts are never logged or stored.
