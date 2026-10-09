//! Schema and connection tests: migrations, persistence, pool roles and
//! change signals.

use tokio::sync::broadcast::error::TryRecvError;

use super::*;

const INSERT_MEETING: &str = "INSERT INTO meetings (id, title, created_at, updated_at) VALUES (?, 'Standup', '2026-10-08', '2026-10-08')";

async fn table_names(database: &Database) -> Vec<String> {
    sqlx::query_scalar("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
        .fetch_all(database.reader())
        .await
        .unwrap()
}

async fn insert_meeting(database: &Database, id: &str) {
    sqlx::query(INSERT_MEETING)
        .bind(id)
        .execute(database.writer())
        .await
        .unwrap();
}

#[tokio::test]
async fn every_migration_applies_to_an_empty_database() {
    let database = test_db().await;

    let tables = table_names(&database).await;
    for table in Table::ALL
        .iter()
        .filter(|table| **table != Table::ExampleNotes)
    {
        let name = serde_json::to_value(table).unwrap();
        assert!(
            tables.iter().any(|t| name == t.as_str()),
            "missing table {name}"
        );
    }
    assert!(
        !tables.iter().any(|t| t == "licensing"),
        "FND-02 dropped `licensing`"
    );
}

#[tokio::test]
async fn foreign_keys_are_enforced() {
    let database = test_db().await;

    let orphan = sqlx::query(
        "INSERT INTO transcripts (id, meeting_id, transcript, timestamp) VALUES ('t', 'none', '', '')",
    )
    .execute(database.writer())
    .await;

    assert!(
        orphan.is_err(),
        "a transcript must not reference a missing meeting"
    );
}

#[tokio::test]
async fn reopening_a_file_database_keeps_rows_and_reapplies_nothing() {
    let directory = tempfile::tempdir().unwrap();
    let location = DbLocation::File(directory.path().join("nested").join("deino.sqlite"));

    let first = Database::open(&location, &MIGRATOR).await.unwrap();
    insert_meeting(&first, "meeting-1").await;
    first.close().await;
    let second = Database::open(&location, &MIGRATOR).await.unwrap();

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM meetings")
        .fetch_one(second.reader())
        .await
        .unwrap();
    assert_eq!(count, 1);
}

#[tokio::test]
async fn closing_checkpoints_the_wal_away() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("deino.sqlite");
    let database = Database::open(&DbLocation::File(path.clone()), &MIGRATOR)
        .await
        .unwrap();
    insert_meeting(&database, "meeting-1").await;

    database.close().await;

    let wal = std::fs::metadata(directory.path().join("deino.sqlite-wal"));
    assert!(
        wal.is_err() || wal.unwrap().len() == 0,
        "the WAL must be empty after close"
    );
}

#[tokio::test]
async fn readers_cannot_write() {
    let directory = tempfile::tempdir().unwrap();
    let location = DbLocation::File(directory.path().join("deino.sqlite"));
    let database = Database::open(&location, &MIGRATOR).await.unwrap();

    let write = sqlx::query(INSERT_MEETING)
        .bind("meeting-1")
        .execute(database.reader())
        .await;

    assert!(write.is_err(), "reader connections are query_only");
}

#[tokio::test]
async fn a_committed_transaction_publishes_its_tables_once() {
    let database = test_db().await;
    let mut changes = database.changes().subscribe();

    let mut transaction = database.writer().begin().await.unwrap();
    for id in ["meeting-1", "meeting-2"] {
        sqlx::query(INSERT_MEETING)
            .bind(id)
            .execute(&mut *transaction)
            .await
            .unwrap();
    }
    sqlx::query("INSERT INTO meeting_notes (meeting_id, created_at, updated_at) VALUES ('meeting-1', '', '')")
        .execute(&mut *transaction)
        .await
        .unwrap();
    transaction.commit().await.unwrap();

    let expected = TableChanges {
        tables: vec![Table::Meetings, Table::MeetingNotes],
        seq: 1,
    };
    assert_eq!(changes.try_recv().unwrap(), expected);
    assert_eq!(changes.try_recv().unwrap_err(), TryRecvError::Empty);
}

#[tokio::test]
async fn a_rolled_back_transaction_publishes_nothing() {
    let database = test_db().await;
    let mut changes = database.changes().subscribe();

    let mut transaction = database.writer().begin().await.unwrap();
    sqlx::query(INSERT_MEETING)
        .bind("meeting-1")
        .execute(&mut *transaction)
        .await
        .unwrap();
    transaction.rollback().await.unwrap();
    sqlx::query("UPDATE settings SET model = 'x'")
        .execute(database.writer())
        .await
        .unwrap();
    insert_meeting(&database, "meeting-2").await;

    // The rolled-back insert left nothing pending, and the no-op UPDATE touched no rows.
    let expected = TableChanges {
        tables: vec![Table::Meetings],
        seq: 1,
    };
    assert_eq!(changes.try_recv().unwrap(), expected);
    assert_eq!(changes.try_recv().unwrap_err(), TryRecvError::Empty);
}

#[tokio::test]
async fn a_file_that_is_not_a_database_is_reported_as_corruption() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("deino.sqlite");
    std::fs::write(
        &path,
        b"definitely not an SQLite file, but long enough to have a header",
    )
    .unwrap();

    let error = Database::open(&DbLocation::File(path), &MIGRATOR)
        .await
        .unwrap_err();

    assert!(error.is_corruption(), "{error:?}");
}
