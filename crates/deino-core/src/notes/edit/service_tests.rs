//! `NoteEditService` over the seeded fixtures in SQLite and a scripted
//! `notes.edit` (the mock LLM): renames without the model, edits applied in
//! place, redaction, accept and undo.

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use deino_db::test_db;
use deino_doc::{Block, CitationAttrs, Doc, Flavor, Inline, Mark, OriginAttrs};
use deino_domain::{FakeClock, MeetingId};
use deino_llm::{MockProvider, Route, mock_gateway};
use deino_testkit::at;
use deino_testkit::fixture::{Fixtures, MeetingKind, seed_fixtures};

use super::*;
use crate::enhance::{
    EnhancePorts, EnhanceService, EnhancedNotesReader, EnhancedNotesWriter, MaterialSources,
    NewVersion, SqliteEnhancedNotesRepo, TemplateSource, TranscriptLine,
};
use crate::notes::edit::test_support::{ai, ai_link, bullets, doc, heading, mention, para, user};
use crate::redaction::TextRedactor;
use crate::templates::{CATALOG, Template, TemplateError};

/// The secret [`Hide`] redacts.
const SECRET: &str = "hunter2";

/// A redactor that hides [`SECRET`] and remembers what it was given.
#[derive(Default)]
struct Hide {
    seen: Mutex<Vec<String>>,
}

#[async_trait]
impl TextRedactor for Hide {
    async fn redact(&self, texts: Vec<String>) -> Vec<String> {
        self.seen.lock().unwrap().extend(texts.iter().cloned());
        texts
            .into_iter()
            .map(|text| text.replace(SECRET, "[redacted]"))
            .collect()
    }
}

/// The 1:1 template; restoring never asks for it.
struct OneOnOne;

#[async_trait]
impl TemplateSource for OneOnOne {
    async fn template_for(&self, _meeting: &MeetingId) -> Result<Template, TemplateError> {
        let builtin = CATALOG
            .iter()
            .find(|template| template.id == "one_on_one")
            .unwrap();
        Ok(Template {
            id: deino_domain::TemplateId::new(builtin.id),
            is_builtin: true,
            content: builtin.content(),
            created_at: at("2026-10-01T00:00:00Z"),
            updated_at: at("2026-10-01T00:00:00Z"),
        })
    }
}

struct Harness {
    edits: NoteEditService,
    notes: EnhanceService,
    reader: Arc<SqliteEnhancedNotesRepo>,
    mock: Arc<MockProvider>,
    redactor: Arc<Hide>,
    meeting: MeetingId,
    first: Doc,
}

/// The one-on-one fixture with version 1 of its notes ([`first_notes`]).
async fn harness(mock: MockProvider) -> Harness {
    let (sources, repo, meeting, first) = seeded().await;
    let clock = Arc::new(FakeClock::new(at("2026-10-12T09:00:00Z")));
    let mock = Arc::new(mock);
    let gateway = Arc::new(mock_gateway(mock.clone()).gateway);
    let redactor = Arc::new(Hide::default());
    let edits = NoteEditService::new(
        NoteEditPorts {
            reader: repo.clone(),
            writer: repo.clone(),
            transcript: Arc::new(sources.clone()),
            redactor: redactor.clone(),
            gateway: Some(gateway.clone()),
        },
        clock.clone(),
    );
    let ports = EnhancePorts {
        sources,
        reader: repo.clone(),
        writer: repo.clone(),
        templates: Arc::new(OneOnOne),
        gateway,
    };
    let notes = EnhanceService::new(ports, clock);
    Harness {
        edits,
        notes,
        reader: repo,
        mock,
        redactor,
        meeting,
        first,
    }
}

/// The seeded library, the one-on-one with version 1 of its notes stored.
async fn seeded() -> (
    MaterialSources,
    Arc<SqliteEnhancedNotesRepo>,
    MeetingId,
    Doc,
) {
    let database = test_db().await;
    seed_fixtures(&database).await.unwrap();
    let fixtures = Fixtures::load().unwrap();
    let meeting = fixtures.meeting(MeetingKind::OneOnOne).unwrap().id.clone();
    let repo = Arc::new(SqliteEnhancedNotesRepo::new(database.clone()));
    let sources = MaterialSources::sqlite(&database);
    let lines = sources.transcript_of(&meeting).await.unwrap();
    let first = first_notes(&lines);
    repo.insert(&version_one(&meeting, &first)).await.unwrap();
    (sources, repo, meeting, first)
}

/// b1 Pricing; b2 the user's line with a mention; b3 AI text with a link
/// and a citation; b4 a secret; b5 Next steps; b6 a cited paragraph.
fn first_notes(lines: &[TranscriptLine]) -> Doc {
    doc(vec![
        heading("Pricing"),
        bullets(vec![
            vec![user("ask "), mention("Niall"), user(" about pricing")],
            vec![
                ai("Niall owns the "),
                ai_link("Niall deck", "https://example.com/Niall"),
                ai(" "),
                cited(&lines[0]),
            ],
            vec![ai(&format!("Card password is {SECRET} ")), cited(&lines[1])],
        ]),
        heading("Next steps"),
        para(vec![ai("Ship the pilot on Friday "), cited(&lines[1])]),
    ])
    .normalized()
}

fn cited(line: &TranscriptLine) -> Inline {
    Inline::Citation {
        attrs: CitationAttrs {
            segment_id: line.segment_id,
            start_ms: line.start,
        },
        marks: vec![Mark::Origin {
            attrs: OriginAttrs::AI,
        }],
    }
}

fn version_one(meeting: &MeetingId, doc: &Doc) -> NewVersion {
    NewVersion {
        meeting_id: meeting.clone(),
        doc: doc.clone(),
        markdown: deino_doc::to_markdown(doc, Flavor::Readable),
        template_id: None,
        model: "builtin:mock-local".into(),
        basis: None,
        is_user_edited: false,
        at: at("2026-10-12T08:00:00Z"),
    }
}

fn request(harness: &Harness, instruction: &str, selection: Vec<u32>) -> EditRequest {
    EditRequest {
        meeting_id: harness.meeting.clone(),
        version: 1,
        instruction: instruction.into(),
        selection,
    }
}

/// Every citation, in document order.
fn citations(doc: &Doc) -> Vec<CitationAttrs> {
    fn walk(block: &Block, out: &mut Vec<CitationAttrs>) {
        for inline in block.inline_content() {
            if let Inline::Citation { attrs, .. } = inline {
                out.push(*attrs);
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

fn edit_calls(mock: &MockProvider) -> Vec<String> {
    mock.requests()
        .into_iter()
        .filter(|call| call.route == Route::NotesEdit)
        .flat_map(|call| call.messages.into_iter().map(|message| message.text()))
        .collect()
}

#[tokio::test]
async fn a_rename_updates_every_occurrence_without_the_model() {
    let harness = harness(MockProvider::new()).await;

    let proposal = harness
        .edits
        .propose(&request(&harness, "Niall is spelled Nyal", Vec::new()))
        .await
        .unwrap();

    assert_eq!(
        (proposal.method, proposal.occurrences),
        (EditMethod::Rename, Some(3))
    );
    assert!(edit_calls(&harness.mock).is_empty(), "no model call");
    let lossless = deino_doc::to_markdown(&proposal.doc, Flavor::Lossless);
    assert!(
        !lossless.contains("Niall owns")
            && lossless.contains("Nyal deck](https://example.com/Niall)"),
        "{lossless}"
    );
    assert_eq!(
        citations(&proposal.doc),
        citations(&harness.first),
        "citation targets never move"
    );
    assert_eq!(
        proposal.changes.len(),
        2,
        "the mention's line and the AI line"
    );
}

#[tokio::test]
async fn accepting_stores_an_edited_version_and_restoring_the_previous_undoes_it() {
    let harness = harness(MockProvider::new()).await;
    let proposal = harness
        .edits
        .propose(&request(
            &harness,
            "rename Niall to Nyal everywhere",
            Vec::new(),
        ))
        .await
        .unwrap();

    let accepted = harness
        .edits
        .accept(&harness.meeting, 1, proposal.doc.clone())
        .await
        .unwrap();

    assert_eq!((accepted.version, accepted.previous_version), (2, 1));
    let latest = harness
        .reader
        .latest(&harness.meeting)
        .await
        .unwrap()
        .unwrap();
    assert!(
        latest.is_user_edited,
        "regenerating asks before replacing it"
    );
    assert_eq!(latest.doc, proposal.doc);
    let undone = harness
        .notes
        .restore(&harness.meeting, accepted.previous_version)
        .await
        .unwrap();
    assert_eq!((undone.version.version, &undone.doc), (3, &harness.first));
    let stale = harness
        .edits
        .accept(&harness.meeting, 1, proposal.doc)
        .await
        .unwrap_err();
    assert!(matches!(stale, EditError::NotLatest), "{stale}");
}

const SECTION_EDIT: &str = r#"{"changes": [
    {"action": "replace", "block": "b3", "text": "Niall owns the Niall deck, due Monday [s1]"},
    {"action": "delete", "block": "b5"},
    {"action": "delete", "block": "b6"}
]}"#;

#[tokio::test]
async fn the_model_s_changes_apply_in_place_and_keep_everything_else() {
    let harness = harness(MockProvider::new().reply(Route::NotesEdit, SECTION_EDIT)).await;

    let proposal = harness
        .edits
        .propose(&request(
            &harness,
            "remove next steps; the deck is due Monday",
            Vec::new(),
        ))
        .await
        .unwrap();

    assert_eq!(proposal.method, EditMethod::Ai);
    let rows: Vec<BlockChange> = proposal.changes.iter().map(|row| row.change).collect();
    assert_eq!(
        rows,
        [
            BlockChange::Changed,
            BlockChange::Removed,
            BlockChange::Removed
        ]
    );
    assert_eq!(proposal.doc.content.first(), harness.first.content.first());
    let lossless = deino_doc::to_markdown(&proposal.doc, Flavor::Lossless);
    assert!(
        lossless.contains("Niall deck](https://example.com/Niall)"),
        "the link keeps its target: {lossless}"
    );
    assert_eq!(
        citations(&proposal.doc).len(),
        2,
        "the kept lines keep their citations"
    );
    assert!(!lossless.contains("Next steps"));
}

#[tokio::test]
async fn secrets_never_reach_the_model_and_their_lines_are_not_rewritten() {
    let answer = r#"{"changes": [
        {"action": "replace", "block": "b4", "text": "Card password is [redacted] [s2]"},
        {"action": "insert_after", "block": "b4", "text": "Pilot budget agreed [s2]", "kind": "bullet"}
    ]}"#;
    let harness = harness(MockProvider::new().reply(Route::NotesEdit, answer)).await;

    let proposal = harness
        .edits
        .propose(&request(&harness, "add the budget", Vec::new()))
        .await
        .unwrap();

    let sent = edit_calls(&harness.mock).join("\n");
    assert!(
        !sent.contains(SECRET) && sent.contains("[redacted]"),
        "{sent}"
    );
    assert!(sent.contains(r#""locked":true"#));
    assert!(!harness.redactor.seen.lock().unwrap().is_empty());
    let readable = deino_doc::to_markdown(&proposal.doc, Flavor::Readable);
    assert!(
        readable.contains(SECRET),
        "the line with the secret is unchanged"
    );
    assert_eq!(
        proposal.changes.len(),
        1,
        "only the new bullet: {:?}",
        proposal.changes
    );
}

#[tokio::test]
async fn a_selection_scopes_the_edit_and_an_empty_answer_changes_nothing() {
    let harness = harness(MockProvider::new().reply(Route::NotesEdit, SECTION_EDIT)).await;

    let error = harness
        .edits
        .propose(&request(&harness, "tighten this", vec![1]))
        .await
        .unwrap_err();

    assert!(
        matches!(error, EditError::NoChange),
        "b3, b5 and b6 are outside the selection"
    );
    let sent = edit_calls(&harness.mock).join("\n");
    assert!(sent.contains("The user selected b2."), "{sent}");
    let stale = harness
        .edits
        .propose(&EditRequest {
            version: 9,
            ..request(&harness, "x", Vec::new())
        })
        .await
        .unwrap_err();
    assert!(matches!(stale, EditError::NotLatest));
}
