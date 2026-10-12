// Command lists too long for one line of the feature manifest, `include!`d by
// `manifest.rs` (and so by `build.rs`). Like `manifest.rs`, it may only
// contain plain items (no `//!` docs, no `use`).

/// The `transcript` plugin's commands: the TRN-01 read and the TRN-02 edits.
pub const TRANSCRIPT_COMMANDS: &[&str] = &["get", "edit", "delete", "restore", "preview", "replace"];
