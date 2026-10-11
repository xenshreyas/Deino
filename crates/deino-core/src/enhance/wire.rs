//! [`EnhancedDoc`]: the shape the LLM writes notes in
//! (`Gateway::structured::<EnhancedDoc>()`).
//!
//! The doc comments become the JSON schema's descriptions, so they are
//! written for the model. Evidence is a list of the transcript references
//! the prompt shows (`s12`), not segment IDs: short references are copied
//! reliably and cost a few tokens, and [`super::citations`] maps them back
//! to segments, dropping any the transcript does not have. The map step of
//! a long meeting answers in the same shape, so the merge keeps references.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A meeting's notes: sections in order.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct EnhancedDoc {
    /// The sections, in the template's order.
    pub sections: Vec<EnhancedSection>,
}

/// One section of the notes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct EnhancedSection {
    /// The section's heading, as the template names it.
    pub heading: String,
    /// The section's blocks, in reading order.
    pub blocks: Vec<EnhancedBlock>,
}

/// One bullet, paragraph or to-do of a section.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct EnhancedBlock {
    /// How the block reads: a bullet point, a paragraph, or a to-do.
    pub kind: BlockKind,
    /// The block's plain text. For `origin` "user", exactly the line the user typed.
    pub text: String,
    /// "user" for a line the user typed, copied exactly; "ai" for everything you write.
    pub origin: Origin,
    /// References of the transcript lines that support the block (`s12`), most relevant first.
    #[serde(default)]
    pub evidence: Vec<String>,
    /// AI sub-points under this block: detail from the transcript.
    #[serde(default)]
    pub details: Vec<EnhancedDetail>,
}

/// An AI sub-point under a block.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct EnhancedDetail {
    /// The detail's plain text.
    pub text: String,
    /// References of the transcript lines that support it (`s12`).
    #[serde(default)]
    pub evidence: Vec<String>,
}

/// How a block reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum BlockKind {
    /// A bullet point.
    Bullet,
    /// A paragraph of prose.
    Paragraph,
    /// An action to take, shown as a checkbox.
    Todo,
}

/// Who a block's text comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Origin {
    /// The user typed it.
    User,
    /// The AI wrote it.
    Ai,
}
