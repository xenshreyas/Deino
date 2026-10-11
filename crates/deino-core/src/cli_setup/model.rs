//! What Settings → Developer shows about the command-line tool.

use serde::{Deserialize, Serialize};

/// A folder the CLI can be installed into.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum InstallDir {
    /// `~/.local/bin`: the user's own, created when missing.
    UserBin,
    /// `/usr/local/bin`: on every shell's `PATH`, often not writable.
    UsrLocalBin,
}

impl InstallDir {
    /// Both folders, the recommended one first.
    pub const ALL: [Self; 2] = [Self::UserBin, Self::UsrLocalBin];
}

/// What is at an install path now.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LinkState {
    /// Nothing.
    Missing,
    /// A link to this app's CLI.
    Installed,
    /// A link somewhere else, such as an older copy of the app.
    OtherLink {
        /// Where it points.
        target: String,
    },
    /// A file or folder Deino did not make; installing leaves it alone.
    OtherFile,
}

/// One place the CLI can be installed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct InstallTarget {
    /// The folder.
    pub dir: InstallDir,
    /// The link's full path, such as `/Users/me/.local/bin/deino`.
    pub path: String,
    /// What is there now.
    pub state: LinkState,
    /// False when the app cannot write the folder (installing there needs
    /// `sudo`).
    pub writable: bool,
}

/// The CLI's setup, as Settings → Developer shows it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CliStatus {
    /// The CLI inside the app; `None` when this build has none.
    pub bundled: Option<String>,
    /// Where it can be installed, and what is there now.
    pub targets: Vec<InstallTarget>,
    /// What the snippets run: the installed link, else the bundled binary.
    pub command: Option<String>,
    /// `claude mcp add deino -- <command> mcp`.
    pub claude_code: Option<String>,
    /// The `mcpServers` entry for Claude Desktop's config file.
    pub claude_desktop: Option<String>,
}
