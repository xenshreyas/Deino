//! NOT-09's acceptance: the fixture secrets, written into a seeded
//! meeting's transcript, never reach the enhanced-notes prompt or any
//! export, while the transcript keeps them and the near-misses survive.

use std::sync::Arc;

use async_trait::async_trait;
use deino_db::{DbError, test_db};
use deino_domain::{FakeClock, FixedZone, MeetingId, TemplateId};
use deino_export::{ExportFormat, ExportSections, LinkStyle, RenderOptions, render};
use deino_llm::{MockProvider, Route, mock_gateway};
use deino_testkit::at;
use deino_testkit::fixture::{Fixtures, MeetingKind, seed_fixtures};

use super::fixture_tests::{NEAR_MISSES, SECRETS};
use super::*;
use crate::enhance::{
    EnhancePorts, EnhanceService, MaterialSources, Overwrite, SqliteEnhancedNotesRepo,
    TemplateSource,
};
use crate::export::{ExportAssembler, ExportPorts, MeetingNotes, NoteRenderer, RedactedNotes};
use crate::segments::{Page, SegmentReader, SegmentWriter, SqliteSegmentRepo};
use crate::templates::{BuiltinSync, CATALOG, SqliteTemplateRepo, Template, TemplateError};

/// A minimal valid `notes.enhance` answer.
const ANSWER: &str = r#"{"sections":[{"heading":"Updates","blocks":[{"kind":"bullet","text":"Set up staging","origin":"ai","evidence":["s1"]}]}]}"#;

/// The 1:1 template, for every run.
struct OneOnOne;

#[async_trait]
impl TemplateSource for OneOnOne {
    async fn template_for(&self, _meeting: &MeetingId) -> Result<Template, TemplateError> {
        let builtin = CATALOG.iter().find(|template| template.id == "one_on_one");
        let builtin =
            builtin.ok_or_else(|| TemplateError::NotFound(TemplateId::new("one_on_one")))?;
        Ok(Template {
            id: TemplateId::new(builtin.id),
            is_builtin: true,
            content: builtin.content(),
            created_at: at("2026-10-01T00:00:00Z"),
            updated_at: at("2026-10-01T00:00:00Z"),
        })
    }
}

/// A seeded database whose 1:1 transcript says every fixture line.
async fn seeded() -> (deino_db::Database, MeetingId) {
    let database = test_db().await;
    seed_fixtures(&database).await.unwrap();
    let clock = Arc::new(FakeClock::new(at("2026-10-09T09:00:00Z")));
    let templates = Arc::new(SqliteTemplateRepo::new(database.clone()));
    BuiltinSync::new(templates, clock)
        .run(&CATALOG)
        .await
        .unwrap();
    let meeting = Fixtures::load()
        .unwrap()
        .meeting(MeetingKind::OneOnOne)
        .unwrap()
        .id
        .clone();
    let repo = SqliteSegmentRepo::new(database.clone());
    let segments = repo.list(&meeting, Page::ALL).await.unwrap();
    let lines: Vec<&str> = SECRETS
        .iter()
        .map(|(line, _, _)| *line)
        .chain(NEAR_MISSES)
        .collect();
    for (index, segment) in segments.iter().enumerate() {
        let said: Vec<&str> = lines
            .iter()
            .skip(index)
            .step_by(segments.len())
            .copied()
            .collect();
        repo.edit_text(&segment.id, &said.join(" "), at("2026-10-09T09:00:00Z"))
            .await
            .unwrap();
    }
    (database, meeting)
}

/// Asserts `text` holds no fixture secret, every near-miss and a marker.
fn assert_clean(text: &str, label: &str) {
    for (_, secret, _) in SECRETS {
        assert!(!text.contains(secret), "{label} leaked {secret}");
    }
    for near in NEAR_MISSES {
        assert!(text.contains(near), "{label} lost {near}");
    }
    // Markdown escapes the brackets (`\[redacted\]`), which still render as
    // the marker.
    let escaped = REDACTED.replace('[', "\\[").replace(']', "\\]");
    assert!(
        text.contains(REDACTED) || text.contains(&escaped),
        "{label} has no marker"
    );
}

#[tokio::test]
async fn fixture_secrets_never_reach_the_notes_prompt() {
    let (database, meeting) = seeded().await;
    let mock = Arc::new(MockProvider::new().reply(Route::NotesEnhance, ANSWER));
    let repo = Arc::new(SqliteEnhancedNotesRepo::new(database.clone()));
    let ports = EnhancePorts {
        sources: MaterialSources::sqlite(&database).with_redactor(Arc::new(Redactor::regex_only())),
        reader: repo.clone(),
        writer: repo,
        templates: Arc::new(OneOnOne),
        gateway: Arc::new(mock_gateway(mock.clone()).gateway),
    };
    let clock = Arc::new(FakeClock::new(at("2026-10-09T10:00:00Z")));

    EnhanceService::new(ports, clock)
        .enhance(&meeting, Overwrite::KeepEdits)
        .await
        .unwrap();

    let prompt = format!("{:?}", mock.requests());
    assert_clean(&prompt, "the notes prompt");
    let stored = SqliteSegmentRepo::new(database)
        .list(&meeting, Page::ALL)
        .await
        .unwrap();
    let transcript: String = stored.iter().map(|segment| segment.text.as_str()).collect();
    assert!(
        SECRETS
            .iter()
            .all(|(_, secret, _)| transcript.contains(secret)),
        "the transcript keeps the originals"
    );
}

#[tokio::test]
async fn fixture_secrets_never_reach_an_export() {
    let (database, meeting) = seeded().await;
    let zone = Arc::new(FixedZone::hours(0).unwrap());
    let ports =
        ExportPorts::sqlite(&database, zone).with_redactor(Arc::new(Redactor::regex_only()));
    let export = ExportAssembler::new(ports)
        .assemble(&meeting)
        .await
        .unwrap();
    let options = RenderOptions {
        sections: ExportSections::ALL,
        links: LinkStyle::Plain,
    };

    for format in [
        ExportFormat::Markdown,
        ExportFormat::Text,
        ExportFormat::Json,
        ExportFormat::Srt,
        ExportFormat::Vtt,
    ] {
        let rendered = render(format, &export, &options).unwrap();
        assert_clean(rendered.text().unwrap_or_default(), &format!("{format:?}"));
    }
}

#[tokio::test]
async fn stored_notes_are_redacted_on_the_way_out() {
    struct Stored;
    #[async_trait]
    impl NoteRenderer for Stored {
        async fn notes(&self, _meeting: &MeetingId) -> Result<MeetingNotes, DbError> {
            Ok(MeetingNotes {
                enhanced: Some("- Card on file: 4242 4242 4242 4242".to_owned()),
                mine: None,
            })
        }
    }
    let notes = RedactedNotes::new(Arc::new(Stored), Arc::new(Redactor::regex_only()));

    let out = notes.notes(&MeetingId::new("meeting-1")).await.unwrap();

    assert_eq!(out.enhanced.as_deref(), Some("- Card on file: [redacted]"));
    assert_eq!(out.mine, None);
}
