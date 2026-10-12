//! `EnhanceService` over the seeded fixtures in SQLite and a scripted
//! `notes.enhance`: verbatim user lines, valid citations, versions, edit
//! protection, map-reduce, regenerate prompts.

use std::sync::Arc;

use async_trait::async_trait;
use deino_db::{Database, test_db};
use deino_doc::{Author, Block, Doc, Inline, Mark};
use deino_domain::{FakeClock, MeetingId};
use deino_llm::{CompletionRequest, LlmError, MockProvider, ProviderId, Route, mock_gateway};
use deino_testkit::at;
use deino_testkit::fixture::{FixtureMeeting, Fixtures, MeetingKind, seed_fixtures};

use super::*;
use crate::enhance::test_support::one_on_one;
use crate::templates::{BuiltinSync, CATALOG, SqliteTemplateRepo, Template, TemplateError};

mod flows;
mod long_meetings;
mod retention;

/// The template every run uses.
struct Fixed(Template);

#[async_trait]
impl TemplateSource for Fixed {
    async fn template_for(&self, _meeting: &MeetingId) -> Result<Template, TemplateError> {
        Ok(self.0.clone())
    }
}

struct Harness {
    service: EnhanceService,
    database: Database,
    mock: Arc<MockProvider>,
    fixture: FixtureMeeting,
}

async fn harness(kind: MeetingKind, mock: MockProvider) -> Harness {
    let database = test_db().await;
    seed_fixtures(&database).await.unwrap();
    let clock = Arc::new(FakeClock::new(at("2026-10-09T09:00:00Z")));
    let templates = Arc::new(SqliteTemplateRepo::new(database.clone()));
    BuiltinSync::new(templates, clock.clone())
        .run(&CATALOG)
        .await
        .unwrap();
    let mock = Arc::new(mock);
    let repo = Arc::new(SqliteEnhancedNotesRepo::new(database.clone()));
    let ports = EnhancePorts {
        sources: MaterialSources::sqlite(&database),
        reader: repo.clone(),
        writer: repo,
        templates: Arc::new(Fixed(one_on_one())),
        gateway: Arc::new(mock_gateway(mock.clone()).gateway),
    };
    Harness {
        service: EnhanceService::new(ports, clock),
        database,
        mock,
        fixture: Fixtures::load().unwrap().meeting(kind).unwrap().clone(),
    }
}

/// The JSON of an answer with one section of `blocks` (`(origin, text, evidence)`).
fn answer_json(blocks: &[(&str, &str, &[&str])]) -> String {
    let blocks: Vec<serde_json::Value> = blocks
        .iter()
        .map(|(origin, text, evidence)| {
            serde_json::json!({ "kind": "bullet", "text": text, "origin": origin, "evidence": evidence })
        })
        .collect();
    serde_json::json!({ "sections": [{ "heading": "Updates", "blocks": blocks }] }).to_string()
}

/// The plain text of every run by `author`, joined by spaces.
fn text_by(doc: &Doc, author: Author) -> String {
    fn walk(block: &Block, author: Author, out: &mut Vec<String>) {
        for inline in block.inline_content() {
            let ours = inline
                .marks()
                .iter()
                .any(|mark| matches!(mark, Mark::Origin { attrs } if attrs.author == author));
            if let (true, Inline::Text { text, .. }) = (ours, inline) {
                out.push(text.clone());
            }
        }
        block
            .children()
            .into_iter()
            .for_each(|child| walk(child, author, out));
    }
    let mut out = Vec::new();
    doc.content
        .iter()
        .for_each(|block| walk(block, author, &mut out));
    out.join("")
}

/// Every citation's segment ID, in document order.
fn citations(doc: &Doc) -> Vec<deino_domain::SegmentId> {
    fn walk(block: &Block, out: &mut Vec<deino_domain::SegmentId>) {
        for inline in block.inline_content() {
            if let Inline::Citation { attrs, .. } = inline {
                out.push(attrs.segment_id);
            }
        }
        block
            .children()
            .into_iter()
            .for_each(|child| walk(child, out));
    }
    let mut out = Vec::new();
    doc.content.iter().for_each(|block| walk(block, &mut out));
    out
}

/// The user's text of every request on `notes.enhance`.
fn enhance_requests(mock: &MockProvider) -> Vec<CompletionRequest> {
    mock.requests()
        .into_iter()
        .filter(|request| request.route == Route::NotesEnhance)
        .collect()
}

#[tokio::test]
async fn a_missing_model_fails_with_a_message_the_user_can_act_on() {
    let unavailable = LlmError::Unavailable {
        provider: ProviderId::BuiltIn,
        reason: "no model".into(),
    };
    let harness = harness(
        MeetingKind::OneOnOne,
        MockProvider::new().fail(Route::NotesEnhance, unavailable),
    )
    .await;

    let error = harness
        .service
        .enhance(&harness.fixture.id, Overwrite::KeepEdits)
        .await
        .unwrap_err();

    assert_eq!(
        error.to_string(),
        "Add an Anthropic key or pick a local model in Settings → AI."
    );
    assert!(
        harness
            .service
            .view(&harness.fixture.id, None)
            .await
            .unwrap()
            .is_none()
    );
}
