//! The `AudioSource` port: something that delivers one track's audio.
//!
//! A started source pushes mono blocks at its device's native rate, each
//! stamped with the host time of its first frame, into a capture ring it
//! builds for that rate ([`crate::ring::capture_ring`]), and hands back the
//! ring's reading end in a [`SourceStream`]. The conditioner
//! ([`crate::capture`]) resamples to 48 kHz once and feeds the track writer.
//!
//! Implementations: the cpal microphone and the Core Audio process tap
//! (`crate::capture`), and [`MockAudioSource`], which plays generated or
//! fixture signals in real time so the whole recording path runs without
//! hardware. In debug builds `DEINO_MOCK_AUDIO` selects the mock
//! ([`MockAudio::from_env`]): `1` plays the generated voices, any other value
//! names a fixture meeting whose tracks the composition root decodes
//! (`deino_testkit::fixture::PcmTrack`) and plays through
//! [`TestSignal::from_pcm`].

mod mock;
mod signal;

pub use mock::{MOCK_BLOCK, MOCK_SYSTEM_DELAY, MockAudioSource, MockSources};
pub use signal::TestSignal;

use crate::ring::RtSource;
use crate::units::{SampleRate, Track};

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
    /// No device can feed the track (no microphone, or no system audio).
    #[error("no device is available for the {0} track")]
    NoDevice(Track),
    /// The device did not start within its timeout.
    #[error("the {0} device did not start in time")]
    Timeout(Track),
    /// The device or the OS refused to open the stream.
    #[error("could not open the {track} device")]
    Open {
        /// The track.
        track: Track,
        /// The device or OS error.
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
}

/// What a started source delivers into.
#[derive(Debug)]
pub struct SourceStream {
    /// The device's native rate; every block in `blocks` is at this rate.
    pub rate: SampleRate,
    /// The reading end of the capture ring the source pushes into.
    pub blocks: RtSource,
}

/// Delivers one track's audio into a capture ring.
pub trait AudioSource: std::fmt::Debug + Send {
    /// The track this source feeds.
    fn track(&self) -> Track;

    /// Opens the device and starts delivering. The real-time side of a
    /// source may only call [`crate::ring::BlockSink::push_frames`] (or
    /// `push`) and touch atomics. Starting a stopped source again opens a
    /// fresh stream.
    fn start(&mut self) -> Result<SourceStream, SourceError>;

    /// Stops delivering and releases the device. Idempotent.
    fn stop(&mut self);

    /// False once the device has reported an error or disappeared, so the
    /// capture supervisor restarts it. Sources that cannot fail stay alive.
    fn is_alive(&self) -> bool {
        true
    }

    /// True when the open device is a Bluetooth headset in call mode
    /// ([`crate::capture::devices::is_hands_free`]).
    fn hands_free(&self) -> bool {
        false
    }
}

/// Builds a recording's sources; the composition root picks real devices or
/// [`MockAudioSource`]s.
pub trait SourceFactory: std::fmt::Debug + Send + Sync {
    /// The microphone source (it picks its device when started).
    fn microphone(&self) -> Box<dyn AudioSource>;

    /// The system-audio source.
    fn system(&self) -> Box<dyn AudioSource>;
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
