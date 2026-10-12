//! What can go wrong with recipes.

use deino_db::DbError;
use deino_domain::{ErrorClass, ErrorCode, RecipeId};

/// A recipes failure. Messages are user-safe: IDs and field names, never a
/// recipe's prompt.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum RecipeError {
    /// No recipe has the ID.
    #[error("recipe {0} was not found")]
    NotFound(RecipeId),
    /// Built-in recipes are read-only; remix one to change it.
    #[error("built-in recipes cannot be changed; remix it to make your own copy")]
    BuiltIn,
    /// A multi-meeting recipe; only single-meeting recipes run for now (D13).
    #[error("multi-meeting recipes are not available yet")]
    MultiMeeting,
    /// A field is missing, too long or malformed.
    #[error("the recipe's {field} {reason}")]
    Invalid {
        /// The field.
        field: &'static str,
        /// What is wrong with it.
        reason: &'static str,
    },
    /// An import or share string that is not a Deino recipe.
    #[error("this is not a Deino recipe: {0}")]
    Unreadable(&'static str),
    /// The database failed; the source carries the detail.
    #[error("could not load or save the recipe")]
    Storage(#[from] DbError),
}

impl RecipeError {
    /// A field error.
    pub(crate) const fn invalid(field: &'static str, reason: &'static str) -> Self {
        Self::Invalid { field, reason }
    }
}

impl ErrorClass for RecipeError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::NotFound(_) => ErrorCode::NotFound,
            Self::BuiltIn => ErrorCode::PermissionDenied,
            Self::MultiMeeting => ErrorCode::Unavailable,
            Self::Invalid { .. } | Self::Unreadable(_) => ErrorCode::Invalid,
            Self::Storage(_) => ErrorCode::Internal,
        }
    }
}
