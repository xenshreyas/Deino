//! What can go wrong with people and companies.

use deino_db::DbError;
use deino_domain::{CompanyId, ErrorClass, ErrorCode, PersonId};
use deino_llm::LlmError;

use super::{LogoError, MAX_NAME_CHARS};

/// A people or companies failure. Messages are user-safe: IDs and limits,
/// never names, addresses or meeting content.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum EntityError {
    /// No person has the requested ID.
    #[error("person {0} was not found")]
    PersonNotFound(PersonId),
    /// No company has the requested ID.
    #[error("company {0} was not found")]
    CompanyNotFound(CompanyId),
    /// The name is blank once whitespace is trimmed.
    #[error("a name cannot be empty")]
    EmptyName,
    /// The name is longer than [`MAX_NAME_CHARS`].
    #[error("a name can be at most {MAX_NAME_CHARS} characters long")]
    NameTooLong,
    /// The text is not an email address.
    #[error("that is not an email address")]
    InvalidEmail,
    /// The text is not a web domain.
    #[error("that is not a web domain, such as acme.com")]
    InvalidDomain,
    /// Another person already has the email address.
    #[error("someone else already has that email address; merge the two instead")]
    EmailTaken,
    /// Another company already has the domain.
    #[error("another company already has that domain; merge the two instead")]
    DomainTaken,
    /// Something cannot be merged into itself.
    #[error("pick two different entries to merge")]
    SameEntity,
    /// A logo could not be fetched or stored; the source says how.
    #[error(transparent)]
    Logo(#[from] LogoError),
    /// The summary model call failed; the source says how.
    #[error(transparent)]
    Llm(#[from] LlmError),
    /// The database failed; the source carries the detail.
    #[error("could not load or save people and companies")]
    Storage(#[from] DbError),
}

impl ErrorClass for EntityError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::PersonNotFound(_) | Self::CompanyNotFound(_) => ErrorCode::NotFound,
            Self::EmptyName
            | Self::NameTooLong
            | Self::InvalidEmail
            | Self::InvalidDomain
            | Self::SameEntity => ErrorCode::Invalid,
            Self::EmailTaken | Self::DomainTaken => ErrorCode::Conflict,
            Self::Logo(error) => error.code(),
            Self::Llm(error) => error.code(),
            Self::Storage(_) => ErrorCode::Internal,
        }
    }
}

impl EntityError {
    /// `error` as `clash` when it breaks a UNIQUE constraint (an email or a
    /// domain someone else has), else as a storage failure.
    pub(super) fn unique_or(clash: Self) -> impl FnOnce(DbError) -> Self {
        move |error| {
            if error.is_unique_violation() {
                clash
            } else {
                Self::Storage(error)
            }
        }
    }

    /// True when trying again later may succeed: a busy database, or a
    /// provider that is rate limited, overloaded or unreachable.
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::Llm(error) => error.is_retryable(),
            Self::Storage(_) => true,
            _ => false,
        }
    }
}
