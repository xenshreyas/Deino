//! The onboarding section: how far first-run setup got.
//!
//! The legacy onboarding commands (`onboarding.rs` in the app) read and write
//! it until FND-11 rebuilds onboarding.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Where a model download stands.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type,
)]
#[serde(rename_all = "snake_case")]
pub enum ModelDownloadState {
    /// Not on disk.
    #[default]
    NotDownloaded,
    /// Downloading now.
    Downloading,
    /// On disk and ready.
    Downloaded,
}

/// The onboarding section of `Settings`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct OnboardingSettings {
    /// Setup has finished.
    pub completed: bool,
    /// The step the user is on, from 1.
    pub current_step: u8,
    /// The transcription model's download.
    pub parakeet: ModelDownloadState,
    /// The built-in summary model's download.
    pub summary: ModelDownloadState,
    /// The built-in summary model the user picked.
    pub selected_summary_model: Option<String>,
    /// When onboarding last saved its progress; `None` until it first does.
    pub updated_at: Option<DateTime<Utc>>,
}

impl Default for OnboardingSettings {
    fn default() -> Self {
        Self {
            completed: false,
            current_step: 1,
            parakeet: ModelDownloadState::NotDownloaded,
            summary: ModelDownloadState::NotDownloaded,
            selected_summary_model: None,
            updated_at: None,
        }
    }
}

impl OnboardingSettings {
    /// True once onboarding has saved progress at least once.
    pub fn started(&self) -> bool {
        self.updated_at.is_some()
    }
}
