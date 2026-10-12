// Organization feature manifest: the meetings list, people and companies,
// search, folders, tags and calendars (the ORG cards), split from
// `manifest.rs` to keep both files under the size cap.
//
// Like `manifest.rs`, `build.rs` `include!`s it, so it may only contain plain
// items (no `//!` docs, no `use`).

/// The organization feature plugins as `(plugin name, command names)`, under
/// the same rules as `FEATURES`.
pub const ORGANIZE_FEATURES: &[(&str, &[&str])] = &[
    // ── FND-13 meetings; ORG-07 the list, views, trash, bulk actions, merge ──
    (
        "meetings",
        &[
            "list",
            "get",
            "rename",
            "page",
            "matching_ids",
            "views",
            "create_view",
            "update_view",
            "delete_view",
            "trash",
            "restore",
            "trashed",
            "delete_forever",
            "tag_many",
            "untag_many",
            "file_many",
            "merge",
        ],
    ),
    // ── ORG-04 companies ──
    (
        "companies",
        &[
            "list",
            "page",
            "create",
            "update",
            "set_notes",
            "merge",
            "refresh_summary",
            "fetch_logo",
        ],
    ),
    // ── ORG-08 / AI-01 search (features/search.rs) ──
    (
        "search",
        &[
            "search",
            "suggest_filters",
            "reindex_all",
            "index_status",
            "download_model",
            "cancel_download",
            "delete_model",
            "start_backfill",
        ],
    ),
    // ── ORG-05 folders ──
    (
        "folders",
        &[
            "tree",
            "get",
            "create",
            "rename",
            "set_icon",
            "set_description",
            "move_to",
            "delete",
            "meetings",
            "unfiled",
            "folders_of",
            "add_meeting",
            "remove_meeting",
            "rules",
            "add_rule",
            "remove_rule",
        ],
    ),
    // ── ORG-06 tags ──
    (
        "tags",
        &[
            "list", "create", "update", "delete", "tags_of", "links", "apply", "remove",
        ],
    ),
    // ── ORG-01 calendar sync (features/calendar.rs) ──
    (
        "calendar",
        &[
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
        ],
    ),
];
