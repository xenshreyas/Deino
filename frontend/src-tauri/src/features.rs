//! Feature IPC: one inline Tauri plugin per feature.
//!
//! Each feature exposes its commands as `plugin:<feature>|<command>` from a
//! module `features/<feature>.rs` + `features/<feature>/commands.rs` with an
//! `init()` that builds a [`tauri::plugin::TauriPlugin`]. Plugin commands are
//! always checked against the capability ACL, so a feature is registered in
//! three places, one line each (ADR 0002):
//!
//! 1. [`manifest::FEATURES`]: plugin name and command names, from which
//!    `build.rs` generates the plugin's ACL permissions;
//! 2. `lib.rs`: `.plugin(features::<feature>::init())`;
//! 3. `capabilities/default.json`: `"<feature>:default"`.
//!
//! The tests below fail when (1) and (3) drift apart, and prove that a
//! granted plugin command is callable through the real ACL.

pub mod manifest;
pub mod system;

#[cfg(test)]
mod tests {
    use super::manifest::FEATURES;

    /// The app's main capability file, which must grant every feature plugin.
    const DEFAULT_CAPABILITY: &str = include_str!("../capabilities/default.json");

    fn granted_permissions() -> Vec<String> {
        let capability: serde_json::Value =
            serde_json::from_str(DEFAULT_CAPABILITY).expect("capabilities/default.json is valid JSON");
        capability["permissions"]
            .as_array()
            .expect("capability has a permissions array")
            .iter()
            .filter_map(|permission| permission.as_str().map(str::to_string))
            .collect()
    }

    #[test]
    fn every_manifest_feature_is_granted_by_the_default_capability() {
        let granted = granted_permissions();
        for (name, _) in FEATURES {
            let permission = format!("{name}:default");
            assert!(
                granted.contains(&permission),
                "capabilities/default.json must grant \"{permission}\""
            );
        }
    }

    #[test]
    fn manifest_names_are_unique_and_list_commands() {
        let mut names: Vec<&str> = FEATURES.iter().map(|(name, _)| *name).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), FEATURES.len(), "duplicate feature in the manifest");
        assert!(FEATURES.iter().all(|(_, commands)| !commands.is_empty()));
    }

    #[test]
    fn system_feature_is_in_the_manifest() {
        assert!(FEATURES.iter().any(|(name, commands)| {
            *name == super::system::NAME && commands.contains(&"app_version")
        }));
    }
}

#[cfg(test)]
mod ipc_tests;
