//! Builds the folder tree's display order from the flat folder list.

use std::collections::{BTreeMap, HashMap};

use deino_domain::FolderId;

use super::{Folder, FolderTreeEntry};

/// `folders` depth-first: each folder, then its subfolders, siblings by
/// name ignoring case (then by ID, for equal names in different case),
/// with each folder's meeting count from `counts` (0 when absent).
///
/// Iterative, so a deep tree cannot overflow the stack; a folder whose
/// parent is missing (impossible under the foreign key) is left out.
pub(super) fn display_order(
    folders: Vec<Folder>,
    counts: &HashMap<FolderId, u32>,
) -> Vec<FolderTreeEntry> {
    let mut children = children_by_parent(folders);
    let mut entries = Vec::new();
    let mut stack: Vec<(Folder, u32)> = take_children(&mut children, None, 0);
    while let Some((folder, depth)) = stack.pop() {
        stack.extend(take_children(&mut children, Some(folder.id), depth + 1));
        let meeting_count = counts.get(&folder.id).copied().unwrap_or(0);
        entries.push(FolderTreeEntry {
            folder,
            depth,
            meeting_count,
        });
    }
    entries
}

/// Folders grouped by parent, each group sorted by name.
fn children_by_parent(folders: Vec<Folder>) -> BTreeMap<Option<FolderId>, Vec<Folder>> {
    let mut groups: BTreeMap<Option<FolderId>, Vec<Folder>> = BTreeMap::new();
    for folder in folders {
        groups.entry(folder.parent_id).or_default().push(folder);
    }
    for group in groups.values_mut() {
        group.sort_by_cached_key(|folder| (folder.name.to_lowercase(), folder.id));
    }
    groups
}

/// The children of `parent` at `depth`, reversed so the stack pops them in
/// name order.
fn take_children(
    groups: &mut BTreeMap<Option<FolderId>, Vec<Folder>>,
    parent: Option<FolderId>,
    depth: u32,
) -> Vec<(Folder, u32)> {
    let children = groups.remove(&parent).unwrap_or_default();
    children
        .into_iter()
        .rev()
        .map(|folder| (folder, depth))
        .collect()
}

#[cfg(test)]
mod tests {
    use deino_domain::FakeClock;
    use deino_testkit::at;

    use super::*;

    fn folder(name: &str, parent: Option<&Folder>) -> Folder {
        Folder {
            id: FolderId::generate(&FakeClock::new(at("2026-10-09T09:00:00Z"))),
            parent_id: parent.map(|parent| parent.id),
            name: name.into(),
            icon: None,
            description: String::new(),
            created_at: at("2026-10-09T09:00:00Z"),
            updated_at: at("2026-10-09T09:00:00Z"),
        }
    }

    fn rows(entries: &[FolderTreeEntry]) -> Vec<(&str, u32, u32)> {
        entries
            .iter()
            .map(|entry| (entry.folder.name.as_str(), entry.depth, entry.meeting_count))
            .collect()
    }

    #[test]
    fn folders_come_depth_first_with_siblings_by_name() {
        let work = folder("work", None);
        let clients = folder("Clients", Some(&work));
        let acme = folder("Acme", Some(&clients));
        let admin = folder("admin", Some(&work));
        let home = folder("Home", None);
        let counts = HashMap::from([(acme.id, 3), (home.id, 1)]);
        let shuffled = vec![acme, home, admin, work, clients];

        let entries = display_order(shuffled, &counts);

        assert_eq!(
            rows(&entries),
            [
                ("Home", 0, 1),
                ("work", 0, 0),
                ("admin", 1, 0),
                ("Clients", 1, 0),
                ("Acme", 2, 3),
            ]
        );
    }

    #[test]
    fn an_empty_list_is_an_empty_tree() {
        assert!(display_order(Vec::new(), &HashMap::new()).is_empty());
    }
}
