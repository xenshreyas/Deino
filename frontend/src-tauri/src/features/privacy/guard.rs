//! The IPC guard: while Deino is locked, commands that read the library are
//! refused with `permission_denied` (PRV-02).
//!
//! Every feature plugin is registered wrapped in [`Guarded`], a
//! [`Plugin`] that forwards everything to the real plugin and checks the
//! lock in [`Plugin::extend_api`], the one place every plugin command
//! passes through. Legacy commands are wrapped the same way by
//! [`guard_legacy`]. Each plugin declares its [`Access`] where it registers
//! (`features/registry.rs`): recording keeps working while locked, so its
//! plugin, the meeting prompt (`detection`) and what the lock screen itself
//! needs are [`Access::Open`]; everything else is refused.

use std::sync::Arc;

use deino_core::lock::LockService;
use deino_domain::{AppError, ErrorCode};
use serde_json::Value as JsonValue;
use tauri::ipc::Invoke;
use tauri::plugin::Plugin;
use tauri::webview::PageLoadPayload;
use tauri::{AppHandle, Manager, RunEvent, Runtime, Url, Webview, Window};

use crate::features::registry::Access;

/// Legacy commands that read or change meetings, refused while locked.
const LEGACY_LIBRARY_COMMANDS: [&str; 4] = [
    "api_get_meeting_metadata",
    "api_get_meeting_transcripts",
    "api_delete_meeting",
    "open_meeting_folder",
];

/// What a locked app answers instead of the library.
pub(crate) fn locked_error() -> AppError {
    AppError::new(ErrorCode::PermissionDenied, "Deino is locked")
}

/// True when a plugin with `access` may run commands while the app is `locked`.
pub(crate) fn admits_plugin(locked: bool, access: Access) -> bool {
    !locked || access == Access::Open
}

/// True when the legacy `command` may run while the app is `locked`.
pub(crate) fn admits_legacy(locked: bool, command: &str) -> bool {
    !locked || !LEGACY_LIBRARY_COMMANDS.contains(&command)
}

/// True when the app has a lock service and it is locked. Before the
/// service is wired (early startup, tests) nothing is locked.
fn is_locked<R: Runtime>(app: &AppHandle<R>) -> bool {
    app.try_state::<Arc<LockService>>()
        .is_some_and(|lock| lock.is_locked())
}

/// Refuses `invoke` when `admits` says no for the current lock; otherwise
/// hands it back to be run.
fn refuse_if_locked<R: Runtime>(
    invoke: Invoke<R>,
    admits: impl Fn(bool) -> bool,
) -> Option<Invoke<R>> {
    let locked = is_locked(invoke.message.webview_ref().app_handle());
    if admits(locked) {
        return Some(invoke);
    }
    log::debug!(target: "lock", "refused {} while locked", invoke.message.command());
    invoke.resolver.reject(locked_error());
    None
}

/// Wraps the legacy invoke handler with the lock check.
pub(crate) fn guard_legacy<R: Runtime>(
    handler: impl Fn(Invoke<R>) -> bool + Send + Sync + 'static,
) -> impl Fn(Invoke<R>) -> bool + Send + Sync + 'static {
    move |invoke| {
        let command = invoke.message.command().to_owned();
        match refuse_if_locked(invoke, |locked| admits_legacy(locked, &command)) {
            Some(invoke) => handler(invoke),
            None => true,
        }
    }
}

/// A feature plugin behind the lock check.
pub(crate) struct Guarded<P> {
    inner: P,
    access: Access,
}

impl<P> Guarded<P> {
    /// Wraps `inner`, whose commands run while locked only when `access` is
    /// [`Access::Open`].
    pub(crate) fn new(inner: P, access: Access) -> Self {
        Self { inner, access }
    }
}

impl<R: Runtime, P: Plugin<R>> Plugin<R> for Guarded<P> {
    fn name(&self) -> &'static str {
        self.inner.name()
    }

    fn initialize(
        &mut self,
        app: &AppHandle<R>,
        config: JsonValue,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.inner.initialize(app, config)
    }

    // `initialization_script_2` keeps its default, which wraps this one:
    // its type is not public, and no feature plugin sets a script.
    fn initialization_script(&self) -> Option<String> {
        self.inner.initialization_script()
    }

    fn window_created(&mut self, window: Window<R>) {
        self.inner.window_created(window);
    }

    fn webview_created(&mut self, webview: Webview<R>) {
        self.inner.webview_created(webview);
    }

    fn on_navigation(&mut self, webview: &Webview<R>, url: &Url) -> bool {
        self.inner.on_navigation(webview, url)
    }

    fn on_page_load(&mut self, webview: &Webview<R>, payload: &PageLoadPayload<'_>) {
        self.inner.on_page_load(webview, payload);
    }

    fn on_event(&mut self, app: &AppHandle<R>, event: &RunEvent) {
        self.inner.on_event(app, event);
    }

    fn extend_api(&mut self, invoke: Invoke<R>) -> bool {
        let access = self.access;
        match refuse_if_locked(invoke, |locked| admits_plugin(locked, access)) {
            Some(invoke) => self.inner.extend_api(invoke),
            None => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::features::registry::{self, plugins};

    #[test]
    fn guarded_plugins_are_refused_only_while_locked() {
        assert!(admits_plugin(false, Access::Guarded));
        assert!(!admits_plugin(true, Access::Guarded));
        assert!(admits_plugin(true, Access::Open));
    }

    #[test]
    fn library_plugins_are_guarded() {
        for plugin in ["meetings", "notes", "transcript", "search", "people"] {
            assert_eq!(registry::access(plugin), Access::Guarded, "{plugin}");
        }
    }

    /// The PRV-02 policy: only the lock itself, app facts, settings and
    /// recording with its prompt work while locked. Opening another plugin
    /// is a privacy decision, so it changes this test on purpose.
    #[test]
    fn recording_and_the_lock_itself_are_the_only_open_plugins() {
        let open: Vec<&str> = plugins()
            .into_iter()
            .filter(|plugin| plugin.access == Access::Open)
            .map(|plugin| plugin.name)
            .collect();
        assert_eq!(
            open,
            ["detection", "privacy", "recording", "settings", "system"]
        );
    }

    #[test]
    fn legacy_meeting_reads_are_refused_while_locked() {
        assert!(!admits_legacy(true, "api_get_meeting_transcripts"));
        assert!(admits_legacy(false, "api_get_meeting_transcripts"));
        assert!(admits_legacy(true, "get_audio_devices"));
    }
}
