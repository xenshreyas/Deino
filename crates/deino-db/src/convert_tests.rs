//! Encryption at rest end to end on real files: an encrypted database does
//! not open without its key, and encrypting then decrypting a seeded
//! database keeps every table's rows and checksums (PRV-02). SQLCipher is
//! linked on Unix only (Cargo.toml).
#![cfg(unix)]

use std::path::{Path, PathBuf};

use sqlx::Connection;
use sqlx::sqlite::{SqliteConnectOptions, SqliteConnection};

use super::*;
use crate::{Database, DbLocation, MIGRATOR, SCHEMA_ENTRY};

const MEETING: &str = "INSERT INTO meetings (id, title, created_at, updated_at) VALUES (?1, ?2, '2026-10-08', '2026-10-08')";
const SEGMENT: &str =
    "INSERT INTO transcript_segments (id, meeting_id, channel, start_ms, end_ms, text)
     VALUES (?1, ?2, 'mic', ?3, ?3 + 900, ?4)";
const TAG: &str = "INSERT INTO tags (id, name, color, created_at, updated_at)
     VALUES ('0199f000-0000-7000-8000-000000000001', 'Sales', 1, '2026-10-09', '2026-10-09')";
const EMBEDDING: &str = "INSERT INTO embeddings (meeting_id, ref_kind, ref_id, chunk_ix, text, model, vector, created_at)
     VALUES (?1, 'note', ?1, 0, 'chunk', 'test-model', ?2, '2026-10-09')";

/// A migrated, seeded database file in `dir`, closed.
async fn seeded(dir: &Path, key: Option<&DbKey>) -> PathBuf {
    let path = dir.join("deino.sqlite");
    let location = DbLocation::File(path.clone());
    let database = Database::open_keyed(&location, key, &MIGRATOR, &[])
        .await
        .unwrap();
    let writer = database.writer();
    for (index, title) in ["Standup", "Pipeline review", "1:1 with \"Sam\""]
        .iter()
        .enumerate()
    {
        let meeting = format!("0199f000-0000-7000-8000-00000000010{index}");
        sqlx::query(MEETING)
            .bind(&meeting)
            .bind(title)
            .execute(writer)
            .await
            .unwrap();
        for line in 0..20_i64 {
            let id = format!("0199f000-0000-7000-8000-0000000{index}{line:04}");
            let text = format!(
                "line {line} of {title}: ünïcode, 'quotes', and numbers {}",
                line * 7
            );
            sqlx::query(SEGMENT)
                .bind(id)
                .bind(&meeting)
                .bind(line * 1000)
                .bind(text)
                .execute(writer)
                .await
                .unwrap();
        }
        let vector: Vec<u8> = (0..64_u8)
            .map(|byte| {
                byte.wrapping_mul(31)
                    .wrapping_add(u8::try_from(index).unwrap())
            })
            .collect();
        sqlx::query(EMBEDDING)
            .bind(&meeting)
            .bind(vector)
            .execute(writer)
            .await
            .unwrap();
    }
    sqlx::query(TAG).execute(writer).await.unwrap();
    database.close().await;
    path
}

/// One plain connection, with an optional key, outside the crate's checks.
async fn raw(path: &Path, key: Option<&DbKey>) -> Result<SqliteConnection, sqlx::Error> {
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(false);
    let mut connection = crate::cipher::keyed(options, key).connect().await?;
    sqlx::query("SELECT count(*) FROM sqlite_schema")
        .fetch_one(&mut connection)
        .await?;
    Ok(connection)
}

async fn fingerprint_of(path: &Path, key: Option<&DbKey>) -> Fingerprint {
    let mut connection = raw(path, key).await.unwrap();
    let fingerprint = Fingerprint::of(&mut connection).await.unwrap();
    connection.close().await.unwrap();
    fingerprint
}

fn leftovers(path: &Path) -> Vec<PathBuf> {
    [".converting", ".backup", "-wal", "-shm"]
        .iter()
        .map(|suffix| side_file(path, suffix))
        .filter(|side| side.exists())
        .collect()
}

#[tokio::test]
async fn sqlcipher_is_linked() {
    assert!(crate::cipher_available().await.unwrap());
}

#[tokio::test]
async fn an_encrypted_database_does_not_open_without_its_key() {
    let dir = tempfile::tempdir().unwrap();
    let key = DbKey::generate().unwrap();
    let path = seeded(dir.path(), Some(&key)).await;
    assert_eq!(FileFormat::detect(&path).unwrap(), FileFormat::Encrypted);
    let bytes = std::fs::read(&path).unwrap();
    assert!(
        !bytes.windows(7).any(|window| window == b"Standup"),
        "plaintext on disk"
    );

    assert!(raw(&path, None).await.is_err(), "a raw open reads it");
    let wrong = DbKey::generate().unwrap();
    assert!(
        raw(&path, Some(&wrong)).await.is_err(),
        "a wrong key reads it"
    );
    assert!(raw(&path, Some(&key)).await.is_ok());

    let location = DbLocation::File(path.clone());
    let no_key = Database::open(&location, &MIGRATOR).await.unwrap_err();
    assert!(matches!(no_key, DbError::KeyRequired), "{no_key:?}");
    let wrong_key = Database::open_keyed(&location, Some(&wrong), &MIGRATOR, &[])
        .await
        .unwrap_err();
    assert!(matches!(wrong_key, DbError::WrongKey), "{wrong_key:?}");
    assert!(
        !wrong_key.is_corruption(),
        "a wrong key must not look like corruption"
    );
    let read_only = Database::open_read_only(&path, &MIGRATOR)
        .await
        .unwrap_err();
    assert!(matches!(read_only, DbError::KeyRequired), "{read_only:?}");
    let reader = Database::open_read_only_keyed(&path, Some(&key), &MIGRATOR)
        .await
        .unwrap();
    let titles: i64 = sqlx::query_scalar("SELECT count(*) FROM meetings")
        .fetch_one(reader.reader())
        .await
        .unwrap();
    assert_eq!(titles, 3);
    reader.close().await;
}

#[tokio::test]
async fn a_key_for_a_plaintext_database_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let path = seeded(dir.path(), None).await;
    let key = DbKey::generate().unwrap();
    let location = DbLocation::File(path.clone());
    let error = Database::open_keyed(&location, Some(&key), &MIGRATOR, &[])
        .await
        .unwrap_err();
    assert!(matches!(error, DbError::NotEncrypted), "{error:?}");
}

#[tokio::test]
async fn encrypting_then_decrypting_is_lossless() {
    let dir = tempfile::tempdir().unwrap();
    let path = seeded(dir.path(), None).await;
    let original = fingerprint_of(&path, None).await;
    assert_eq!(original.table("meetings").unwrap().rows, 3);
    assert_eq!(original.table("transcript_segments").unwrap().rows, 60);
    assert!(
        original.table("search_fts_data").is_some(),
        "FTS shadow tables are covered"
    );
    assert!(original.table(SCHEMA_ENTRY).is_some());

    let key = DbKey::generate().unwrap();
    convert(&path, Conversion::Encrypt(&key)).await.unwrap();
    assert_eq!(FileFormat::detect(&path).unwrap(), FileFormat::Encrypted);
    assert_eq!(fingerprint_of(&path, Some(&key)).await, original);
    assert!(leftovers(&path).is_empty(), "{:?}", leftovers(&path));

    convert(&path, Conversion::Decrypt(&key)).await.unwrap();
    assert_eq!(FileFormat::detect(&path).unwrap(), FileFormat::Plaintext);
    assert_eq!(fingerprint_of(&path, None).await, original);
    assert!(leftovers(&path).is_empty(), "{:?}", leftovers(&path));

    let database = Database::open(&DbLocation::File(path.clone()), &MIGRATOR)
        .await
        .unwrap();
    let hits: i64 =
        sqlx::query_scalar("SELECT count(*) FROM search_fts WHERE search_fts MATCH 'pipeline'")
            .fetch_one(database.reader())
            .await
            .unwrap();
    assert!(hits > 0, "the search index survives the round trip");
    database.close().await;
}

#[tokio::test]
async fn converting_the_wrong_way_changes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let path = seeded(dir.path(), None).await;
    let before = std::fs::read(&path).unwrap();
    let key = DbKey::generate().unwrap();
    let error = convert(&path, Conversion::Decrypt(&key)).await.unwrap_err();
    assert!(matches!(error, DbError::NotEncrypted), "{error:?}");
    convert(&path, Conversion::Encrypt(&key)).await.unwrap();
    let again = convert(&path, Conversion::Encrypt(&key)).await.unwrap_err();
    assert!(matches!(again, DbError::AlreadyEncrypted), "{again:?}");
    let wrong = DbKey::generate().unwrap();
    let error = convert(&path, Conversion::Decrypt(&wrong))
        .await
        .unwrap_err();
    assert!(matches!(error, DbError::WrongKey), "{error:?}");
    assert!(leftovers(&path).is_empty(), "{:?}", leftovers(&path));
    convert(&path, Conversion::Decrypt(&key)).await.unwrap();
    assert_ne!(std::fs::read(&path).unwrap().len(), 0);
    assert_eq!(before.get(..16), std::fs::read(&path).unwrap().get(..16));
}

#[test]
fn recovery_restores_the_original_when_the_swap_stopped_halfway() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("deino.sqlite");
    std::fs::write(side_file(&path, ".backup"), b"original").unwrap();
    std::fs::write(side_file(&path, ".converting"), b"copy").unwrap();
    assert_eq!(recover(&path).unwrap(), Recovery::RestoredOriginal);
    assert_eq!(std::fs::read(&path).unwrap(), b"original");
    assert!(leftovers(&path).is_empty());
}

#[test]
fn recovery_keeps_a_finished_swap_and_drops_partial_copies() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("deino.sqlite");
    assert_eq!(recover(&path).unwrap(), Recovery::Clean);
    std::fs::write(&path, b"converted").unwrap();
    std::fs::write(side_file(&path, ".backup"), b"original").unwrap();
    assert_eq!(recover(&path).unwrap(), Recovery::DiscardedBackup);
    assert_eq!(std::fs::read(&path).unwrap(), b"converted");
    std::fs::write(side_file(&path, ".converting"), b"half").unwrap();
    assert_eq!(recover(&path).unwrap(), Recovery::DiscardedCopy);
    assert_eq!(std::fs::read(&path).unwrap(), b"converted");
    assert!(leftovers(&path).is_empty());
}
