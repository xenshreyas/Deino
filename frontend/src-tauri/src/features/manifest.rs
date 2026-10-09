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
// Append-only: adding a feature is one line here, one `.plugin(..)` call in
// `lib.rs`, one entry in `features::builders()` and one `"<name>:default"`
// entry in `capabilities/default.json`.

/// Every inline feature plugin as `(plugin name, command names)`.
///
/// The plugin name must equal the feature module's `NAME`, and the command
/// names must match the functions in its `collect_commands!` exactly.
pub const FEATURES: &[(&str, &[&str])] = &[
    // ── FND-02 system ──
    ("system", &["app_version"]),
    // ── FND-13 meetings ──
    ("meetings", &["list", "get", "rename"]),
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
];

/// Feature plugins compiled into debug builds only. `build.rs` registers
/// them, and `capabilities/dev/` grants them, only when debug assertions are on.
pub const DEV_FEATURES: &[(&str, &[&str])] = &[
    // ── FND-13 reference feature (features/_example.rs) ──
    ("example", &["list", "add"]),
    // ── FND-04 developer tools (features/dev.rs) ──
    ("dev", &["seed_fixtures"]),
];
