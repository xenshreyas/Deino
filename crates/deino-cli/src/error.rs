//! [`CliError`]: every way a command or tool call fails, with the stable
//! [`ErrorCode`] scripts and MCP clients branch on, and its exit code.

use std::path::PathBuf;

use deino_db::DbError;
use deino_domain::{ErrorClass, ErrorCode};
use deino_export::RenderError;
use serde::Serialize;

use deino_core::export::ExportError;
use deino_core::query::QueryError;

/// A command's failure. Messages are user-safe: IDs, names the caller
/// typed and paths, never meeting content.
#[derive(Debug, thiserror::Error)]
pub enum CliError {
    /// The platform has no user data folder and no `--db` was given.
    #[error("cannot find the Deino database; pass --db <path> or set DEINO_DB")]
    NoDataDir,
    /// The database could not be opened.
    #[error("{}", database_message(.0))]
    Database(#[from] DbError),
    /// A read failed or named something that does not exist.
    #[error(transparent)]
    Query(#[from] QueryError),
    /// A meeting could not be exported.
    #[error(transparent)]
    Export(#[from] ExportError),
    /// A renderer failed.
    #[error(transparent)]
    Render(#[from] RenderError),
    /// An argument is malformed.
    #[error("{0}")]
    Invalid(String),
    /// Output could not be written.
    #[error("could not write {}", .path.display())]
    Write {
        /// Where.
        path: PathBuf,
        /// Why.
        #[source]
        source: std::io::Error,
    },
    /// Output could not be serialized.
    #[error("could not write the output as JSON")]
    Json(#[from] serde_json::Error),
    /// The MCP connection could not start or broke.
    #[error("the MCP connection failed: {0}")]
    Mcp(String),
    /// The async runtime could not start.
    #[error("could not start")]
    Start(#[source] std::io::Error),
}

/// What a failure looks like as JSON (`--json`, MCP tool errors).
#[derive(Debug, Serialize)]
pub struct ErrorBody {
    /// The stable code: `not_found`, `invalid`, `unavailable`, `internal`, ...
    pub code: ErrorCode,
    /// What went wrong, for a person.
    pub message: String,
}

impl CliError {
    /// The stable code of this failure.
    pub fn code(&self) -> ErrorCode {
        match self {
            Self::NoDataDir | Self::Database(_) => ErrorCode::Unavailable,
            Self::Query(error) => error.code(),
            Self::Export(error) => error.code(),
            Self::Invalid(_) => ErrorCode::Invalid,
            Self::Render(_)
            | Self::Write { .. }
            | Self::Json(_)
            | Self::Mcp(_)
            | Self::Start(_) => ErrorCode::Internal,
        }
    }

    /// The process exit code: 3 not found, 4 invalid input, 5 database
    /// unavailable, 1 anything else (clap uses 2 for usage errors).
    pub fn exit_code(&self) -> u8 {
        match self.code() {
            ErrorCode::NotFound => 3,
            ErrorCode::Invalid | ErrorCode::Conflict => 4,
            ErrorCode::Unavailable | ErrorCode::PermissionDenied => 5,
            _ => 1,
        }
    }

    /// The failure as JSON.
    pub fn body(&self) -> ErrorBody {
        ErrorBody {
            code: self.code(),
            message: self.to_string(),
        }
    }
}

/// Database failures, with what to do about them.
fn database_message(error: &DbError) -> String {
    match error {
        DbError::Missing(path) => format!(
            "no Deino database at {}; open the Deino app once, or pass --db <path> or set DEINO_DB",
            path.display()
        ),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use deino_core::query::EntityKind;

    use super::*;

    #[test]
    fn codes_and_exit_codes_follow_the_failure() {
        let missing = CliError::Query(QueryError::NotFound {
            kind: EntityKind::Meeting,
            reference: "meeting-x".into(),
        });
        let no_db = CliError::Database(DbError::Missing(PathBuf::from("/x/deino.sqlite")));

        assert_eq!(
            (missing.code(), missing.exit_code()),
            (ErrorCode::NotFound, 3)
        );
        assert_eq!(
            (no_db.code(), no_db.exit_code()),
            (ErrorCode::Unavailable, 5)
        );
        assert!(no_db.to_string().contains("open the Deino app once"));
        assert_eq!(CliError::Invalid("bad".into()).exit_code(), 4);
    }

    #[test]
    fn the_body_serializes_its_code_in_snake_case() {
        let body = CliError::Invalid("--from must be YYYY-MM-DD".into()).body();

        assert_eq!(
            serde_json::to_value(body).unwrap(),
            serde_json::json!({ "code": "invalid", "message": "--from must be YYYY-MM-DD" })
        );
    }
}
