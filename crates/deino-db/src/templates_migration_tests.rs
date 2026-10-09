//! NOT-04 migration round trip: `meetings.template_id` becomes a reference to
//! `templates`, legacy built-in names map onto their successors, and the new
//! constraints hold.

use std::borrow::Cow;

use sqlx::migrate::Migrator;

use super::*;

/// The version of `20261009160738_not04_templates.sql`.
const NOT04_VERSION: i64 = 20_261_009_160_738;

/// A migrator with only the migrations before NOT-04.
fn before_templates() -> Migrator {
    let migrations = MIGRATOR
        .iter()
        .filter(|m| m.version < NOT04_VERSION)
        .cloned();
    Migrator {
        migrations: Cow::Owned(migrations.collect()),
        ..Migrator::DEFAULT
    }
}

/// A database just before NOT-04 with meetings naming legacy templates.
async fn database_with_legacy_templates() -> Database {
    let database = Database::open(&DbLocation::Memory, &before_templates())
        .await
        .unwrap();
    let meetings = [
        ("meeting-a", Some("daily_standup")),
        ("meeting-b", Some("psychatric_session")),
        ("meeting-c", Some("my_custom_file")),
        ("meeting-d", None),
    ];
    for (id, template) in meetings {
        sqlx::query(
            "INSERT INTO meetings (id, title, created_at, updated_at, folder_path, template_id)
             VALUES (?1, 'Sync', '2026-10-01T09:00:00+00:00', '2026-10-01T09:00:00+00:00', '/tmp/a', ?2)",
        )
        .bind(id)
        .bind(template)
        .execute(database.writer())
        .await
        .unwrap();
    }
    database
}

async fn template_of(database: &Database, meeting: &str) -> Option<String> {
    sqlx::query_scalar("SELECT template_id FROM meetings WHERE id = ?1")
        .bind(meeting)
        .fetch_one(database.reader())
        .await
        .unwrap()
}

#[tokio::test]
async fn legacy_template_names_map_onto_placeholder_built_ins() {
    let database = database_with_legacy_templates().await;

    MIGRATOR.run(database.writer()).await.unwrap();

    assert_eq!(
        template_of(&database, "meeting-a").await.as_deref(),
        Some("standup")
    );
    assert_eq!(
        template_of(&database, "meeting-b").await.as_deref(),
        Some("therapy_soap")
    );
    assert_eq!(template_of(&database, "meeting-c").await, None);
    assert_eq!(template_of(&database, "meeting-d").await, None);
    let placeholders: Vec<(String, Option<i64>)> =
        sqlx::query_as("SELECT id, builtin_version FROM templates ORDER BY id")
            .fetch_all(database.reader())
            .await
            .unwrap();
    assert_eq!(
        placeholders,
        [
            ("standup".into(), Some(0)),
            ("therapy_soap".into(), Some(0))
        ]
    );
}

#[tokio::test]
async fn a_meeting_cannot_name_a_missing_template_and_forgets_a_deleted_one() {
    let database = database_with_legacy_templates().await;
    MIGRATOR.run(database.writer()).await.unwrap();

    let dangling = sqlx::query("UPDATE meetings SET template_id = 'nope' WHERE id = 'meeting-d'")
        .execute(database.writer())
        .await;
    sqlx::query("DELETE FROM templates WHERE id = 'standup'")
        .execute(database.writer())
        .await
        .unwrap();

    assert!(
        dangling.is_err(),
        "the foreign key rejects unknown templates"
    );
    assert_eq!(template_of(&database, "meeting-a").await, None);
}

#[tokio::test]
async fn sections_belong_to_their_template_and_check_their_format() {
    let database = test_db().await;
    sqlx::query(
        "INSERT INTO templates (id, name, created_at, updated_at)
         VALUES ('mine', 'Mine', '2026-10-09T08:00:00Z', '2026-10-09T08:00:00Z')",
    )
    .execute(database.writer())
    .await
    .unwrap();
    let insert = "INSERT INTO template_sections (template_id, position, title, format) VALUES (?1, 0, 'Notes', ?2)";

    let unknown_format = sqlx::query(insert)
        .bind("mine")
        .bind("poem")
        .execute(database.writer())
        .await;
    let orphan = sqlx::query(insert)
        .bind("ghost")
        .bind("bullets")
        .execute(database.writer())
        .await;
    sqlx::query(insert)
        .bind("mine")
        .bind("bullets")
        .execute(database.writer())
        .await
        .unwrap();
    sqlx::query("DELETE FROM templates WHERE id = 'mine'")
        .execute(database.writer())
        .await
        .unwrap();
    let left: i64 = sqlx::query_scalar("SELECT count(*) FROM template_sections")
        .fetch_one(database.reader())
        .await
        .unwrap();

    assert!(unknown_format.is_err());
    assert!(orphan.is_err());
    assert_eq!(left, 0, "sections are deleted with their template");
}
