//! [`TrashService`] over the real schema and [`TrashBin`]: batch trash and
//! restore (which restores everything), delete for good with files, and
//! the 30-day purge.

use std::io;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::TimeDelta;
use deino_db::test_db;
use deino_domain::{FakeClock, MeetingId};
use deino_testkit::{at, seed};

use super::*;
use crate::meetings::test_seed as rows;
use crate::meetings::{MeetingRepo, RecordedWork, SqliteMeetingRepo};

/// Records which files would be deleted.
#[derive(Default)]
struct RecordedFiles(Mutex<Vec<StoredFiles>>);

#[async_trait]
impl MeetingFiles for RecordedFiles {
    async fn delete(&self, files: &StoredFiles) -> io::Result<()> {
        self.0.lock().unwrap().push(files.clone());
        Ok(())
    }
}

struct World {
    trash: TrashService,
    repo: SqliteMeetingRepo,
    files: Arc<RecordedFiles>,
    clock: Arc<FakeClock>,
    work: Arc<RecordedWork>,
}

async fn world(ids: &[&str]) -> World {
    let database = test_db().await;
    for id in ids {
        seed::meeting(&database, id, "Standup", at("2026-10-01T09:00:00Z")).await;
    }
    let repo = SqliteMeetingRepo::new(database);
    let files = Arc::new(RecordedFiles::default());
    let clock = Arc::new(FakeClock::new(at("2026-10-05T09:00:00Z")));
    let work = Arc::new(RecordedWork::default());
    let trash = TrashService::new(TrashDeps {
        bin: Arc::new(repo.clone()),
        files: files.clone(),
        clock: clock.clone(),
        work: work.clone(),
    });
    World {
        trash,
        repo,
        files,
        clock,
        work,
    }
}

fn ids(names: &[&str]) -> Vec<MeetingId> {
    names.iter().copied().map(MeetingId::new).collect()
}

async fn count(repo: &SqliteMeetingRepo, table: &str) -> i64 {
    sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {table}"))
        .fetch_one(repo.database.reader())
        .await
        .unwrap()
}

#[tokio::test]
async fn trashing_hides_meetings_everywhere_and_restoring_brings_back_everything() {
    let world = world(&["m0", "m1"]).await;
    let db = &world.repo.database;
    let person = rows::person(db, 1, None, None).await;
    rows::attend(db, "m0", &person, "invitee").await;
    let tag = rows::tag(db, 20, "Customer").await;
    rows::tag_meeting(db, "m0", &tag).await;
    rows::segment(db, 30, "m0", (0, 1_000, None)).await;
    seed::notes(db, "m0", "- ship it").await;

    let trashed = world
        .trash
        .trash(&ids(&["m0", "m0", "missing"]))
        .await
        .unwrap();
    assert_eq!(trashed.changed, ids(&["m0"]));
    assert_eq!(world.work.cancelled(), ids(&["m0"]), "its pipeline stops");
    assert_eq!(world.repo.find(&MeetingId::new("m0")).await.unwrap(), None);
    assert_eq!(
        world.trash.list().await.unwrap()[0].purges_at,
        at("2026-11-04T09:00:00Z")
    );

    let restored = world.trash.restore(&ids(&["m0", "m1"])).await.unwrap();
    assert_eq!(restored.changed, ids(&["m0"]), "m1 was never in the trash");
    assert!(
        world
            .repo
            .find(&MeetingId::new("m0"))
            .await
            .unwrap()
            .is_some()
    );
    for table in [
        "meeting_attendees",
        "meeting_tags",
        "transcript_segments",
        "meeting_notes",
    ] {
        assert_eq!(count(&world.repo, table).await, 1, "{table} came back");
    }
}

#[tokio::test]
async fn deleting_for_good_skips_live_meetings_and_deletes_the_files_of_every_part() {
    let world = world(&["m0", "m1"]).await;
    let db = &world.repo.database;
    rows::set(
        db,
        "m0",
        ("audio_mic_path", Some("/data/recordings/m0/mic.m4a")),
    )
    .await;
    sqlx::query(
        "INSERT INTO meeting_merge_parts (meeting_id, position, source_meeting_id, title, held_at,
             offset_ms, duration_ms, audio_mic_path, merged_at)
         VALUES ('m0', 1, 'gone', 'Earlier', '2026-10-01T09:00:00Z', 1000, 1000,
             '/data/recordings/gone/mic.m4a', '2026-10-02T09:00:00Z')",
    )
    .execute(db.writer())
    .await
    .unwrap();
    world.trash.trash(&ids(&["m0"])).await.unwrap();

    let deleted = world
        .trash
        .delete_forever(&ids(&["m0", "m1"]))
        .await
        .unwrap();

    assert_eq!(deleted.changed, ids(&["m0"]), "m1 is not in the trash");
    let owners: Vec<String> = world
        .files
        .0
        .lock()
        .unwrap()
        .iter()
        .map(|f| f.owner.to_string())
        .collect();
    assert_eq!(owners, ["m0", "gone"]);
    assert_eq!(count(&world.repo, "meeting_merge_parts").await, 0);
    assert_eq!(count(&world.repo, "meetings").await, 1);
}

#[tokio::test]
async fn the_purge_deletes_only_meetings_trashed_over_thirty_days_ago() {
    let world = world(&["old", "recent"]).await;
    world.trash.trash(&ids(&["old"])).await.unwrap();
    world.clock.advance(TimeDelta::days(10));
    world.trash.trash(&ids(&["recent"])).await.unwrap();
    world.clock.advance(TimeDelta::days(21));

    let purged = world.trash.purge_expired().await.unwrap();
    let again = world.trash.purge_expired().await.unwrap();

    assert_eq!(purged.changed, ids(&["old"]));
    assert!(again.changed.is_empty());
    let left: Vec<String> = world
        .trash
        .list()
        .await
        .unwrap()
        .into_iter()
        .map(|m| m.id.to_string())
        .collect();
    assert_eq!(left, ["recent"]);
}

#[tokio::test]
async fn deleting_the_sample_for_good_also_deletes_its_action_items() {
    let sample = crate::sample::SAMPLE_MEETING_ID;
    let world = world(&[sample, "real"]).await;
    let db = &world.repo.database;
    rows::action_item(db, 1, sample).await;
    rows::action_item(db, 2, "real").await;
    world.trash.trash(&ids(&[sample, "real"])).await.unwrap();

    world
        .trash
        .delete_forever(&ids(&[sample, "real"]))
        .await
        .unwrap();

    let left: Vec<Option<String>> = sqlx::query_scalar("SELECT meeting_id FROM action_items")
        .fetch_all(db.reader())
        .await
        .unwrap();
    assert_eq!(left, [None], "the real meeting's item outlives it");
}
