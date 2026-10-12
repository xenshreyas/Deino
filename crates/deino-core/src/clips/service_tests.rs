//! [`ClipService`] over the real `clips` table and in-memory files.

use std::path::PathBuf;
use std::sync::Arc;

use deino_db::{Database, test_db};
use deino_domain::{ErrorClass, ErrorCode, FakeClock, MeetingId, Millis};
use deino_testkit::{at, seed};

use super::*;

const MEETING: &str = "meeting-a";
const ROOT: &str = "/data/recordings";

struct Harness {
    service: ClipService,
    files: Arc<FakeClipFiles>,
    cutter: Arc<FakeClipCutter>,
    finder: Arc<FakeFinder>,
    database: Database,
}

async fn harness_with(media: ClipMedia, failing: bool) -> Harness {
    let database = test_db().await;
    seed::meeting(&database, MEETING, "Standup", at("2026-10-08T09:00:00Z")).await;
    let files = Arc::new(FakeClipFiles::under(ROOT));
    let cutter = Arc::new(if failing {
        FakeClipCutter::failing(files.clone())
    } else {
        FakeClipCutter::writing_to(files.clone())
    });
    let finder = Arc::new(FakeFinder::default());
    let file = if media == ClipMedia::Video {
        "screen.mp4"
    } else {
        "audio.mp4"
    };
    let source = ClipSource {
        path: PathBuf::from(ROOT).join(MEETING).join(file),
        media,
    };
    let service = ClipService::new(ClipDeps {
        repo: Arc::new(SqliteClipRepo::new(database.clone())),
        sources: Arc::new(FakeClipSources::at(source)),
        files: files.clone(),
        cutter: cutter.clone(),
        finder: finder.clone(),
        clock: Arc::new(FakeClock::new(at("2026-10-12T10:00:00Z"))),
    });
    Harness {
        service,
        files,
        cutter,
        finder,
        database,
    }
}

async fn harness() -> Harness {
    harness_with(ClipMedia::Audio, false).await
}

fn new_clip(name: &str, start: u32, end: u32) -> NewClip {
    NewClip {
        meeting_id: MeetingId::new(MEETING),
        name: name.into(),
        start: Millis(start),
        end: Millis(end),
    }
}

fn clip_path(name: &str) -> PathBuf {
    PathBuf::from(ROOT).join(MEETING).join("clips").join(name)
}

#[tokio::test]
async fn a_clip_is_cut_beside_the_recording_and_listed() {
    let h = harness().await;

    let clip = h
        .service
        .create(new_clip("  Pricing: Q4 ", 3_000, 7_500))
        .await
        .unwrap();

    assert_eq!(clip.name, "Pricing: Q4");
    assert_eq!(clip.meeting_title, "Standup");
    assert_eq!(PathBuf::from(&clip.path), clip_path("Pricing Q4.m4a"));
    let cut = &h.cutter.cuts()[0];
    assert_eq!((cut.start, cut.end), (Millis(3_000), Millis(7_500)));
    assert_eq!(
        cut.source,
        PathBuf::from(ROOT).join(MEETING).join("audio.mp4")
    );
    let listed = h.service.list(&MeetingId::new(MEETING)).await.unwrap();
    assert_eq!(listed, vec![clip]);
}

#[tokio::test]
async fn a_meeting_with_a_screen_recording_gets_an_mp4() {
    let h = harness_with(ClipMedia::Video, false).await;

    let clip = h.service.create(new_clip("Demo", 0, 1_000)).await.unwrap();

    assert_eq!(clip.media, ClipMedia::Video);
    assert!(clip.path.ends_with("Demo.mp4"), "{}", clip.path);
}

#[tokio::test]
async fn names_stay_unique_in_the_folder() {
    let h = harness().await;
    h.files.add(clip_path("intro.m4a"));

    let first = h.service.create(new_clip("Intro", 0, 1_000)).await.unwrap();
    let second = h.service.create(new_clip("INTRO", 0, 1_000)).await.unwrap();

    assert!(first.path.ends_with("Intro 2.m4a"), "{}", first.path);
    assert!(second.path.ends_with("INTRO 3.m4a"), "{}", second.path);
}

#[tokio::test]
async fn an_empty_name_or_range_is_refused_before_cutting() {
    let h = harness().await;

    let empty_name = h
        .service
        .create(new_clip("  ", 0, 1_000))
        .await
        .unwrap_err();
    let empty_range = h
        .service
        .create(new_clip("Intro", 2_000, 2_000))
        .await
        .unwrap_err();

    assert!(matches!(empty_name, ClipError::EmptyName));
    assert!(matches!(empty_range, ClipError::EmptyRange));
    assert_eq!(empty_range.code(), ErrorCode::Invalid);
    assert!(h.cutter.cuts().is_empty());
}

#[tokio::test]
async fn a_failed_cut_saves_nothing() {
    let h = harness_with(ClipMedia::Audio, true).await;

    let error = h
        .service
        .create(new_clip("Intro", 0, 1_000))
        .await
        .unwrap_err();

    assert!(matches!(error, ClipError::Cut(_)));
    assert!(h.service.list_all().await.unwrap().is_empty());
}

#[tokio::test]
async fn renaming_moves_the_file_to_a_free_safe_name() {
    let h = harness().await;
    let clip = h.service.create(new_clip("Intro", 0, 1_000)).await.unwrap();
    h.service.create(new_clip("Outro", 0, 1_000)).await.unwrap();

    let renamed = h.service.rename(&clip.id, "Outro/again").await.unwrap();
    let same = h.service.rename(&clip.id, "outro AGAIN").await.unwrap();

    assert_eq!(renamed.name, "Outro/again");
    assert_eq!(PathBuf::from(&renamed.path), clip_path("Outro again.m4a"));
    assert_eq!(PathBuf::from(&same.path), clip_path("outro AGAIN.m4a"));
    assert_eq!(
        h.files.paths(),
        vec![clip_path("Outro.m4a"), clip_path("outro AGAIN.m4a")]
    );
}

#[tokio::test]
async fn deleting_removes_the_row_and_the_file() {
    let h = harness().await;
    let clip = h.service.create(new_clip("Intro", 0, 1_000)).await.unwrap();

    h.service.delete(&clip.id).await.unwrap();

    assert!(h.files.paths().is_empty());
    assert!(matches!(
        h.service.delete(&clip.id).await,
        Err(ClipError::NotFound)
    ));
}

#[tokio::test]
async fn only_a_file_inside_the_app_folders_is_revealed() {
    let h = harness().await;
    let clip = h.service.create(new_clip("Intro", 0, 1_000)).await.unwrap();

    h.service.reveal(&clip.id).await.unwrap();
    h.files.remove(&clip_path("Intro.m4a")).await.unwrap();
    let missing = h.service.reveal(&clip.id).await.unwrap_err();

    assert_eq!(h.finder.shown(), vec![clip_path("Intro.m4a")]);
    assert!(matches!(missing, ClipError::FileMissing));
}

#[tokio::test]
async fn a_trashed_meetings_clips_are_hidden() {
    let h = harness().await;
    let clip = h.service.create(new_clip("Intro", 0, 1_000)).await.unwrap();

    sqlx::query("UPDATE meetings SET deleted_at = '2026-10-12T11:00:00Z' WHERE id = ?1")
        .bind(MEETING)
        .execute(h.database.writer())
        .await
        .unwrap();

    assert!(h.service.list_all().await.unwrap().is_empty());
    assert!(
        h.service
            .list(&MeetingId::new(MEETING))
            .await
            .unwrap()
            .is_empty()
    );
    assert!(matches!(
        h.service.reveal(&clip.id).await,
        Err(ClipError::NotFound)
    ));
}
