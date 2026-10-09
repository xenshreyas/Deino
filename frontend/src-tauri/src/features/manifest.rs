// Feature manifest: the single list of inline feature plugins and their commands.
//
// This file is shared by two compilation units, so it may only contain plain
// items (no `//!` docs, no `use`):
// - `build.rs` `include!`s it to generate each plugin's ACL permissions
//   (`allow-<command>`, `deny-<command>` and the `<name>:default` set);
// - the app compiles it as `features::manifest`, where the capability guard
//   test checks that every feature is granted in `capabilities/default.json`.
//
// Append-only: adding a feature is one line here, one `.plugin(..)` call in
// `lib.rs` and one `"<name>:default"` entry in `capabilities/default.json`.

/// Every inline feature plugin as `(plugin name, command names)`.
///
/// The plugin name must equal the feature module's `NAME`, and the command
/// names must match the functions in its `generate_handler!` exactly.
pub const FEATURES: &[(&str, &[&str])] = &[
    // ── FND-02 system ──
    ("system", &["app_version"]),
];
