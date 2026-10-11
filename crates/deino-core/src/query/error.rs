//! What can go wrong reading the library. Messages name what was asked for
//! (an ID, a name the caller typed) and never meeting content.

use deino_db::DbError;
use deino_domain::{ErrorClass, ErrorCode, MeetingId};

use crate::action_items::ActionItemError;
use crate::export::ExportError;
use crate::folders::FolderError;
use crate::search::SearchError;

/// The kinds of things a caller can name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntityKind {
    /// A meeting.
    Meeting,
    /// A tag.
    Tag,
    /// A folder.
    Folder,
    /// A person.
    Person,
    /// A company.
    Company,
}

impl EntityKind {
    /// The word for it in messages.
    pub const fn noun(self) -> &'static str {
        match self {
            Self::Meeting => "meeting",
            Self::Tag => "tag",
            Self::Folder => "folder",
            Self::Person => "person",
            Self::Company => "company",
        }
    }

    /// The word for several of them.
    pub const fn plural(self) -> &'static str {
        match self {
            Self::Meeting => "meetings",
            Self::Tag => "tags",
            Self::Folder => "folders",
            Self::Person => "people",
            Self::Company => "companies",
        }
    }
}

/// A read failure.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum QueryError {
    /// Nothing matches the reference.
    #[error("no {} matches \"{reference}\"", .kind.noun())]
    NotFound {
        /// What was looked for.
        kind: EntityKind,
        /// The ID or name the caller gave.
        reference: String,
    },
    /// Several things match the reference; the caller must pick one.
    #[error("\"{reference}\" matches several {}: {}", .kind.plural(), .candidates.join(", "))]
    Ambiguous {
        /// What was looked for.
        kind: EntityKind,
        /// The name the caller gave.
        reference: String,
        /// The names that matched.
        candidates: Vec<String>,
    },
    /// A time or date range ends before it starts.
    #[error("the range ends before it starts")]
    InvalidRange,
    /// The database failed; the source carries the detail.
    #[error("could not read the Deino database")]
    Storage(#[from] DbError),
    /// Action items could not be read.
    #[error(transparent)]
    ActionItems(#[from] ActionItemError),
    /// Folders could not be read.
    #[error(transparent)]
    Folders(#[from] FolderError),
    /// A meeting could not be assembled.
    #[error(transparent)]
    Export(ExportError),
    /// Search failed.
    #[error(transparent)]
    Search(#[from] SearchError),
}

impl QueryError {
    /// The meeting `id` does not exist.
    pub fn meeting_not_found(id: &MeetingId) -> Self {
        Self::NotFound {
            kind: EntityKind::Meeting,
            reference: id.to_string(),
        }
    }
}

impl From<ExportError> for QueryError {
    fn from(error: ExportError) -> Self {
        match error {
            ExportError::NotFound(id) => Self::meeting_not_found(&id),
            ExportError::Storage(error) => Self::Storage(error),
            other => Self::Export(other),
        }
    }
}

impl ErrorClass for QueryError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::NotFound { .. } => ErrorCode::NotFound,
            Self::Ambiguous { .. } | Self::InvalidRange => ErrorCode::Invalid,
            Self::Storage(error) => error.code(),
            Self::ActionItems(error) => error.code(),
            Self::Folders(error) => error.code(),
            Self::Export(error) => error.code(),
            Self::Search(error) => error.code(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messages_name_the_reference_and_the_candidates() {
        let missing = QueryError::NotFound {
            kind: EntityKind::Tag,
            reference: "Sales".into(),
        };
        let ambiguous = QueryError::Ambiguous {
            kind: EntityKind::Person,
            reference: "ma".into(),
            candidates: vec!["Maya Chen".into(), "Marco Rossi".into()],
        };

        assert_eq!(missing.to_string(), "no tag matches \"Sales\"");
        assert_eq!(missing.code(), ErrorCode::NotFound);
        assert_eq!(
            ambiguous.to_string(),
            "\"ma\" matches several people: Maya Chen, Marco Rossi"
        );
        assert_eq!(ambiguous.code(), ErrorCode::Invalid);
    }

    #[test]
    fn an_export_not_found_is_a_missing_meeting() {
        let error = QueryError::from(ExportError::NotFound(MeetingId::new("meeting-x")));

        assert!(matches!(
            error,
            QueryError::NotFound {
                kind: EntityKind::Meeting,
                ..
            }
        ));
    }
}
