//! [`Template`]: a stored template, built-in or the user's own.

use chrono::{DateTime, Utc};
use deino_domain::TemplateId;
use serde::Serialize;

use super::TemplateContent;

/// A note template.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Template {
    /// A slug for built-ins (`one_on_one`), a UUID for the user's own.
    pub id: TemplateId,
    /// True for the templates Deino ships. They cannot be edited or deleted;
    /// duplicating one makes an editable copy.
    pub is_builtin: bool,
    /// What the template says.
    pub content: TemplateContent,
    /// When it was created (for built-ins: first seeded).
    pub created_at: DateTime<Utc>,
    /// When it last changed (for built-ins: last upgraded).
    pub updated_at: DateTime<Utc>,
}
