//! What can go wrong in the folders feature.

use deino_db::DbError;
use deino_domain::{ErrorClass, ErrorCode, FolderId, FolderRuleId, MeetingId, TagId};

use super::{MAX_FOLDER_DESCRIPTION_CHARS, MAX_FOLDER_NAME_CHARS};

/// A folders failure. Messages are user-safe: IDs and limits, never folder
/// names or meeting content.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum FolderError {
    /// No folder has the requested ID.
    #[error("folder {0} was not found")]
    NotFound(FolderId),
    /// No meeting outside the trash has the requested ID.
    #[error("meeting {0} was not found")]
    MeetingNotFound(MeetingId),
    /// A tag rule names a tag that does not exist.
    #[error("tag {0} was not found")]
    TagNotFound(TagId),
    /// No auto-add rule has the requested ID.
    #[error("rule {0} was not found")]
    RuleNotFound(FolderRuleId),
    /// The folder name is blank once whitespace is trimmed.
    #[error("a folder name cannot be empty")]
    EmptyName,
    /// The folder name is longer than [`MAX_FOLDER_NAME_CHARS`].
    #[error("a folder name can be at most {MAX_FOLDER_NAME_CHARS} characters long")]
    NameTooLong,
    /// The description is longer than [`MAX_FOLDER_DESCRIPTION_CHARS`].
    #[error("a folder description can be at most {MAX_FOLDER_DESCRIPTION_CHARS} characters long")]
    DescriptionTooLong,
    /// The icon is not a lower-case icon name.
    #[error("an icon name uses lower-case letters, digits and dashes")]
    InvalidIcon,
    /// An attendee-domain rule's domain is not a domain.
    #[error("an attendee domain looks like \"acme.com\"")]
    InvalidDomain,
    /// A sibling folder already has the name, ignoring case.
    #[error("a folder with that name already exists here")]
    NameTaken,
    /// The new parent is the folder itself or one of its subfolders.
    #[error("a folder cannot move into itself or one of its subfolders")]
    WouldCycle,
    /// The database failed; the source carries the detail.
    #[error("could not load or save folders")]
    Storage(#[from] DbError),
}

impl ErrorClass for FolderError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::NotFound(_)
            | Self::MeetingNotFound(_)
            | Self::TagNotFound(_)
            | Self::RuleNotFound(_) => ErrorCode::NotFound,
            Self::EmptyName
            | Self::NameTooLong
            | Self::DescriptionTooLong
            | Self::InvalidIcon
            | Self::InvalidDomain => ErrorCode::Invalid,
            Self::NameTaken | Self::WouldCycle => ErrorCode::Conflict,
            Self::Storage(_) => ErrorCode::Internal,
        }
    }
}

impl FolderError {
    /// `error` as [`FolderError::NameTaken`] when it is a sibling-name clash
    /// (`idx_folders_parent_id_name`), else as a storage failure.
    pub(super) fn naming(error: DbError) -> Self {
        if error.is_unique_violation() {
            Self::NameTaken
        } else {
            Self::Storage(error)
        }
    }
}
