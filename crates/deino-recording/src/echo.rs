//! Echo and bleed: removing the call's audio from the microphone.
//!
//! On laptop speakers the microphone hears the other side of the call, so
//! the Me lane would transcribe "Them" a second time. The primary defence
//! is acoustic echo cancellation ([`SonoraAec`], WebRTC's AEC3; ADR 0026):
//! the system track is the reference (far-end) stream and the microphone
//! the capture stream, processed in [`FRAME`]-sample (10 ms) frames on the
//! live pipeline's thread, never on a real-time callback. What residue
//! survives is caught on the text side by the holdback ([`crate::bleed`]).
//!
//! On headphones the microphone hears no playback, so there is nothing to
//! cancel: [`EchoMode::Isolated`] skips both defences (anarlog's
//! `headphone_only_output`), as does the user's "I use headphones" setting.
//! The *stored* microphone track is never cancelled; only the copy the Me
//! lane transcribes is.

mod sonora_aec;

use deino_audio::capture::OutputRoute;
pub use sonora_aec::SonoraAec;

/// Samples per echo-canceller frame: 10 ms at 16 kHz.
pub const FRAME: usize = 160;

/// Whether the microphone can hear the call.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum EchoMode {
    /// Speakers: cancel the echo and hold back suspected bleed.
    Cancel,
    /// Headphones: the microphone is isolated; nothing to remove.
    Isolated,
}

impl EchoMode {
    /// The mode for the user's setting and the probed output route. The
    /// setting wins; an unknown route is treated as speakers, the safe side.
    pub fn choose(uses_headphones: bool, route: Option<OutputRoute>) -> Self {
        if uses_headphones || route == Some(OutputRoute::Headphones) {
            Self::Isolated
        } else {
            Self::Cancel
        }
    }
}

/// Removes the reference's echo from captured frames.
pub trait EchoCanceller: Send {
    /// Cancels one [`FRAME`]: `reference` is what the speakers played,
    /// `capture` the microphone, cleaned in place. Allocation-free.
    fn process(&mut self, reference: &[f32], capture: &mut [f32]);
}

/// An echo canceller that leaves the microphone untouched (tests, and
/// platforms or modes without cancellation).
#[derive(Clone, Copy, Debug, Default)]
pub struct NoEchoCanceller;

impl EchoCanceller for NoEchoCanceller {
    fn process(&mut self, _reference: &[f32], _capture: &mut [f32]) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headphones_isolate_the_microphone_and_speakers_need_cancelling() {
        assert_eq!(
            EchoMode::choose(false, Some(OutputRoute::Headphones)),
            EchoMode::Isolated
        );
        assert_eq!(
            EchoMode::choose(false, Some(OutputRoute::Speakers)),
            EchoMode::Cancel
        );
        assert_eq!(EchoMode::choose(false, None), EchoMode::Cancel);
    }

    #[test]
    fn the_headphones_setting_overrides_detection() {
        assert_eq!(
            EchoMode::choose(true, Some(OutputRoute::Speakers)),
            EchoMode::Isolated
        );
    }
}
