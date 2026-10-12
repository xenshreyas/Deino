// Debug-only feature manifest: the feature plugins compiled into debug builds.
//
// Like `manifest.rs`, `build.rs` `include!`s it, so it may only contain plain
// items (no `//!` docs, no `use`).

/// Feature plugins compiled into debug builds only. `build.rs` registers
/// them, and `capabilities/dev/` grants them, only when debug assertions are on.
pub const DEV_FEATURES: &[(&str, &[&str])] = &[
    // ── FND-13 reference feature (features/_example.rs) ──
    ("example", &["list", "add"]),
    // ── FND-04 developer tools (features/dev.rs) ──
    ("dev", &["seed_fixtures"]),
];
