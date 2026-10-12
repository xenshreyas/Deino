//! CAP-09's acceptance: a recorded slide deck's text shows up in the
//! enhanced notes. The fixture slides (`fixtures/screen/*.png`) are kept as
//! a seeded meeting's snapshots, read by `ScreenContextService` (OCR behind
//! its trait: a fake here, Vision in `deino-screen`'s own test of the same
//! slides), and NOT-02 writes the notes with them beside the transcript.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use deino_db::test_db;
use deino_domain::{FakeClock, MeetingId, Millis, TemplateId};
use deino_llm::{MockProvider, Route, mock_gateway};
use deino_testkit::at;
use deino_testkit::fixture::{Fixtures, MeetingKind, seed_fixtures};

use super::*;
use crate::enhance::{
    EnhancePorts, EnhanceService, MaterialSources, Overwrite, SqliteEnhancedNotesRepo,
    TemplateSource,
};
use crate::redaction::NoRedaction;
use crate::templates::{BuiltinSync, CATALOG, SqliteTemplateRepo, Template, TemplateError};

/// The slides, where they were shown, and what they say.
const SLIDES: [(&str, u32, &str); 2] = [
    (
        "slide-1.png",
        12_000,
        "Q3 Business Review\n- Revenue grew 40% year over year\n- Churn fell to 2.1%",
    ),
    (
        "slide-2.png",
        30_000,
        "Launch plan\n- Beta opens on November 14\n- Owner: Priya Raman",
    ),
];

/// The model's notes, using the first slide's figure.
const ANSWER: &str = r#"{"sections":[{"heading":"Updates","blocks":[{"kind":"bullet","text":"Revenue grew 40% year over year, per the Q3 slide","origin":"ai","evidence":["s1"]}]}]}"#;

/// OCR from the slides' table, keyed by the snapshot's file name.
struct SlideOcr(HashMap<String, String>);

impl ScreenOcr for SlideOcr {
    fn read(&self, png: &Path) -> Result<String, OcrError> {
        assert!(png.exists(), "the snapshot's picture is read from its folder");
        let name = png.file_name().and_then(|name| name.to_str()).unwrap_or("");
        self.0
            .get(name)
            .cloned()
            .ok_or_else(|| OcrError::Failed(name.to_owned()))
    }
}

/// The 1:1 template.
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

/// Copies the fixture slides into the meeting's snapshot folder and stores
/// their rows; the OCR table of their file names.
async fn record_slides(repo: &SqliteScreenRepo, recordings: &Path, meeting: &MeetingId) -> SlideOcr {
    let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/screen");
    let folder = recordings.join(meeting.as_str()).join(SNAPSHOTS_DIR);
    std::fs::create_dir_all(&folder).unwrap();
    let mut table = HashMap::new();
    for (slide, at_ms, text) in SLIDES {
        let file_name = format!("{at_ms:010}.png");
        std::fs::copy(fixtures.join(slide), folder.join(&file_name)).unwrap();
        let row = NewSnapshot {
            meeting: meeting.clone(),
            at: Millis(at_ms),
            file_name: file_name.clone(),
            width: 960,
            height: 540,
            phash: 1,
        };
        repo.add_snapshot(&row, at("2026-10-09T09:00:00Z")).await.unwrap();
        table.insert(file_name, text.to_owned());
    }
    SlideOcr(table)
}

#[tokio::test]
async fn a_recorded_slide_decks_text_reaches_the_enhanced_notes() {
    let database = test_db().await;
    seed_fixtures(&database).await.unwrap();
    let templates = Arc::new(SqliteTemplateRepo::new(database.clone()));
    let sync_clock = Arc::new(FakeClock::new(at("2026-10-09T09:00:00Z")));
    BuiltinSync::new(templates, sync_clock).run(&CATALOG).await.unwrap();
    let meeting = Fixtures::load().unwrap().meeting(MeetingKind::OneOnOne).unwrap().id.clone();
    let recordings = tempfile::tempdir().unwrap();
    let repo = Arc::new(SqliteScreenRepo::new(database.clone()));
    let ocr = record_slides(&repo, recordings.path(), &meeting).await;

    let context = ScreenContextService::new(ScreenContextParts {
        repo: repo.clone(),
        ocr: Arc::new(ocr),
        describer: Arc::new(NoDescriptions),
        redactor: Arc::new(NoRedaction),
        recordings: recordings.path().to_path_buf(),
    });
    assert_eq!(context.read(&meeting).await.unwrap().read, 2);

    let mock = Arc::new(MockProvider::new().reply(Route::NotesEnhance, ANSWER));
    let notes = Arc::new(SqliteEnhancedNotesRepo::new(database.clone()));
    let ports = EnhancePorts {
        sources: MaterialSources::sqlite(&database),
        reader: notes.clone(),
        writer: notes,
        templates: Arc::new(OneOnOne),
        gateway: Arc::new(mock_gateway(mock.clone()).gateway),
    };
    let clock = Arc::new(FakeClock::new(at("2026-10-09T10:00:00Z")));
    let view = EnhanceService::new(ports, clock)
        .enhance(&meeting, Overwrite::KeepEdits)
        .await
        .unwrap();

    let prompt: String = mock
        .requests()
        .iter()
        .flat_map(|request| &request.messages)
        .map(deino_llm::Message::text)
        .collect();
    assert!(
        prompt.contains(
            "[screen 00:12] Q3 Business Review · - Revenue grew 40% year over year · - Churn fell to 2.1%"
        ),
        "{prompt}"
    );
    assert!(prompt.contains("[screen 00:30] Launch plan"), "{prompt}");
    assert!(format!("{view:?}").contains("Revenue grew 40% year over year"));
}
