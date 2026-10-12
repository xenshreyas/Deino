use std::path::PathBuf;
use std::sync::Arc;

use super::*;
use crate::export::testing::{STANDUP, seeded_assembler};
use crate::export::{
    ExportServiceDeps, FakeVault, RecordedClipboard, RecordedOpener, ScriptedDialog,
};

const PLANNING: &str = "meeting-fixture-planning";
const FOLDER: &str = "/Users/me/Exports";

async fn service(vault: Arc<FakeVault>) -> ExportService {
    let (_, assembler) = seeded_assembler().await;
    ExportService::new(ExportServiceDeps {
        assembler: Arc::new(assembler),
        vault,
        save_dialog: Arc::new(ScriptedDialog::answering(None)),
        clipboard: Arc::new(RecordedClipboard::default()),
        opener: Arc::new(RecordedOpener::default()),
    })
}

fn picking(folder: Option<&str>) -> ScriptedDialog {
    ScriptedDialog::answering(folder.map(PathBuf::from))
}

fn ids(names: &[&str]) -> Vec<MeetingId> {
    names.iter().copied().map(MeetingId::new).collect()
}

#[tokio::test]
async fn every_meeting_lands_in_the_chosen_folder_under_its_suggested_name() {
    let vault = Arc::new(FakeVault::default());
    let service = service(vault.clone()).await;

    let saved = service
        .save_many(
            &ids(&[STANDUP, PLANNING]),
            ExportFormat::Markdown,
            &picking(Some(FOLDER)),
        )
        .await
        .unwrap();

    assert_eq!(
        saved,
        Some(SavedExports {
            count: 2,
            folder: "Exports".into()
        })
    );
    let paths = vault.paths();
    assert_eq!(paths.len(), 2);
    assert!(paths.iter().all(|path| path.starts_with(FOLDER)));
    let standup = vault
        .read(format!("{FOLDER}/2026-10-08 Eng standup.md"))
        .unwrap_or_default();
    assert!(standup.starts_with("---"), "Markdown with frontmatter");
}

#[tokio::test]
async fn taken_names_are_numbered_and_the_users_files_are_never_overwritten() {
    let vault = Arc::new(FakeVault::default());
    let existing = format!("{FOLDER}/2026-10-08 Eng standup.md");
    vault.put(&existing, "mine");
    let service = service(vault.clone()).await;

    service
        .save_many(
            &ids(&[STANDUP, STANDUP]),
            ExportFormat::Markdown,
            &picking(Some(FOLDER)),
        )
        .await
        .unwrap();

    assert_eq!(vault.read(&existing).as_deref(), Some("mine"));
    assert!(
        vault
            .read(format!("{FOLDER}/2026-10-08 Eng standup (2).md"))
            .is_some()
    );
    assert!(
        vault
            .read(format!("{FOLDER}/2026-10-08 Eng standup (3).md"))
            .is_some()
    );
}

#[tokio::test]
async fn a_cancelled_folder_dialog_writes_nothing() {
    let vault = Arc::new(FakeVault::default());
    let service = service(vault.clone()).await;

    let saved = service
        .save_many(&ids(&[STANDUP]), ExportFormat::Pdf, &picking(None))
        .await
        .unwrap();

    assert_eq!(saved, None);
    assert!(vault.paths().is_empty());
}

#[tokio::test]
async fn an_unknown_meeting_stops_the_batch() {
    let vault = Arc::new(FakeVault::default());
    let service = service(vault.clone()).await;

    let error = service
        .save_many(
            &ids(&[STANDUP, "missing"]),
            ExportFormat::Text,
            &picking(Some(FOLDER)),
        )
        .await
        .unwrap_err();

    assert!(matches!(error, ExportError::NotFound(_)));
    assert_eq!(vault.paths().len(), 1, "the files already written stay");
}

#[test]
fn numbered_names_keep_their_extension() {
    assert_eq!(numbered("Standup.md", 1), "Standup.md");
    assert_eq!(numbered("Standup.md", 2), "Standup (2).md");
    assert_eq!(numbered("Standup", 3), "Standup (3)");
}
