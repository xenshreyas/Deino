//! What retention removed from a meeting, what one sweep did, and when a
//! period's items expire.

use chrono::{DateTime, TimeDelta, Utc};
use deino_domain::MeetingId;
use serde::Serialize;

use crate::settings::RetentionPeriod;

/// What retention removed from one meeting, for the meeting page.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct MeetingRetention {
    /// When the recording files were deleted; `null` while they are kept.
    pub audio_deleted_at: Option<DateTime<Utc>>,
    /// When the transcript was deleted; `null` while it is kept. The notes
    /// stay, but cannot be regenerated.
    pub transcript_deleted_at: Option<DateTime<Utc>>,
}

/// The meetings one sweep changed, each list in the order it acted.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SweepOutcome {
    /// Moved to the trash ("everything" expired).
    pub trashed: Vec<MeetingId>,
    /// Transcripts deleted.
    pub transcripts: Vec<MeetingId>,
    /// Recordings deleted.
    pub recordings: Vec<MeetingId>,
}

impl SweepOutcome {
    /// True when the sweep changed nothing.
    pub fn is_empty(&self) -> bool {
        self.trashed.is_empty() && self.transcripts.is_empty() && self.recordings.is_empty()
    }
}

/// Meetings saved at or before this instant have outlived `period` at `now`;
/// `None` when the period keeps them forever.
pub fn cutoff(now: DateTime<Utc>, period: RetentionPeriod) -> Option<DateTime<Utc>> {
    period
        .days()
        .map(|days| now - TimeDelta::days(i64::from(days)))
}

#[cfg(test)]
mod tests {
    use deino_testkit::at;

    use super::*;

    #[test]
    fn the_cutoff_is_the_period_before_now() {
        let now = at("2026-10-31T09:00:00Z");

        assert_eq!(
            cutoff(now, RetentionPeriod::Days30),
            Some(at("2026-10-01T09:00:00Z"))
        );
        assert_eq!(cutoff(now, RetentionPeriod::Never), None);
    }

    #[test]
    fn the_status_is_camel_case() {
        let status = MeetingRetention {
            audio_deleted_at: Some(at("2026-10-01T09:00:00Z")),
            transcript_deleted_at: None,
        };

        assert_eq!(
            serde_json::to_string(&status).unwrap(),
            r#"{"audioDeletedAt":"2026-10-01T09:00:00Z","transcriptDeletedAt":null}"#
        );
    }
}
