//! The `AudioSource` port: something that delivers one track's audio.
//!
//! A source pushes 48 kHz mono blocks, each stamped with the host time of its
//! first frame, into the track's [`RtSink`]. Real sources (the cpal
//! microphone, the Core Audio process tap) arrive in later cards; this card
//! ships [`MockAudioSource`], which plays generated signals in real time so
//! the whole recording path runs without hardware. In debug builds,
//! `DEINO_MOCK_AUDIO` selects it ([`MockAudio::from_env`]): `1` plays the
//! generated voices, any other value names a fixture meeting whose tracks
//! the composition root decodes (`deino_testkit::fixture::PcmTrack`) and
//! plays through [`TestSignal::from_pcm`].

mod mock;
mod signal;

pub use mock::MockAudioSource;
pub use signal::TestSignal;

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

/// What `DEINO_MOCK_AUDIO` asks the capture side to play instead of the
/// real devices.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MockAudio {
    /// The generated voices ([`MockAudioSource::meeting`]): `1` or `true`.
    Generated,
    /// The tracks of the fixture meeting with this ID.
    Fixture(String),
}

impl MockAudio {
    /// Parses a `DEINO_MOCK_AUDIO` value; unset, empty, `0` and `false` mean
    /// real capture.
    pub fn parse(value: Option<&str>) -> Option<Self> {
        match value.map(str::trim)? {
            "" | "0" | "false" => None,
            "1" | "true" => Some(Self::Generated),
            fixture => Some(Self::Fixture(fixture.to_owned())),
        }
    }

    /// The mock `DEINO_MOCK_AUDIO` selects in a debug build; always `None`
    /// in release builds, which capture for real.
    pub fn from_env() -> Option<Self> {
        if !cfg!(debug_assertions) {
            return None;
        }
        Self::parse(std::env::var(MOCK_AUDIO_ENV).ok().as_deref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_variable_selects_generated_or_fixture_audio() {
        assert_eq!(MockAudio::parse(Some("1")), Some(MockAudio::Generated));
        assert_eq!(MockAudio::parse(Some(" true ")), Some(MockAudio::Generated));
        assert_eq!(
            MockAudio::parse(Some("fx-standup")),
            Some(MockAudio::Fixture("fx-standup".into()))
        );
        for value in [None, Some(""), Some("0"), Some("false")] {
            assert_eq!(MockAudio::parse(value), None, "{value:?}");
        }
    }
}
