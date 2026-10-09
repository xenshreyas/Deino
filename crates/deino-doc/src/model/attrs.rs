//! Node and mark attributes, named as TipTap names them.

use deino_domain::{Millis, PersonId, SegmentId};
use serde::{Deserialize, Serialize};

/// A heading's level, 1 (largest) to 6.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct HeadingAttrs {
    /// 1 to 6; [`validate`](crate::validate) rejects anything else.
    pub level: u8,
}

/// Where a numbered list starts counting.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct OrderedListAttrs {
    /// The first item's number.
    #[serde(default = "first_number")]
    pub start: u32,
}

impl Default for OrderedListAttrs {
    fn default() -> Self {
        Self {
            start: first_number(),
        }
    }
}

/// Lists count from 1 unless told otherwise.
const fn first_number() -> u32 {
    1
}

/// A code block's language (`rust`, `sql`), used for the Markdown fence.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct CodeBlockAttrs {
    /// The language, if the author named one.
    #[serde(default)]
    pub language: Option<String>,
}

/// Whether a checklist item is ticked.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct TaskItemAttrs {
    /// True once ticked.
    #[serde(default)]
    pub checked: bool,
}

/// An image reference.
///
/// In notes, `src` is relative to the meeting's folder (`images/<id>.png`)
/// so the document survives the app data folder moving; the UI resolves it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct ImageAttrs {
    /// Where the image is.
    pub src: String,
    /// Alternative text.
    #[serde(default)]
    pub alt: Option<String>,
    /// A tooltip title.
    #[serde(default)]
    pub title: Option<String>,
}

/// A mentioned person and the label shown on their chip.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct MentionAttrs {
    /// The person (TipTap's mention `id`).
    pub id: PersonId,
    /// Their name as written when they were mentioned.
    pub label: String,
}

/// A transcript moment that backs a statement.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CitationAttrs {
    /// The cited segment.
    pub segment_id: SegmentId,
    /// Where the segment starts in the recording, for playback.
    pub start_ms: Millis,
}

/// A hyperlink target. Only `http`, `https` and `mailto` links are valid.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct LinkAttrs {
    /// The target URL.
    pub href: String,
    /// A tooltip title.
    #[serde(default)]
    pub title: Option<String>,
}

/// Who wrote a run of text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum Author {
    /// Typed by the user.
    User,
    /// Written by the AI (NOT-02).
    Ai,
}

/// The provenance of a run of text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct OriginAttrs {
    /// Who wrote it.
    pub author: Author,
    /// True once the user has edited AI text; it stays AI text, marked edited.
    /// Always false for user text.
    #[serde(default)]
    pub edited: bool,
}

impl OriginAttrs {
    /// Text the user typed.
    pub const USER: Self = Self {
        author: Author::User,
        edited: false,
    };

    /// Text the AI wrote, untouched.
    pub const AI: Self = Self {
        author: Author::Ai,
        edited: false,
    };
}
