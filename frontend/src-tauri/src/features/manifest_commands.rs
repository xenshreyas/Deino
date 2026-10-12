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

/// Every command of the `calendar` plugin (features/calendar.rs).
pub const CALENDAR_COMMANDS: &[&str] = &[
    // ── ORG-01 calendar sync ──
    "status",
    "enable",
    "disable",
    "refresh",
    "open_access_settings",
    "list_calendars",
    "set_calendar_visible",
    "upcoming",
    "event",
    "connect",
    "disconnect",
    // ── ORG-03 event linking and related meetings ──
    "meeting_event",
    "event_suggestions",
    "link_event",
    "unlink_event",
    "related_meetings",
    // ── ORG-02 auto-record rules and Join ──
    "record_plan",
    "choose_record",
    "join",
];
