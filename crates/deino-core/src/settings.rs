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
pub use service::SettingsService;
pub use store::{JsonFileSettingsStore, SettingsStore};
