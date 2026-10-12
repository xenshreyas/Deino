//! Where opening the library stands, for the UI's startup screen (PRV-02).
//!
//! Most launches open the database before the window shows anything, so the
//! UI only ever sees [`LibraryStatus::Ready`]. When the file must first be
//! encrypted or decrypted, startup runs that in the background and the UI
//! shows a calm progress screen until it is ready; a key that cannot be
//! read keeps a blocking explanation on screen instead of the app.

use std::sync::{PoisonError, RwLock};

use deino_core::privacy::{Direction, PrivacyError, StartupOutcome};
use deino_db::DbError;
use serde::Serialize;

use crate::database::manager::OpenError;

/// Why the library cannot open.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum LibraryProblem {
    /// The file is encrypted and its key is not in the Keychain.
    KeyMissing,
    /// The Keychain did not hand over the key.
    KeyDenied,
    /// The saved key does not open the file.
    WrongKey,
    /// Something else failed; the log has the details.
    OpenFailed,
}

impl LibraryProblem {
    /// The problem behind `error`.
    pub fn of(error: &OpenError) -> Self {
        match error {
            OpenError::Privacy(PrivacyError::KeyMissing)
            | OpenError::Database(DbError::KeyRequired) => Self::KeyMissing,
            OpenError::Privacy(PrivacyError::KeyUnreadable(_)) => Self::KeyDenied,
            OpenError::Privacy(PrivacyError::KeyMalformed)
            | OpenError::Database(DbError::WrongKey) => Self::WrongKey,
            OpenError::Privacy(PrivacyError::Database(_)) | OpenError::Database(_) => {
                Self::OpenFailed
            }
        }
    }

    /// True for a key problem: the app keeps running and explains it, and
    /// nothing on disk is touched.
    pub fn is_key_problem(self) -> bool {
        self != Self::OpenFailed
    }
}

/// Where opening the library stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LibraryStatus {
    /// Startup has not reached the database yet.
    Opening,
    /// The file is being encrypted or decrypted before it opens.
    Converting {
        /// Which way.
        direction: Direction,
    },
    /// The database is open.
    Ready {
        /// True when the file on disk is encrypted.
        encrypted: bool,
        /// What startup did to the file.
        outcome: StartupOutcome,
    },
    /// The database cannot open.
    Unavailable {
        /// Why.
        problem: LibraryProblem,
    },
}

/// The current [`LibraryStatus`], shared by startup and the command.
#[derive(Debug)]
pub struct LibraryStatusCell(RwLock<LibraryStatus>);

impl Default for LibraryStatusCell {
    fn default() -> Self {
        Self(RwLock::new(LibraryStatus::Opening))
    }
}

impl LibraryStatusCell {
    /// The status now.
    pub fn get(&self) -> LibraryStatus {
        *self.0.read().unwrap_or_else(PoisonError::into_inner)
    }

    /// Replaces the status.
    pub fn set(&self, status: LibraryStatus) {
        *self.0.write().unwrap_or_else(PoisonError::into_inner) = status;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_problems_are_told_apart_from_failures() {
        let missing = OpenError::Privacy(PrivacyError::KeyMissing);
        let wrong = OpenError::Database(DbError::WrongKey);
        let other = OpenError::Database(DbError::Mismatch);

        assert_eq!(LibraryProblem::of(&missing), LibraryProblem::KeyMissing);
        assert_eq!(LibraryProblem::of(&wrong), LibraryProblem::WrongKey);
        assert!(!LibraryProblem::of(&other).is_key_problem());
    }

    #[test]
    fn the_status_serializes_with_a_kind_tag() {
        let ready = LibraryStatus::Ready {
            encrypted: true,
            outcome: StartupOutcome::Converted {
                direction: Direction::Encrypt,
            },
        };

        assert_eq!(
            serde_json::to_string(&ready).unwrap(),
            r#"{"kind":"ready","encrypted":true,"outcome":{"kind":"converted","direction":"encrypt"}}"#
        );
    }
}
