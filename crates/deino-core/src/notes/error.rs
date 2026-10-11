//! What can go wrong in the notes feature. Messages name IDs and limits,
//! never what the user wrote.

use deino_db::DbError;
use deino_doc::DocError;
use deino_domain::{ErrorClass, ErrorCode, MeetingId};

use super::{MAX_DOC_BYTES, MAX_IMAGE_BYTES};
use crate::meetings::MeetingError;

/// A notes failure.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum NotesError {
    /// No meeting has the requested ID.
    #[error("meeting {0} was not found")]
    NotFound(MeetingId),
    /// The document breaks the editor's schema.
    #[error("the notes are not a valid document")]
    InvalidDoc(#[source] DocError),
    /// The document is larger than [`MAX_DOC_BYTES`].
    #[error("notes can be at most {} MB", MAX_DOC_BYTES / 1024 / 1024)]
    TooLarge,
    /// An image node points somewhere other than the meeting's images.
    #[error("an image in the notes is not one of this meeting's images")]
    ForeignImage,
    /// The bytes are not a PNG, JPEG, GIF or WebP image.
    #[error("images must be PNG, JPEG, GIF or WebP")]
    UnsupportedImage,
    /// The image is larger than [`MAX_IMAGE_BYTES`].
    #[error("images can be at most {} MB", MAX_IMAGE_BYTES / 1024 / 1024)]
    ImageTooLarge,
    /// Creating the scheduled meeting failed (for example, a blank title).
    #[error(transparent)]
    Meeting(#[from] MeetingError),
    /// Reading or writing the meeting's image folder failed.
    #[error("could not save the image")]
    Files(#[source] std::io::Error),
    /// The database failed; the source carries the detail.
    #[error("could not load or save the notes")]
    Storage(#[from] DbError),
}

impl ErrorClass for NotesError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::NotFound(_) => ErrorCode::NotFound,
            Self::InvalidDoc(_)
            | Self::TooLarge
            | Self::ForeignImage
            | Self::UnsupportedImage
            | Self::ImageTooLarge => ErrorCode::Invalid,
            Self::Meeting(error) => error.code(),
            Self::Files(_) | Self::Storage(_) => ErrorCode::Internal,
        }
    }
}
