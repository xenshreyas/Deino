//! The `deino` command (DEV-02, DEV-04, ADR 0035): the user's meetings,
//! notes, transcripts and action items, read straight from the Deino
//! database, in the terminal and over MCP for Claude.
//!
//! - [`Cli`] and [`execute`]: `deino meetings`, `meetings read`,
//!   `transcripts read`, `search`, `action-items`, `people`, `companies`,
//!   `tags`, `folders`, `calendar`, `export`, `status`, each with `--json`.
//! - [`mcp`]: `deino mcp`, the same reads as MCP tools over stdio.
//! - [`request`]: each read defined once ([`request::Request`]), for both.
//! - [`markdown`]: the compact Markdown both print.
//!
//! The database is opened read-only (`deino_db::Database::open_read_only`):
//! the binary never creates, migrates or writes it, and it refuses a schema
//! it was not built for. Its path is `--db`, `DEINO_DB`, or the app's own
//! ([`db_path`]). Everything it reads comes from `deino_core::query`.

mod cli;
pub mod db_path;
pub mod embedder;
mod error;
mod export;
pub mod markdown;
pub mod mcp;
mod open;
pub mod request;
mod status;

pub use cli::{Cli, Command, execute};
pub use error::{CliError, ErrorBody};
pub use open::Opened;
