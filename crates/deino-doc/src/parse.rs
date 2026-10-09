//! Markdown → [`Doc`]: parses CommonMark + GFM with markdown-rs and maps its
//! syntax tree onto the document model.
//!
//! Adapted from anarlog's `crates/tiptap/src/from_md.rs` (MIT). Deino keeps
//! nested formatting exact (anarlog flattens `**a *b* c**` into one run),
//! turns soft line breaks into hard breaks so a note keeps the author's
//! lines, lifts images out of paragraphs (an image is a block in the
//! editor), and reads back the inline HTML that [`to_markdown`](crate::to_markdown)
//! writes for what Markdown cannot say: `<u>`, origin spans, mentions and
//! citations. Constructs the editor has no node for keep their text: a table
//! becomes one paragraph per row, unknown HTML stays literal.

mod block;
mod inline;

use markdown::ParseOptions;

use crate::{Doc, DocError};

/// Parses `markdown` into a document. The result always passes
/// [`validate`](crate::validate), and an empty input gives [`Doc::empty`].
pub fn from_markdown(markdown: &str) -> Result<Doc, DocError> {
    let tree = markdown::to_mdast(markdown, &options())
        .map_err(|message| DocError::Markdown(message.to_string()))?;
    let content = tree
        .children()
        .map(|nodes| block::blocks(nodes))
        .unwrap_or_default();
    if content.is_empty() {
        return Ok(Doc::empty());
    }
    Ok(Doc { content }.normalized())
}

/// GFM without the constructs that would change a round trip: bare URLs
/// stay text (the editor links them explicitly), `~one~` is not
/// strikethrough, and there are no footnotes or front matter.
fn options() -> ParseOptions {
    let mut options = ParseOptions::gfm();
    options.constructs.gfm_autolink_literal = false;
    options.constructs.gfm_footnote_definition = false;
    options.constructs.gfm_label_start_footnote = false;
    options.constructs.frontmatter = false;
    options.gfm_strikethrough_single_tilde = false;
    options
}
