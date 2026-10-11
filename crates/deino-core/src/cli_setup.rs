//! Settings → Developer's command-line setup (DEV-02/04, ADR 0035): where
//! the `deino` CLI bundled in the app is, installing it on the `PATH` as a
//! symlink, and the snippets that connect Claude Code and Claude Desktop to
//! its MCP server.
//!
//! - [`CliSetupService`]: [`CliStatus`] (the bundled binary, each
//!   [`InstallTarget`] and what is there now, the snippets) and
//!   [`CliSetupService::install`], which links `deino` into `~/.local/bin`
//!   or `/usr/local/bin` and never replaces a file it did not make.
//! - [`Links`] is the file-system port ([`FsLinks`] in the app).
//! - [`snippets`]: `claude mcp add …` and the Claude Desktop config.

mod error;
mod links;
mod model;
mod service;
#[cfg(test)]
mod service_tests;
pub mod snippets;

pub use error::CliSetupError;
pub use links::{Entry, FsLinks, Links};
pub use model::{CliStatus, InstallDir, InstallTarget, LinkState};
pub use service::{CLI_NAME, CliSetupService};
