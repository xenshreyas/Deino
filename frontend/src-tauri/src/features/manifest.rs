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
// Append-only: adding a feature is one line here (or in
// `organize_manifest.rs` for the organization features), one `.plugin(..)`
// call in `lib.rs`, one entry in `features::builders()` and one
// `"<name>:default"` entry in `capabilities/default.json` (long command
// lists: `manifest_commands.rs`).

include!("manifest_commands.rs");
/// Every inline feature plugin as `(plugin name, command names)`.
///
/// The plugin name must equal the feature module's `NAME`, and the command
/// names must match the functions in its `collect_commands!` exactly.
pub const FEATURES: &[(&str, &[&str])] = &[
    // ── FND-02 system ──
    ("system", &["app_version"]),
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
    ("insights", INSIGHTS_COMMANDS),
    // ── AUT-01 automations (features/automations.rs) ──
    ("automations", AUTOMATIONS_COMMANDS),
    // ── NOT-05 profile (features/profile.rs) ──
    ("profile", &["update", "context"]),
    // ── CAP-11 imports and watched folders (features/import.rs) ──
    ("import", IMPORT_COMMANDS),
    // ── TRN-05 custom vocabulary (features/vocabulary.rs) ──
    (
        "vocabulary",
        &["list", "create", "update", "delete", "corrections", "undo"],
    ),
    // ── EXP-01/EXP-02 export (features/export.rs) ──
    (
        "export",
        &[
            "preview",
            "save",
            "save_many",
            "copy",
            "email",
            "choose_folder",
            "start_full_export",
            "cancel_full_export",
            "full_export_status",
        ],
    ),
    ("recording", RECORDING_COMMANDS), // FND-05b, CAP-02, CAP-07, CAP-10 (recording_manifest.rs)
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
    // ── TRN-01/02 transcript view and edits, TRN-03 player (features/{transcript,player}.rs) ──
    ("transcript", TRANSCRIPT_COMMANDS),
    ("player", &["media", "download"]),
    // ── PRV-02 app lock and the library status (features/privacy.rs) ──
    ("privacy", &["state", "touch", "lock", "unlock", "library"]),
    // ── FND-11 onboarding: the permissions it asks for ──
    ("onboarding", ONBOARDING_COMMANDS),
    // ── CAP-05 speaker tags (features/speaker_tags.rs) ──
    ("speaker-tags", &["status", "open_accessibility_settings"]),
    ("stats", &["overview", "heatmap", "recap", "write_recap"]), // ORG-09 (features/stats.rs)
];
include!("recording_manifest.rs"); // `RECORDING_COMMANDS`, beside this file
