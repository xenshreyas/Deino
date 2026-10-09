//! Feature IPC: one inline Tauri plugin per feature (ADR 0002, ADR 0007).
//!
//! Each feature exposes `NAME`, `specta()` (its `tauri_specta::Builder`) and
//! `init()` from `features/<feature>.rs`, with thin commands in
//! `features/<feature>/commands.rs`. Plugin commands are always checked
//! against the capability ACL, so a feature is registered in four places,
//! one line each:
//!
//! 1. [`manifest::FEATURES`]: the plugin name and command names, from which
//!    `build.rs` generates the plugin's ACL permissions;
//! 2. `lib.rs`: `.plugin(features::<feature>::init())`;
//! 3. `capabilities/default.json`: `"<feature>:default"`;
//! 4. [`builders`]: its bindings builder, exported to TypeScript.
//!
//! The guard tests below and in `bindings.rs` fail when these drift apart.
//! [`_example`] is the reference feature to copy.

#[cfg(debug_assertions)]
pub mod _example;
pub mod db;
pub mod detection;
#[cfg(debug_assertions)]
pub mod dev;
pub mod llm;
/// The feature plugins and their commands (shared with `build.rs`, so it has no `//!` docs).
pub mod manifest;
pub mod meetings;
pub mod network;
pub mod pipeline;
pub mod profile;
pub mod secrets;
pub mod settings;
pub mod system;
pub mod templates;
pub mod transcription;

/// Every feature's bindings builder, keyed by the file it is exported to
/// under `frontend/src/lib/ipc/generated/`.
#[cfg(test)]
pub(crate) fn builders() -> Vec<(&'static str, tauri_specta::Builder<tauri::Wry>)> {
    let mut builders = vec![
        (db::NAME, db::specta()),
        (detection::NAME, detection::specta()),
        (meetings::NAME, meetings::specta()),
        (network::NAME, network::specta()),
        (pipeline::NAME, pipeline::specta()),
        (secrets::NAME, secrets::specta()),
        (llm::NAME, llm::specta()),
        (profile::NAME, profile::specta()),
        (settings::NAME, settings::specta()),
        (templates::NAME, templates::specta()),
        (transcription::NAME, transcription::specta()),
        (system::NAME, system::specta()),
    ];
    #[cfg(debug_assertions)]
    builders.push((_example::NAME, _example::specta()));
    #[cfg(debug_assertions)]
    builders.push((dev::NAME, dev::specta()));
    builders
}

#[cfg(test)]
mod tests {
    use super::manifest::{DEV_FEATURES, FEATURES};

    /// The capability files that grant feature plugins.
    const CAPABILITIES: [&str; 4] = [
        include_str!("../capabilities/default.json"),
        include_str!("../capabilities/detection-prompt.json"),
        include_str!("../capabilities/dev/example.json"),
        include_str!("../capabilities/dev/dev.json"),
    ];

    fn granted_permissions(capability: &str) -> Vec<String> {
        let capability: serde_json::Value =
            serde_json::from_str(capability).expect("capability files are valid JSON");
        capability["permissions"]
            .as_array()
            .expect("capability has a permissions array")
            .iter()
            .filter_map(|permission| permission.as_str().map(str::to_string))
            .collect()
    }

    #[test]
    fn every_manifest_feature_is_granted_by_a_capability() {
        let granted: Vec<String> = CAPABILITIES
            .iter()
            .flat_map(|c| granted_permissions(c))
            .collect();
        for (name, _) in FEATURES.iter().chain(DEV_FEATURES) {
            let permission = format!("{name}:default");
            assert!(
                granted.contains(&permission),
                "no capability grants \"{permission}\""
            );
        }
    }

    #[test]
    fn release_capabilities_grant_no_dev_feature() {
        let granted = granted_permissions(CAPABILITIES[0]);
        for (name, _) in DEV_FEATURES {
            assert!(
                !granted.contains(&format!("{name}:default")),
                "capabilities/default.json ships in release builds, where \"{name}\" does not exist"
            );
        }
    }

    #[test]
    fn manifest_names_are_unique_and_list_commands() {
        let mut names: Vec<&str> = FEATURES
            .iter()
            .chain(DEV_FEATURES)
            .map(|(name, _)| *name)
            .collect();
        let count = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), count, "duplicate feature in the manifest");
        assert!(
            FEATURES
                .iter()
                .chain(DEV_FEATURES)
                .all(|(_, commands)| !commands.is_empty())
        );
    }
}

#[cfg(test)]
mod ipc_tests;
