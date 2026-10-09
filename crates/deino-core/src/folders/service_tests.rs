//! [`FolderService`] over the fake ports: names, moves, deletion and the tree.

use std::sync::Arc;

use chrono::TimeDelta;
use deino_domain::{FakeClock, FolderId, MeetingId};
use deino_testkit::at;

use super::*;
use crate::meetings::Meeting;

const NOW: &str = "2026-10-09T09:00:00Z";

struct Harness {
    folders: FolderService,
    repo: Arc<FakeFolderRepo>,
    clock: Arc<FakeClock>,
}

fn meeting(id: &str, created_at: &str) -> Meeting {
    Meeting {
        id: MeetingId::new(id),
        title: "Sync".into(),
        created_at: at(created_at),
        updated_at: at(created_at),
    }
}

fn harness() -> Harness {
    let repo = Arc::new(FakeFolderRepo::with_meetings([
        meeting("meeting-a", "2026-10-01T09:00:00Z"),
        meeting("meeting-b", "2026-10-02T09:00:00Z"),
    ]));
    let clock = Arc::new(FakeClock::new(at(NOW)));
    let ports = FolderPorts::over(repo.clone(), clock.clone());
    Harness {
        folders: FolderService::new(&ports),
        repo,
        clock,
    }
}

fn new_folder(name: &str, parent: Option<&Folder>) -> NewFolder {
    NewFolder {
        name: name.into(),
        parent_id: parent.map(|parent| parent.id),
    }
}

impl Harness {
    async fn create(&self, name: &str, parent: Option<&Folder>) -> Folder {
        self.folders
            .create(&new_folder(name, parent))
            .await
            .unwrap()
    }

    async fn file(&self, meeting: &str, folder: &Folder) {
        let at = self.clock.now();
        self.repo
            .file_meeting(&MeetingId::new(meeting), &folder.id, at)
            .await
            .unwrap();
    }
}

fn shape(tree: &FolderTree) -> Vec<(&str, u32, u32)> {
    tree.entries
        .iter()
        .map(|entry| (entry.folder.name.as_str(), entry.depth, entry.meeting_count))
        .collect()
}

#[tokio::test]
async fn create_trims_the_name_and_stamps_the_clock() {
    let harness = harness();

    let folder = harness.create("  Clients ", None).await;

    assert_eq!(folder.name, "Clients");
    assert_eq!((folder.created_at, folder.updated_at), (at(NOW), at(NOW)));
    assert_eq!(harness.folders.get(&folder.id).await.unwrap(), folder);
}

#[tokio::test]
async fn sibling_names_are_unique_ignoring_case_but_not_across_parents() {
    let harness = harness();
    let work = harness.create("Work", None).await;

    let clash = harness.folders.create(&new_folder("work", None)).await;
    let nested = harness
        .folders
        .create(&new_folder("Work", Some(&work)))
        .await;

    assert!(matches!(clash, Err(FolderError::NameTaken)));
    assert!(nested.is_ok());
}

#[tokio::test]
async fn creating_inside_a_missing_parent_is_not_found() {
    let harness = harness();
    let missing = NewFolder {
        name: "Notes".into(),
        parent_id: Some(FolderId::generate(harness.clock.as_ref())),
    };

    let error = harness.folders.create(&missing).await.unwrap_err();

    assert!(matches!(error, FolderError::NotFound(_)));
}

#[tokio::test]
async fn rename_may_change_case_but_not_take_a_siblings_name() {
    let harness = harness();
    let work = harness.create("work", None).await;
    harness.create("Home", None).await;
    harness.clock.advance(TimeDelta::minutes(1));

    let recased = harness.folders.rename(&work.id, "Work").await.unwrap();
    let taken = harness.folders.rename(&work.id, "home").await;

    assert_eq!(
        (recased.name.as_str(), recased.updated_at),
        ("Work", harness.clock.now())
    );
    assert!(matches!(taken, Err(FolderError::NameTaken)));
}

#[tokio::test]
async fn icons_and_descriptions_are_validated_and_can_be_cleared() {
    let harness = harness();
    let work = harness.create("Work", None).await;

    let iconed = harness
        .folders
        .set_icon(&work.id, Some("briefcase"))
        .await
        .unwrap();
    let invalid = harness.folders.set_icon(&work.id, Some("Brief case")).await;
    let cleared = harness.folders.set_icon(&work.id, None).await.unwrap();
    let described = harness
        .folders
        .set_description(&work.id, "  Client work \n")
        .await
        .unwrap();

    assert_eq!(iconed.icon.as_deref(), Some("briefcase"));
    assert!(matches!(invalid, Err(FolderError::InvalidIcon)));
    assert_eq!(cleared.icon, None);
    assert_eq!(described.description, "Client work");
}

#[tokio::test]
async fn moving_a_nested_folder_keeps_its_subfolders_and_their_meetings() {
    let harness = harness();
    let work = harness.create("Work", None).await;
    let clients = harness.create("Clients", Some(&work)).await;
    let acme = harness.create("Acme", Some(&clients)).await;
    let archive = harness.create("Archive", None).await;
    harness.file("meeting-a", &clients).await;
    harness.file("meeting-b", &acme).await;

    let moved = harness
        .folders
        .move_to(&clients.id, Some(&archive.id))
        .await
        .unwrap();

    assert_eq!(moved.parent_id, Some(archive.id));
    assert_eq!(
        shape(&harness.folders.tree().await.unwrap()),
        [
            ("Archive", 0, 0),
            ("Clients", 1, 1),
            ("Acme", 2, 1),
            ("Work", 0, 0)
        ]
    );
    let in_acme = harness.repo.meetings_in(&acme.id).await.unwrap();
    assert_eq!(in_acme.len(), 1);
}

#[tokio::test]
async fn moves_into_the_folders_own_subtree_or_onto_a_name_clash_are_conflicts() {
    let harness = harness();
    let work = harness.create("Work", None).await;
    let clients = harness.create("Clients", Some(&work)).await;
    let home = harness.create("Home", None).await;
    harness.create("Clients", Some(&home)).await;

    let into_child = harness.folders.move_to(&work.id, Some(&clients.id)).await;
    let into_itself = harness.folders.move_to(&work.id, Some(&work.id)).await;
    let clash = harness.folders.move_to(&clients.id, Some(&home.id)).await;
    let to_top = harness.folders.move_to(&clients.id, None).await.unwrap();

    assert!(matches!(into_child, Err(FolderError::WouldCycle)));
    assert!(matches!(into_itself, Err(FolderError::WouldCycle)));
    assert!(matches!(clash, Err(FolderError::NameTaken)));
    assert_eq!(to_top.parent_id, None);
}

#[tokio::test]
async fn deleting_a_folder_removes_its_subtree_and_keeps_every_meeting() {
    let harness = harness();
    let work = harness.create("Work", None).await;
    let clients = harness.create("Clients", Some(&work)).await;
    harness.file("meeting-a", &clients).await;

    harness.folders.delete(&work.id).await.unwrap();

    let tree = harness.folders.tree().await.unwrap();
    assert!(tree.entries.is_empty());
    assert_eq!(tree.unfiled_count, 2);
    assert!(matches!(
        harness.folders.delete(&work.id).await,
        Err(FolderError::NotFound(_))
    ));
}

#[tokio::test]
async fn the_tree_counts_filed_meetings_and_the_unfiled_ones() {
    let harness = harness();
    let work = harness.create("Work", None).await;
    harness.file("meeting-a", &work).await;

    let tree = harness.folders.tree().await.unwrap();

    assert_eq!(shape(&tree), [("Work", 0, 1)]);
    assert_eq!(tree.unfiled_count, 1);
}
