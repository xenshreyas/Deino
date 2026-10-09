//! The folder ports' contract, run against the SQLite repository and the
//! fake, plus what only the real schema can check.

use deino_db::test_db;
use deino_domain::{FakeClock, FolderId, MeetingId};
use deino_testkit::{at, seed};

use super::*;
use crate::meetings::Meeting;
use crate::testing::query_plan;

const NOW: &str = "2026-10-08T09:00:00Z";

fn folder(name: &str, parent: Option<&Folder>) -> Folder {
    Folder {
        id: FolderId::generate(&FakeClock::new(at(NOW))),
        parent_id: parent.map(|parent| parent.id),
        name: name.into(),
        icon: None,
        description: String::new(),
        created_at: at(NOW),
        updated_at: at(NOW),
    }
}

fn meetings() -> [Meeting; 2] {
    ["2026-10-01T09:00:00Z", "2026-10-02T09:00:00Z"].map(|created_at| Meeting {
        id: MeetingId::new(format!("meeting-{created_at}")),
        title: "Standup".into(),
        created_at: at(created_at),
        updated_at: at(created_at),
    })
}

async fn sqlite_repo() -> SqliteFolderRepo {
    let database = test_db().await;
    for meeting in meetings() {
        seed::meeting(
            &database,
            meeting.id.as_str(),
            &meeting.title,
            meeting.created_at,
        )
        .await;
    }
    SqliteFolderRepo::new(database)
}

fn names(folders: &[Folder]) -> Vec<&str> {
    folders.iter().map(|folder| folder.name.as_str()).collect()
}

/// `work/Clients/Acme` and `Home`, as stored by [`tree`].
struct Tree {
    work: Folder,
    clients: Folder,
    acme: Folder,
    home: Folder,
}

async fn tree(repo: &impl FolderRepo) -> Tree {
    let work = folder("work", None);
    let clients = folder("Clients", Some(&work));
    let acme = folder("Acme", Some(&clients));
    let home = folder("Home", None);
    for created in [&work, &clients, &acme, &home] {
        repo.create_folder(created).await.unwrap();
    }
    Tree {
        work,
        clients,
        acme,
        home,
    }
}

/// The tree lists by parent (top level first), then by name ignoring case.
async fn the_tree_lists_by_parent_then_name(repo: &impl FolderRepo) {
    let Tree { work, .. } = tree(repo).await;

    assert_eq!(names(&repo.children(None).await.unwrap()), ["Home", "work"]);
    assert_eq!(
        names(&repo.children(Some(&work.id)).await.unwrap()),
        ["Clients"]
    );
    assert_eq!(
        &names(&repo.folders().await.unwrap())[..2],
        ["Home", "work"]
    );
}

/// Moves into the folder's own subtree are refused; others go through.
async fn moves_never_make_a_cycle(repo: &impl FolderRepo) {
    let Tree {
        work, acme, home, ..
    } = tree(repo).await;
    let later = at("2026-10-09T09:00:00Z");
    let move_to = |id, parent| repo.move_folder(id, parent, later);

    assert_eq!(
        move_to(&work.id, Some(&acme.id)).await.unwrap(),
        FolderMove::WouldCycle
    );
    assert_eq!(
        move_to(&work.id, Some(&work.id)).await.unwrap(),
        FolderMove::WouldCycle
    );
    assert_eq!(
        move_to(&acme.id, Some(&home.id)).await.unwrap(),
        FolderMove::Moved
    );
    assert_eq!(
        move_to(&folder("x", None).id, None).await.unwrap(),
        FolderMove::NotFound
    );
    assert_eq!(
        names(&repo.children(Some(&home.id)).await.unwrap()),
        ["Acme"]
    );
}

/// Deleting a folder deletes its subfolders, and nothing else.
async fn deleting_a_folder_deletes_its_subtree(repo: &impl FolderRepo) {
    let Tree { work, clients, .. } = tree(repo).await;

    assert!(repo.delete_folder(&work.id).await.unwrap());

    assert_eq!(repo.folder(&clients.id).await.unwrap(), None);
    assert_eq!(names(&repo.folders().await.unwrap()), ["Home"]);
}

/// Meetings file into many folders, list newest first, and survive deletion.
async fn meetings_file_into_folders(repo: &(impl FolderRepo + MeetingFolderRepo)) {
    let (work, home) = (folder("Work", None), folder("Home", None));
    repo.create_folder(&work).await.unwrap();
    repo.create_folder(&home).await.unwrap();
    let [older, newer] = meetings();
    for (meeting, target) in [
        (&older, &work),
        (&newer, &work),
        (&older, &home),
        (&older, &home),
    ] {
        repo.file_meeting(&meeting.id, &target.id, at(NOW))
            .await
            .unwrap();
    }

    assert_eq!(
        repo.meetings_in(&work.id).await.unwrap(),
        [newer.clone(), older.clone()]
    );
    assert_eq!(
        names(&repo.folders_of(&older.id).await.unwrap()),
        ["Home", "Work"]
    );
    assert!(repo.unfile_meeting(&older.id, &home.id).await.unwrap());
    assert!(!repo.unfile_meeting(&older.id, &home.id).await.unwrap());
}

/// Deleting a folder takes its meetings out of it and leaves them alone.
async fn deleting_a_folder_unfiles_its_meetings(repo: &(impl FolderRepo + MeetingFolderRepo)) {
    let work = folder("Work", None);
    repo.create_folder(&work).await.unwrap();
    let [meeting, _] = meetings();
    repo.file_meeting(&meeting.id, &work.id, at(NOW))
        .await
        .unwrap();

    repo.delete_folder(&work.id).await.unwrap();

    assert!(repo.folders_of(&meeting.id).await.unwrap().is_empty());
}

#[tokio::test]
async fn sqlite_deleting_a_folder_unfiles_its_meetings() {
    deleting_a_folder_unfiles_its_meetings(&sqlite_repo().await).await;
}

#[tokio::test]
async fn fake_deleting_a_folder_unfiles_its_meetings() {
    deleting_a_folder_unfiles_its_meetings(&FakeFolderRepo::default()).await;
}

#[tokio::test]
async fn sqlite_the_tree_lists_by_parent_then_name() {
    the_tree_lists_by_parent_then_name(&sqlite_repo().await).await;
}

#[tokio::test]
async fn fake_the_tree_lists_by_parent_then_name() {
    the_tree_lists_by_parent_then_name(&FakeFolderRepo::default()).await;
}

#[tokio::test]
async fn sqlite_moves_never_make_a_cycle() {
    moves_never_make_a_cycle(&sqlite_repo().await).await;
}

#[tokio::test]
async fn fake_moves_never_make_a_cycle() {
    moves_never_make_a_cycle(&FakeFolderRepo::default()).await;
}

#[tokio::test]
async fn sqlite_deleting_a_folder_deletes_its_subtree() {
    deleting_a_folder_deletes_its_subtree(&sqlite_repo().await).await;
}

#[tokio::test]
async fn fake_deleting_a_folder_deletes_its_subtree() {
    deleting_a_folder_deletes_its_subtree(&FakeFolderRepo::default()).await;
}

#[tokio::test]
async fn sqlite_meetings_file_into_folders() {
    meetings_file_into_folders(&sqlite_repo().await).await;
}

#[tokio::test]
async fn fake_meetings_file_into_folders() {
    meetings_file_into_folders(&FakeFolderRepo::with_meetings(meetings())).await;
}

#[tokio::test]
async fn sibling_names_are_unique_ignoring_case_even_at_the_top_level() {
    let repo = sqlite_repo().await;
    let work = folder("Work", None);
    repo.create_folder(&work).await.unwrap();

    assert!(repo.create_folder(&folder("work", None)).await.is_err());
    repo.create_folder(&folder("Work", Some(&work)))
        .await
        .unwrap();
}

#[tokio::test]
async fn updating_a_folder_never_moves_it() {
    let repo = sqlite_repo().await;
    let work = folder("Work", None);
    let mut notes = folder("Notes", Some(&work));
    repo.create_folder(&work).await.unwrap();
    repo.create_folder(&notes).await.unwrap();

    notes.parent_id = None;
    notes.description = "Weekly notes".into();
    assert!(repo.update_folder(&notes).await.unwrap());

    let stored = repo.folder(&notes.id).await.unwrap().unwrap();
    assert_eq!(
        (stored.parent_id, stored.description.as_str()),
        (Some(work.id), "Weekly notes")
    );
}

#[tokio::test]
async fn folder_lists_are_read_through_their_indexes() {
    let repo = sqlite_repo().await;
    let plan = |sql| query_plan(&repo.database, sql);

    assert_eq!(
        plan(include_str!("queries/folders.sql")).await,
        ["SCAN folders USING INDEX idx_folders_parent_id"]
    );
    assert_eq!(
        plan(include_str!("queries/children.sql")).await,
        ["SEARCH folders USING INDEX idx_folders_parent_id (parent_id=?)"]
    );
    assert_eq!(
        plan(include_str!("queries/meetings_in.sql")).await,
        [
            "SEARCH mf USING COVERING INDEX idx_meeting_folders_folder_id (folder_id=?)",
            "SEARCH m USING INDEX sqlite_autoindex_meetings_1 (id=?)",
            "USE TEMP B-TREE FOR ORDER BY"
        ]
    );
    assert_eq!(
        plan(include_str!("queries/folders_of.sql")).await,
        [
            "SEARCH mf USING COVERING INDEX sqlite_autoindex_meeting_folders_1 (meeting_id=?)",
            "SEARCH f USING INDEX sqlite_autoindex_folders_1 (id=?)",
            "USE TEMP B-TREE FOR ORDER BY"
        ]
    );
}
