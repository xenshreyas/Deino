//! Circleback against recorded answers of `GET /meetings` (two pages
//! linked by RFC 8288), `GET /meeting/{id}` and its transcript.

mod support;

use std::sync::Arc;

use deino_core::importers::{ImportPhase, ImportWorld};
use deino_importers::CirclebackImporter;
use support::{FixtureClient, Recorded, count_for, run, shared, speakers};

const NEXT: &str = "https://circleback.ai/api/meetings?cursor=eyJwYWdlIjoyfQ";

fn circleback() -> Arc<FixtureClient> {
    let mut client = FixtureClient::default()
        .route(
            "GET /api/meetings",
            Recorded::file("circleback/meetings-page-1.json")
                .header("link", format!("<{NEXT}>; rel=\"next\"")),
        )
        .route(
            "GET /api/meetings?cursor=eyJwYWdlIjoyfQ",
            Recorded::file("circleback/meetings-page-2.json"),
        );
    for (id, meeting, transcript) in [
        ("48211", "circleback/meeting-48211.json", "circleback/transcript-48211.json"),
        ("48230", "circleback/meeting-48230.json", "circleback/transcript-48230.json"),
        ("48302", "circleback/meeting-48302.json", "circleback/transcript-48302.json"),
    ] {
        client = client
            .route(&format!("GET /api/meeting/{id}"), Recorded::file(meeting))
            .route(&format!("GET /api/meeting/{id}/transcript"), Recorded::file(transcript));
    }
    Arc::new(client)
}

#[tokio::test]
async fn every_page_is_imported_with_notes_speakers_people_and_action_items() {
    let world = ImportWorld::new().await;
    let client = circleback();

    let progress = run(&world, &CirclebackImporter::new(shared(&client), "cb_test")).await;

    assert_eq!((progress.phase, progress.imported, progress.failed), (ImportPhase::Finished, 3, 0));
    let title = "Northwind renewal call";
    assert_eq!(speakers(&world, title).await, ["Marcus Webb", "Priya Raman", "Speaker"]);
    assert_eq!(count_for(&world, "transcript_segments", title).await, 5);
    assert_eq!(count_for(&world, "meeting_attendees", title).await, 2);
    assert_eq!(count_for(&world, "action_items", title).await, 2);
    assert_eq!(count_for(&world, "meeting_tags", title).await, 1);
    assert_eq!(count_for(&world, "enhanced_notes", title).await, 1);
    let requests = client.requests();
    assert!(requests.iter().all(|r| r.url.host_str() == Some("circleback.ai")));
    assert!(requests.iter().all(|r| r.headers.get("authorization") == Some("Bearer cb_test")));
}

#[tokio::test]
async fn importing_twice_adds_nothing() {
    let world = ImportWorld::new().await;
    let importer = CirclebackImporter::new(shared(&circleback()), "cb_test");

    run(&world, &importer).await;
    let again = run(&world, &importer).await;

    assert_eq!((again.imported, again.skipped), (0, 3));
    assert_eq!(world.meeting_count().await, 3);
}

#[tokio::test]
async fn imported_meetings_are_searchable() {
    let world = ImportWorld::new().await;
    run(&world, &CirclebackImporter::new(shared(&circleback()), "cb_test")).await;

    let hits = world.search("procurement").await;

    assert!(!hits.is_empty());
    let titles = world.titles().await;
    let renewal = titles.iter().find(|(_, title)| title == "Northwind renewal call").unwrap();
    assert!(hits.iter().any(|hit| hit.meeting_id.as_ref() == Some(&renewal.0)));
}

#[tokio::test]
async fn a_refused_key_fails_the_run_and_imports_nothing() {
    let world = ImportWorld::new().await;
    let client = Arc::new(
        FixtureClient::default().route("GET /api/meetings", Recorded::status(401, "circleback/meetings-page-2.json")),
    );

    let progress = run(&world, &CirclebackImporter::new(shared(&client), "cb_wrong")).await;

    assert_eq!(progress.phase, ImportPhase::Failed);
    assert_eq!(world.meeting_count().await, 0);
}
