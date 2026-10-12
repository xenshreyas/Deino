//! Fireflies (GraphQL `transcripts`) and Fathom (`GET /external/v1/meetings`
//! with everything included) against recorded answers.

mod support;

use std::sync::Arc;

use deino_core::importers::{ImportPhase, ImportWorld};
use deino_importers::{FathomImporter, FirefliesImporter};
use support::{FixtureClient, Recorded, count_for, run, shared, speakers};

const FATHOM_LIST: &str = "GET /external/v1/meetings?include_transcript=true&include_summary=true&include_action_items=true";

fn fireflies() -> Arc<FixtureClient> {
    Arc::new(FixtureClient::default().route(
        "POST /graphql skip=0",
        Recorded::file("fireflies/transcripts-skip-0.json"),
    ))
}

fn fathom() -> Arc<FixtureClient> {
    Arc::new(
        FixtureClient::default()
            .route(FATHOM_LIST, Recorded::file("fathom/meetings-page-1.json"))
            .route(
                &format!("{FATHOM_LIST}&cursor=eyJwYWdlX251bSI6Mn0%3D"),
                Recorded::file("fathom/meetings-page-2.json"),
            ),
    )
}

#[tokio::test]
async fn fireflies_transcripts_summaries_and_owned_action_items_are_imported() {
    let world = ImportWorld::new().await;
    let client = fireflies();

    let progress = run(&world, &FirefliesImporter::new(shared(&client), "ff_test")).await;

    assert_eq!((progress.phase, progress.imported), (ImportPhase::Finished, 2));
    let title = "Pipeline review";
    assert_eq!(speakers(&world, title).await, ["Dana Lee", "Raj Patel"]);
    assert_eq!(count_for(&world, "transcript_segments", title).await, 3);
    assert_eq!(count_for(&world, "meeting_attendees", title).await, 2);
    assert_eq!(count_for(&world, "action_items", title).await, 2);
    assert!(!world.search("Contoso").await.is_empty());
    let request = &client.requests()[0];
    assert_eq!(request.url.host_str(), Some("api.fireflies.ai"));
    assert_eq!(request.headers.get("authorization"), Some("Bearer ff_test"));
}

#[tokio::test]
async fn a_fireflies_auth_error_names_the_key() {
    let world = ImportWorld::new().await;
    let client = Arc::new(
        FixtureClient::default().route("POST /graphql skip=0", Recorded::file("fireflies/auth-error.json")),
    );

    let progress = run(&world, &FirefliesImporter::new(shared(&client), "ff_wrong")).await;

    assert_eq!(progress.phase, ImportPhase::Failed);
    assert!(progress.error.unwrap().contains("API key"));
}

#[tokio::test]
async fn fathom_meetings_are_imported_with_the_recorder_as_me() {
    let world = ImportWorld::new().await;
    let client = fathom();

    let progress = run(&world, &FathomImporter::new(shared(&client), "fathom_test")).await;

    assert_eq!((progress.phase, progress.imported), (ImportPhase::Finished, 3));
    let title = "Kickoff: Beacon migration";
    assert_eq!(speakers(&world, title).await, ["Me", "Morgan Hale"]);
    assert_eq!(count_for(&world, "action_items", title).await, 2);
    assert_eq!(count_for(&world, "meeting_attendees", title).await, 2);
    assert!(!world.search("warehouse").await.is_empty());
    assert!(client.requests().iter().all(|r| {
        r.url.host_str() == Some("api.fathom.ai") && r.headers.get("x-api-key") == Some("fathom_test")
    }));
}

#[tokio::test]
async fn importing_twice_adds_nothing() {
    let world = ImportWorld::new().await;
    let fathom_importer = FathomImporter::new(shared(&fathom()), "fathom_test");
    let fireflies_importer = FirefliesImporter::new(shared(&fireflies()), "ff_test");

    run(&world, &fathom_importer).await;
    run(&world, &fireflies_importer).await;
    let fathom_again = run(&world, &fathom_importer).await;
    let fireflies_again = run(&world, &fireflies_importer).await;

    assert_eq!((fathom_again.imported, fathom_again.skipped), (0, 3));
    assert_eq!((fireflies_again.imported, fireflies_again.skipped), (0, 2));
    assert_eq!(world.meeting_count().await, 5);
}
