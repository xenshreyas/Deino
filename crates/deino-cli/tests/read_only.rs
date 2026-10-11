//! The binary never writes the database: after every command its bytes and
//! modification time are unchanged, and a database it does not understand
//! is refused, never migrated.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "test helpers fail loudly, like the tests they serve"
)]

#[path = "support/sample.rs"]
mod sample;

use std::path::Path;
use std::time::SystemTime;

use sample::Sample;
use sha2::{Digest, Sha256};

/// Every read command, as the integration tests run them.
const COMMANDS: &[&[&str]] = &[
    &["status"],
    &["meetings"],
    &["meetings", "--tag", "Customer", "--folder", "Customers"],
    &[
        "meetings",
        "read",
        "meeting-fixture-standup",
        "meeting-fixture-planning",
    ],
    &["transcripts", "read", "meeting-fixture-planning"],
    &["search", "postmortem"],
    &["action-items", "--status", "any"],
    &["people"],
    &["companies"],
    &["tags"],
    &["folders"],
    &["calendar", "--from", "2026-09-28", "--to", "2026-10-09"],
    &["export", "meeting-fixture-standup", "-f", "json"],
];

fn fingerprint(path: &Path) -> (Vec<u8>, SystemTime) {
    let hash = Sha256::digest(std::fs::read(path).unwrap()).to_vec();
    let modified = std::fs::metadata(path).unwrap().modified().unwrap();
    (hash, modified)
}

async fn applied_migrations(path: &Path) -> i64 {
    let pool = sqlx::SqlitePool::connect(&format!("sqlite://{}?mode=ro", path.display()))
        .await
        .unwrap();
    let count = sqlx::query_scalar("SELECT COUNT(*) FROM _sqlx_migrations")
        .fetch_one(&pool)
        .await
        .unwrap();
    pool.close().await;
    count
}

#[tokio::test]
async fn no_command_changes_the_file() {
    let sample = Sample::new().await;
    let before = fingerprint(&sample.path);

    for command in COMMANDS {
        sample.deino(command).assert().success();
    }

    assert_eq!(fingerprint(&sample.path), before);
    // A WAL reader may create SQLite's side files, but never logs a write.
    let log = sample.path.with_extension("sqlite-wal");
    let logged = std::fs::metadata(&log).map_or(0, |metadata| metadata.len());
    assert_eq!(logged, 0, "nothing was written to the write-ahead log");
}

#[tokio::test]
async fn an_older_database_is_refused_and_left_unmigrated() {
    let sample = Sample::new().await;
    let pool = sqlx::SqlitePool::connect(&format!("sqlite://{}", sample.path.display()))
        .await
        .unwrap();
    sqlx::query(
        "DELETE FROM _sqlx_migrations WHERE version = (SELECT MAX(version) FROM _sqlx_migrations)",
    )
    .execute(&pool)
    .await
    .unwrap();
    pool.close().await;
    let applied = applied_migrations(&sample.path).await;
    let before = fingerprint(&sample.path);

    let output = sample.deino(&["meetings"]).assert().code(5);

    let stderr = String::from_utf8_lossy(&output.get_output().stderr).into_owned();
    assert!(
        stderr.contains("older than this binary expects"),
        "{stderr}"
    );
    assert_eq!(fingerprint(&sample.path), before);
    assert_eq!(applied_migrations(&sample.path).await, applied);
}
