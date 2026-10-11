use std::io::Read;
use std::path::Path;
use std::sync::OnceLock;

use deino_domain::FakeClock;
use deino_testkit::at;

use super::*;
use crate::export::ScriptedDialog;
use crate::export::testing::seeded_assembler;
use crate::meetings::SqliteMeetingRepo;

/// Records every status; cancels the export at the first progress when asked.
#[derive(Default)]
struct Events {
    seen: Mutex<Vec<FullExportStatus>>,
    cancel: OnceLock<Arc<FullExportService>>,
}

impl FullExportEvents for Events {
    fn changed(&self, status: &FullExportStatus) {
        self.seen.lock().unwrap().push(status.clone());
        if let Some(service) = self.cancel.get().filter(|_| status.done > 0) {
            service.cancel();
        }
    }
}

async fn service(events: Arc<Events>) -> Arc<FullExportService> {
    let (database, assembler) = seeded_assembler().await;
    Arc::new(FullExportService::new(FullExportDeps {
        assembler: Arc::new(assembler),
        lifecycle: Arc::new(SqliteMeetingRepo::new(database)),
        events,
        clock: Arc::new(FakeClock::new(at("2026-10-09T12:00:00Z"))),
    }))
}

fn entries(zip: &Path) -> Vec<String> {
    let archive = zip::ZipArchive::new(std::fs::File::open(zip).unwrap()).unwrap();
    let mut names: Vec<String> = archive.file_names().map(str::to_owned).collect();
    names.sort();
    names
}

/// How many of `names` have the extension `extension`.
fn count_with(names: &[String], extension: &str) -> usize {
    names
        .iter()
        .filter(|name| {
            Path::new(name)
                .extension()
                .is_some_and(|found| found == extension)
        })
        .count()
}

/// The text of the archive's entry `name`.
fn read_entry(zip: &Path, name: &str) -> String {
    let mut archive = zip::ZipArchive::new(std::fs::File::open(zip).unwrap()).unwrap();
    let mut text = String::new();
    archive
        .by_name(name)
        .unwrap()
        .read_to_string(&mut text)
        .unwrap();
    text
}

#[tokio::test]
async fn every_meeting_lands_in_one_zip_with_the_csv_tables() {
    let folder = tempfile::tempdir().unwrap();
    let target = folder.path().join("account.zip");
    let service = service(Arc::new(Events::default())).await;
    let dialog = ScriptedDialog::answering(Some(target.clone()));

    assert!(service.start(&dialog, false).await.unwrap());
    service.until_idle().await;

    assert_eq!(
        dialog.requests()[0].file_name,
        "Deino export 2026-10-09.zip"
    );
    let names = entries(&target);
    assert_eq!(
        (count_with(&names, "md"), count_with(&names, "json")),
        (8, 8)
    );
    for table in ["meetings.csv", "action_items.csv", "people.csv"] {
        assert!(names.contains(&format!("deino-export/{table}")), "{table}");
    }
    assert!(names.contains(&"deino-export/meetings/2026/10/2026-10-08 Eng standup.md".to_owned()));
    let meetings = read_entry(&target, "deino-export/meetings.csv");
    assert_eq!(meetings.lines().count(), 9);
}

#[tokio::test]
async fn progress_climbs_to_a_finished_status_naming_the_archive() {
    let folder = tempfile::tempdir().unwrap();
    let events = Arc::new(Events::default());
    let service = service(events.clone()).await;

    let dialog = ScriptedDialog::answering(Some(folder.path().join("account.zip")));
    service.start(&dialog, false).await.unwrap();
    service.until_idle().await;

    let last = service.status();
    assert_eq!(
        (last.state, last.done, last.total),
        (FullExportState::Finished, 8, 8)
    );
    assert_eq!(last.file_name.as_deref(), Some("account.zip"));
    let progress: Vec<u32> = events
        .seen
        .lock()
        .unwrap()
        .iter()
        .map(|status| status.done)
        .collect();
    assert!(progress.is_sorted(), "{progress:?}");
}

#[tokio::test]
async fn a_cancelled_export_leaves_no_file_behind() {
    let folder = tempfile::tempdir().unwrap();
    let target = folder.path().join("account.zip");
    let events = Arc::new(Events::default());
    let service = service(events.clone()).await;
    events.cancel.set(Arc::clone(&service)).ok();

    service
        .start(&ScriptedDialog::answering(Some(target.clone())), false)
        .await
        .unwrap();
    service.until_idle().await;

    assert_eq!(service.status().state, FullExportState::Cancelled);
    assert_eq!(std::fs::read_dir(folder.path()).unwrap().count(), 0);
}

#[tokio::test]
async fn one_export_runs_at_a_time_and_a_cancelled_dialog_starts_none() {
    let folder = tempfile::tempdir().unwrap();
    let service = service(Arc::new(Events::default())).await;

    assert!(
        !service
            .start(&ScriptedDialog::answering(None), false)
            .await
            .unwrap()
    );
    assert_eq!(service.status().state, FullExportState::Idle);

    let dialog = ScriptedDialog::answering(Some(folder.path().join("a.zip")));
    service.start(&dialog, false).await.unwrap();
    let second = service.start(&dialog, false).await;

    assert!(matches!(second, Err(ExportError::Busy)));
    service.until_idle().await;
}
