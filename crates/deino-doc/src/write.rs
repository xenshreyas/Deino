//! [`Doc`] → Markdown.
//!
//! anarlog converts TipTap JSON to mdast and prints it with a vendored,
//! patched `mdast_util_to_markdown`, then strips every backslash escape for
//! readability, which makes its output lossy. Deino writes Markdown straight
//! from the typed model instead: a small writer that escapes only where
//! Markdown would otherwise change the meaning, so [`Flavor::Lossless`]
//! output parses back to the same document (see `round_trip_tests.rs`).

mod block;
mod escape;
mod inline;
mod list;

use crate::Doc;

/// What the Markdown is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Flavor {
    /// Parses back to the same document: underline, provenance, mentions and
    /// citations are written as the inline HTML in [`crate::html`]'s table.
    /// For copy and paste, exports that re-import, and tests.
    Lossless,
    /// For people and LLMs: mentions read `@Priya`, citations `[12:04]`, and
    /// underline and provenance are dropped. Line breaks are plain newlines.
    /// This is what `meeting_notes.notes_markdown` stores.
    Readable,
}

/// Writes `doc` as CommonMark + GFM, ending with a newline unless it is blank.
pub fn to_markdown(doc: &Doc, flavor: Flavor) -> String {
    let mut markdown = block::blocks(&doc.content, flavor);
    if !markdown.is_empty() {
        markdown.push('\n');
    }
    markdown
}
