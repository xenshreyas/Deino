//! The [`Authenticator`] port: proves the Mac's owner is present.

use async_trait::async_trait;
use deino_domain::{ErrorClass, ErrorCode};

/// Why the owner was not authenticated.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum AuthError {
    /// The owner dismissed the prompt.
    #[error("unlocking was cancelled")]
    Cancelled,
    /// Touch ID or the password did not match.
    #[error("Deino could not confirm it is you")]
    Failed,
    /// This Mac cannot authenticate its owner (no password set, or not macOS).
    #[error("this Mac cannot unlock Deino with Touch ID or a password")]
    Unavailable,
}

impl ErrorClass for AuthError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::Cancelled => ErrorCode::Cancelled,
            Self::Failed => ErrorCode::PermissionDenied,
            Self::Unavailable => ErrorCode::Unavailable,
        }
    }
}

/// Asks the Mac's owner to prove they are present.
#[async_trait]
pub trait Authenticator: Send + Sync {
    /// Shows the system prompt (Touch ID, falling back to the password)
    /// with `reason`, and resolves once the owner answers.
    async fn authenticate(&self, reason: &str) -> Result<(), AuthError>;
}

#[cfg(any(test, feature = "test-support"))]
pub use fake::FakeAuthenticator;

#[cfg(any(test, feature = "test-support"))]
mod fake {
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use async_trait::async_trait;

    use super::{AuthError, Authenticator};

    /// Answers every prompt the same way, and counts prompts.
    #[derive(Debug, Default)]
    pub struct FakeAuthenticator {
        answer: Mutex<Option<AuthError>>,
        prompts: AtomicUsize,
    }

    impl FakeAuthenticator {
        /// Makes the next prompts fail with `error` (`None` succeeds).
        pub fn answer(&self, error: Option<AuthError>) {
            *self
                .answer
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = error;
        }

        /// How many prompts were shown.
        pub fn prompts(&self) -> usize {
            self.prompts.load(Ordering::SeqCst)
        }
    }

    #[async_trait]
    impl Authenticator for FakeAuthenticator {
        async fn authenticate(&self, _reason: &str) -> Result<(), AuthError> {
            self.prompts.fetch_add(1, Ordering::SeqCst);
            let answer = *self
                .answer
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            answer.map_or(Ok(()), Err)
        }
    }
}
