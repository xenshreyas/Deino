//! Retention never deletes anything outside the app's folders: not an
//! import's original, not a path that climbs out with `..`, not a clip or a
//! merged part stored elsewhere, not what a symlink in the recordings
//! folder points at, and not another meeting's recordings.

use std::path::{Path, PathBuf};

use deino_domain::{ErrorClass, ErrorCode, MeetingId};
use deino_testkit::at;

use super::RetentionError;
use super::test_world::{SAVED, World, touch, world};
use crate::settings::RetentionPeriod::{Days7, Never};

/// The user's files around the app's folders, all of which must survive.
struct Outside {
    original: PathBuf,
    escape: PathBuf,
    clip: PathBuf,
    part: PathBuf,
    linked: PathBuf,
    neighbour: PathBuf,
}

impl Outside {
    fn all(&self) -> [&Path; 6] {
        [
            &self.original,
            &self.escape,
            &self.clip,
            &self.part,
            &self.linked,
            &self.neighbour,
        ]
    }
}

/// Meeting `m0` with a file inside its recordings folder and every kind of
/// path that leads outside the app's folders; `m1` is still young.
async fn hostile_meeting(world: &World) -> Outside {
    world.meeting("m0").await;
    let outside = Outside {
        original: world.path("Desktop/interview.m4a"),
        escape: world.path("Desktop/escape.m4a"),
        clip: world.path("Desktop/clips/outside.m4a"),
        part: world.path("Desktop/part.m4a"),
        linked: world.path("Desktop/linked/keep.m4a"),
        neighbour: world.path("recordings/m1/mic.m4a"),
    };
    for path in outside.all() {
        touch(path);
    }
    record_paths(world, &outside.original).await;
    world.clip("m0", 1, &outside.clip).await;
    let part = outside.part.display().to_string();
    world
        .exec(
            "INSERT INTO meeting_merge_parts (meeting_id, position, source_meeting_id, title, held_at,
                 offset_ms, duration_ms, audio_path, merged_at)
             VALUES ('m0', 1, '../Desktop', 'Part', ?1, 0, 1000, ?2, ?1)",
            &[SAVED, &part],
        )
        .await;
    link(
        &world.path("Desktop/linked"),
        &world.path("recordings/m0/linked"),
    );
    deino_testkit::seed::meeting(&world.database, "m1", "Later", at("2026-10-07T09:00:00Z")).await;
    world.record("m1", &outside.neighbour).await;
    outside
}

/// `m0`'s recording: the import's `original`, a path that climbs out of
/// the recordings folder with `..`, and a screen recording inside it.
async fn record_paths(world: &World, original: &Path) {
    let climbing = world.path("recordings/../Desktop/escape.m4a");
    let inside = world.path("recordings/m0/screen.mp4");
    touch(&inside);
    let paths = [original, &climbing, &inside].map(|p| p.display().to_string());
    world
        .exec(
            "UPDATE meetings SET audio_path = ?2, audio_mic_path = ?3, video_path = ?4 WHERE id = ?1",
            &["m0", &paths[0], &paths[1], &paths[2]],
        )
        .await;
}

#[cfg(unix)]
fn link(target: &Path, at: &Path) {
    std::os::unix::fs::symlink(target, at).unwrap();
}

#[cfg(not(unix))]
fn link(_target: &Path, _at: &Path) {}

#[tokio::test]
async fn expired_recordings_never_take_files_outside_the_app_folders() {
    let world = world().await;
    world.keep(Days7, Never, Never);
    let outside = hostile_meeting(&world).await;

    world.days_pass(7);
    let outcome = world.service.sweep().await.unwrap();

    assert_eq!(outcome.recordings, vec![MeetingId::new("m0")]);
    assert!(!world.path("recordings/m0").exists(), "its own folder goes");
    for path in outside.all() {
        assert!(path.exists(), "{} was deleted", path.display());
    }
}

#[tokio::test]
async fn expired_meetings_purged_from_the_trash_never_take_files_outside_either() {
    let world = world().await;
    world.keep(Never, Never, Days7);
    let outside = hostile_meeting(&world).await;

    world.days_pass(7);
    world.service.sweep().await.unwrap();
    world.days_pass(31);
    let purged = world.trash.purge_expired().await.unwrap();

    assert_eq!(purged.changed, vec![MeetingId::new("m0")]);
    for path in outside.all() {
        assert!(path.exists(), "{} was deleted", path.display());
    }
}

#[tokio::test]
async fn the_status_of_a_missing_meeting_is_not_found() {
    let world = world().await;

    let error = world
        .service
        .status(&MeetingId::new("missing"))
        .await
        .unwrap_err();

    assert!(matches!(error, RetentionError::MeetingNotFound(_)));
    assert_eq!(error.code(), ErrorCode::NotFound);
}
