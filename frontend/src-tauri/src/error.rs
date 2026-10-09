//! The command boundary's error handling.
//!
//! Feature commands end with `.into_ipc()`: the module error is logged with
//! its whole source chain (the only place a chain is logged) and converted
//! into [`AppError`], the `{ code, message }` shape the UI receives
//! (ADR 0009). Messages are user-safe by construction (`ErrorClass`), so
//! nothing here can leak meeting content to the UI or the log.

use std::error::Error;
use std::fmt;

use deino_domain::{AppError, ErrorClass, ErrorCode};

/// Converts a service result into a command result.
pub(crate) trait IntoIpc<T> {
    /// Logs the error chain, if any, and maps the error to [`AppError`].
    fn into_ipc(self) -> Result<T, AppError>;
}

impl<T, E: ErrorClass> IntoIpc<T> for Result<T, E> {
    fn into_ipc(self) -> Result<T, AppError> {
        self.map_err(|error| {
            let code = error.code();
            // Rejected input and missing rows are expected; failures are not.
            let level = if is_failure(code) {
                log::Level::Warn
            } else {
                log::Level::Debug
            };
            log::log!(target: "ipc", level, "{}", Chain(&error));
            AppError::from(error)
        })
    }
}

/// True for codes that mean something broke, rather than a refused request.
fn is_failure(code: ErrorCode) -> bool {
    matches!(
        code,
        ErrorCode::Unavailable | ErrorCode::Network | ErrorCode::Llm | ErrorCode::Internal
    )
}

/// Displays an error followed by each of its sources: `a: b: c`.
struct Chain<'a>(&'a dyn Error);

impl fmt::Display for Chain<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)?;
        let mut source = self.0.source();
        while let Some(error) = source {
            write!(formatter, ": {error}")?;
            source = error.source();
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, thiserror::Error)]
    #[error("could not load notes")]
    struct Outer(#[source] std::io::Error);

    impl ErrorClass for Outer {
        fn code(&self) -> ErrorCode {
            ErrorCode::Internal
        }
    }

    #[test]
    fn into_ipc_keeps_the_user_message_and_code() {
        let result: Result<(), Outer> = Err(Outer(std::io::Error::other("disk on fire")));

        let error = result.into_ipc().unwrap_err();

        assert_eq!(
            error,
            AppError::new(ErrorCode::Internal, "could not load notes")
        );
    }

    #[test]
    fn the_logged_chain_includes_every_source() {
        let error = Outer(std::io::Error::other("disk on fire"));

        assert_eq!(
            Chain(&error).to_string(),
            "could not load notes: disk on fire"
        );
    }
}
