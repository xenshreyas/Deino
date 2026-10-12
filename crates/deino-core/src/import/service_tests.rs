use std::sync::Arc;
use std::time::Duration;

use deino_domain::{FakeClock, ImportFailure, MeetingSource, MeetingStatus};
use deino_testkit::at;

use super::*;
use crate::import::FakeIngest;
use crate::meetings::FakeMeetingRepo;

const NOW: &str = "2026-10-12T10:00:00Z";
const MODIFIED: &str = "2026-10-12T09:30:00Z";

struct World {
    service: ImportService,
    ingest: Arc<FakeIngest>,
    meetings: Arc<FakeMeetingRepo>,
}

fn world() -> World {
    let ingest = Arc::new(FakeIngest::default());
    let meetings = Arc::new(FakeMeetingRepo::default());
    let clock: Arc<dyn Clock> = Arc::new(FakeClock::new(at(NOW)));
    let lifecycle = Arc::new(MeetingLifecycleService::new(
        meetings.clone(),
        clock.clone(),
    ));
    let service = ImportService::new(ImportDeps {
        ingest: ingest.clone(),
        meetings: meetings.clone(),
        lifecycle,
        recordings: PathBuf::from("/recordings"),
        clock,
    });
    World {
        service,
        ingest,
        meetings,
    }
}

fn facts(size_bytes: u64) -> MediaFacts {
    MediaFacts {
        size_bytes,
        modified: Some(at(MODIFIED)),
    }
}

fn request(path: &str) -> ImportRequest {
    ImportRequest {
        source: PathBuf::from(path),
        title: None,
    }
}

#[tokio::test]
async fn an_audio_file_becomes_a_processing_import_named_after_the_file() {
    let world = world();
    world
        .ingest
        .add("/in/Weekly sync.m4a", facts(10), Duration::from_mins(30));

    let imported = world
        .service
        .import(&request("/in/Weekly sync.m4a"))
        .await
        .unwrap();

    assert_eq!(imported.title, "Weekly sync");
    assert_eq!(imported.duration, Seconds(1_800));
    assert!(!imported.has_video);
    let state = world
        .meetings
        .state(&imported.meeting_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(state.status, MeetingStatus::Processing);
    assert_eq!(state.source, MeetingSource::Import);
    let folder = format!("/recordings/{}", imported.meeting_id);
    assert_eq!(state.audio.audio_path, Some(format!("{folder}/audio.m4a")));
    assert_eq!(state.audio.video_path, None);
}

#[tokio::test]
async fn the_recording_ends_when_the_file_was_written_and_lasts_its_length() {
    let world = world();
    world
        .ingest
        .add("/in/a.mp3", facts(10), Duration::from_mins(10));

    let imported = world.service.import(&request("/in/a.mp3")).await.unwrap();

    let state = world
        .meetings
        .state(&imported.meeting_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(state.ended_at, Some(at(MODIFIED)));
    assert_eq!(state.started_at, Some(at("2026-10-12T09:20:00Z")));
    assert_eq!(state.duration, Some(Seconds(600)));
}

#[tokio::test]
async fn a_video_keeps_its_file_for_playback_and_a_given_title_wins() {
    let world = world();
    world
        .ingest
        .add("/in/screen.mov", facts(10), Duration::from_mins(1));
    let request = ImportRequest {
        source: PathBuf::from("/in/screen.mov"),
        title: Some("Design review".into()),
    };

    let imported = world.service.import(&request).await.unwrap();

    assert!(imported.has_video);
    assert_eq!(imported.title, "Design review");
    let state = world
        .meetings
        .state(&imported.meeting_id)
        .await
        .unwrap()
        .unwrap();
    assert!(state.audio.video_path.unwrap().ends_with("video.mp4"));
}

#[tokio::test]
async fn files_that_cannot_be_imported_are_refused_before_any_work() {
    let world = world();
    world.ingest.add(
        "/in/huge.mp4",
        facts(MAX_IMPORT_BYTES + 1),
        Duration::from_secs(1),
    );

    let text = world
        .service
        .import(&request("/in/notes.txt"))
        .await
        .unwrap_err();
    let missing = world
        .service
        .import(&request("/in/gone.wav"))
        .await
        .unwrap_err();
    let huge = world
        .service
        .import(&request("/in/huge.mp4"))
        .await
        .unwrap_err();

    assert!(matches!(text, ImportError::UnsupportedFile), "{text:?}");
    assert_eq!(missing.failure(), ImportFailure::Missing);
    assert_eq!(huge.failure(), ImportFailure::TooLarge);
    assert!(world.ingest.written.lock().unwrap().is_empty());
}

#[tokio::test]
async fn a_failed_conversion_creates_no_meeting() {
    let world = world();
    world
        .ingest
        .add_broken("/in/silent.mov", facts(10), || ImportError::NoAudio);

    let error = world
        .service
        .import(&request("/in/silent.mov"))
        .await
        .unwrap_err();

    assert_eq!(error.failure(), ImportFailure::NoAudio);
    let meetings = world
        .meetings
        .list_by_status(MeetingStatus::Processing)
        .await
        .unwrap();
    assert!(meetings.is_empty());
}

#[test]
fn media_kinds_follow_the_extension_in_any_case() {
    assert_eq!(MediaKind::of(Path::new("/a/b.M4A")), Some(MediaKind::Audio));
    assert_eq!(
        MediaKind::of(Path::new("/a/b.webm")),
        Some(MediaKind::Video)
    );
    assert_eq!(
        MediaKind::of(Path::new("/a/b.opus")),
        Some(MediaKind::Audio)
    );
    assert_eq!(MediaKind::of(Path::new("/a/b.docx")), None);
    assert_eq!(MediaKind::of(Path::new("/a/no-extension")), None);
}
