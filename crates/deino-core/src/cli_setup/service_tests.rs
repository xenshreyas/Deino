//! `CliSetupService` over an in-memory file system.

use std::collections::{HashMap, HashSet};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use super::*;
use crate::testing::lock;

const BUNDLED: &str = "/Applications/Deino.app/Contents/MacOS/cli";
const HOME: &str = "/Users/me";
const USER_LINK: &str = "/Users/me/.local/bin/deino";
const SYSTEM_LINK: &str = "/usr/local/bin/deino";

/// Paths and their entries; folders in `writable` accept new files.
#[derive(Default)]
struct FakeLinks {
    entries: Mutex<HashMap<PathBuf, Entry>>,
    writable: Mutex<HashSet<PathBuf>>,
}

impl FakeLinks {
    fn with(entries: &[(&str, Entry)], writable: &[&str]) -> Arc<Self> {
        let links = Self::default();
        lock(&links.entries).extend(
            entries
                .iter()
                .map(|(path, entry)| (PathBuf::from(path), entry.clone())),
        );
        lock(&links.writable).extend(writable.iter().map(PathBuf::from));
        Arc::new(links)
    }
}

impl Links for FakeLinks {
    fn inspect(&self, path: &Path) -> io::Result<Entry> {
        Ok(lock(&self.entries)
            .get(path)
            .cloned()
            .unwrap_or(Entry::Missing))
    }

    fn is_writable(&self, dir: &Path) -> bool {
        lock(&self.writable).contains(dir)
    }

    fn create_dir_all(&self, dir: &Path) -> io::Result<()> {
        lock(&self.writable).insert(dir.to_path_buf());
        Ok(())
    }

    fn replace_link(&self, target: &Path, link: &Path) -> io::Result<()> {
        lock(&self.entries).insert(link.to_path_buf(), Entry::Link(target.to_path_buf()));
        Ok(())
    }
}

fn service(links: Arc<FakeLinks>, bundled: Option<&str>) -> CliSetupService {
    CliSetupService::new(links, bundled.map(PathBuf::from), PathBuf::from(HOME))
}

#[test]
fn without_an_install_the_snippets_run_the_bundled_binary() {
    let status = service(FakeLinks::with(&[], &[]), Some(BUNDLED)).status();

    assert_eq!(status.bundled.as_deref(), Some(BUNDLED));
    assert_eq!(status.command.as_deref(), Some(BUNDLED));
    assert_eq!(
        status.claude_code.as_deref(),
        Some("claude mcp add deino -- /Applications/Deino.app/Contents/MacOS/cli mcp")
    );
    assert!(
        status
            .targets
            .iter()
            .all(|target| target.state == LinkState::Missing)
    );
}

#[test]
fn installing_creates_the_user_folder_and_links_the_binary() {
    let links = FakeLinks::with(&[], &[]);

    let status = service(links.clone(), Some(BUNDLED))
        .install(InstallDir::UserBin)
        .unwrap();

    assert_eq!(
        links.inspect(Path::new(USER_LINK)).unwrap(),
        Entry::Link(PathBuf::from(BUNDLED))
    );
    assert_eq!(status.targets[0].state, LinkState::Installed);
    assert_eq!(status.command.as_deref(), Some(USER_LINK));
}

#[test]
fn an_older_link_is_replaced_and_reported_until_then() {
    let old = Entry::Link(PathBuf::from("/Volumes/Old/Deino.app/Contents/MacOS/cli"));
    let links = FakeLinks::with(&[(SYSTEM_LINK, old)], &["/usr/local/bin"]);
    let setup = service(links, Some(BUNDLED));

    let before = setup.status();
    let after = setup.install(InstallDir::UsrLocalBin).unwrap();

    assert!(matches!(
        before.targets[1].state,
        LinkState::OtherLink { .. }
    ));
    assert_eq!(after.targets[1].state, LinkState::Installed);
}

#[test]
fn a_file_deino_did_not_make_is_never_replaced() {
    let links = FakeLinks::with(&[(USER_LINK, Entry::File)], &["/Users/me/.local/bin"]);

    let error = service(links.clone(), Some(BUNDLED))
        .install(InstallDir::UserBin)
        .unwrap_err();

    assert!(matches!(error, CliSetupError::Occupied { .. }));
    assert_eq!(links.inspect(Path::new(USER_LINK)).unwrap(), Entry::File);
}

#[test]
fn a_folder_the_app_cannot_write_returns_the_sudo_command() {
    let error = service(FakeLinks::with(&[], &[]), Some(BUNDLED))
        .install(InstallDir::UsrLocalBin)
        .unwrap_err();

    assert_eq!(
        error.to_string(),
        "Deino cannot write to that folder; run this in Terminal instead: \
         sudo ln -sf '/Applications/Deino.app/Contents/MacOS/cli' '/usr/local/bin/deino'"
    );
}

#[test]
fn a_build_without_the_cli_cannot_install_it() {
    let setup = service(FakeLinks::with(&[], &[]), None);

    assert!(matches!(
        setup.install(InstallDir::UserBin),
        Err(CliSetupError::NotBundled)
    ));
    assert_eq!(setup.status().claude_code, None);
}
