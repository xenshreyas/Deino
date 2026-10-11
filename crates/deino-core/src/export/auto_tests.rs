use deino_export::ExportSections;

use super::*;
use crate::export::FakeVault;
use crate::export::testing::{STANDUP, seeded_assembler};
use crate::settings::{FakeSettingsStore, RecordedSettingsEvents};

const VAULT: &str = "/Users/me/Vault";
const NOTE: &str = "/Users/me/Vault/2026/10/2026-10-08 Eng standup.md";

async fn settings(change: impl FnOnce(&mut ExportSettings)) -> Arc<SettingsService> {
    let mut saved = Settings::default();
    saved.export.auto_export = true;
    saved.export.folder = Some(VAULT.to_owned());
    change(&mut saved.export);
    let store = Arc::new(FakeSettingsStore::with(Some(saved)));
    Arc::new(
        SettingsService::load(store, Arc::new(RecordedSettingsEvents::default()))
            .await
            .unwrap(),
    )
}

async fn service(
    vault: Arc<FakeVault>,
    change: impl FnOnce(&mut ExportSettings),
) -> AutoExportService {
    let (_, assembler) = seeded_assembler().await;
    AutoExportService::new(Arc::new(assembler), settings(change).await, vault)
}

fn standup() -> MeetingId {
    MeetingId::new(STANDUP)
}

#[tokio::test]
async fn nothing_is_written_while_auto_export_is_off() {
    let vault = Arc::new(FakeVault::with_folder(VAULT));
    let service = service(vault.clone(), |export| export.auto_export = false).await;

    let outcome = service.export(&standup()).await.unwrap();

    assert_eq!(
        outcome,
        AutoExportOutcome::Skipped(AutoExportSkip::Disabled)
    );
    assert!(vault.paths().is_empty());
}

#[tokio::test]
async fn without_a_folder_or_with_a_missing_one_the_export_fails() {
    let no_folder = service(Arc::new(FakeVault::default()), |export| {
        export.folder = None;
    })
    .await;
    let unplugged = service(Arc::new(FakeVault::default()), |_| {}).await;

    assert!(matches!(
        no_folder.export(&standup()).await,
        Err(ExportError::NoFolder)
    ));
    let error = unplugged.export(&standup()).await.unwrap_err();
    assert!(matches!(error, ExportError::FolderUnavailable(_)));
    assert!(error.is_retryable());
}

#[tokio::test]
async fn the_note_lands_at_the_templated_path_and_names_its_meeting() {
    let vault = Arc::new(FakeVault::with_folder(VAULT));
    let service = service(vault.clone(), |_| {}).await;

    let outcome = service.export(&standup()).await.unwrap();

    assert_eq!(outcome, AutoExportOutcome::Exported { files: 1 });
    let note = vault.read(NOTE).unwrap();
    assert_eq!(owner(&note).as_deref(), Some(STANDUP));
    assert!(note.contains("# Eng standup"));
}

#[tokio::test]
async fn running_again_overwrites_its_own_note_and_nothing_else() {
    let vault = Arc::new(FakeVault::with_folder(VAULT));
    let service = service(vault.clone(), |_| {}).await;
    service.export(&standup()).await.unwrap();
    vault.put(
        NOTE,
        &vault
            .read(NOTE)
            .unwrap()
            .replace("# Eng standup", "# edited"),
    );

    service.export(&standup()).await.unwrap();

    assert_eq!(vault.paths().len(), 1);
    assert!(vault.read(NOTE).unwrap().contains("# Eng standup"));
}

#[tokio::test]
async fn with_overwriting_off_an_exported_note_is_left_as_edited() {
    let vault = Arc::new(FakeVault::with_folder(VAULT));
    let service = service(vault.clone(), |export| export.overwrite = false).await;
    service.export(&standup()).await.unwrap();

    let outcome = service.export(&standup()).await.unwrap();

    assert_eq!(
        outcome,
        AutoExportOutcome::Skipped(AutoExportSkip::AlreadyExported)
    );
}

#[tokio::test]
async fn a_file_that_is_not_the_meetings_is_never_overwritten() {
    let vault = Arc::new(FakeVault::with_folder(VAULT));
    vault.put(NOTE, "# My own standup notes\n");
    let service = service(vault.clone(), |_| {}).await;

    service.export(&standup()).await.unwrap();

    assert_eq!(
        vault.read(NOTE).as_deref(),
        Some("# My own standup notes\n")
    );
    let second = "/Users/me/Vault/2026/10/2026-10-08 Eng standup (2).md";
    assert_eq!(
        owner(&vault.read(second).unwrap()).as_deref(),
        Some(STANDUP)
    );
}

#[tokio::test]
async fn obsidian_mode_links_people_and_gives_each_a_page_once() {
    let vault = Arc::new(FakeVault::with_folder(VAULT));
    vault.put("/Users/me/Vault/People/Maya Chen.md", "# Maya, my notes\n");
    let service = service(vault.clone(), |export| export.obsidian = true).await;

    let outcome = service.export(&standup()).await.unwrap();

    // The note and four new pages: Maya's page was already there.
    assert_eq!(outcome, AutoExportOutcome::Exported { files: 5 });
    assert!(vault.read(NOTE).unwrap().contains("- [[Diego Alvarez]]"));
    assert_eq!(
        vault.read("/Users/me/Vault/People/Maya Chen.md").as_deref(),
        Some("# Maya, my notes\n")
    );
    let page = vault.read("/Users/me/Vault/People/Priya Nair.md").unwrap();
    assert!(page.contains("email: \"priya@deino.dev\""));
}

#[tokio::test]
async fn a_transcript_sidecar_holds_the_transcript_and_is_linked() {
    let vault = Arc::new(FakeVault::with_folder(VAULT));
    let service = service(vault.clone(), |export| {
        export.transcript_sidecar = true;
        export.sections = ExportSections::ALL;
    })
    .await;

    service.export(&standup()).await.unwrap();

    let note = vault.read(NOTE).unwrap();
    let sidecar = vault
        .read("/Users/me/Vault/2026/10/2026-10-08 Eng standup – Transcript.md")
        .unwrap();
    assert!(note.contains("[[2026-10-08 Eng standup – Transcript]]"));
    assert!(!note.contains("Morning, everyone."));
    assert!(sidecar.contains("**Shreyas:** Morning, everyone."));
    assert_eq!(owner(&sidecar).as_deref(), Some(STANDUP));
}

#[tokio::test]
async fn the_chosen_folder_is_saved_in_the_settings() {
    let service = service(Arc::new(FakeVault::default()), |export| {
        export.folder = None;
    })
    .await;
    let dialog =
        crate::export::ScriptedDialog::answering(Some(PathBuf::from("/Users/me/Obsidian")));

    let settings = service.choose_folder(&dialog).await.unwrap().unwrap();

    assert_eq!(
        settings.export.folder.as_deref(),
        Some("/Users/me/Obsidian")
    );
    let cancelled = crate::export::ScriptedDialog::answering(None);
    assert_eq!(service.choose_folder(&cancelled).await.unwrap(), None);
}

/// Exports every fixture meeting into `root` in Obsidian mode, with
/// transcript sidecars and every section.
async fn export_vault(root: &Path) {
    let (_, assembler) = seeded_assembler().await;
    let export = ExportSettings {
        auto_export: true,
        folder: Some(root.to_string_lossy().into_owned()),
        obsidian: true,
        transcript_sidecar: true,
        sections: ExportSections::ALL,
        ..ExportSettings::default()
    };
    let saved = Settings {
        export,
        ..Settings::default()
    };
    let store = Arc::new(FakeSettingsStore::with(Some(saved)));
    let events = Arc::new(RecordedSettingsEvents::default());
    let settings = Arc::new(SettingsService::load(store, events).await.unwrap());
    let assembler = Arc::new(assembler);
    let vault = Arc::new(crate::export::FsVault);
    let service = AutoExportService::new(assembler.clone(), settings, vault);
    for meeting in assembler.meeting_ids().await.unwrap() {
        service.export(&meeting).await.unwrap();
    }
}

/// The EXP-02 acceptance in miniature: a real folder, every fixture, and
/// every `[[wiki link]]` in every note resolves to a note in the vault.
#[tokio::test]
async fn an_exported_vault_has_no_dangling_wiki_links() {
    let root = tempfile::tempdir().unwrap();

    export_vault(root.path()).await;

    let notes = markdown_files(root.path());
    let names: Vec<String> = notes
        .iter()
        .filter_map(|path| path.file_stem())
        .map(|stem| stem.to_string_lossy().into_owned())
        .collect();
    let people = root.path().join(PEOPLE_FOLDER);
    // Eight meeting notes and eight transcript sidecars.
    assert_eq!(
        notes
            .iter()
            .filter(|path| !path.starts_with(&people))
            .count(),
        16
    );
    let links: Vec<(String, &PathBuf)> = notes
        .iter()
        .flat_map(|note| {
            wiki_links(&std::fs::read_to_string(note).unwrap())
                .into_iter()
                .map(move |link| (link, note))
        })
        .collect();
    assert!(links.len() > 50, "{} links", links.len());
    for (link, note) in links {
        assert!(
            names.contains(&link),
            "[[{link}]] in {} resolves",
            note.display()
        );
    }
}

/// Writes the fixture vault into `$DEINO_EXPORT_VAULT` to open in Obsidian:
/// `DEINO_EXPORT_VAULT=/tmp/vault cargo test -p deino-core --lib -- --ignored write_sample_vault`.
#[tokio::test]
#[ignore = "writes a vault for manual inspection"]
async fn write_sample_vault() {
    if let Ok(root) = std::env::var("DEINO_EXPORT_VAULT") {
        std::fs::create_dir_all(&root).unwrap();
        export_vault(Path::new(&root)).await;
    }
}

fn markdown_files(folder: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(folder).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files.extend(markdown_files(&path));
        } else if path.extension().is_some_and(|extension| extension == "md") {
            files.push(path);
        }
    }
    files
}

fn wiki_links(text: &str) -> Vec<String> {
    text.split("[[")
        .skip(1)
        .filter_map(|rest| rest.split_once("]]").map(|(link, _)| link.to_owned()))
        .collect()
}
