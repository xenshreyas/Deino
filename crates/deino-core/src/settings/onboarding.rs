//! The onboarding section: how far first-run setup got (FND-11).
//!
//! Onboarding is six steps, each of which the user finishes or skips. The
//! UI's step machine (`features/onboarding/machine.ts`) moves between them and
//! saves every move here as a settings patch, so setup resumes where it
//! stopped, also after a restart, and Settings can reopen it later. Model
//! downloads and permissions are never copied in: the UI reads them live.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// One onboarding step, in the order the flow shows them.
#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    specta::Type,
)]
#[serde(rename_all = "snake_case")]
pub enum OnboardingStep {
    /// What Deino is and how it keeps data on the Mac.
    #[default]
    Welcome,
    /// Microphone, system audio, notifications, Accessibility and Calendar.
    Permissions,
    /// The local models and the provider for notes.
    AiSetup,
    /// A calendar for upcoming meetings.
    Calendar,
    /// Who the user is, for every AI prompt (NOT-05).
    Profile,
    /// A short test recording.
    TryIt,
}

/// How the user left a step.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum StepOutcome {
    /// The user went through it.
    Done,
    /// The user skipped it; Settings can resume it.
    Skipped,
}

/// The onboarding section of `Settings`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct OnboardingSettings {
    /// Setup has finished; the app opens straight to Home.
    pub completed: bool,
    /// The step the flow shows next.
    pub step: OnboardingStep,
    /// How each visited step was left; unvisited steps are absent.
    pub outcomes: BTreeMap<OnboardingStep, StepOutcome>,
}

impl OnboardingSettings {
    /// Onboarding for someone who finished setup before FND-11 (the legacy
    /// import): completed, with no record of individual steps.
    pub fn finished() -> Self {
        Self {
            completed: true,
            ..Self::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn a_fresh_install_starts_at_the_welcome_step() {
        let onboarding = OnboardingSettings::default();

        assert!(!onboarding.completed);
        assert_eq!(onboarding.step, OnboardingStep::Welcome);
        assert!(onboarding.outcomes.is_empty());
    }

    #[test]
    fn progress_uses_the_names_the_ui_sends() {
        let onboarding = OnboardingSettings {
            completed: false,
            step: OnboardingStep::TryIt,
            outcomes: BTreeMap::from([
                (OnboardingStep::AiSetup, StepOutcome::Done),
                (OnboardingStep::Calendar, StepOutcome::Skipped),
            ]),
        };

        let value = serde_json::to_value(&onboarding).unwrap();

        assert_eq!(
            value,
            json!({
                "completed": false,
                "step": "try_it",
                "outcomes": { "ai_setup": "done", "calendar": "skipped" },
            })
        );
        assert_eq!(
            serde_json::from_value::<OnboardingSettings>(value).unwrap(),
            onboarding
        );
    }

    #[test]
    fn a_settings_file_from_before_fnd_11_keeps_only_completion() {
        let old = json!({
            "completed": true, "currentStep": 4, "parakeet": "downloaded",
            "summary": "downloaded", "selectedSummaryModel": null, "updatedAt": null,
        });

        let onboarding: OnboardingSettings = serde_json::from_value(old).unwrap();

        assert_eq!(onboarding, OnboardingSettings::finished());
    }
}
