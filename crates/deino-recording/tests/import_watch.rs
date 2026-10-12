//! CAP-11 end to end: a recording dropped into a watched folder becomes a
//! finished meeting, once.
//!
//! Real SQLite, real decoding and AAC encoding (`NativeIngest`), the
//! `final_transcription` rules over the scripted batch engine, and a fake
//! clock for the 10 s stability rule. The two-hour case runs by hand with
//! the real engine: `examples/import_bench.rs`.
#![cfg(target_os = "macos")]
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::print_stderr,
    reason = "an integration-test crate: helpers fail the test by panicking"
)]

#[path = "support/import_rig.rs"]
mod import_rig;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::TimeDelta;
use deino_core::meetings::{Meeting, MeetingLifecycleRepo};
use deino_core::segments::{Page, SegmentReader};
use deino_core::watch::PROCESSED_FOLDER;
use deino_domain::{Channel, FakeClock, MeetingSource, MeetingStatus};
use deino_testkit::Fixtures;
use deino_testkit::at;
use deino_testkit::fixture::MeetingKind;
use import_rig::{ImportRig, ScriptedBatch, import_rig};

/// The one-on-one fixture's system track: AAC in an MPEG-4 file.
fn recording() -> PathBuf {
    let fixtures = Fixtures::load().unwrap();
    let meeting = fixtures.meeting(MeetingKind::OneOnOne).unwrap();
    let audio = meeting.audio.as_ref().unwrap();
    fixtures.audio_path(audio, Channel::System).unwrap()
}

/// A rig over a fresh database, its fake clock, and the folder it watches.
async fn watching(root: &Path, move_to_processed: bool) -> (ImportRig, Arc<FakeClock>) {
    let clock = Arc::new(FakeClock::new(at("2026-10-12T10:00:00Z")));
    let database = deino_db::test_db().await;
    let rig = import_rig(database, root, Arc::new(ScriptedBatch), clock.clone()).await;
    let folder = rig.watch.add_folder(&rig.inbox).await.unwrap();
    rig.watch
        .set_move_to_processed(&folder.id, move_to_processed)
        .await
        .unwrap();
    (rig, clock)
}

/// Scans now and 10 s later, when a file dropped before is ready.
async fn settle(rig: &ImportRig, clock: &FakeClock) {
    rig.watch.scan().await.unwrap();
    clock.advance(TimeDelta::seconds(10));
    rig.watch.scan().await.unwrap();
}

async fn finished(rig: &ImportRig) -> Vec<Meeting> {
    rig.meetings
        .list_by_status(MeetingStatus::Ready)
        .await
        .unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_recording_dropped_into_a_watched_folder_becomes_a_finished_meeting_once() {
    let root = tempfile::tempdir().unwrap();
    let (rig, clock) = watching(root.path(), false).await;
    std::fs::copy(recording(), rig.inbox.join("Customer call.mp4")).unwrap();

    rig.watch.scan().await.unwrap();
    assert!(
        finished(&rig).await.is_empty(),
        "not before it has been still for 10 s"
    );
    clock.advance(TimeDelta::seconds(10));
    rig.watch.scan().await.unwrap();
    settle(&rig, &clock).await;

    let ready = finished(&rig).await;
    assert_eq!(ready.len(), 1, "one finished meeting, imported once");
    let state = rig.meetings.state(&ready[0].id).await.unwrap().unwrap();
    assert_eq!(state.source, MeetingSource::Import);
    assert!(state.duration.unwrap().0 >= 30, "{:?}", state.duration);
    let video = PathBuf::from(state.audio.video_path.unwrap());
    assert!(video.exists() && video.starts_with(&rig.recordings));
    let segments = rig.segments.list(&ready[0].id, Page::ALL).await.unwrap();
    assert!(!segments.is_empty());
    assert!(
        segments
            .iter()
            .all(|segment| segment.channel == Channel::Mixed)
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn with_move_to_processed_the_imported_file_moves_aside() {
    let root = tempfile::tempdir().unwrap();
    let (rig, clock) = watching(root.path(), true).await;
    std::fs::copy(recording(), rig.inbox.join("Customer call.mp4")).unwrap();

    settle(&rig, &clock).await;

    assert_eq!(finished(&rig).await.len(), 1);
    assert!(!rig.inbox.join("Customer call.mp4").exists());
    assert!(
        rig.inbox
            .join(PROCESSED_FOLDER)
            .join("Customer call.mp4")
            .exists()
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_file_without_audio_lands_in_the_failure_list() {
    let root = tempfile::tempdir().unwrap();
    let (rig, clock) = watching(root.path(), true).await;

    std::fs::write(rig.inbox.join("broken.mov"), b"not a movie").unwrap();
    settle(&rig, &clock).await;

    let failures = rig.watch.failures().await.unwrap();
    assert_eq!(failures.len(), 1);
    assert!(failures[0].path.ends_with("broken.mov"));
    assert!(
        rig.inbox.join("broken.mov").exists(),
        "a failed file stays put"
    );
    assert!(std::fs::read_dir(&rig.recordings).map_or(true, |mut dir| dir.next().is_none()));
}
