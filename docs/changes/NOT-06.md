# NOT-06 · Edit notes by asking

## Progress log (PARITY_PLAN §7.1)
NOT-06: ☑️ done on branch `feat/NOT-06-edit-notes`
([ADR 2026-10-12](../adr/2026-10-12-edit-notes-by-asking.md); entry point
an "Edit with AI" command bar on the enhanced notes instead of the parked
chat (D13); `deino-core::notes::edit`: `EditRequest` (meeting, version,
instruction, `selection` as text-block positions), `Rename` (plain renames
done without the model: "X is spelled Y", "rename X to Y everywhere",
"replace X with Y", "it's Y, not X", "X → Y"; whole words, ignoring case,
text runs and mention labels, never a link's `href`, a mention's person or a
citation), otherwise the `notes.edit` route through the gateway with the
notes as ID'd JSON blocks (`b3`, kind, origin, text with `[s12]` references)
and the transcript, answering `EditAnswer` (replace / delete / insert_after
block changes) that `plan` checks (selection, locked blocks, unknown IDs)
and `apply` applies in place; rewritten blocks keep the marks of kept words,
new words are AI text, dropped citations are put back, new references
resolve to real segments; NOT-09's redactor runs over the request, blocks
and transcript when the setting is on and locks blocks with a secret;
`EditProposal` with a word diff (`DiffBlock`, `DiffSegment`); `accept`
stores a new version marked edited, undo is `enhance|restore` of the
previous one; plugin `notes-edit` (`propose`, `accept`); UI in
`features/notes/edit/`: the status row's Edit with AI button, ⌘⇧E while the
caret is in the notes, the bar with the selection chip, the preview inline
or side by side, Accept / Reject, the "Notes edited · Undo" toast; prompts
`notes_edit.{system,user}.md.jinja`; dev:mock mock of the plugin)

## Follow-ups (PARITY_PLAN §7.2)
- AI-02-lite / AI-02: the single-meeting "Ask about this meeting" panel
  (being built in parallel) and later the chat agent can offer "edit the
  notes" by calling `notes-edit|propose` and rendering the same
  `DiffPreview`; the card's chat tool `edit_notes` is that call.
- Owner QA: ⌘⇧E already means Enhance / Regenerate on the meeting page;
  NOT-06 takes it only while the caret is in the enhanced notes (the editor
  stops the event). Decide whether Edit with AI should own ⌘⇧E everywhere
  once notes exist, or move to another shortcut.
- NOT-06: the transcript in the `notes.edit` prompt is cut at 60,000
  characters; for a selection, send the transcript around the selected
  blocks' citations instead, and map-reduce long meetings like NOT-02.
- NOT-06: scope is per text block; a partly selected line is edited whole.
  Character-level scope needs inline splitting the block changes cannot
  target yet.
- NOT-06: deleting a list item's line deletes its nested items; hoist them
  instead when the model deletes only the parent.
- NOT-06: add a `notes.edit` eval (`crates/deino-llm/tests/llm_eval/`) over
  the fixture notes: renames, "more detailed" with valid citations, section
  removal, and untouched blocks byte-identical.

## Architecture (ARCHITECTURE.md)
- `crates/deino-core` row / features: `notes::edit` (NOT-06, ADR
  2026-10-12-edit-notes-by-asking): edit enhanced notes by asking; the
  rename shortcut, block-change answers applied in place, the word diff,
  accept as a new version.
- The app crate: `features/notes_edit.rs` (plugin `notes-edit`),
  `composition/notes_edit.rs` (attach hook, `ReadModel`, order 30).
- LLM routes: `notes.edit` is now used (`NoteEditService`, structured
  `EditAnswer`); prompts `notes_edit.system.md.jinja`,
  `notes_edit.user.md.jinja` (`deino_llm::prompts::notes_edit`).
- Frontend: `features/notes/edit/` (bar, preview, store, selection
  numbering shared with Rust's text-block order).

## CLAUDE.md
- `crates/deino-core` row: after `notes (…)`, add "`notes::edit` (NOT-06:
  editing enhanced notes by asking; renames without the model, block
  changes applied in place; ADR 2026-10-12-edit-notes-by-asking)".

## Plan updates
- NOT-06 Accept: [x] Rename-everywhere requests update every occurrence
  (including mention labels and link text; citation targets, link targets
  and mention IDs never move): `notes::edit::rename_tests`,
  `service_tests::a_rename_updates_every_occurrence_without_the_model`.
  [x] Undo works:
  `service_tests::accepting_stores_an_edited_version_and_restoring_the_previous_undoes_it`,
  `edit-bar.test.tsx`. Tests use the mock LLM (`MockProvider`).
- NOT-06 card: the entry point is the "Edit with AI" bar, not meeting chat
  (D13).
