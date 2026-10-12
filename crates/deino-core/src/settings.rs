//! Settings: every user preference as one typed, versioned document.
//!
//! - [`Settings`] is the document, one `#[serde(default)]` section per area
//!   ([`AiSettings`], [`AudioSettings`], [`UiSettings`], …), so a file
//!   written by an older build always loads.
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

mod ai;
mod api;
mod audio;
mod automations;
mod briefs;
mod calendar;
mod consent;
mod detection;
mod error;
mod events;
mod export;
#[cfg(any(test, feature = "test-support"))]
mod fake;
mod integrations;
pub mod legacy;
mod llm_access;
mod model;
mod network;
mod notifications;
mod onboarding;
mod organize;
mod patch;
mod personal;
mod profile;
mod retention;
mod service;
mod shortcuts;
mod speakers;
mod store;
mod templates;
mod transcription;
mod ui;

pub use ai::{AiSettings, CustomOpenAiSettings, LlmProvider};
pub use api::{ApiSettings, DEFAULT_API_PORT, MIN_API_PORT};
pub use audio::{AudioSettings, AutoStopOptions, PanelEdge, PanelPosition, RecordingSettings};
pub use automations::AutomationSettings;
pub use briefs::{BriefAudience, BriefSettings};
pub use calendar::{
    AutoRecordSettings, CalendarSettings, ConnectorSettings, MAX_CLIENT_ID_CHARS,
    MAX_HIDDEN_CALENDARS, ReminderSettings,
};
pub use consent::{
    ChatMessageSettings, ConsentSettings, DEFAULT_CHAT_MESSAGE, MAX_CHAT_MESSAGE_CHARS,
};
pub use detection::{
    AutoStartSettings, DetectionSettings, IgnoredApp, MAX_AUTO_START_SECONDS,
    MIN_AUTO_START_SECONDS,
};
pub use error::SettingsError;
pub use events::SettingsEvents;
pub use export::{DEFAULT_FILE_NAME, DEFAULT_SUBFOLDER, ExportSettings};
#[cfg(any(test, feature = "test-support"))]
pub use fake::{FakeSettingsStore, RecordedSettingsEvents};
pub use integrations::{IntegrationSettings, MIN_REDIRECT_PORT, OAuthAppSettings};
pub use legacy::{LegacyFileImport, LegacySqliteImport};
pub use llm_access::{LlmAccess, secret_for};
pub use model::{SETTINGS_VERSION, Settings};
pub use network::NetworkSettings;
pub use notifications::{MAX_REMINDER_MINUTES, NotificationPreferences, NotificationSettings};
pub use onboarding::{OnboardingSettings, OnboardingStep, StepOutcome};
pub use organize::OrganizeSettings;
pub use patch::SettingsPatch;
pub use personal::{
    AppLockSettings, DEFAULT_LOCK_IDLE_MINUTES, MAX_LOCK_IDLE_MINUTES, MIN_LOCK_IDLE_MINUTES,
    PrivacySettings,
};
pub use profile::{
    MAX_CONTEXT_CHARS, MAX_FOCUS_AREA_CHARS, MAX_FOCUS_AREAS, MAX_PROFILE_EMAILS,
    MAX_PROFILE_LINE_CHARS, MAX_WRITING_STYLE_CHARS, ProfileSettings,
};
pub use retention::{RetentionPeriod, RetentionSettings};
pub use service::SettingsService;
pub use shortcuts::{
    DEFAULT_ASK_HOTKEY, DEFAULT_QUICK_NOTE_HOTKEY, DEFAULT_RECORD_HOTKEY, ShortcutSettings,
};
pub use speakers::{IdentifyThresholds, SpeakerSettings};
pub use store::{JsonFileSettingsStore, SettingsStore};
pub use templates::TemplateSettings;
pub use transcription::{DEFAULT_PARAKEET_MODEL, TranscriptionEngine, TranscriptionSettings};
pub use ui::{
    BetaSettings, MAX_TRANSCRIPT_PANE_RATIO, MIN_TRANSCRIPT_PANE_RATIO, MeetingsListPrefs, Theme,
    UiSettings,
};
