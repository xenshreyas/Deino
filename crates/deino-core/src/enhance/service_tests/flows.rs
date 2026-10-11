//! One-pass flows: the first version, regenerating, edit protection, the
//! pipeline's skips, regenerate prompts, restore and citations.

use deino_doc::Doc;
use deino_testkit::seed;

use super::*;
use crate::enhance::user_lines::user_lines;
use crate::notes::{NotesRepo, SqliteNotesRepo, StoredNotes};

const FIRST_LINE: &str = "v tired post fernhollow. slept 11h, still wiped";

fn one_on_one_answer() -> String {
    answer_json(&[
        ("user", FIRST_LINE, &["s1"]),
        (
            "ai",
            "Maya is coming off two weeks of incident work",
            &["s2", "s99999"],
        ),
    ])
}

#[tokio::test]
async fn the_notes_quote_every_user_line_and_cite_only_real_segments() {
    let harness = harness(
        MeetingKind::OneOnOne,
        MockProvider::new().reply(Route::NotesEnhance, &one_on_one_answer()),
    )
    .await;
    let meeting = &harness.fixture.id;

    let view = harness
        .service
        .enhance(meeting, Overwrite::KeepEdits)
        .await
        .unwrap();

    let typed = deino_doc::from_markdown(harness.fixture.user_notes.as_deref().unwrap()).unwrap();
    let user_text = text_by(&view.doc, Author::User);
    for line in user_lines(&typed) {
        assert!(
            user_text.contains(&line.text),
            "missing user line {:?}",
            line.text
        );
    }
    assert!(text_by(&view.doc, Author::Ai).contains("two weeks of incident work"));
    let cited = citations(&view.doc);
    assert_eq!(cited.len(), 2, "s99999 is dropped");
    assert_eq!(
        cited[0].to_string(),
        deino_testkit::fixture::fixture_uuid(&format!("segment/{meeting}/0"))
    );
    assert_eq!(
        (view.version.version, view.version.model.as_str()),
        (1, "builtin:mock-local")
    );
    assert_eq!(view.version.template_name.as_deref(), Some("1:1"));
    assert!(view.is_latest && view.reasons.is_empty());
}

/// Writes version 1 of `meeting`'s notes, then saves the user's edit of it.
async fn written_then_edited(harness: &Harness, meeting: &MeetingId) {
    harness
        .service
        .enhance(meeting, Overwrite::KeepEdits)
        .await
        .unwrap();
    harness
        .service
        .save(meeting, 1, deino_doc::from_markdown("- my edit").unwrap())
        .await
        .unwrap();
}

#[tokio::test]
async fn regenerating_keeps_versions_and_asks_before_replacing_edits() {
    let mock = MockProvider::new()
        .reply(Route::NotesEnhance, &one_on_one_answer())
        .reply(Route::NotesEnhance, &one_on_one_answer());
    let harness = harness(MeetingKind::OneOnOne, mock).await;
    let meeting = &harness.fixture.id;
    written_then_edited(&harness, meeting).await;

    let refused = harness.service.enhance(meeting, Overwrite::KeepEdits).await;
    let replaced = harness
        .service
        .enhance(meeting, Overwrite::ReplaceEdits)
        .await
        .unwrap();

    assert!(matches!(refused, Err(EnhanceError::WouldReplaceEdits)));
    assert_eq!(replaced.version.version, 2);
    let versions = harness.service.versions(meeting).await.unwrap();
    let flags: Vec<(u32, bool)> = versions
        .iter()
        .map(|v| (v.version, v.is_user_edited))
        .collect();
    assert_eq!(
        flags,
        [(2, false), (1, true)],
        "the edited version stays in the history"
    );
    let old = harness
        .service
        .view(meeting, Some(1))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(old.doc, deino_doc::from_markdown("- my edit").unwrap());
    assert!(!old.is_latest);
    let stale = harness.service.save(meeting, 1, Doc::empty()).await;
    assert!(matches!(stale, Err(EnhanceError::NotLatest)));
}

#[tokio::test]
async fn the_pipeline_writes_once_then_skips_until_something_changes() {
    let mock = MockProvider::new().reply(
        Route::NotesEnhance,
        &answer_json(&[("ai", "Release cut at four", &["s3"])]),
    );
    let harness = harness(MeetingKind::Standup, mock).await;
    let meeting = &harness.fixture.id;
    seed::meeting(
        &harness.database,
        "meeting-empty",
        "New note",
        at("2026-10-09T08:00:00Z"),
    )
    .await;

    let first = harness.service.run_for_pipeline(meeting).await.unwrap();
    let again = harness.service.run_for_pipeline(meeting).await.unwrap();
    let empty = harness
        .service
        .run_for_pipeline(&MeetingId::new("meeting-empty"))
        .await
        .unwrap();
    harness
        .service
        .save(meeting, 1, deino_doc::from_markdown("- edited").unwrap())
        .await
        .unwrap();
    let edited = harness.service.run_for_pipeline(meeting).await.unwrap();

    assert_eq!(first, PipelineRun::Written(1));
    assert_eq!(again, PipelineRun::Skipped(UP_TO_DATE));
    assert_eq!(empty, PipelineRun::Skipped(NOTHING_TO_WRITE));
    assert_eq!(edited, PipelineRun::Skipped(KEPT_EDITS));
    assert_eq!(
        enhance_requests(&harness.mock).len(),
        1,
        "skips never call the LLM"
    );
}

#[tokio::test]
async fn editing_the_users_notes_after_enhancing_offers_to_regenerate() {
    let harness = harness(
        MeetingKind::OneOnOne,
        MockProvider::new().reply(Route::NotesEnhance, &one_on_one_answer()),
    )
    .await;
    let meeting = &harness.fixture.id;
    harness
        .service
        .enhance(meeting, Overwrite::KeepEdits)
        .await
        .unwrap();
    let doc = deino_doc::from_markdown("- v tired, said so twice").unwrap();
    let later = at("2026-10-09T10:00:00Z");
    SqliteNotesRepo::new(harness.database.clone())
        .save(&StoredNotes {
            meeting_id: meeting.clone(),
            markdown: "- v tired, said so twice".into(),
            doc,
            created_at: later,
            updated_at: later,
        })
        .await
        .unwrap();

    let view = harness.service.view(meeting, None).await.unwrap().unwrap();

    assert_eq!(view.reasons, [RegenerateReason::NotesEdited]);
}

#[tokio::test]
async fn restoring_copies_an_old_version_and_citations_bring_their_excerpts() {
    let harness = harness(
        MeetingKind::OneOnOne,
        MockProvider::new().reply(Route::NotesEnhance, &one_on_one_answer()),
    )
    .await;
    let meeting = &harness.fixture.id;
    let first = harness
        .service
        .enhance(meeting, Overwrite::KeepEdits)
        .await
        .unwrap();

    let restored = harness.service.restore(meeting, 1).await.unwrap();
    let excerpts = harness
        .service
        .cited(meeting, &citations(&first.doc))
        .await
        .unwrap();

    assert_eq!((restored.version.version, &restored.doc), (2, &first.doc));
    let spoken = &harness.fixture.segments[0];
    assert_eq!(excerpts[0].text, spoken.text);
    assert_eq!(excerpts[0].start, spoken.start_ms);
    assert!(!excerpts[0].speaker.is_empty());
}
