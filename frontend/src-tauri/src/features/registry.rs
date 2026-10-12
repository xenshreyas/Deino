//! The feature plugin registry: every feature plugin adds itself to
//! [`PLUGINS`] from its own `features/<feature>.rs`, so no shared list names it.
//!
//! ```ignore
//! // features/stats.rs, below `NAME`, `specta()` and `init()`:
//! super::registry::register_plugin!(Guarded);
//! ```
//!
//! `lib.rs` registers [`plugins`] with the app, each behind the app lock's
//! guard (PRV-02) with the [`Access`] it declared, and the bindings test
//! exports each one's [`FeaturePlugin::specta`] builder. The entries live in
//! a linker section (`linkme`), gathered when the binary is linked.

use linkme::distributed_slice;
use tauri::Wry;
use tauri::plugin::TauriPlugin;

/// Whether a plugin's commands run while Deino is locked (PRV-02, ADR 0050).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Access {
    /// Refused while locked: everything that reads or changes the library.
    /// The default, and the access of any plugin that is not registered.
    Guarded,
    /// Admitted while locked: the lock itself, app facts, settings (the theme
    /// and the gates), and recording with its meeting prompt.
    Open,
}

/// One feature plugin, as its own file registers it.
pub struct FeaturePlugin {
    /// The plugin name, the feature's `NAME`: `plugin:<name>|<command>`.
    pub name: &'static str,
    /// Builds the plugin for the app.
    pub init: fn() -> TauriPlugin<Wry>,
    /// The plugin's commands and events, for the TypeScript bindings.
    pub specta: fn() -> tauri_specta::Builder<Wry>,
    /// Whether its commands run while the app is locked.
    pub access: Access,
}

/// Every feature plugin compiled into this build, in no particular order.
#[allow(
    unsafe_code,
    reason = "linkme declares the slice over a linker section; no unsafe code of ours"
)]
#[distributed_slice]
pub static PLUGINS: [FeaturePlugin];

/// Adds the calling feature module to [`PLUGINS`] with the given [`Access`]
/// (`Guarded` or `Open`). The module must define `NAME`, `init()` and `specta()`.
macro_rules! register_plugin {
    ($access:ident) => {
        /// Registers this feature's plugin with the app (`features/registry.rs`).
        #[allow(
            unsafe_code,
            reason = "linkme places the entry in a linker section; no unsafe code of ours"
        )]
        #[linkme::distributed_slice($crate::features::registry::PLUGINS)]
        static PLUGIN: $crate::features::registry::FeaturePlugin =
            $crate::features::registry::FeaturePlugin {
                name: NAME,
                init,
                specta,
                access: $crate::features::registry::Access::$access,
            };
    };
}
pub(crate) use register_plugin;

/// Every registered plugin, sorted by name so startup order is the same in
/// every build. Plugins only mount their own commands, events and state, so
/// they do not depend on each other's order.
pub fn plugins() -> Vec<&'static FeaturePlugin> {
    let mut plugins: Vec<&'static FeaturePlugin> = PLUGINS.iter().collect();
    plugins.sort_by_key(|plugin| plugin.name);
    plugins
}

/// The access `name` registered with; [`Access::Guarded`] when no plugin of
/// that name is registered, so an unknown plugin is never open while locked.
pub fn access(name: &str) -> Access {
    PLUGINS
        .iter()
        .find(|plugin| plugin.name == name)
        .map_or(Access::Guarded, |plugin| plugin.access)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_names_are_unique() {
        let names: Vec<&str> = plugins().iter().map(|plugin| plugin.name).collect();
        let mut unique = names.clone();
        unique.dedup();
        assert_eq!(names, unique, "a plugin is registered twice");
    }

    #[test]
    fn every_plugin_builds_under_its_registered_name() {
        for plugin in plugins() {
            assert_eq!(
                tauri::plugin::Plugin::<Wry>::name(&(plugin.init)()),
                plugin.name
            );
        }
    }

    #[test]
    fn an_unknown_plugin_is_guarded() {
        assert_eq!(access("not-a-plugin"), Access::Guarded);
    }
}
