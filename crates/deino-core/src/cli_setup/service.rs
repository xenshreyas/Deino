//! [`CliSetupService`]: the CLI's install state and the install itself.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::snippets::{claude_code, claude_desktop};
use super::{CliSetupError, CliStatus, Entry, InstallDir, InstallTarget, LinkState, Links};

/// The command the CLI is installed as.
pub const CLI_NAME: &str = "deino";
/// The system-wide folder.
const USR_LOCAL_BIN: &str = "/usr/local/bin";

/// Reports and changes where the bundled CLI is installed.
pub struct CliSetupService {
    links: Arc<dyn Links>,
    bundled: Option<PathBuf>,
    home: PathBuf,
}

impl CliSetupService {
    /// A service for the CLI at `bundled` (`None` when the build has none),
    /// installing into folders under `home` or the system's.
    pub fn new(links: Arc<dyn Links>, bundled: Option<PathBuf>, home: PathBuf) -> Self {
        Self {
            links,
            bundled,
            home,
        }
    }

    /// The bundled CLI, each install target and the snippets.
    pub fn status(&self) -> CliStatus {
        let targets: Vec<InstallTarget> = InstallDir::ALL
            .iter()
            .map(|dir| self.target(*dir))
            .collect();
        let installed = targets
            .iter()
            .find(|target| target.state == LinkState::Installed)
            .map(|target| target.path.clone());
        let command = installed.or_else(|| self.bundled.as_deref().map(display));
        CliStatus {
            bundled: self.bundled.as_deref().map(display),
            claude_code: command.as_deref().map(claude_code),
            claude_desktop: command.as_deref().map(claude_desktop),
            command,
            targets,
        }
    }

    /// Links `deino` in `dir` to the bundled CLI, replacing a link (an older
    /// install) but never a file. `~/.local/bin` is created when missing; a
    /// folder the app cannot write fails with the `sudo` command to run.
    pub fn install(&self, dir: InstallDir) -> Result<CliStatus, CliSetupError> {
        let bundled = self.bundled.as_ref().ok_or(CliSetupError::NotBundled)?;
        let link = self.link_path(dir);
        if self.links.inspect(&link)? == Entry::File {
            return Err(CliSetupError::Occupied {
                path: display(&link),
            });
        }
        let folder = self.folder(dir);
        if dir == InstallDir::UserBin {
            self.links.create_dir_all(&folder)?;
        }
        if !self.links.is_writable(&folder) {
            let command = format!("sudo ln -sf '{}' '{}'", display(bundled), display(&link));
            return Err(CliSetupError::NotWritable { command });
        }
        self.links.replace_link(bundled, &link)?;
        Ok(self.status())
    }

    fn target(&self, dir: InstallDir) -> InstallTarget {
        let path = self.link_path(dir);
        let state = match self.links.inspect(&path) {
            Ok(Entry::Missing) => LinkState::Missing,
            Ok(Entry::Link(target)) if Some(&target) == self.bundled.as_ref() => {
                LinkState::Installed
            }
            Ok(Entry::Link(target)) => LinkState::OtherLink {
                target: display(&target),
            },
            Ok(Entry::File) | Err(_) => LinkState::OtherFile,
        };
        InstallTarget {
            dir,
            path: display(&path),
            writable: self.links.is_writable(&self.folder(dir)),
            state,
        }
    }

    fn folder(&self, dir: InstallDir) -> PathBuf {
        match dir {
            InstallDir::UserBin => self.home.join(".local").join("bin"),
            InstallDir::UsrLocalBin => PathBuf::from(USR_LOCAL_BIN),
        }
    }

    fn link_path(&self, dir: InstallDir) -> PathBuf {
        self.folder(dir).join(CLI_NAME)
    }
}

fn display(path: &Path) -> String {
    path.display().to_string()
}
