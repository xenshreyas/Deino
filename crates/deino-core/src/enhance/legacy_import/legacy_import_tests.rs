//! The legacy summaries become version 1 of the enhanced notes before
//! `summary_processes` is dropped.

use async_trait::async_trait;
use deino_db::{DataMigration, DataMigrationError, Database, DbLocation, MIGRATOR};
use deino_doc::{Author, Mark};
use deino_domain::MeetingId;

use super::*;
use crate::enhance::{EnhancedNotesReader, SqliteEnhancedNotesRepo};

/// Writes legacy rows just before the import runs.
struct Seed(Vec<&'static str>);

#[async_trait]
impl DataMigration for Seed {
    fn runs_before(&self) -> i64 {
        DROP_LEGACY_SUMMARIES_VERSION
    }

    async fn run(&self, database: &Database) -> Result<(), DataMigrationError> {
        for statement in &self.0 {
            sqlx::query(statement).execute(database.writer()).await?;
        }
        Ok(())
    }
}

const MEETINGS: &str = "INSERT INTO meetings (id, title, created_at, updated_at) VALUES
    ('meeting-md', 'Planning', '2026-09-28T09:00:00Z', '2026-09-28T09:00:00Z'),
    ('meeting-sections', 'Old', '2025-01-01T09:00:00Z', '2025-01-01T09:00:00Z'),
    ('meeting-failed', 'Failed', '2026-09-28T09:00:00Z', '2026-09-28T09:00:00Z')";
const SUMMARIES: &str = r##"INSERT INTO summary_processes (meeting_id, status, created_at, updated_at, result) VALUES
    ('meeting-md', 'completed', '2026-09-28T10:00:00Z', '2026-09-28T10:05:00Z', '{"markdown":"# Q4 planning\n\n## Decisions\n\n- Cut route optimisation"}'),
    ('meeting-sections', 'completed', '2025-01-01T10:00:00Z', '2025-01-01T10:00:00Z', '{"_section_order":["Summary"],"Summary":{"title":"Summary","blocks":[{"type":"bullet","content":"Shipped v1"}]}}'),
    ('meeting-failed', 'failed', '2026-09-28T10:00:00Z', '2026-09-28T10:00:00Z', NULL)"##;

#[tokio::test]
async fn completed_summaries_become_ai_written_first_versions() {
    let seed = Seed(vec![MEETINGS, SUMMARIES]);
    let steps: [&dyn DataMigration; 2] = [&seed, &LegacySummaryImport];

    let database = Database::open_with(&DbLocation::Memory, &MIGRATOR, &steps)
        .await
        .unwrap();

    let repo = SqliteEnhancedNotesRepo::new(database.clone());
    let planning = repo
        .latest(&MeetingId::new("meeting-md"))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        (planning.version, planning.model.as_str()),
        (1, LEGACY_MODEL)
    );
    assert!(planning.markdown.contains("- Cut route optimisation"));
    assert_eq!(
        planning.basis, None,
        "no regenerate prompts for imported notes"
    );
    let old = repo
        .latest(&MeetingId::new("meeting-sections"))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(old.markdown, "## Summary\n\n- Shipped v1\n");
    assert_eq!(
        repo.latest(&MeetingId::new("meeting-failed"))
            .await
            .unwrap(),
        None
    );
    let gone: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM sqlite_master WHERE name = 'summary_processes'")
            .fetch_one(database.reader())
            .await
            .unwrap();
    assert_eq!(gone, 0);
}

#[test]
fn every_run_of_an_imported_summary_is_the_ais() {
    let doc = summary_doc(r###"{"markdown":"## Decisions\n\n- Ship **Friday**"}"###).unwrap();

    let marks: Vec<&Mark> = doc
        .content
        .iter()
        .flat_map(deino_doc::Block::inline_content)
        .flat_map(deino_doc::Inline::marks)
        .collect();
    assert!(
        marks
            .iter()
            .any(|m| matches!(m, Mark::Origin { attrs } if attrs.author == Author::Ai))
    );
    deino_doc::validate(&doc).unwrap();
}

#[test]
fn blank_or_unknown_summaries_are_not_imported() {
    assert_eq!(summary_doc(r#"{"markdown":"  "}"#), None);
    assert_eq!(summary_doc("not json"), None);
    assert_eq!(summary_doc(r#"{"summary_json":[]}"#), None);
}
