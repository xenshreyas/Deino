//! The speakers section: naming speakers by voice (CAP-04).
//!
//! The thresholds are the diarization spike's, measured with WeSpeaker
//! ResNet34-LM on synthetic voices (`docs/references/spike-diarization.md`);
//! real cross-meeting scores run lower, so they live here to be calibrated
//! on real meetings without a release.

use serde::{Deserialize, Serialize};

use crate::settings::SettingsError;

/// The speakers section of `Settings`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct SpeakerSettings {
    /// Name speakers from voice profiles and the calendar after each
    /// meeting (the `identify_speakers` stage). On by default.
    pub auto_identify: bool,
    /// When a voice counts as a known person.
    pub thresholds: IdentifyThresholds,
}

impl Default for SpeakerSettings {
    fn default() -> Self {
        Self {
            auto_identify: true,
            thresholds: IdentifyThresholds::default(),
        }
    }
}

impl SpeakerSettings {
    /// Checks the thresholds' ranges and order.
    pub(super) fn validate(&self) -> Result<(), SettingsError> {
        self.thresholds.validate()
    }
}

/// Cosine similarities between a speaker's voice and a person's best
/// exemplar that decide what `identify_speakers` does.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct IdentifyThresholds {
    /// A calendar attendee is named automatically from this score...
    pub auto_assign: f32,
    /// ...anyone else only from this one (a stranger's voice must be clearer).
    pub auto_assign_non_attendee: f32,
    /// ...and only when the best score beats the runner-up by this much, in
    /// both directions (mutual best).
    pub margin: f32,
    /// Below automatic naming, from this score the UI asks "Is this Alex?".
    pub suggest: f32,
}

impl Default for IdentifyThresholds {
    fn default() -> Self {
        Self {
            auto_assign: 0.72,
            auto_assign_non_attendee: 0.77,
            margin: 0.08,
            suggest: 0.55,
        }
    }
}

impl IdentifyThresholds {
    /// Every value in `[0, 1]`, and `suggest ≤ autoAssign ≤ autoAssignNonAttendee`.
    fn validate(&self) -> Result<(), SettingsError> {
        let values = [
            self.auto_assign,
            self.auto_assign_non_attendee,
            self.margin,
            self.suggest,
        ];
        if !values.iter().all(|value| (0.0..=1.0).contains(value)) {
            return Err(SettingsError::invalid(
                "speakers.thresholds",
                "must be between 0 and 1",
            ));
        }
        let ordered =
            self.suggest <= self.auto_assign && self.auto_assign <= self.auto_assign_non_attendee;
        if !ordered {
            return Err(SettingsError::invalid(
                "speakers.thresholds",
                "must rise from suggest to autoAssign to autoAssignNonAttendee",
            ));
        }
        Ok(())
    }
}

crate::settings::register_section! {
    /// Naming speakers by voice (CAP-04).
    speakers: SpeakerSettings,
    order: 70,
    validate: SpeakerSettings::validate,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identification_is_on_with_the_spike_thresholds() {
        let settings = SpeakerSettings::default();
        assert!(settings.auto_identify);
        assert!((settings.thresholds.auto_assign - 0.72).abs() < f32::EPSILON);
        assert!(settings.validate().is_ok());
    }

    #[test]
    fn thresholds_out_of_range_or_order_are_rejected() {
        let mut settings = SpeakerSettings::default();
        settings.thresholds.margin = 1.5;
        assert!(settings.validate().is_err());
        let mut settings = SpeakerSettings::default();
        settings.thresholds.suggest = 0.9;
        assert!(settings.validate().is_err());
        let mut settings = SpeakerSettings::default();
        settings.thresholds.auto_assign_non_attendee = 0.6;
        assert!(settings.validate().is_err());
    }

    #[test]
    fn a_partial_section_keeps_the_other_defaults() {
        let settings: SpeakerSettings =
            serde_json::from_str(r#"{ "thresholds": { "autoAssign": 0.7 } }"#).unwrap();
        assert!(settings.auto_identify);
        assert!((settings.thresholds.auto_assign - 0.7).abs() < f32::EPSILON);
        assert!((settings.thresholds.suggest - 0.55).abs() < f32::EPSILON);
    }
}
