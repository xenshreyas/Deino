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

/// The `import` plugin's commands: imports and watched folders (CAP-11).
pub const IMPORT_COMMANDS: &[&str] = &[
    "pick_file",
    "check",
    "start",
    "watched_folders",
    "add_watched_folder",
    "remove_watched_folder",
    "set_move_to_processed",
    "watch_failures",
    "retry_failure",
    "dismiss_failure",
];
