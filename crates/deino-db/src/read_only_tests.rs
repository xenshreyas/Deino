//! Read-only opening: the schema check, and that nothing is ever written.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use super::*;
use crate::{DbLocation, MIGRATOR};

fn versions(list: &[i64]) -> BTreeSet<i64> {
    list.iter().copied().collect()
}

/// A migrated database file in `dir`, closed so its WAL is checkpointed.
async fn migrated_file(dir: &Path) -> PathBuf {
    let path = dir.join("deino.sqlite");
    let database = Database::open(&DbLocation::File(path.clone()), &MIGRATOR)
        .await
        .unwrap();
    sqlx::query("INSERT INTO meetings (id, title, created_at, updated_at) VALUES ('m', 'Standup', '2026-10-08', '2026-10-08')")
        .execute(database.writer())
        .await
        .unwrap();
    database.close().await;
    path
}

/// Runs `sql` on the file through a normal, writable connection.
async fn tamper(path: &Path, sql: &str) {
    let database = Database::connect(&DbLocation::File(path.to_path_buf()))
        .await
        .unwrap();
    sqlx::query(sql).execute(database.writer()).await.unwrap();
    database.close().await;
}

fn fingerprint(path: &Path) -> (Vec<u8>, SystemTime) {
    let bytes = std::fs::read(path).unwrap();
    let modified = std::fs::metadata(path).unwrap().modified().unwrap();
    (bytes, modified)
}

#[test]
fn the_same_versions_are_compatible() {
    assert_eq!(
        SchemaCheck::compare(&versions(&[1, 2]), &versions(&[1, 2])),
        Ok(())
    );
}

#[test]
fn unknown_applied_versions_mean_a_newer_database() {
    assert_eq!(
        SchemaCheck::compare(&versions(&[1, 2, 3]), &versions(&[1, 2])),
        Err(SchemaCheck::Newer { unknown: 1 })
    );
    assert_eq!(
        SchemaCheck::compare(&versions(&[1, 3]), &versions(&[1, 2])),
        Err(SchemaCheck::Newer { unknown: 1 })
    );
}

#[test]
fn missing_versions_mean_an_older_database() {
    assert_eq!(
        SchemaCheck::compare(&versions(&[1]), &versions(&[1, 2, 3])),
        Err(SchemaCheck::Older { missing: 2 })
    );
}

#[tokio::test]
async fn reads_work_and_writes_are_refused_without_touching_the_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = migrated_file(dir.path()).await;
    let before = fingerprint(&path);

    let database = Database::open_read_only(&path, &MIGRATOR).await.unwrap();
    let title: String = sqlx::query_scalar("SELECT title FROM meetings WHERE id = 'm'")
        .fetch_one(database.reader())
        .await
        .unwrap();
    let write = sqlx::query("UPDATE meetings SET title = 'Changed'")
        .execute(database.writer())
        .await;
    database.close().await;

    assert_eq!(title, "Standup");
    assert!(write.is_err(), "a read-only database must refuse writes");
    assert_eq!(fingerprint(&path), before, "the file must be untouched");
}

#[tokio::test]
async fn a_missing_file_is_reported_and_never_created() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("deino.sqlite");

    let error = Database::open_read_only(&path, &MIGRATOR)
        .await
        .unwrap_err();

    assert!(matches!(error, DbError::Missing(ref missing) if *missing == path));
    assert!(!path.exists());
}

#[tokio::test]
async fn an_older_database_is_refused_and_not_migrated() {
    let dir = tempfile::tempdir().unwrap();
    let path = migrated_file(dir.path()).await;
    tamper(
        &path,
        "DELETE FROM _sqlx_migrations WHERE version = (SELECT MAX(version) FROM _sqlx_migrations)",
    )
    .await;
    let before = fingerprint(&path);

    let error = Database::open_read_only(&path, &MIGRATOR)
        .await
        .unwrap_err();

    assert!(matches!(
        error,
        DbError::Schema(SchemaCheck::Older { missing: 1 })
    ));
    assert!(error.to_string().contains("open the Deino app once"));
    assert_eq!(fingerprint(&path), before, "no migration may run");
}

#[tokio::test]
async fn a_newer_database_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let path = migrated_file(dir.path()).await;
    tamper(
        &path,
        "INSERT INTO _sqlx_migrations (version, description, success, checksum, execution_time)
         VALUES (99991231235959, 'from the future', 1, x'00', 0)",
    )
    .await;

    let error = Database::open_read_only(&path, &MIGRATOR)
        .await
        .unwrap_err();

    assert!(matches!(
        error,
        DbError::Schema(SchemaCheck::Newer { unknown: 1 })
    ));
}

#[tokio::test]
async fn a_database_without_migrations_is_not_deino() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("other.sqlite");
    tamper(&path, "CREATE TABLE notes (id INTEGER PRIMARY KEY)").await;

    let error = Database::open_read_only(&path, &MIGRATOR)
        .await
        .unwrap_err();

    assert!(matches!(error, DbError::NotDeino(_)));
}

#[tokio::test]
async fn it_reads_beside_the_running_app_and_sees_its_latest_commit() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("deino.sqlite");
    let app = Database::open(&DbLocation::File(path.clone()), &MIGRATOR)
        .await
        .unwrap();
    sqlx::query("INSERT INTO meetings (id, title, created_at, updated_at) VALUES ('live', 'Live', '2026-10-08', '2026-10-08')")
        .execute(app.writer())
        .await
        .unwrap();

    let database = Database::open_read_only(&path, &MIGRATOR).await.unwrap();
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM meetings WHERE id = 'live'")
        .fetch_one(database.reader())
        .await
        .unwrap();
    database.close().await;
    app.close().await;

    assert_eq!(count, 1, "the commit still in the app's WAL is visible");
}
