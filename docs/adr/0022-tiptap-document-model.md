# 0022. Store notes as TipTap JSON and read them in Rust with `deino-doc`

- **Status:** Accepted
- **Date:** 2026-10-09
- **Card:** NOT-01 (prepares NOT-02, NOT-03)

## Context

The notepad needs a rich editor (Markdown shortcuts, checklists, images,
@mentions), and NOT-02 needs Rust to build and read the same documents: the
`enhance_notes` stage turns LLM output into a document whose runs carry
provenance (user or AI, and AI text the user edited) and citations
(`segment_id`, `start_ms`), and exports, search, MCP and LLM context need it
back as Markdown. The inherited summary editor, BlockNote 0.36, keeps its
own block JSON that only its JavaScript can produce, needed a pinned single
ProseMirror and an HSL variable bridge, and was about half of the meeting
page's JavaScript. `docs/references/architecture.md` §7 recommended TipTap 3
with anarlog's `crates/tiptap` (TipTap JSON ↔ Markdown, validation, insta
tests) as the Rust side.

## Decision

- The editor is **TipTap 3** (MIT): StarterKit, task lists, mention and
  placeholder, plus Deino's `origin` mark, `citation` node and a stored-image
  node. Its `getJSON()` is the stored format (`meeting_notes.notes_json`).
- **`crates/deino-doc`** (no Tauri, no I/O), ported from anarlog's
  `crates/tiptap`, owns that format in Rust: a typed model (`Doc`, `Block`,
  `Inline`, `Mark`) whose serde names are TipTap's, so it deserializes the
  editor's JSON directly and specta generates the matching TypeScript;
  `validate` (the ProseMirror content rules plus Deino's mark and link
  rules); `normalize` (sorted marks, merged runs); `from_markdown`
  (markdown-rs, CommonMark + GFM); `to_markdown` in two flavors.
- **Markdown has two flavors.** `Lossless` round-trips every node and mark:
  what Markdown cannot say is inline HTML (`<u>`, `<span data-origin>`,
  `<mention … />`, `<citation … />`). `Readable` is for people and LLMs
  (`@Priya`, `[12:04]`, no provenance) and is what `notes_markdown` stores.
- The **UI validates at the boundary** with a zod schema typed against the
  generated `doc` type; Rust validates again before storing.
- Notes are saved **whole** on every autosave (about a second after typing
  pauses), never as diffs: the document is small, the write is one row, and
  a crash loses at most that second.

```rust
let doc = deino_doc::from_markdown("- [ ] send the deck to <mention data-id=\"…\" data-label=\"Priya\" />")?;
deino_doc::validate(&doc)?;
let for_the_llm = deino_doc::to_markdown(&doc, deino_doc::Flavor::Readable);
```

## Alternatives

- **Keep BlockNote** (`ui.md` §5.2's earlier choice): its JSON is only
  producible in JavaScript, so NOT-02 would have to round-trip through the
  webview or keep a second format. Rejected.
- **Raw ProseMirror** (what anarlog runs today): full control over node
  views, but weeks of work for the Markdown input rules, lists and
  suggestions TipTap ships. TipTap's JSON is ProseMirror's, so moving later
  keeps the stored format.
- **Port anarlog's Markdown writer as is** (mdast plus a vendored, patched
  `mdast_util_to_markdown`, then stripping every backslash escape): its
  output is not lossless (`* item` typed as text comes back as a list).
  Deino writes Markdown from the typed model with targeted escaping, tested
  by round trips of every node and mark and of the fixtures' notes.
- **A JS Markdown converter** (`@tiptap/markdown`): a second implementation
  of the same mapping that would drift from Rust's. The summary view asks
  Rust (`notes|markdown_to_doc`) instead; only `dev:mock` has a small
  stand-in converter.

## Consequences

- One document model for the notepad, enhanced notes, exports and search.
  A new node or mark is added in four places (model, Markdown, TipTap
  extension, zod schema); `docs/ARCHITECTURE.md` §8 lists them.
- Whitespace at the edge of bold, italic or strikethrough moves just outside
  it in Markdown (CommonMark cannot express it); provenance and links keep
  their whitespace. Empty paragraphs are not kept in Markdown.
- BlockNote and its dependencies are gone: the app's JavaScript went from
  2,746 kB to 1,460 kB (gzip 785 → 445 kB), and TipTap loads lazily on
  first use.
- Revisit if collaborative editing (Yjs) or a block-handle UI is ever
  wanted: both work on ProseMirror and keep this JSON.
