//! The feature plugins' ACL as `build.rs` generated it, and the guard tests
//! that keep it, the capabilities and the registered plugins in step.
//!
//! `build/features.rs` reads each feature's `NAME` and `collect_commands!`
//! and writes [`FEATURES`]; tauri-build turns the same list into the
//! plugins' permissions. `bindings.rs` checks that the generated bindings
//! invoke exactly these commands.

use std::collections::BTreeSet;
use std::path::Path;

use super::registry;

/// One feature plugin's ACL entry.
pub(crate) struct AclFeature {
    /// The plugin name.
    pub(crate) name: &'static str,
    /// Its commands, each allowed by `<name>:default`.
    pub(crate) commands: &'static [&'static str],
    /// True when the plugin exists in debug builds only.
    pub(crate) debug_only: bool,
}

include!(concat!(env!("OUT_DIR"), "/feature_acl.rs"));

/// The capability files tauri-build read: the hand-written ones and the
/// generated grants, as `build.rs` laid them out.
fn capabilities() -> Vec<serde_json::Value> {
    let dir = Path::new(env!("OUT_DIR")).join("capabilities");
    std::fs::read_dir(dir)
        .expect("build.rs wrote the capabilities")
        .map(|entry| std::fs::read_to_string(entry.unwrap().path()).unwrap())
        .map(|json| serde_json::from_str(&json).expect("capability files are valid JSON"))
        .collect()
}

fn granted(capability: &serde_json::Value) -> BTreeSet<String> {
    capability["permissions"]
        .as_array()
        .expect("capability has a permissions array")
        .iter()
        .filter_map(|permission| permission.as_str().map(str::to_owned))
        .collect()
}

#[test]
fn every_feature_is_granted_to_the_main_window() {
    let main: BTreeSet<String> = capabilities()
        .iter()
        .filter(|capability| capability["windows"] == serde_json::json!(["main"]))
        .flat_map(granted)
        .collect();
    for feature in FEATURES {
        let permission = format!("{}:default", feature.name);
        assert!(
            main.contains(&permission),
            "no capability grants \"{permission}\""
        );
    }
}

#[test]
fn hand_written_capabilities_grant_no_debug_only_feature() {
    // `capabilities/*.json` ship in release builds, where these plugins do not exist.
    let shipped = Path::new(env!("CARGO_MANIFEST_DIR")).join("capabilities");
    for entry in std::fs::read_dir(shipped).unwrap() {
        let json = std::fs::read_to_string(entry.unwrap().path()).unwrap();
        let permissions = granted(&serde_json::from_str(&json).unwrap());
        for feature in FEATURES.iter().filter(|feature| feature.debug_only) {
            let permission = format!("{}:default", feature.name);
            assert!(
                !permissions.contains(&permission),
                "{permission} ships in release builds"
            );
        }
    }
}

#[test]
fn every_feature_has_commands_and_a_registered_plugin() {
    let registered: BTreeSet<&str> = registry::plugins().iter().map(|p| p.name).collect();
    for feature in FEATURES {
        assert!(
            !feature.commands.is_empty(),
            "{} lists no commands",
            feature.name
        );
        assert!(
            registered.contains(feature.name),
            "{} is not registered",
            feature.name
        );
    }
}

/// The reference feature declares `#![cfg(debug_assertions)]` in its own file.
#[cfg(debug_assertions)]
#[test]
fn a_debug_only_feature_is_found_by_its_marker() {
    let example = FEATURES.iter().find(|f| f.name == super::_example::NAME);
    assert!(example.is_some_and(|feature| feature.debug_only));
}

#[cfg(not(debug_assertions))]
#[test]
fn release_builds_have_no_debug_only_feature() {
    assert!(FEATURES.iter().all(|feature| !feature.debug_only));
}
