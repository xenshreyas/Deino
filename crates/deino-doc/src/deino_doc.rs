//! Deino's note document: TipTap JSON as typed Rust, to and from Markdown.
//!
//! The editor (TipTap 3, ADR 0022) stores ProseMirror JSON; this crate is
//! where Rust reads and writes it, so the notes service, the NOT-02
//! enhancement stage, exports, search and MCP all share one model:
//!
//! - [`Doc`], [`Block`], [`Inline`], [`Mark`]: the schema, serialized exactly
//!   as TipTap's `getJSON()`, extended with what NOT-02 and NOT-03 need: the
//!   [`Mark::Origin`] provenance mark (user or AI, and AI text the user
//!   edited), [`Inline::Citation`] (a transcript segment and its start), and
//!   [`Inline::Mention`] people chips;
//! - [`from_markdown`]: CommonMark + GFM → [`Doc`];
//! - [`to_markdown`]: [`Doc`] → Markdown, [`Flavor::Lossless`] (round-trips)
//!   or [`Flavor::Readable`] (for people and LLMs);
//! - [`validate`]: the schema rules the types cannot express;
//! - [`Doc::normalize`]: the canonical form documents are stored in.
//!
//! Adapted from anarlog's `crates/tiptap` (MIT; see `THIRD_PARTY_NOTICES.md`).
//! The crate is pure: no I/O, no Tauri, no async.

mod html;
mod model;
mod normalize;
mod parse;
mod validate;
mod write;

#[cfg(test)]
mod parse_tests;
#[cfg(test)]
mod round_trip_tests;
#[cfg(test)]
mod test_support;
#[cfg(test)]
mod validate_tests;
#[cfg(test)]
mod write_tests;

pub use model::{
    Author, Block, CitationAttrs, CodeBlockAttrs, Doc, HeadingAttrs, ImageAttrs, Inline, LinkAttrs,
    ListItem, Mark, MentionAttrs, OrderedListAttrs, OriginAttrs, TaskItem, TaskItemAttrs,
};
pub use parse::from_markdown;
pub use validate::{Issue, is_safe_href, validate};
pub use write::{Flavor, to_markdown};

/// Why a document could not be read or accepted.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum DocError {
    /// The Markdown parser refused the input (only MDX syntax can fail, and
    /// MDX is off, so this is a guard).
    #[error("could not parse the Markdown: {0}")]
    Markdown(String),
    /// The document breaks the schema; each issue names its place.
    #[error("the document is invalid ({} issues, first {})", .0.len(), first_issue(.0))]
    Invalid(Vec<Issue>),
}

fn first_issue(issues: &[Issue]) -> String {
    issues.first().map_or_else(String::new, ToString::to_string)
}
