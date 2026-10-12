//! [`RetentionService::sweep`] on a simulated clock over the real schema,
//! trash and files: each period deletes exactly its part, on the day it
//! expires, once.

use deino_domain::MeetingId;
use deino_testkit::{at, seed};

use super::test_world::{SAVED, world};
use crate::meetings::{MeetingRepo, SqliteMeetingRepo};
use crate::settings::RetentionPeriod::{Days7, Days30, Days90, Never};

fn ids(names: &[&str]) -> Vec<MeetingId> {
    names.iter().copied().map(MeetingId::new).collect()
}

#[tokio::test]
async fn nothing_is_deleted_while_every_period_is_never() {
    let world = world().await;
    world.meeting("m0").await;
    world.line("m0", 1, "hello").await;
    world
        .record("m0", &world.path("recordings/m0/mic.m4a"))
        .await;
    world.days_pass(3_650);

    let outcome = world.service.sweep().await.unwrap();

    assert!(outcome.is_empty());
    assert!(world.path("recordings/m0/mic.m4a").exists());
}

#[tokio::test]
async fn recordings_and_clips_go_on_the_day_they_expire_and_the_notes_stay() {
    let world = world().await;
    world.keep(Days7, Days30, Never);
    let m0 = world.meeting("m0").await;
    world.line("m0", 1, "hello").await;
    seed::notes(&world.database, "m0", "- ship it").await;
    let track = world.path("recordings/m0/mic.m4a");
    let clip = world.path("recordings/m0/clips/intro.m4a");
    let image = world.path("meetings/m0/images/a.png");
    world.record("m0", &track).await;
    world.clip("m0", 1, &clip).await;
    super::test_world::touch(&image);

    world.days_pass(6);
    assert!(world.service.sweep().await.unwrap().is_empty(), "day 6");
    world.days_pass(1);
    let outcome = world.service.sweep().await.unwrap();

    assert_eq!(outcome.recordings, ids(&["m0"]));
    assert!(outcome.transcripts.is_empty(), "the transcript has 30 days");
    assert!(!track.exists() && !clip.exists());
    assert!(!world.path("recordings/m0").exists());
    assert!(image.exists(), "note images are notes");
    assert_eq!(world.count("SELECT count(*) FROM clips").await, 0);
    assert_eq!(
        world
            .count("SELECT count(*) FROM meetings WHERE audio_mic_path IS NULL")
            .await,
        1
    );
    let status = world.service.status(&m0).await.unwrap();
    assert_eq!(status.audio_deleted_at, Some(at("2026-10-08T09:00:00Z")));
    assert_eq!(status.transcript_deleted_at, None);
    assert!(world.service.sweep().await.unwrap().is_empty(), "once only");
}

#[tokio::test]
async fn transcripts_go_with_their_search_entries_and_the_notes_stay() {
    let world = world().await;
    world.keep(Never, Days30, Never);
    let m0 = world.meeting("m0").await;
    world.line("m0", 1, "the launch slips").await;
    world.line("m0", 2, "to friday").await;
    seed::notes(&world.database, "m0", "- launch on friday").await;
    world.embed("m0", ("note", "m0"), "launch on friday").await;

    world.days_pass(30);
    let outcome = world.service.sweep().await.unwrap();

    assert_eq!(outcome.transcripts, ids(&["m0"]));
    assert_eq!(
        world
            .count("SELECT count(*) FROM transcript_segments")
            .await,
        0
    );
    let docs = "SELECT count(*) FROM search_documents WHERE kind = ";
    assert_eq!(world.count(&format!("{docs}'segment'")).await, 0);
    assert_eq!(world.count(&format!("{docs}'note'")).await, 1);
    assert_eq!(
        world
            .count("SELECT count(*) FROM embeddings WHERE ref_kind = 'transcript'")
            .await,
        0
    );
    assert_eq!(world.count("SELECT count(*) FROM embeddings").await, 1);
    assert_eq!(world.count("SELECT count(*) FROM meeting_notes").await, 1);
    assert_eq!(world.work.reindexed(), ids(&["m0"]), "semantic index too");
    let status = world.service.status(&m0).await.unwrap();
    assert_eq!(
        status.transcript_deleted_at,
        Some(at("2026-10-31T09:00:00Z"))
    );
}

#[tokio::test]
async fn everything_moves_meetings_to_the_trash_and_the_purge_deletes_them() {
    let world = world().await;
    world.keep(Never, Never, Days90);
    world.meeting("m0").await;
    let track = world.path("recordings/m0/mic.m4a");
    world.record("m0", &track).await;
    let repo = SqliteMeetingRepo::new(world.database.clone());

    world.days_pass(90);
    let outcome = world.service.sweep().await.unwrap();

    assert_eq!(outcome.trashed, ids(&["m0"]));
    assert_eq!(world.work.cancelled(), ids(&["m0"]));
    assert_eq!(repo.find(&MeetingId::new("m0")).await.unwrap(), None);
    assert!(track.exists(), "the trash keeps it for 30 days");
    assert!(world.service.sweep().await.unwrap().is_empty());

    world.days_pass(31);
    let purged = world.trash.purge_expired().await.unwrap();

    assert_eq!(purged.changed, ids(&["m0"]));
    assert!(!track.exists());
}

#[tokio::test]
async fn meetings_in_progress_or_in_the_trash_are_left_alone() {
    let world = world().await;
    world.keep(Days7, Days7, Days7);
    for (n, (id, status)) in [(1, ("live", "recording")), (3, ("busy", "processing"))] {
        world.meeting(id).await;
        world
            .exec(
                "UPDATE meetings SET status = ?2 WHERE id = ?1",
                &[id, status],
            )
            .await;
        world.line(id, n, "still talking").await;
    }
    world.meeting("binned").await;
    world.line("binned", 2, "old").await;
    world
        .exec(
            "UPDATE meetings SET deleted_at = ?2 WHERE id = ?1",
            &["binned", SAVED],
        )
        .await;

    world.days_pass(400);
    let outcome = world.service.sweep().await.unwrap();

    assert!(outcome.is_empty(), "{outcome:?}");
    assert_eq!(
        world
            .count("SELECT count(*) FROM transcript_segments")
            .await,
        3
    );
}

#[tokio::test]
async fn a_longer_period_leaves_younger_meetings() {
    let world = world().await;
    world.keep(Never, Days30, Never);
    world.meeting("old").await;
    world.line("old", 1, "old").await;
    seed::meeting(
        &world.database,
        "new",
        "Standup",
        at("2026-10-20T09:00:00Z"),
    )
    .await;
    world.line("new", 2, "new").await;

    world.days_pass(31);
    let outcome = world.service.sweep().await.unwrap();

    assert_eq!(outcome.transcripts, ids(&["old"]));
    assert_eq!(
        world
            .count("SELECT count(*) FROM transcript_segments")
            .await,
        1
    );
}
