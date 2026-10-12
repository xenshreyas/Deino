//! Feature IPC: one inline Tauri plugin per feature (ADR 0002, ADR 0007).
//!
//! Each feature exposes `NAME`, `specta()` (its `tauri_specta::Builder`) and
//! `init()` from `features/<feature>.rs`, with thin commands in
//! `features/<feature>/commands.rs`. Plugin commands are always checked
//! against the capability ACL, so a feature is registered in four places,
//! one line each:
//!
//! 1. [`manifest::FEATURES`] (or [`organize_manifest::ORGANIZE_FEATURES`]):
//!    the plugin name and command names, from which `build.rs` generates the
//!    plugin's ACL permissions;
//! 2. `lib.rs`: `.plugin(features::<feature>::init())`;
//! 3. `capabilities/default.json`: `"<feature>:default"`;
//! 4. [`builders`]: its bindings builder, exported to TypeScript.
//!
//! The guard tests below and in `bindings.rs` fail when these drift apart.
//! [`_example`] is the reference feature to copy.

#[cfg(debug_assertions)]
pub mod _example;
pub mod action_items;
pub mod calendar;
pub mod companies;
pub mod db;
pub mod detection;
#[cfg(debug_assertions)]
pub mod dev;
/// The debug-only feature plugins (shared with `build.rs`, so it has no `//!` docs).
pub mod dev_manifest;
pub mod developer;
pub mod enhance;
pub mod export;
pub mod folders;
pub mod insights;
pub mod integrations;
pub mod llm;
/// The feature plugins and their commands (shared with `build.rs`, so it has no `//!` docs).
pub mod manifest;
pub mod meetings;
pub mod network;
pub mod notes;
pub mod onboarding;
/// The organization feature plugins (shared with `build.rs`, so it has no `//!` docs).
pub mod organize_manifest;
pub mod people;
pub mod pipeline;
pub mod player;
pub mod profile;
pub mod recording;
pub mod search;
pub mod secrets;
pub mod settings;
pub mod speaker_tags;
pub mod speakers;
pub mod stats;
pub mod system;
pub mod tags;
pub mod templates;
pub mod transcript;
pub mod transcription;
pub mod vocabulary;

/// Every feature's bindings builder, keyed by the file it is exported to
/// under `frontend/src/lib/ipc/generated/`.
#[cfg(test)]
pub(crate) fn builders() -> Vec<(&'static str, tauri_specta::Builder<tauri::Wry>)> {
    let mut builders = vec![
        (action_items::NAME, action_items::specta()),
        (db::NAME, db::specta()),
        (detection::NAME, detection::specta()),
        (developer::NAME, developer::specta()),
        (onboarding::NAME, onboarding::specta()),
        (export::NAME, export::specta()),
        (calendar::NAME, calendar::specta()),
        (enhance::NAME, enhance::specta()),
        (companies::NAME, companies::specta()),
        (meetings::NAME, meetings::specta()),
        (network::NAME, network::specta()),
        (pipeline::NAME, pipeline::specta()),
        (notes::NAME, notes::specta()),
        (people::NAME, people::specta()),
        (recording::NAME, recording::specta()),
        (secrets::NAME, secrets::specta()),
        (llm::NAME, llm::specta()),
        (profile::NAME, profile::specta()),
        (search::NAME, search::specta()),
        (settings::NAME, settings::specta()),
        (templates::NAME, templates::specta()),
        (transcription::NAME, transcription::specta()),
        (system::NAME, system::specta()),
        (folders::NAME, folders::specta()),
        (integrations::NAME, integrations::specta()),
        (insights::NAME, insights::specta()),
        (tags::NAME, tags::specta()),
        (speakers::NAME, speakers::specta()),
        (stats::NAME, stats::specta()),
        (transcript::NAME, transcript::specta()),
        (vocabulary::NAME, vocabulary::specta()),
        (speaker_tags::NAME, speaker_tags::specta()),
        (player::NAME, player::specta()),
    ];
    #[cfg(debug_assertions)]
    builders.push((_example::NAME, _example::specta()));
    #[cfg(debug_assertions)]
    builders.push((dev::NAME, dev::specta()));
    builders
}

#[cfg(test)]
mod tests {
    use super::dev_manifest::DEV_FEATURES;
    use super::manifest::FEATURES;
    use super::organize_manifest::ORGANIZE_FEATURES;

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
        for (name, _) in FEATURES.iter().chain(ORGANIZE_FEATURES).chain(DEV_FEATURES) {
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
            .chain(ORGANIZE_FEATURES)
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
                .chain(ORGANIZE_FEATURES)
                .chain(DEV_FEATURES)
                .all(|(_, commands)| !commands.is_empty())
        );
    }
}

#[cfg(test)]
mod ipc_tests;
