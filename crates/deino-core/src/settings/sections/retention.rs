//! Retention (PRV-01): how long Deino keeps recordings, transcripts and
//! whole meetings, part of the privacy section.
//!
//! Each is "never" (kept forever) or a number of days. A transcript is kept
//! at least as long as its recording: the notes are written from the
//! transcript, and a recording without one could not be searched, read or
//! turned into notes again.

use serde::{Deserialize, Serialize};

use crate::settings::SettingsError;

/// How long something is kept.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum RetentionPeriod {
    /// Kept until the user deletes it.
    #[default]
    Never,
    /// Deleted 7 days after the meeting.
    Days7,
    /// Deleted 30 days after the meeting.
    Days30,
    /// Deleted 90 days after the meeting.
    Days90,
    /// Deleted 365 days after the meeting.
    Days365,
}

impl RetentionPeriod {
    /// The number of days, or `None` for [`Self::Never`].
    pub const fn days(self) -> Option<u16> {
        match self {
            Self::Never => None,
            Self::Days7 => Some(7),
            Self::Days30 => Some(30),
            Self::Days90 => Some(90),
            Self::Days365 => Some(365),
        }
    }

    /// True when this keeps things at least as long as `other`.
    pub const fn keeps_at_least(self, other: Self) -> bool {
        match (self.days(), other.days()) {
            (None, _) => true,
            (Some(_), None) => false,
            (Some(mine), Some(theirs)) => mine >= theirs,
        }
    }
}

/// The retention part of the privacy section. Everything is kept by default.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct RetentionSettings {
    /// Recordings (audio and screen video, clips included).
    pub audio: RetentionPeriod,
    /// Transcripts; never shorter than `audio`. The notes stay.
    pub transcript: RetentionPeriod,
    /// Whole meetings, which move to the trash.
    pub everything: RetentionPeriod,
}

impl RetentionSettings {
    /// Checks that the transcript is kept at least as long as the audio.
    pub(super) fn validate(self) -> Result<(), SettingsError> {
        if !self.transcript.keeps_at_least(self.audio) {
            return Err(SettingsError::invalid(
                "privacy.retention.transcript",
                "must be kept at least as long as the audio",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn everything_is_kept_by_default_and_in_old_files() {
        let old: RetentionSettings = serde_json::from_str("{}").unwrap();

        assert_eq!(old, RetentionSettings::default());
        assert_eq!(old.audio, RetentionPeriod::Never);
        assert!(old.validate().is_ok());
    }

    #[test]
    fn periods_are_camel_case_and_count_days() {
        let json = serde_json::to_string(&RetentionPeriod::Days365).unwrap();

        assert_eq!(json, r#""days365""#);
        assert_eq!(RetentionPeriod::Days30.days(), Some(30));
        assert_eq!(RetentionPeriod::Never.days(), None);
    }

    #[test]
    fn never_keeps_longer_than_any_number_of_days() {
        assert!(RetentionPeriod::Never.keeps_at_least(RetentionPeriod::Days365));
        assert!(RetentionPeriod::Never.keeps_at_least(RetentionPeriod::Never));
        assert!(!RetentionPeriod::Days365.keeps_at_least(RetentionPeriod::Never));
        assert!(RetentionPeriod::Days30.keeps_at_least(RetentionPeriod::Days30));
        assert!(!RetentionPeriod::Days7.keeps_at_least(RetentionPeriod::Days30));
    }

    #[test]
    fn a_transcript_shorter_than_the_audio_is_rejected() {
        let mut retention = RetentionSettings {
            audio: RetentionPeriod::Days30,
            transcript: RetentionPeriod::Days7,
            everything: RetentionPeriod::Never,
        };
        assert!(retention.validate().is_err());

        retention.audio = RetentionPeriod::Never;
        assert!(retention.validate().is_err(), "audio kept forever");

        retention.transcript = RetentionPeriod::Never;
        assert!(retention.validate().is_ok());
    }
}
