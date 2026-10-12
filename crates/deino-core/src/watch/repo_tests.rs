use std::path::PathBuf;

use deino_db::test_db;
use deino_domain::{FakeClock, MeetingId, MeetingSource, MeetingStatus};
use deino_testkit::at;

use super::*;
use crate::meetings::{MeetingLifecycleRepo, MeetingTitle, NewMeeting, SqliteMeetingRepo};

const ADDED: &str = "2026-10-12T09:00:00Z";

async fn repo() -> (SqliteWatchRepo, Database) {
    let database = test_db().await;
    (SqliteWatchRepo::new(database.clone()), database)
}

/// Inserts an imported meeting with `id`.
async fn insert_meeting(database: &Database, id: &str) -> MeetingId {
    let id = MeetingId::new(id);
    SqliteMeetingRepo::new(database.clone())
        .create(&NewMeeting {
            id: id.clone(),
            title: MeetingTitle::parse("Imported").unwrap(),
            status: MeetingStatus::Processing,
            source: MeetingSource::Import,
            created_at: at(ADDED),
            started_at: None,
            folder_path: None,
        })
        .await
        .unwrap();
    id
}

fn folder(path: &str) -> WatchedFolder {
    WatchedFolder {
        id: WatchedFolderId::generate(&FakeClock::new(at(ADDED))),
        path: path.into(),
        move_to_processed: false,
        created_at: at(ADDED),
    }
}

fn sighting(name: &str) -> Sighting {
    Sighting {
        path: PathBuf::from("/watch").join(name),
        size_bytes: 1_000,
        modified_ms: 1_760_000_000_000,
    }
}

fn handled(
    folder: &WatchedFolder,
    file: Sighting,
    meeting_id: Option<MeetingId>,
    failure: Option<ImportFailure>,
) -> HandledFile {
    HandledFile {
        id: WatchedFileId::generate(&FakeClock::new(at(ADDED))),
        folder_id: folder.id,
        sighting: file,
        meeting_id,
        failure,
        handled_at: at("2026-10-12T10:00:00Z"),
    }
}

#[tokio::test]
async fn folders_round_trip_and_toggle_move_to_processed() {
    let (repo, _) = repo().await;
    let watched = folder("/watch");
    repo.add_folder(&watched).await.unwrap();

    assert!(repo.set_move_to_processed(&watched.id, true).await.unwrap());

    let folders = repo.folders().await.unwrap();
    assert_eq!(folders.len(), 1);
    assert!(folders[0].move_to_processed);
    assert_eq!(folders[0].path, "/watch");
    assert!(repo.add_folder(&folder("/watch")).await.is_err());
}

#[tokio::test]
async fn the_ledger_matches_path_size_and_modification_time() {
    let (repo, database) = repo().await;
    let watched = folder("/watch");
    repo.add_folder(&watched).await.unwrap();
    let meeting = insert_meeting(&database, "meeting-a").await;
    repo.record(&handled(&watched, sighting("a.mp4"), Some(meeting), None))
        .await
        .unwrap();

    let changed = Sighting {
        size_bytes: 2_000,
        ..sighting("a.mp4")
    };

    assert!(repo.handled(&watched.id, &sighting("a.mp4")).await.unwrap());
    assert!(!repo.handled(&watched.id, &changed).await.unwrap());
    assert!(!repo.handled(&watched.id, &sighting("b.mp4")).await.unwrap());
    // Recording the same file again keeps the first row.
    repo.record(&handled(
        &watched,
        sighting("a.mp4"),
        None,
        Some(ImportFailure::Failed),
    ))
    .await
    .unwrap();
    assert!(repo.failures().await.unwrap().is_empty());
}

#[tokio::test]
async fn failures_list_until_retried_or_dismissed() {
    let (repo, _) = repo().await;
    let watched = folder("/watch");
    repo.add_folder(&watched).await.unwrap();
    let first = handled(
        &watched,
        sighting("a.mov"),
        None,
        Some(ImportFailure::NoAudio),
    );
    let second = handled(
        &watched,
        sighting("b.mov"),
        None,
        Some(ImportFailure::Damaged),
    );
    repo.record(&first).await.unwrap();
    repo.record(&second).await.unwrap();

    assert_eq!(repo.failures().await.unwrap().len(), 2);
    assert_eq!(
        repo.forget_failure(&first.id).await.unwrap(),
        Some(watched.id)
    );
    assert!(repo.dismiss_failure(&second.id, at(ADDED)).await.unwrap());

    assert!(repo.failures().await.unwrap().is_empty());
    assert!(!repo.handled(&watched.id, &sighting("a.mov")).await.unwrap());
    assert!(repo.handled(&watched.id, &sighting("b.mov")).await.unwrap());
}

#[tokio::test]
async fn removing_a_folder_removes_its_ledger() {
    let (repo, _) = repo().await;
    let watched = folder("/watch");
    repo.add_folder(&watched).await.unwrap();
    repo.record(&handled(
        &watched,
        sighting("a.mov"),
        None,
        Some(ImportFailure::Failed),
    ))
    .await
    .unwrap();

    assert!(repo.remove_folder(&watched.id).await.unwrap());

    assert!(repo.failures().await.unwrap().is_empty());
    assert!(!repo.remove_folder(&watched.id).await.unwrap());
}
