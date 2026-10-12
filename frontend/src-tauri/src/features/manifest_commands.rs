// Command lists too long for one line of the feature manifest, `include!`d by
// `manifest.rs` (and so by `build.rs`). Like `manifest.rs`, it may only
// contain plain items (no `//!` docs, no `use`).

/// The `transcript` plugin's commands: the TRN-01 read and the TRN-02 edits.
pub const TRANSCRIPT_COMMANDS: &[&str] = &["get", "edit", "delete", "restore", "preview", "replace"];

/// The `onboarding` plugin's commands: the permissions it asks for (FND-11).
pub const ONBOARDING_COMMANDS: &[&str] = &[
    "permissions",
    "request_permission",
    "open_permission_settings",
    "import_sample",
];

/// The `clips` plugin's commands (TRN-04).
pub const CLIPS_COMMANDS: &[&str] = &["create", "list", "list_all", "rename", "delete", "reveal"];

/// The `comments` plugin's commands (TRN-06).
pub const COMMENTS_COMMANDS: &[&str] = &["list", "start", "reply", "edit", "set_resolved", "delete"];
