//! Settings: every user preference as one typed, versioned document.
//!
//! - [`Settings`] is the document, one `#[serde(default)]` section per area
//!   ([`AiSettings`], [`AudioSettings`], [`UiSettings`], …), so a file
//!   written by an older build always loads. Each section lives in its own
//!   file in `settings/sections/` and registers itself with
//!   [`register_section!`]; `build.rs` generates the struct from those
//!   registrations (REG-02).
//! - [`SettingsService`] owns the current value: reads come from memory,
//!   and every change is validated, saved atomically through the
//!   [`SettingsStore`] port ([`JsonFileSettingsStore`] in the app) and
//!   announced through the [`SettingsEvents`] port.
//! - The UI changes settings with a [`SettingsPatch`], an RFC 7396 merge
//!   patch that is parsed back into [`Settings`] before anything is saved.
//! - [`legacy`] moves the stores that predate FND-06 into this one.
//!
//! Secrets are never settings: API keys live in the Keychain
//! (`deino-secrets`), named by [`LlmProvider::api_key`].

mod error;
mod events;
#[cfg(any(test, feature = "test-support"))]
mod fake;
pub mod legacy;
mod llm_access;
mod model;
mod patch;
mod section;
#[cfg(test)]
mod section_scan_tests;
mod sections;
mod service;
mod store;

pub use error::SettingsError;
pub use events::SettingsEvents;
#[cfg(any(test, feature = "test-support"))]
pub use fake::{FakeSettingsStore, RecordedSettingsEvents};
pub use legacy::{LegacyFileImport, LegacySqliteImport};
pub use llm_access::{LlmAccess, secret_for};
pub use model::{SETTINGS_VERSION, Settings};
pub use patch::SettingsPatch;
pub(crate) use section::{SettingsSection, register_section};
// Every section's types, so a new section is exported by its file alone.
pub use sections::*;
pub use personal::{
    AppLockSettings, DEFAULT_LOCK_IDLE_MINUTES, MAX_LOCK_IDLE_MINUTES, MIN_LOCK_IDLE_MINUTES,
    PrivacySettings,
};
pub use profile::{
    MAX_CONTEXT_CHARS, MAX_FOCUS_AREA_CHARS, MAX_FOCUS_AREAS, MAX_PROFILE_EMAILS,
    MAX_PROFILE_LINE_CHARS, MAX_WRITING_STYLE_CHARS, ProfileSettings,
};
pub use retention::{RetentionPeriod, RetentionSettings};
pub use screen::{CONSENT_REMINDERS, ScreenSettings};
pub use service::SettingsService;
pub use store::{JsonFileSettingsStore, SettingsStore};
