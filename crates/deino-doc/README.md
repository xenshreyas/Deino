# deino-doc

## Role

The note document (ADR 0022): TipTap/ProseMirror JSON as typed Rust, to and
from Markdown. Ported from anarlog's `crates/tiptap` (MIT); see
`THIRD_PARTY_NOTICES.md`.

## Owns

- The model: `Doc`, `Block`, `ListItem`, `TaskItem`, `Inline`, `Mark` and
  their attributes, serialized with TipTap's names, so the editor's
  `getJSON()` deserializes directly and the generated TypeScript matches.
  Beyond TipTap's StarterKit: the `origin` mark (user or AI, `edited`),
  `citation` nodes (`segmentId`, `startMs`), person `mention`s, task items,
  images by reference.
- `from_markdown`: CommonMark + GFM (markdown-rs) → `Doc`; nested marks are
  kept exactly, soft line breaks become hard breaks, images become blocks,
  unsafe links lose their target, tables become paragraphs.
- `to_markdown`: `Flavor::Lossless` (round-trips; Deino's extras as inline
  HTML, see `html.rs`) and `Flavor::Readable` (for people and LLMs).
- `validate`: the schema rules the types cannot express; `Doc::normalize`.

## Does Not Own

- Storage (`deino_core::notes`), the editor (`frontend/src/features/notepad`),
  images on disk, and what the notes mean (NOT-02's enhancement).

## Invariants

- `from_markdown` output always validates and is normalized.
- Every node and mark survives `Doc → Lossless Markdown → Doc`
  (`round_trip_tests.rs`), except whitespace at the edge of `**`, `*` and
  `~~` (moved outside, as CommonMark requires) and empty paragraphs.
- Validation messages never quote the document's text.
- No I/O, no Tauri, no async.
