//! A folder of exported meetings: JSON in the local API's import shape and
//! Markdown notes; other files are ignored and a broken file is counted.

mod support;

use deino_core::importers::{ImportPhase, ImportWorld};
use deino_importers::FolderImporter;
use support::{count_for, fixtures_dir, run, speakers};

fn folder() -> FolderImporter {
    FolderImporter::new(fixtures_dir().join("folder"))
}

#[tokio::test]
async fn json_and_markdown_files_become_meetings() {
    let world = ImportWorld::new().await;

    let progress = run(&world, &folder()).await;

    assert_eq!(
        (progress.phase, progress.total, progress.imported, progress.failed),
        (ImportPhase::Finished, 3, 2, 1)
    );
    let titles: Vec<String> = world.titles().await.into_iter().map(|(_, title)| title).collect();
    assert_eq!(titles, ["Vendor call: Acme logistics", "Board prep"]);
    assert_eq!(speakers(&world, "Vendor call: Acme logistics").await, ["Me", "Rosa Diaz"]);
    assert_eq!(count_for(&world, "meeting_attendees", "Vendor call: Acme logistics").await, 1);
    assert!(!world.search("retention cohort").await.is_empty());
}

#[tokio::test]
async fn importing_twice_adds_nothing_but_retries_the_broken_file() {
    let world = ImportWorld::new().await;

    run(&world, &folder()).await;
    let again = run(&world, &folder()).await;

    assert_eq!((again.imported, again.skipped, again.failed), (0, 2, 1));
    assert_eq!(world.meeting_count().await, 2);
}

#[tokio::test]
async fn a_missing_folder_fails_the_run() {
    let world = ImportWorld::new().await;

    let progress = run(&world, &FolderImporter::new(fixtures_dir().join("absent"))).await;

    assert_eq!(progress.phase, ImportPhase::Failed);
}
