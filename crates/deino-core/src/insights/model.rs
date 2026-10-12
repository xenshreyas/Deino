//! An insight definition: what to ask of a meeting, in what shape, how many
//! answers to keep, and when it runs on its own.

use chrono::{DateTime, Utc};
use deino_domain::{ConditionGroups, InsightId};
use serde::{Deserialize, Serialize};

/// One typed field of an insight's answers ("Budget": a number).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct InsightField {
    /// The column name, unique within the insight ignoring case.
    pub name: String,
    /// What a value of it is.
    pub kind: FieldKind,
}

/// The type of a field's values.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FieldKind {
    /// Free text.
    Text,
    /// A number ("120", "4.5").
    Number,
    /// Yes or no.
    Boolean,
    /// A calendar day.
    Date,
    /// One of a fixed list ("Red", "Yellow", "Green").
    Enum {
        /// The allowed values, in the order the editor lists them.
        options: Vec<String>,
    },
}

impl FieldKind {
    /// The kind's name, as the prompt and the schema describe it.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Number => "number",
            Self::Boolean => "yes/no",
            Self::Date => "date (YYYY-MM-DD)",
            Self::Enum { .. } => "one of the listed options",
        }
    }
}

/// How many answers an insight keeps per meeting.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Cardinality {
    /// As many as the meeting holds ("every question asked"), up to
    /// [`MAX_AUTO_ANSWERS`](super::MAX_AUTO_ANSWERS).
    Auto,
    /// At most `count` ("one": "How did they hear about us?").
    Fixed {
        /// 1 to [`MAX_FIXED_ANSWERS`](super::MAX_FIXED_ANSWERS).
        count: u8,
    },
}

impl Cardinality {
    /// The most answers a run keeps.
    pub fn limit(self) -> usize {
        match self {
            Self::Auto => super::MAX_AUTO_ANSWERS,
            Self::Fixed { count } => usize::from(count),
        }
    }
}

/// When an insight runs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum ApplyWhen {
    /// After every meeting the conditions match (no conditions: every
    /// meeting), as the post-meeting pipeline's `insights` stage.
    Automatic {
        /// Which meetings (AUT-01's condition model).
        conditions: ConditionGroups,
    },
    /// Only when the user runs it on a meeting.
    Manual,
}

/// What the user edits: everything about an insight but its identity.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct InsightInput {
    /// Its name ("Competitor mentions").
    pub name: String,
    /// What to extract or write, with an example: the prompt.
    pub description: String,
    /// The answer's typed fields; none means each answer is one text.
    pub fields: Vec<InsightField>,
    /// How many answers to keep.
    pub cardinality: Cardinality,
    /// When it runs.
    pub apply_when: ApplyWhen,
}

/// A stored insight definition.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct InsightDef {
    /// Its ID.
    pub id: InsightId,
    /// Its name.
    pub name: String,
    /// The prompt.
    pub description: String,
    /// The typed fields, in column order.
    pub fields: Vec<InsightField>,
    /// How many answers to keep.
    pub cardinality: Cardinality,
    /// When it runs.
    pub apply_when: ApplyWhen,
    /// Bumped by every edit of what it asks; results remember theirs.
    pub revision: u32,
    /// When it was created.
    pub created_at: DateTime<Utc>,
    /// When it last changed.
    pub updated_at: DateTime<Utc>,
}

impl InsightDef {
    /// True when its answers are one text each, not typed fields.
    pub fn is_free_text(&self) -> bool {
        self.fields.is_empty()
    }

    /// True when edits from `input` change what the insight asks, so its
    /// existing results answer an older question.
    pub fn asks_differently(&self, input: &InsightInput) -> bool {
        self.name != input.name
            || self.description != input.description
            || self.fields != input.fields
            || self.cardinality != input.cardinality
    }

    /// The conditions under which it runs on its own; `None` for a manual one.
    pub fn automatic_conditions(&self) -> Option<&ConditionGroups> {
        match &self.apply_when {
            ApplyWhen::Automatic { conditions } => Some(conditions),
            ApplyWhen::Manual => None,
        }
    }
}
