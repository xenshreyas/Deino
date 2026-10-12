//! `LinkService`: every route dispatched over fakes, the hotkey toggle and
//! the app lock.

use std::str::FromStr;
use std::sync::Arc;

use deino_domain::{CalendarEventId, ErrorClass, ErrorCode, MeetingId, Millis};

use super::{
    FakeLock, FakeRecorder, FakeShell, Intent, LinkError, LinkService, MAX_PENDING, Navigation,
};

struct Harness {
    recorder: Arc<FakeRecorder>,
    lock: Arc<FakeLock>,
    shell: Arc<FakeShell>,
    service: LinkService,
}

fn harness(recorder: FakeRecorder) -> Harness {
    let recorder = Arc::new(recorder);
    let lock = Arc::new(FakeLock::default());
    let shell = Arc::new(FakeShell::default());
    let service = LinkService::new(recorder.clone(), lock.clone(), shell.clone());
    Harness {
        recorder,
        lock,
        shell,
        service,
    }
}

/// Opens `link` and returns what the UI would take.
async fn navigations(link: &str) -> Vec<Navigation> {
    let h = harness(FakeRecorder::default());
    h.service.open(link).await.unwrap();
    assert_eq!(h.shell.focused(), 1, "{link} brings the window forward");
    assert_eq!(h.shell.told(), 1, "{link} tells the UI");
    assert!(h.recorder.calls().is_empty(), "{link} never records");
    h.service.take()
}

#[tokio::test]
async fn a_meeting_link_queues_the_meeting_at_its_moment() {
    let expected = Navigation::Meeting {
        meeting_id: MeetingId::new("meeting-fixture-standup"),
        at_ms: Some(Millis(231_259)),
    };
    let taken = navigations("deino://meeting/meeting-fixture-standup?t=231259").await;
    assert_eq!(taken, vec![expected.clone()]);
    let alias = navigations("deino://meetings/meeting-fixture-standup?t=231259").await;
    assert_eq!(alias, vec![expected]);
}

#[tokio::test]
async fn a_new_note_link_queues_a_note_for_its_event() {
    assert_eq!(
        navigations("deino://new-note").await,
        vec![Navigation::NewNote { event_id: None }]
    );
    let id = "0190f5a2-7c1e-7b3a-9d2e-5f6a7b8c9d0e";
    let event_id = Some(CalendarEventId::from_str(id).unwrap());
    let link = format!("deino://new-note?event={id}");
    assert_eq!(
        navigations(&link).await,
        vec![Navigation::NewNote { event_id }]
    );
}

#[tokio::test]
async fn a_search_link_queues_the_search() {
    assert_eq!(
        navigations("deino://search?q=release").await,
        vec![Navigation::Search {
            query: "release".to_owned()
        }]
    );
}

#[tokio::test]
async fn a_recipe_link_queues_the_recipe_without_its_data() {
    assert_eq!(
        navigations("deino://recipe?data=abc").await,
        vec![Navigation::Recipe]
    );
}

#[tokio::test]
async fn record_links_go_through_the_recorder_without_the_window() {
    let h = harness(FakeRecorder::default());
    h.service.open("deino://record/start").await.unwrap();
    h.service.open("deino://record/stop").await.unwrap();
    assert_eq!(h.recorder.calls(), vec!["start", "stop"]);
    assert_eq!((h.shell.focused(), h.shell.told()), (0, 0));
    assert!(h.service.take().is_empty());
}

#[tokio::test]
async fn the_recorders_refusal_is_reported() {
    let h = harness(FakeRecorder::default());
    let error = h.service.open("deino://record/stop").await.unwrap_err();
    assert!(matches!(error, LinkError::Recorder(_)));
    assert_eq!(error.code(), ErrorCode::Conflict);
}

#[tokio::test]
async fn the_hotkey_toggle_starts_when_idle_and_stops_when_recording() {
    let idle = harness(FakeRecorder::default());
    idle.service.perform(Intent::ToggleRecording).await.unwrap();
    assert_eq!(idle.recorder.calls(), vec!["start"]);

    let busy = harness(FakeRecorder::recording());
    busy.service.perform(Intent::ToggleRecording).await.unwrap();
    assert_eq!(busy.recorder.calls(), vec!["stop"]);
}

#[tokio::test]
async fn a_bad_link_does_nothing() {
    let h = harness(FakeRecorder::default());
    let error = h.service.open("deino://search").await.unwrap_err();
    assert_eq!(error.code(), ErrorCode::Invalid);
    assert_eq!((h.shell.focused(), h.shell.told()), (0, 0));
    assert!(h.service.take().is_empty());
}

#[tokio::test]
async fn while_locked_a_navigation_waits_for_the_unlock() {
    let h = harness(FakeRecorder::default());
    h.lock.set(true);
    h.service.open("deino://meeting/abc").await.unwrap();
    assert_eq!(h.shell.focused(), 1, "the lock screen comes forward");
    assert!(
        h.service.take().is_empty(),
        "nothing is revealed while locked"
    );

    h.lock.set(false);
    let taken = h.service.take();
    assert_eq!(taken.len(), 1);
    assert!(h.service.take().is_empty(), "taking empties the queue");
}

#[tokio::test]
async fn while_locked_recording_links_still_work() {
    let h = harness(FakeRecorder::default());
    h.lock.set(true);
    h.service.open("deino://record/start").await.unwrap();
    assert_eq!(h.recorder.calls(), vec!["start"]);
}

#[tokio::test]
async fn the_queue_keeps_the_newest_navigations() {
    let h = harness(FakeRecorder::default());
    for index in 0..=MAX_PENDING {
        let link = format!("deino://search?q=q{index}");
        h.service.open(&link).await.unwrap();
    }
    let taken = h.service.take();
    assert_eq!(taken.len(), MAX_PENDING);
    assert_eq!(
        taken.first(),
        Some(&Navigation::Search {
            query: "q1".to_owned()
        })
    );
}

#[tokio::test]
async fn ask_queues_the_ask_panel() {
    let h = harness(FakeRecorder::default());
    h.service
        .perform(Intent::Show(Navigation::Ask))
        .await
        .unwrap();
    assert_eq!(h.service.take(), vec![Navigation::Ask]);
}
