//! Granola against recorded answers of `GET /v1/notes` (two pages by
//! cursor) and `GET /v1/notes/{id}?include=transcript`, including a note
//! whose transcript is too large to inline (413).

mod support;

use std::sync::Arc;

use deino_core::importers::{ImportPhase, ImportWorld};
use deino_importers::GranolaImporter;
use support::{FixtureClient, Recorded, count_for, run, shared, speakers};

const CURSOR: &str = "eyJjcmVhdGVkX2F0IjoiMjAyNi0wOC0wNVQxMDozMDowMFoifQ%3D%3D";

fn granola() -> Arc<FixtureClient> {
    let client = FixtureClient::default()
        .route("GET /v1/notes?page_size=30", Recorded::file("granola/notes-page-1.json"))
        .route(
            &format!("GET /v1/notes?page_size=30&cursor={CURSOR}"),
            Recorded::file("granola/notes-page-2.json"),
        )
        .route(
            "GET /v1/notes/not_1d3tmYTlCICgjy?include=transcript",
            Recorded::file("granola/note-not_1d3tmYTlCICgjy.json"),
        )
        .route(
            "GET /v1/notes/not_8sKq2LmP0aXwBz?include=transcript",
            Recorded::file("granola/note-not_8sKq2LmP0aXwBz.json"),
        )
        .route(
            "GET /v1/notes/not_Zr4vT9nQ1cHbYe?include=transcript",
            Recorded::status(413, "granola/transcript-too-large.json"),
        )
        .route(
            "GET /v1/notes/not_Zr4vT9nQ1cHbYe",
            Recorded::file("granola/note-not_Zr4vT9nQ1cHbYe.json"),
        );
    Arc::new(client)
}

#[tokio::test]
async fn notes_transcripts_and_attendees_are_imported() {
    let world = ImportWorld::new().await;
    let client = granola();

    let progress = run(&world, &GranolaImporter::new(shared(&client), "grn_test")).await;

    assert_eq!((progress.phase, progress.imported, progress.failed), (ImportPhase::Finished, 3, 0));
    let title = "Quarterly planning with design";
    // The microphone is the user; others keep Granola's name or label.
    assert_eq!(speakers(&world, title).await, ["Ines Duarte", "Me", "Speaker B"]);
    assert_eq!(count_for(&world, "transcript_segments", title).await, 3);
    assert_eq!(count_for(&world, "meeting_attendees", title).await, 2);
    let organisers: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM meeting_attendees WHERE role = 'organizer'",
    )
    .fetch_one(world.database.reader())
    .await
    .unwrap();
    assert_eq!(organisers, 1);
    assert!(client.requests().iter().all(|r| {
        r.url.host_str() == Some("public-api.granola.ai")
            && r.headers.get("authorization") == Some("Bearer grn_test")
    }));
}

#[tokio::test]
async fn a_transcript_too_large_to_inline_still_imports_the_note() {
    let world = ImportWorld::new().await;

    run(&world, &GranolaImporter::new(shared(&granola()), "grn_test")).await;

    let title = "All-hands rehearsal";
    assert_eq!(count_for(&world, "enhanced_notes", title).await, 1);
    assert_eq!(count_for(&world, "transcript_segments", title).await, 0);
    assert!(!world.search("customer story").await.is_empty());
}

#[tokio::test]
async fn importing_twice_adds_nothing() {
    let world = ImportWorld::new().await;
    let importer = GranolaImporter::new(shared(&granola()), "grn_test");

    run(&world, &importer).await;
    let again = run(&world, &importer).await;

    assert_eq!((again.imported, again.skipped), (0, 3));
    assert_eq!(world.meeting_count().await, 3);
}

#[tokio::test]
async fn the_private_notes_follow_the_summary_and_are_searchable() {
    let world = ImportWorld::new().await;

    run(&world, &GranolaImporter::new(shared(&granola()), "grn_test")).await;

    assert!(!world.search("contractor budget").await.is_empty());
    assert!(!world.search("design system").await.is_empty());
}
