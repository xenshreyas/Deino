use std::path::PathBuf;
use std::sync::Arc;

use chrono::TimeDelta;
use deino_domain::{FakeClock, ImportFailure};
use deino_testkit::at;

use super::*;
use crate::import::ImportError;
use crate::watch::{FakeImporter, FakeWatchFs, FakeWatchRepo};

const FOLDER: &str = "/Users/me/Recordings";

struct World {
    service: WatchService,
    fs: Arc<FakeWatchFs>,
    repo: Arc<FakeWatchRepo>,
    importer: Arc<FakeImporter>,
    clock: Arc<FakeClock>,
}

fn world() -> World {
    let fs = Arc::new(FakeWatchFs::default());
    fs.add_folder(Path::new(FOLDER));
    let repo = Arc::new(FakeWatchRepo::default());
    let importer = Arc::new(FakeImporter::default());
    let clock = Arc::new(FakeClock::new(at("2026-10-12T10:00:00Z")));
    let service = WatchService::new(WatchDeps {
        repo: repo.clone(),
        fs: fs.clone(),
        importer: importer.clone(),
        clock: clock.clone(),
    });
    World {
        service,
        fs,
        repo,
        importer,
        clock,
    }
}

fn file(name: &str) -> PathBuf {
    Path::new(FOLDER).join(name)
}

impl World {
    /// Scans now and again `seconds` later.
    async fn scan_over(&self, seconds: i64) {
        self.service.scan().await.unwrap();
        self.clock.advance(TimeDelta::seconds(seconds));
        self.service.scan().await.unwrap();
    }

    fn imported(&self) -> Vec<PathBuf> {
        self.importer.imported.lock().unwrap().clone()
    }
}

#[tokio::test]
async fn a_new_file_imports_once_it_has_been_still_for_ten_seconds() {
    let world = world();
    world.service.add_folder(Path::new(FOLDER)).await.unwrap();
    world.fs.put(&file("call.mp4"), 100, 1);

    world.scan_over(9).await;
    assert!(world.imported().is_empty());
    world.clock.advance(TimeDelta::seconds(1));
    world.service.scan().await.unwrap();

    assert_eq!(world.imported(), [file("call.mp4")]);
}

#[tokio::test]
async fn a_file_is_never_imported_twice_even_after_a_restart() {
    let world = world();
    world.service.add_folder(Path::new(FOLDER)).await.unwrap();
    world.fs.put(&file("call.mp4"), 100, 1);
    world.scan_over(10).await;

    // A fresh service over the same ledger, as after relaunching the app.
    let restarted = WatchService::new(WatchDeps {
        repo: world.repo.clone(),
        fs: world.fs.clone(),
        importer: world.importer.clone(),
        clock: world.clock.clone(),
    });
    restarted.scan().await.unwrap();
    world.clock.advance(TimeDelta::seconds(10));
    restarted.scan().await.unwrap();

    assert_eq!(world.imported().len(), 1);
}

#[tokio::test]
async fn files_already_in_a_folder_when_it_is_added_are_left_alone() {
    let world = world();
    world.fs.put(&file("old.m4a"), 10, 1);

    world.service.add_folder(Path::new(FOLDER)).await.unwrap();
    world.scan_over(10).await;

    assert!(world.imported().is_empty());
}

#[tokio::test]
async fn an_imported_file_moves_to_processed_when_the_folder_asks() {
    let world = world();
    let folder = world.service.add_folder(Path::new(FOLDER)).await.unwrap();
    world
        .service
        .set_move_to_processed(&folder.id, true)
        .await
        .unwrap();
    world.fs.put(&file("call.mp4"), 100, 1);

    world.scan_over(10).await;

    assert_eq!(*world.fs.moved.lock().unwrap(), [file("call.mp4")]);
}

#[tokio::test]
async fn a_failure_is_listed_stays_put_and_imports_again_on_retry() {
    let world = world();
    world.service.add_folder(Path::new(FOLDER)).await.unwrap();
    world
        .importer
        .fail_on(file("broken.mov"), || ImportError::NoAudio);
    world.fs.put(&file("broken.mov"), 5, 1);
    world.scan_over(10).await;

    let failures = world.service.failures().await.unwrap();
    assert_eq!(failures.len(), 1);
    assert_eq!(failures[0].failure, ImportFailure::NoAudio);
    assert!(world.fs.moved.lock().unwrap().is_empty());

    world.importer.heal(&file("broken.mov"));
    world.service.retry(&failures[0].id).await.unwrap();
    world.scan_over(10).await;

    assert_eq!(world.imported().len(), 2);
    assert!(world.service.failures().await.unwrap().is_empty());
}

#[tokio::test]
async fn a_dismissed_failure_leaves_the_list_and_is_not_retried() {
    let world = world();
    world.service.add_folder(Path::new(FOLDER)).await.unwrap();
    world
        .importer
        .fail_on(file("broken.mov"), || ImportError::UnsupportedFormat);
    world.fs.put(&file("broken.mov"), 5, 1);
    world.scan_over(10).await;
    let failure = world.service.failures().await.unwrap().remove(0);

    world.service.dismiss(&failure.id).await.unwrap();
    world.scan_over(10).await;

    assert!(world.service.failures().await.unwrap().is_empty());
    assert_eq!(world.imported().len(), 1);
}

#[tokio::test]
async fn adding_a_missing_or_watched_folder_is_refused() {
    let world = world();
    world.service.add_folder(Path::new(FOLDER)).await.unwrap();

    let again = world
        .service
        .add_folder(Path::new(FOLDER))
        .await
        .unwrap_err();
    let missing = world
        .service
        .add_folder(Path::new("/nope"))
        .await
        .unwrap_err();

    assert!(matches!(again, WatchError::AlreadyWatched), "{again:?}");
    assert!(matches!(missing, WatchError::NotAFolder), "{missing:?}");
}

#[tokio::test]
async fn removing_a_folder_stops_its_imports() {
    let world = world();
    let folder = world.service.add_folder(Path::new(FOLDER)).await.unwrap();
    world.fs.put(&file("call.mp4"), 100, 1);
    world.service.scan().await.unwrap();

    world.service.remove_folder(&folder.id).await.unwrap();
    world.clock.advance(TimeDelta::seconds(10));
    world.service.scan().await.unwrap();

    assert!(world.imported().is_empty());
    assert!(world.service.folders().await.unwrap().is_empty());
}
