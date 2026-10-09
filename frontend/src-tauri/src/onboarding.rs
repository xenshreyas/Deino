//! The legacy onboarding commands, stored in the typed settings.
//!
//! The onboarding UI (`contexts/OnboardingContext.tsx`) still speaks the
//! original `snake_case` `OnboardingStatus` through these four app commands.
//! Since FND-06 they read and write `Settings::onboarding` instead of the
//! `onboarding-status.json` store, which the legacy import moved. FND-11
//! replaces them with a typed onboarding plugin.

#![allow(
    clippy::let_underscore_must_use,
    reason = "#[tauri::command] on an async fn with a State<'_> argument expands to a `let _` type check of its Result"
)]

use std::sync::Arc;

use deino_core::settings::{
    LlmProvider, ModelDownloadState, OnboardingSettings, Settings, SettingsService,
    TranscriptionEngine, TranscriptionSettings,
};
use deino_domain::{Clock, SystemClock};
use serde::{Deserialize, Serialize};
use tauri::State;

/// The onboarding step the flow ends on.
const LAST_STEP: u8 = 4;

/// Onboarding progress in the shape the onboarding UI sends and expects.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OnboardingStatus {
    /// The status format, always "1.0".
    pub version: String,
    /// Setup has finished.
    pub completed: bool,
    /// The step the user is on, from 1.
    pub current_step: u8,
    /// Where the model downloads stand.
    pub model_status: ModelStatus,
    /// When the status was last saved, RFC 3339.
    pub last_updated: String,
}

/// Model download states, as the onboarding UI names them.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct ModelStatus {
    /// The transcription model.
    pub parakeet: ModelDownloadState,
    /// The built-in summary model.
    pub summary: ModelDownloadState,
    /// The summary model the user picked.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_summary_model: Option<String>,
}

impl OnboardingStatus {
    fn from_settings(onboarding: &OnboardingSettings) -> Self {
        Self {
            version: "1.0".to_owned(),
            completed: onboarding.completed,
            current_step: onboarding.current_step,
            model_status: ModelStatus {
                parakeet: onboarding.parakeet,
                summary: onboarding.summary,
                selected_summary_model: onboarding.selected_summary_model.clone(),
            },
            last_updated: onboarding
                .updated_at
                .map(|at| at.to_rfc3339())
                .unwrap_or_default(),
        }
    }

    fn into_settings(self, clock: &dyn Clock) -> OnboardingSettings {
        OnboardingSettings {
            completed: self.completed,
            current_step: self.current_step,
            parakeet: self.model_status.parakeet,
            summary: self.model_status.summary,
            selected_summary_model: self.model_status.selected_summary_model,
            updated_at: Some(clock.now()),
        }
    }
}

/// The saved status, or `None` before onboarding first saves one.
#[tauri::command]
pub async fn get_onboarding_status(
    settings: State<'_, Arc<SettingsService>>,
) -> Result<Option<OnboardingStatus>, String> {
    let onboarding = settings.get().await.onboarding;
    Ok(onboarding
        .started()
        .then(|| OnboardingStatus::from_settings(&onboarding)))
}

/// Saves the onboarding progress.
#[tauri::command]
pub async fn save_onboarding_status_cmd(
    settings: State<'_, Arc<SettingsService>>,
    status: OnboardingStatus,
) -> Result<(), String> {
    let onboarding = status.into_settings(&SystemClock);
    update(&settings, |s| s.onboarding = onboarding).await
}

/// Forgets the onboarding progress, so onboarding starts again.
#[tauri::command]
pub async fn reset_onboarding_status_cmd(
    settings: State<'_, Arc<SettingsService>>,
) -> Result<(), String> {
    update(&settings, |s| s.onboarding = OnboardingSettings::default()).await
}

/// Finishes onboarding with the built-in summary `model` and Parakeet
/// transcription, in one atomic settings change.
#[tauri::command]
pub async fn complete_onboarding(
    settings: State<'_, Arc<SettingsService>>,
    model: String,
) -> Result<(), String> {
    let now = SystemClock.now();
    update(&settings, |s| complete(s, model, now)).await?;
    log::info!(target: "onboarding", "onboarding completed");
    Ok(())
}

fn complete(settings: &mut Settings, model: String, now: chrono::DateTime<chrono::Utc>) {
    settings.ai.provider = LlmProvider::BuiltInAi;
    settings.ai.model.clone_from(&model);
    settings
        .ai
        .provider_models
        .insert(LlmProvider::BuiltInAi, model.clone());
    settings.transcription = TranscriptionSettings {
        engine: TranscriptionEngine::Parakeet,
        ..TranscriptionSettings::default()
    };
    settings.onboarding = OnboardingSettings {
        completed: true,
        current_step: LAST_STEP,
        parakeet: ModelDownloadState::Downloaded,
        summary: ModelDownloadState::Downloaded,
        selected_summary_model: Some(model),
        updated_at: Some(now),
    };
}

/// Applies `change` and reports failure as the legacy commands always have.
async fn update(
    settings: &SettingsService,
    change: impl FnOnce(&mut Settings) + Send,
) -> Result<(), String> {
    settings
        .update(change)
        .await
        .map(drop)
        .map_err(|error| format!("Failed to save onboarding status: {error}"))
}

#[cfg(test)]
mod tests {
    use deino_testkit::{FakeClock, at};

    use super::*;

    #[test]
    fn a_status_from_the_ui_round_trips_through_the_settings() {
        let status: OnboardingStatus = serde_json::from_str(
            r#"{"version":"1.0","completed":false,"current_step":2,
                "model_status":{"parakeet":"downloading","summary":"not_downloaded"},
                "last_updated":"2026-05-30T00:00:00Z"}"#,
        )
        .unwrap();
        let clock = FakeClock::new(at("2026-10-09T08:00:00Z"));

        let onboarding = status.into_settings(&clock);
        let back = OnboardingStatus::from_settings(&onboarding);

        assert_eq!(onboarding.parakeet, ModelDownloadState::Downloading);
        assert_eq!(back.current_step, 2);
        assert_eq!(back.last_updated, "2026-10-09T08:00:00+00:00");
        assert_eq!(back.model_status.selected_summary_model, None);
    }

    #[test]
    fn completing_switches_to_the_built_in_model_and_parakeet() {
        let mut settings = Settings::default();
        settings.transcription.engine = TranscriptionEngine::Whisper;

        complete(
            &mut settings,
            "qwen3.5:2b".into(),
            at("2026-10-09T08:00:00Z"),
        );

        assert_eq!(settings.ai.provider, LlmProvider::BuiltInAi);
        assert_eq!(settings.ai.model, "qwen3.5:2b");
        assert_eq!(settings.transcription, TranscriptionSettings::default());
        assert!(settings.onboarding.completed && settings.onboarding.started());
    }
}
