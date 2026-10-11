# 0030. Enhanced notes from a structured answer, with a provenance guard and versioned rows

- **Status:** Accepted
- **Date:** 2026-10-11
- **Card:** NOT-02 (enhanced notes engine), NOT-03 (citations UI)

## Context

NOT-02 replaces the inherited summary (a Markdown blob in
`summary_processes`, written by the legacy summary service and the
`legacy_summary` pipeline stage) with Granola-style enhanced notes: the
user's own lines kept verbatim and shown in ink, the AI's additions in grey,
every AI line citing the transcript moments behind it (NOT-03), editable,
regenerable, with a version history. Three properties are product
promises, not prompt suggestions:

1. **Nothing the user typed is lost or reworded.**
2. **Every citation points at a real segment of this meeting.**
3. **A user's edit is never overwritten without asking.**

Models do not reliably keep text verbatim, invent IDs when shown UUIDs, and
long meetings exceed small context windows.

## Decision

**The model answers a typed structure; Rust assembles the document.**

- `notes.enhance` returns `EnhancedDoc` (`Gateway::structured`): sections
  of blocks, each `{kind, text, origin: user|ai, evidence[], details[]}`.
  The prompt (askama, snapshot-tested) shows the transcript as short
  references, `[s12 04:31] Speaker: text`, never segment UUIDs; `evidence`
  holds those references. `SegmentIndex` resolves them, drops any that do
  not exist, keeps at most three per line.
- **Provenance guard** (`enhance::assemble`): a block the model marks
  `user` must match a line the user actually typed (exact after
  normalising, or word-set Jaccard ≥ 0.75); it is then rendered from the
  user's own inline content (marks, mentions, links kept), not from the
  model's text. A `user` block that matches nothing is downgraded to AI.
  Typed lines the model dropped are re-placed under the section whose
  text overlaps them most, or collected under "From your notes". The
  guarantee therefore holds whatever the model does.
- `to_doc` turns the draft into a `deino-doc` TipTap document: `origin`
  marks (`author`, `edited`) on every run, `citation` inline nodes after
  AI text; Markdown comes from the same document (`to_markdown`).
- **Long meetings: map-reduce.** Over 30 minutes (or beyond the target's
  context budget) the transcript is cut into ~20-minute parts (a tail under
  5 minutes joins the previous part), noted in parallel (4 at a time for a
  hosted model, 1 for the local one) and merged; one target is pinned for
  every call of a run. References are global, so they survive the merge.

**Storage: `enhanced_notes`, one row per version.** Primary key
`(meeting_id, version)`, `doc_json` (checked to be a JSON `doc`),
`markdown`, `template_id` (`SET NULL`), `model`, `basis_json`,
`is_user_edited`, timestamps. Regenerating inserts a new version; Restore
copies an old one forward. Only the newest version is editable; saving an
edit sets `is_user_edited`, and `EnhanceService::enhance` refuses
(`conflict`) to write over an edited newest version unless the caller
passes `Overwrite::ReplaceEdits`, which the UI only does after the
"Replace your edits?" dialog. Automatic runs (the `enhance_notes` stage,
order 50, skipped when "Generate notes automatically" is off) never
replace edits.

**Regenerate prompts from a basis fingerprint.** Each version stores what
it was written from (attendee IDs, segment count and an FNV digest of their
IDs, transcript end, transcript edit time, user notes' update time). The
view compares it with the meeting now and returns `RegenerateReason`s
(attendees changed, transcript edited, notes edited, audio appended) for
the banner.

**Legacy summaries become version 1.** `LegacySummaryImport`, a
`DataMigration` that runs just before `…_not02_drop_legacy_summaries.sql`,
converts each completed `summary_processes` result into a document (model
`legacy`, no citations) so nobody loses a summary; then
`summary_processes` and `transcript_chunks` are dropped and the legacy
summary code is deleted.

**Citations UI (NOT-03).** Citation nodes stay in the document but are
hidden in the editor; a magnifier in the left gutter appears beside the cited
line under the pointer or the caret (⌘⇧S opens it) and opens a popover with
the excerpts (`enhance::cited`: speaker, time, text), a Play button
(disabled until TRN-03's player) and "Show in transcript"
(`?tab=transcript&segment=…`).

## Alternatives

- **Free-form Markdown with inline markers** (`[[user]]…`, `^[s12]`), as the
  legacy summary and several open-source notepads do. Fewer tokens, but
  verbatim keeping and citation validity would rest on the model; parsing
  markers out of prose is fragile, and nothing could be repaired.
- **Diffing the model's Markdown against the user's notes** to recover
  provenance. Works for light edits, fails on reordering and merging, and
  cannot tell a user line from an AI paraphrase of it.
- **UUIDs as citations.** The model copies 36-character IDs badly and they
  triple the transcript's token count; short references cost a few tokens
  and are validated against an index anyway.
- **One row per meeting, overwritten.** Simpler, but loses the history the
  card asks for and makes "keep my edit" a race; versions also give the
  auto-export and EXP-01 a stable "newest" to read (`EnhancedNotesReader`).

## Consequences

- `deino-core::enhance` owns the engine; `deino-pipeline` runs it as
  `enhance_notes` (50) and `auto_title` (65, a title from the notes when the
  meeting has a placeholder name; the calendar title wins without an LLM
  call). The app exposes the `enhance` plugin (`get`, `versions`,
  `enhance`, `save`, `restore`, `cited`).
- Exports (EXP-01/02) and tag digests (ORG-06) read the newest version
  through `EnhancedNotesReader`; nothing reads `summary_processes`.
- The `notes.enhance` eval (`crates/deino-llm/tests/llm_eval/notes.rs`) runs
  the whole engine over every fixture meeting and scores verbatim user
  lines, cited AI lines (≥ 90%) and ground-truth coverage; offline trials
  run in `cargo test`, live ones with `DEINO_EVAL=1 pnpm eval notes.enhance`.
- `deino-llm` gains a dev-only dependency on `deino-core` for that eval (a
  dev-dependency cycle Cargo allows).
