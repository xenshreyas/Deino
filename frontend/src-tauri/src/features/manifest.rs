// Feature manifest: the single list of inline feature plugins and their commands.
//
// This file is shared by two compilation units, so it may only contain plain
// items (no `//!` docs, no `use`):
// - `build.rs` `include!`s it to generate each plugin's ACL permissions
//   (`allow-<command>`, `deny-<command>` and the `<name>:default` set);
// - the app compiles it as `features::manifest`, where the guard tests check
//   that every feature is granted in `capabilities/` and that these command
//   lists equal the commands in the generated TypeScript bindings.
//
// Append-only: a feature is one line here, one `.plugin(..)` in `lib.rs`, one entry in
// `features::builders()` and one `"<name>:default"` in `capabilities/default.json`.

/// Every inline feature plugin as `(plugin name, command names)`.
///
/// The plugin name must equal the feature module's `NAME`, and the command
/// names must match the functions in its `collect_commands!` exactly.
pub const FEATURES: &[(&str, &[&str])] = &[
    // ── FND-02 system ──
    ("system", &["app_version"]),
    // ── FND-13 meetings ──
    ("meetings", &["list", "get", "rename"]),
    // ── NOT-01 notepad and @mention search ──
    (
        "notes",
        &[
            "get",
            "save",
            "create_scheduled",
            "save_image",
            "attach_dropped_image",
            "markdown_to_doc",
            "doc_to_markdown",
        ],
    ),
    (
        "people",
        &[
            "search",
            "list",
            "page",
            "create",
            "update",
            "set_notes",
            "merge",
            "from_mention",
            "refresh_summary",
            "attendees",
            "add_attendee",
            "remove_attendee",
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
    // ── NOT-02/NOT-03 enhanced notes and citations (features/enhance.rs) ──
    (
        "enhance",
        &["get", "versions", "enhance", "save", "restore", "cited"],
    ),
    // ── FND-06 settings and secrets ──
    ("settings", &["get", "patch"]),
    ("secrets", &["set", "status", "delete"]),
    // ── CAP-01 transcription (features/transcription.rs) ──
    (
        "transcription",
        &[
            "list_models",
            "download_model",
            "cancel_download",
            "delete_model",
        ],
    ),
    // ── CAP-06 meeting detection (features/detection.rs) ──
    (
        "detection",
        &["current", "take_notes", "dismiss", "ignore_app", "settings"],
    ),
    // ── FND-09 LLM gateway: Settings → AI ──
    (
        "llm",
        &[
            "providers",
            "models",
            "routes",
            "set_route",
            "test_route",
            "ollama_pull",
            "ollama_delete",
        ],
    ),
    // ── FND-12 network controls ──
    (
        "network",
        &["services", "set_service_enabled", "egress_log"],
    ),
    // ── FND-08 pipeline ──
    ("pipeline", &["status", "rerun"]),
    // ── NOT-04 templates (features/templates.rs) ──
    (
        "templates",
        &[
            "list",
            "get",
            "create",
            "update",
            "duplicate",
            "delete",
            "import_json",
            "export_json",
            "preview",
            "meeting_template",
            "set_meeting_template",
        ],
    ),
    // ── AI-05 action items (features/action_items.rs) ──
    (
        "action-items",
        &[
            "list",
            "today",
            "create",
            "update",
            "bulk_update",
            "delete",
            "mark_all_mine_done",
            "filter_options",
            "views",
            "create_view",
            "update_view",
            "delete_view",
        ],
    ),
    // ── AI-06 custom insights (features/insights.rs) ──
    (
        "insights",
        &[
            "templates",
            "list",
            "get",
            "create",
            "update",
            "delete",
            "for_meeting",
            "table",
            "candidates",
            "run",
            "export_csv",
        ],
    ),
    // ── NOT-05 profile (features/profile.rs) ──
    ("profile", &["update", "context"]),
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
    // ── TRN-05 custom vocabulary (features/vocabulary.rs) ──
    (
        "vocabulary",
        &["list", "create", "update", "delete", "corrections", "undo"],
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
    // ── EXP-01/EXP-02 export (features/export.rs) ──
    (
        "export",
        &[
            "preview",
            "save",
            "copy",
            "email",
            "choose_folder",
            "start_full_export",
            "cancel_full_export",
            "full_export_status",
        ],
    ),
    // ── FND-05b / CAP-02 live recorder (features/recording.rs) ──
    ("recording", &["start", "stop", "pause", "resume", "state"]),
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
    // ── DEV-02/04 Settings → Developer: the CLI and Claude setup ──
    ("developer", &["cli_status", "install_cli"]),
    // ── CAP-04 speaker naming and voice profiles ──
    (
        "speakers",
        &[
            "participants",
            "candidates",
            "rename",
            "merge",
            "confirm_suggestion",
            "dismiss_suggestion",
            "voice_profiles",
            "delete_voiceprint",
            "forget_voice",
        ],
    ),
    // ── INT-00 integrations (features/integrations.rs) ──
    (
        "integrations",
        &[
            "providers",
            "connect",
            "cancel_connect",
            "connect_token",
            "check",
            "disconnect",
            "set_app",
            "remove_app",
            "targets",
            "deliver",
            "deliveries",
        ],
    ),
    // ── TRN-01 transcript view (features/transcript.rs) ──
    ("transcript", &["get"]),
    // ── FND-11 onboarding: the permissions it asks for ──
    (
        "onboarding",
        &[
            "permissions",
            "request_permission",
            "open_permission_settings",
            "import_sample",
        ],
    ),
    // ── CAP-05 speaker tags (features/speaker_tags.rs) ──
    ("speaker-tags", &["status", "open_accessibility_settings"]),
    ("stats", &["overview", "heatmap", "recap", "write_recap"]), // ORG-09 (features/stats.rs)
];
