//! The `AudioSource` port: something that delivers one track's audio.
//!
//! A source pushes 48 kHz mono blocks, each stamped with the host time of its
//! first frame, into the track's [`RtSink`]. Real sources (the cpal
//! microphone, the Core Audio process tap) arrive in later cards; this card
//! ships [`MockAudioSource`], which plays generated signals in real time so
//! the whole recording path runs without hardware. In debug builds,
//! `DEINO_MOCK_AUDIO=1` selects it ([`mock_sources_from_env`]).

mod mock;
mod signal;

use std::sync::Arc;

pub use mock::MockAudioSource;
pub use signal::TestSignal;

use crate::clock::HostClock;
use crate::ring::RtSink;
use crate::units::Track;

/// The environment variable that swaps real capture for [`MockAudioSource`]s
/// in debug builds.
pub const MOCK_AUDIO_ENV: &str = "DEINO_MOCK_AUDIO";

/// Why a source could not start.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SourceError {
    /// The source is already running.
    #[error("the {0} source is already running")]
    AlreadyStarted(Track),
    /// The source's thread could not be started.
    #[error("could not start the {track} source")]
    Spawn {
        /// The track.
        track: Track,
        /// The OS error.
        #[source]
        source: std::io::Error,
    },
}

/// Delivers one track's audio into a ring.
pub trait AudioSource: std::fmt::Debug + Send {
    /// The track this source feeds.
    fn track(&self) -> Track;

    /// Starts delivering blocks into `sink`. The real-time side of a source
    /// may only call [`RtSink::push`].
    fn start(&mut self, sink: RtSink) -> Result<(), SourceError>;

    /// Stops delivering and releases the sink. Idempotent.
    fn stop(&mut self);
}

/// Whether a `DEINO_MOCK_AUDIO` value turns the mock on (`1` or `true`).
pub fn mock_audio_enabled(value: Option<&str>) -> bool {
    matches!(value.map(str::trim), Some("1" | "true"))
}

/// Mock sources for both tracks when `DEINO_MOCK_AUDIO` asks for them in a
/// debug build; `None` otherwise (release builds always capture for real).
pub fn mock_sources_from_env(clock: &Arc<dyn HostClock>) -> Option<[MockAudioSource; 2]> {
    let requested = mock_audio_enabled(std::env::var(MOCK_AUDIO_ENV).ok().as_deref());
    (cfg!(debug_assertions) && requested).then(|| MockAudioSource::meeting(clock))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_explicit_values_enable_the_mock() {
        assert!(mock_audio_enabled(Some("1")));
        assert!(mock_audio_enabled(Some(" true ")));
        for value in [None, Some(""), Some("0"), Some("false"), Some("yes")] {
            assert!(!mock_audio_enabled(value), "{value:?}");
        }
    }
}
