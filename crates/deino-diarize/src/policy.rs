//! Per-meeting parameters, decided by the spike
//! (`docs/references/spike-diarization.md`, "Parameters by meeting length"):
//!
//! | Recording | step | `min_cluster_speech` | bounds from the calendar |
//! |---|---|---|---|
//! | under 2 min | 2 s | 3 s | at most n |
//! | 2–30 min | 2 s | 12 s | exactly n when trusted, else at most n |
//! | over 30 min | 3 s | 12 s | as above |
//!
//! The threshold stays anarlog's AMI-tuned 0.7045; the 1,800-window cap
//! grows the step further past about 90 minutes. A 5 s step breaks the
//! majority vote and is never used. `min_cluster_speech = 12 s` collapses
//! clips under about a minute, hence 3 s for short recordings.

use std::time::Duration;

use crate::config::{DiarizationConfig, RuntimeConfig, SpeakerBounds};

/// Recordings shorter than this are "short".
pub const SHORT_RECORDING: Duration = Duration::from_mins(2);
/// Recordings longer than this are "long".
pub const LONG_RECORDING: Duration = Duration::from_mins(30);
/// The window step up to [`LONG_RECORDING`].
pub const STEP: Duration = Duration::from_secs(2);
/// The window step past [`LONG_RECORDING`].
pub const LONG_STEP: Duration = Duration::from_secs(3);
/// `min_cluster_speech` for short recordings.
pub const SHORT_MIN_CLUSTER_SPEECH: Duration = Duration::from_secs(3);
/// `min_cluster_speech` otherwise.
pub const MIN_CLUSTER_SPEECH: Duration = Duration::from_secs(12);

/// How many speakers the calendar expects on the diarized track.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ExpectedSpeakers {
    /// No calendar information: the threshold decides.
    #[default]
    Unknown,
    /// No more than this many (the invitee list may include no-shows).
    AtMost(usize),
    /// This many, from a calendar the meeting trusts.
    Exactly(usize),
}

/// The diarizer configuration for a recording of `duration`.
pub fn config_for(duration: Duration, runtime: RuntimeConfig) -> DiarizationConfig {
    let step = if duration > LONG_RECORDING {
        LONG_STEP
    } else {
        STEP
    };
    let min_cluster_speech = if duration < SHORT_RECORDING {
        SHORT_MIN_CLUSTER_SPEECH
    } else {
        MIN_CLUSTER_SPEECH
    };
    DiarizationConfig {
        step,
        min_cluster_speech,
        runtime,
        ..DiarizationConfig::default()
    }
}

/// Community-1's settings: VBx clustering with `plda`, one speaker per
/// frame, and a 2 s window step. On the reference call, against the Python
/// engine: 3 s left 41 words on another speaker and added two stray splits;
/// 1 s left 36 but gave the call's opening greeting to the wrong person;
/// 2 s left 40, split lines exactly as often and got the opening right.
/// The step still grows past [`COMMUNITY_MAX_WINDOWS`] windows, so a long
/// call's clustering stays within a few hundred megabytes.
pub fn community_config(
    duration: Duration,
    runtime: RuntimeConfig,
    plda: std::sync::Arc<crate::vbx::Plda>,
) -> DiarizationConfig {
    DiarizationConfig {
        step: COMMUNITY_STEP,
        max_windows: COMMUNITY_MAX_WINDOWS,
        vbx: Some(plda),
        exclusive: true,
        ..config_for(duration, runtime)
    }
}

/// Community-1's window step.
pub const COMMUNITY_STEP: Duration = Duration::from_secs(2);
/// The most windows Community-1's settings run (2.5 hours at 2 s).
pub const COMMUNITY_MAX_WINDOWS: std::num::NonZeroUsize = match std::num::NonZeroUsize::new(4_500) {
    Some(count) => count,
    None => std::num::NonZeroUsize::MIN,
};

/// The speaker bounds for a recording of `duration`. Short recordings only
/// ever get an upper bound: someone invited may not have spoken yet.
pub fn bounds_for(duration: Duration, expected: ExpectedSpeakers) -> SpeakerBounds {
    match expected {
        ExpectedSpeakers::Unknown | ExpectedSpeakers::AtMost(0) | ExpectedSpeakers::Exactly(0) => {
            SpeakerBounds::default()
        }
        ExpectedSpeakers::Exactly(count) if duration >= SHORT_RECORDING => {
            SpeakerBounds::exact(count)
        }
        ExpectedSpeakers::Exactly(count) | ExpectedSpeakers::AtMost(count) => {
            SpeakerBounds::at_most(count)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINUTE: Duration = Duration::from_mins(1);

    #[test]
    fn the_step_grows_only_past_thirty_minutes() {
        let runtime = RuntimeConfig::default();
        assert_eq!(config_for(MINUTE, runtime.clone()).step, STEP);
        assert_eq!(config_for(30 * MINUTE, runtime.clone()).step, STEP);
        assert_eq!(config_for(31 * MINUTE, runtime).step, LONG_STEP);
    }

    #[test]
    fn short_recordings_need_less_speech_per_cluster() {
        let runtime = RuntimeConfig::default();
        let short = config_for(MINUTE + MINUTE / 2, runtime.clone());
        assert_eq!(short.min_cluster_speech, SHORT_MIN_CLUSTER_SPEECH);
        assert_eq!(
            config_for(2 * MINUTE, runtime).min_cluster_speech,
            MIN_CLUSTER_SPEECH
        );
    }

    #[test]
    fn the_threshold_and_runtime_are_kept() {
        let runtime = RuntimeConfig {
            accelerate_embedding: false,
            ..RuntimeConfig::default()
        };
        let config = config_for(10 * MINUTE, runtime.clone());
        assert!((config.threshold - 0.7045).abs() < f32::EPSILON);
        assert_eq!(config.runtime, runtime);
    }

    #[test]
    fn exact_counts_become_upper_bounds_on_short_recordings() {
        assert_eq!(
            bounds_for(MINUTE, ExpectedSpeakers::Exactly(3)),
            SpeakerBounds::at_most(3)
        );
        assert_eq!(
            bounds_for(5 * MINUTE, ExpectedSpeakers::Exactly(3)),
            SpeakerBounds::exact(3)
        );
        assert_eq!(
            bounds_for(5 * MINUTE, ExpectedSpeakers::AtMost(3)),
            SpeakerBounds::at_most(3)
        );
    }

    #[test]
    fn unknown_or_zero_counts_leave_the_threshold_in_charge() {
        assert_eq!(
            bounds_for(5 * MINUTE, ExpectedSpeakers::Unknown),
            SpeakerBounds::default()
        );
        assert_eq!(
            bounds_for(5 * MINUTE, ExpectedSpeakers::Exactly(0)),
            SpeakerBounds::default()
        );
    }
}
