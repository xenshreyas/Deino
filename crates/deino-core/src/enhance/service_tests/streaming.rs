//! Streamed notes: the drafts a run publishes while the answer arrives in
//! pieces, a retry, Stop, and a page opened mid-run.

use deino_llm::CompletionEvent;

use super::*;
use crate::enhance::partial::{complete_lines, line_count};
use crate::enhance::progress::{NotesPhase, NotesRunEnd, RecordedProgress, Reported};

const FIRST_LINE: &str = "v tired post fernhollow. slept 11h, still wiped";

/// An answer in the schema's field order, as a model writes it
/// (`answer_json` sorts the keys).
fn streamed_answer() -> String {
    format!(
        r#"{{"sections":[{{"heading":"Updates","blocks":[
{{"kind":"bullet","text":"{FIRST_LINE}","origin":"user","evidence":["s1"]}},
{{"kind":"bullet","text":"Maya is coming off two weeks of incident work","origin":"ai","evidence":["s2"]}},
{{"kind":"bullet","text":"She wants to move the pilot review to Thursday","origin":"ai","evidence":["s3"]}}
]}}]}}"#
    )
}

/// The service of `harness`, reporting to a recorder.
fn recorded(harness: Harness) -> (EnhanceService, Arc<RecordedProgress>, MeetingId) {
    let progress = Arc::new(RecordedProgress::default());
    let meeting = harness.fixture.id.clone();
    let service = harness.service.with_progress(progress.clone());
    (service, progress, meeting)
}

#[tokio::test]
async fn a_streamed_answer_shows_its_lines_in_order_then_the_saved_notes_match() {
    let answer = streamed_answer();
    let mock = MockProvider::new().reply_in_chunks(Route::NotesEnhance, &answer, 9);
    let (service, progress, meeting) = recorded(harness(MeetingKind::OneOnOne, mock).await);

    let view = service
        .enhance(&meeting, Overwrite::KeepEdits)
        .await
        .unwrap();

    let drafts = progress.drafts();
    assert_eq!(drafts[0].phase, NotesPhase::Reading);
    assert_eq!(drafts[0].lines, 0);
    let counts: Vec<u32> = drafts.iter().map(|draft| draft.lines).collect();
    assert!(
        counts.windows(2).all(|pair| pair[0] <= pair[1]),
        "{counts:?}"
    );
    assert_eq!(
        drafts.len(),
        1 + 4 + 1,
        "reading, a draft per new line, saving"
    );
    let saving = drafts.last().unwrap();
    assert_eq!(saving.phase, NotesPhase::Saving);
    let expected = line_count(&complete_lines(&answer));
    assert_eq!(saving.lines as usize, expected);
    // The saved notes add citations (after a space) and put back the user
    // lines the answer left out, in a closing section; nothing else moves.
    let compact = |doc: &Doc| text_by(doc, Author::Ai).replace(' ', "");
    assert!(compact(&view.doc).starts_with(&compact(&saving.doc)));
    assert!(text_by(&saving.doc, Author::User).contains(FIRST_LINE));
    let end = progress.reports().last().cloned().unwrap();
    assert!(
        matches!(&end, Reported::Ended(ended, NotesRunEnd::Written { version: 1, kept })
            if *ended == meeting && *kept > 0),
        "{end:?}"
    );
    assert!(service.draft(&meeting).is_none(), "the run is forgotten");
}

#[tokio::test]
async fn a_cut_off_answer_keeps_its_lines_stale_until_the_retry_writes_one() {
    let answer = streamed_answer();
    let cut = answer.find("She wants").unwrap();
    let cut_off = vec![CompletionEvent::Text(answer[..cut].to_owned())];
    let mock = MockProvider::new()
        .events(Route::NotesEnhance, cut_off)
        .reply_in_chunks(Route::NotesEnhance, &answer, 40);
    let (service, progress, meeting) = recorded(harness(MeetingKind::OneOnOne, mock).await);

    service
        .enhance(&meeting, Overwrite::KeepEdits)
        .await
        .unwrap();

    let drafts = progress.drafts();
    let retrying = drafts
        .iter()
        .position(|draft| draft.phase == NotesPhase::Retrying)
        .unwrap();
    assert!(drafts[retrying].stale);
    assert!(drafts[retrying].lines > 0, "the old lines stay, dimmed");
    let fresh = &drafts[retrying + 1];
    assert_eq!(fresh.phase, NotesPhase::Reading);
    assert!(!fresh.stale);
}

#[tokio::test]
async fn stopping_a_pipeline_run_saves_nothing_and_is_a_skip() {
    let answer = streamed_answer();
    let cut = answer.find("Maya is").unwrap();
    let mock = MockProvider::new().stall(Route::NotesEnhance, &answer[..cut]);
    let harness = harness(MeetingKind::OneOnOne, mock).await;
    let reader = SqliteEnhancedNotesRepo::new(harness.database.clone());
    let (service, progress, meeting) = recorded(harness);

    let stop_once_a_line_shows = async {
        for _ in 0..100_000 {
            if service
                .draft(&meeting)
                .is_some_and(|draft| draft.lines >= 2)
            {
                assert!(service.stop(&meeting));
                return;
            }
            tokio::task::yield_now().await;
        }
        panic!("no line was ever shown");
    };
    let (run, ()) = tokio::join!(service.run_for_pipeline(&meeting), stop_once_a_line_shows);

    assert_eq!(run.unwrap(), PipelineRun::Skipped(STOPPED));
    assert!(reader.latest(&meeting).await.unwrap().is_none());
    let end = progress.reports().last().cloned().unwrap();
    assert_eq!(end, Reported::Ended(meeting.clone(), NotesRunEnd::Stopped));
    assert!(!service.stop(&meeting), "nothing is running any more");
}

#[tokio::test]
async fn a_run_refused_before_writing_reports_nothing() {
    let mock = MockProvider::new().reply(Route::NotesEnhance, &streamed_answer());
    let (service, progress, meeting) = recorded(harness(MeetingKind::OneOnOne, mock).await);
    service
        .enhance(&meeting, Overwrite::KeepEdits)
        .await
        .unwrap();
    let before = progress.reports().len();
    let latest = service.view(&meeting, None).await.unwrap().unwrap();
    service
        .save(&meeting, latest.version.version, latest.doc)
        .await
        .unwrap();

    let refused = service.enhance(&meeting, Overwrite::KeepEdits).await;

    assert!(matches!(refused, Err(EnhanceError::WouldReplaceEdits)));
    assert_eq!(progress.reports().len(), before);
}
