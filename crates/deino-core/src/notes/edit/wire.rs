//! [`EditAnswer`]: the shape the `notes.edit` route answers in
//! (`Gateway::structured::<EditAnswer>()`).
//!
//! The doc comments become the JSON schema's descriptions, so they are
//! written for the model. The answer is a list of changes to blocks named by
//! their `b3` IDs, not a rewritten document: everything the model leaves out
//! stays byte for byte, origin marks, formatting and citations included.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The changes to make to the notes.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct EditAnswer {
    /// The changes, in reading order. Empty when nothing should change.
    pub changes: Vec<EditChange>,
}

/// One change to one block.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct EditChange {
    /// What to do with the block.
    pub action: EditAction,
    /// The block's id (`b3`): the block to replace or delete, or the one the new block follows.
    pub block: String,
    /// The new text, for `replace` and `insert_after`; plain text, with `[s12]` references after the statements they support.
    #[serde(default)]
    pub text: String,
    /// The new block's kind, for `insert_after`.
    #[serde(default)]
    pub kind: Option<NewKind>,
}

/// What a change does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EditAction {
    /// Replace the block's whole text.
    Replace,
    /// Remove the block.
    Delete,
    /// Add a new block after this one.
    InsertAfter,
}

/// The kind of a new block.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum NewKind {
    /// A section heading.
    Heading,
    /// A paragraph.
    Paragraph,
    /// A bullet point.
    #[default]
    Bullet,
    /// An action to take, shown as a checkbox.
    Todo,
}
