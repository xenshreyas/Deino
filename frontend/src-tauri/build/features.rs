//! The feature plugins' ACL, read from each feature's own files (REG-01).
//!
//! A feature plugin with commands is a `src/features/<module>.rs` beside a
//! `src/features/<module>/commands.rs`. Its plugin name is the module's
//! `pub const NAME: &str = "..";`, its commands are what its
//! `collect_commands![..]` lists (the last path segment of each), and it is
//! debug-only when its file declares `#![cfg(debug_assertions)]`.
//!
//! From that one source this module
//! - registers every plugin with tauri-build, which generates its `allow-*`
//!   and `deny-*` permissions and a `<name>:default` set allowing them all;
//! - writes the capabilities tauri-build reads to `$OUT_DIR/capabilities/`:
//!   the hand-written `capabilities/*.json`, plus `features.json` granting the
//!   `main` window every feature (and, in debug builds, `dev-features.json`
//!   for the debug-only ones);
//! - writes `$OUT_DIR/feature_acl.rs`, the same list for the guard tests.

use std::fs;
use std::path::{Path, PathBuf};

/// The feature modules.
const FEATURES_DIR: &str = "src/features";
/// The hand-written capabilities: windows, core permissions and the grants a
/// window other than `main` needs.
const CAPABILITIES_DIR: &str = "capabilities";
/// Marks a feature file whose module only exists in debug builds.
const DEBUG_ONLY: &str = "#![cfg(debug_assertions)]";

/// One feature plugin's ACL.
pub struct Feature {
    /// The plugin name, e.g. `action-items`.
    pub name: String,
    /// Its command names, in `collect_commands!` order.
    pub commands: Vec<String>,
    /// True when the plugin is compiled into debug builds only.
    pub debug_only: bool,
}

/// Every feature plugin with commands under `src/features/`, sorted by name.
pub fn scan() -> Result<Vec<Feature>, String> {
    let mut features = Vec::new();
    for module in feature_modules()? {
        let source = read(&Path::new(FEATURES_DIR).join(format!("{module}.rs")))?;
        let commands = read(&Path::new(FEATURES_DIR).join(&module).join("commands.rs"))?;
        features.push(Feature {
            name: plugin_name(&source)
                .ok_or_else(|| format!("features/{module}.rs has no `pub const NAME`"))?,
            commands: command_names(&commands).ok_or_else(|| {
                format!("features/{module}/commands.rs has no `collect_commands![..]`")
            })?,
            debug_only: source.lines().any(|line| line.trim() == DEBUG_ONLY),
        });
    }
    features.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(features)
}

/// The modules under `src/features/` that have a `commands.rs`.
fn feature_modules() -> Result<Vec<String>, String> {
    let entries = fs::read_dir(FEATURES_DIR).map_err(|e| format!("{FEATURES_DIR}: {e}"))?;
    let mut modules: Vec<String> = entries
        .filter_map(Result::ok)
        .filter(|entry| entry.path().join("commands.rs").is_file())
        .filter_map(|entry| entry.file_name().into_string().ok())
        .collect();
    modules.sort();
    Ok(modules)
}

fn read(path: &Path) -> Result<String, String> {
    fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))
}

/// The string in `pub const NAME: &str = "<name>";`.
fn plugin_name(source: &str) -> Option<String> {
    let line = source
        .lines()
        .find(|line| line.trim_start().starts_with("pub const NAME: &str ="))?;
    let quoted = line.split('"').nth(1)?;
    Some(quoted.to_owned())
}

/// The command names in `collect_commands![a, b, module::c]`, ignoring comments.
fn command_names(source: &str) -> Option<Vec<String>> {
    let start = source.find("collect_commands![")? + "collect_commands![".len();
    let rest = source.get(start..)?;
    let list = rest.get(..rest.find(']')?)?;
    let names = list
        .lines()
        .map(|line| line.split("//").next().unwrap_or_default())
        .flat_map(|line| line.split(','))
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(|item| item.rsplit("::").next().unwrap_or(item).to_owned())
        .collect();
    Some(names)
}

/// Registers every feature in `features` as an inline plugin.
pub fn attributes(features: &[Feature], capabilities: &'static str) -> tauri_build::Attributes {
    features
        .iter()
        .fold(tauri_build::Attributes::new(), |attributes, feature| {
            let commands: Vec<&'static str> = feature
                .commands
                .iter()
                .map(|command| &*Box::leak(command.clone().into_boxed_str()))
                .collect();
            attributes.plugin(
                Box::leak(feature.name.clone().into_boxed_str()),
                tauri_build::InlinedPlugin::new()
                    .commands(Box::leak(commands.into_boxed_slice()))
                    .default_permission(tauri_build::DefaultPermissionRule::AllowAllCommands),
            )
        })
        .capabilities_path_pattern(capabilities)
}

/// Copies the hand-written capabilities into `out_dir/capabilities/` beside
/// the generated grants, and returns the glob tauri-build reads them with.
pub fn write_capabilities(features: &[Feature], out_dir: &Path) -> Result<&'static str, String> {
    let dir = out_dir.join("capabilities");
    if dir.exists() {
        fs::remove_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    for source in json_files(Path::new(CAPABILITIES_DIR))? {
        let name = source.file_name().ok_or("a capability file has no name")?;
        fs::copy(&source, dir.join(name)).map_err(|e| format!("{}: {e}", source.display()))?;
    }
    let (debug, release): (Vec<&Feature>, Vec<&Feature>) =
        features.iter().partition(|feature| feature.debug_only);
    write_grant(&dir.join("features.json"), "features", &release)?;
    if !debug.is_empty() {
        write_grant(&dir.join("dev-features.json"), "dev-features", &debug)?;
    }
    let pattern = dir.join("*.json").display().to_string();
    Ok(Box::leak(pattern.into_boxed_str()))
}

/// The `.json` files directly in `dir`.
fn json_files(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let entries = fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    Ok(entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .collect())
}

/// A capability granting the `main` window each feature's `<name>:default`.
fn write_grant(path: &Path, identifier: &str, features: &[&Feature]) -> Result<(), String> {
    let permissions: Vec<String> = features
        .iter()
        .map(|feature| format!("{}:default", feature.name))
        .collect();
    let capability = serde_json::json!({
        "identifier": identifier,
        "description": "Generated by build.rs (build/features.rs): every feature plugin's commands for the main window",
        "windows": ["main"],
        "permissions": permissions,
    });
    let json = serde_json::to_string_pretty(&capability).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| format!("{}: {e}", path.display()))
}

/// Writes `out_dir/feature_acl.rs`: `FEATURES`, the scanned list, for the
/// app's guard tests (`features/acl.rs`).
pub fn write_acl(features: &[Feature], out_dir: &Path) -> Result<(), String> {
    let entries = features.iter().map(|feature| {
        format!(
            "    AclFeature {{ name: {:?}, commands: &{:?}, debug_only: {} }},",
            feature.name, feature.commands, feature.debug_only
        )
    });
    let lines: Vec<String> = [
        "// Generated by build.rs (build/features.rs). Do not edit.".to_owned(),
        "/// Every feature plugin with commands, as `build.rs` found them, by name.".to_owned(),
        "pub const FEATURES: &[AclFeature] = &[".to_owned(),
    ]
    .into_iter()
    .chain(entries)
    .chain(["];".to_owned()])
    .collect();
    let path = out_dir.join("feature_acl.rs");
    fs::write(&path, lines.join("\n") + "\n").map_err(|e| format!("{}: {e}", path.display()))
}

/// Re-runs the build script when a feature's name, commands or build
/// flavour can have changed: `features.rs` (a new or removed module), each
/// feature file and its `commands.rs`, and the hand-written capabilities.
pub fn rerun_if_changed() -> Result<(), String> {
    println!("cargo:rerun-if-changed={FEATURES_DIR}.rs");
    println!("cargo:rerun-if-changed={CAPABILITIES_DIR}");
    for module in feature_modules()? {
        println!("cargo:rerun-if-changed={FEATURES_DIR}/{module}.rs");
        println!("cargo:rerun-if-changed={FEATURES_DIR}/{module}/commands.rs");
    }
    Ok(())
}
