//! A sample database file for the integration tests, and the binary run
//! against it in UTC.

#![allow(dead_code, reason = "each test binary uses part of the shared support")]

use std::path::PathBuf;

use assert_cmd::Command;
use deino_core::query::seed_sample_library;
use deino_db::{Database, DbLocation, MIGRATOR};
use deino_domain::FixedZone;
use tempfile::TempDir;

/// A seeded, closed database file in its own temporary folder.
pub struct Sample {
    /// Keeps the folder alive.
    pub dir: TempDir,
    /// The database file.
    pub path: PathBuf,
}

impl Sample {
    /// Seeds the sample library (fixture meetings, tags, folders, action
    /// items, AI notes) into a new file, due days in UTC.
    pub async fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("deino.sqlite");
        let database = Database::open(&DbLocation::File(path.clone()), &MIGRATOR)
            .await
            .unwrap();
        seed_sample_library(&database, &FixedZone::hours(0).unwrap())
            .await
            .unwrap();
        database.close().await;
        Self { dir, path }
    }

    /// `deino --db <sample> <args>` in UTC, with no inherited `DEINO_DB`.
    pub fn deino(&self, args: &[&str]) -> Command {
        let mut command = Command::cargo_bin("deino-cli").unwrap();
        command
            .env("TZ", "UTC")
            .env_remove("DEINO_DB")
            .arg("--db")
            .arg(&self.path)
            .args(args);
        command
    }

    /// What `deino <args>` prints, after checking it succeeded.
    pub fn stdout(&self, args: &[&str]) -> String {
        let output = self.deino(args).output().unwrap();
        assert!(
            output.status.success(),
            "deino {args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap()
    }
}
