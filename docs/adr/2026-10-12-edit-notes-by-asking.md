# Edit notes by asking: block changes applied in place, a rename shortcut, versions for undo

- **Status:** Accepted
- **Date:** 2026-10-12
- **Card:** NOT-06 (edit notes by asking)

## Context

NOT-06 lets the user change their enhanced notes in words: "make this more
detailed", "Niall is spelled Nyal", "remove the pricing section", scoped to
the selection when there is one, previewed as a diff, accepted or rejected,
and undoable. The card puts it in the meeting chat, which D13 parks, so the
entry point is an "Edit with AI" command bar on the notes. ADR 0030's
promises still hold: the user's typed lines and the citations are not lost,
and a user's edit is never overwritten without asking.

The stored notes are a TipTap document (ADR 0022), not NOT-02's
`EnhancedDoc`: once the user has edited them they can hold ordered lists,
quotes, images, links, mentions, bold text and AI text marked edited, none
of which `EnhancedDoc` can carry. Models also reproduce long documents
imperfectly: asked to return the whole notes, they reword lines nobody asked
about and drop citations.

## Decision

**The model answers a list of block changes; Rust applies them in place.**

- The notes are shown as one JSON object per text block (every paragraph,
  heading and code block in reading order) in `EnhancedDoc`'s vocabulary:
  `{id: "b3", kind, origin: user|ai|mixed, text}`, citations written as the
  transcript references `[s12]` NOT-02 uses, mentions as `@Name`. The
  transcript follows, so new detail can be cited.
- `notes.edit` returns `EditAnswer { changes: [{action: replace | delete |
  insert_after, block, text, kind}] }` through `Gateway::structured`. Every
  block the answer does not name stays byte for byte: origin marks,
  formatting, links, mentions, citations, images.
- A replaced block's new text is aligned with its old words (longest common
  subsequence over words): kept words keep their run's marks, kept mentions
  and citations stay the same nodes, new words are AI text. New `[s12]`
  references resolve to citations of real transcript lines, unknown ones
  are dropped, and citations the new text left out are put back at its end,
  so a rewrite never loses evidence.
- The rules are enforced in Rust whatever the model answers: changes
  outside the selection are dropped; a locked block (outside the selection,
  a code block, or one whose text had a secret redacted) may be deleted but
  never rewritten; unknown IDs are dropped.

**Plain renames skip the model.** A request that is wholly a rename ("X is
spelled Y", "rename X to Y everywhere", "replace X with Y", "it's Y, not
X", "X → Y"), with both names at most four words and not starting with an
article or "this", replaces every whole-word occurrence, ignoring case and
keeping the occurrence's capital: in text runs (a link's text included,
never its `href`) and mention labels (never the person ID). Citation nodes
are not touched, so their targets never move. A rename that matches
nothing falls through to the model.

**The selection is a list of text block positions.** The UI numbers
ProseMirror's textblocks in document order exactly as Rust numbers the
document's paragraphs, headings and code blocks, so no positions or text
offsets cross IPC. Scope is per block: a partly selected line is in scope.

**Redaction (NOT-09).** With "Hide sensitive information" on, the request,
the blocks and the transcript are redacted in one batch before the prompt
is built. A block whose text changed under redaction is locked: deleting it
is allowed, rewriting it is not, so a secret is neither sent nor replaced
by `[redacted]` in the notes.

**Accept writes a version; undo restores one.** Proposing stores nothing.
Accepting (`notes-edit|accept`, with the version the proposal was made
from, refused with `conflict` once it is not the newest) inserts a new
`enhanced_notes` version marked `is_user_edited`, keeping the base's
template, model and basis, so regenerating asks before replacing it. Undo
is the existing `enhance|restore` of the previous version.

**⌘⇧E.** The card asks for ⌘⇧E, which already enhances or regenerates on
the meeting page. While the caret is in the enhanced notes, ⌘⇧E opens
"Edit with AI" (the editor stops the event); anywhere else on the page it
stays Enhance.

## Alternatives

- **Rewrite the whole `EnhancedDoc` and rebuild with NOT-02's assembler.**
  Loses everything `EnhancedDoc` cannot express, rewords untouched lines,
  and the provenance guard would put back user lines the user asked to
  remove. Rejected.
- **Free text or a Markdown diff from the model.** Cheaper to prompt, but
  provenance and citations would rest on the model, and patches of Markdown
  apply unreliably. Rejected.
- **Character offsets for the selection.** Exact, but ProseMirror positions
  count node boundaries that Rust's model does not, and a partial-line edit
  would need inline splitting the model cannot target. Block granularity is
  what users select in practice.
- **Store the proposal server-side.** Accept would send an ID instead of
  the document, but proposals would need expiry and a table; the document
  goes back validated, like a save.

## Consequences

- `deino-core::notes::edit` owns the feature; the app exposes the
  `notes-edit` plugin (`propose`, `accept`); the UI lives in
  `features/notes/edit/`.
- The transcript in the prompt is cut at 60,000 characters; edits of a long
  meeting's last part may lack its detail.
- A deleted list item takes its nested items with it.
- When the chat agent (AI-02) or the single-meeting Ask panel lands, it can
  call the same `propose` and show the same preview as its `edit_notes`
  tool.
