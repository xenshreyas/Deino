//! Generated test signals: deterministic audio as a pure function of time.
//!
//! The mock source plays these until FND-04's fixture tracks are wired in
//! ([`TestSignal::Looped`] plays any decoded clip). Each sample depends only
//! on its frame index, so a test can predict exactly what a file must hold.

use std::f32::consts::TAU;
use std::sync::Arc;
use std::time::Duration;

use crate::units::{Frames, STORAGE_RATE};

/// Length of one click in [`TestSignal::Clicks`] (1 ms).
const CLICK_FRAMES: u64 = 48;

/// One "syllable" of [`TestSignal::Speechlike`]: 180 ms of voice, 120 ms of pause.
const SYLLABLE: Frames = Frames(14_400);

/// Voiced part of a syllable.
const VOICED: Frames = Frames(8_640);

/// A deterministic mono signal at 48 kHz.
#[derive(Clone, Debug, PartialEq)]
pub enum TestSignal {
    /// A steady sine tone.
    Tone {
        /// Frequency in hertz.
        hz: f32,
        /// Peak amplitude (full scale is 1.0).
        amplitude: f32,
    },
    /// A 1 ms full-amplitude click every `every`, silence between: for
    /// alignment tests.
    Clicks {
        /// Time between clicks.
        every: Duration,
        /// Click amplitude.
        amplitude: f32,
    },
    /// Tone bursts with pauses, like syllables of speech, at `hz`.
    Speechlike {
        /// Pitch of the voiced parts.
        hz: f32,
    },
    /// A recorded clip, looped (fixture audio once FND-04 lands).
    Looped(Arc<[f32]>),
}

impl TestSignal {
    /// The microphone preset: a 220 Hz speech-like voice.
    pub fn microphone() -> Self {
        Self::Speechlike { hz: 220.0 }
    }

    /// The system preset: a 330 Hz speech-like voice.
    pub fn system() -> Self {
        Self::Speechlike { hz: 330.0 }
    }

    /// The sample at timeline frame `frame`.
    pub fn sample(&self, frame: Frames) -> f32 {
        match self {
            Self::Tone { hz, amplitude } => amplitude * sine(*hz, frame),
            Self::Clicks { every, amplitude } => click(*every, *amplitude, frame),
            Self::Speechlike { hz } => speechlike(*hz, frame),
            Self::Looped(clip) => looped(clip, frame),
        }
    }

    /// Fills `out` with the samples starting at frame `start`.
    pub fn fill(&self, start: Frames, out: &mut [f32]) {
        for (offset, slot) in (0u64..).zip(out.iter_mut()) {
            *slot = self.sample(start + Frames(offset));
        }
    }
}

/// `sin(2π · hz · t)` at `frame`, with the phase kept small for precision.
fn sine(hz: f32, frame: Frames) -> f32 {
    let period = (f64::from(STORAGE_RATE) / f64::from(hz)).max(1.0);
    #[allow(
        clippy::cast_precision_loss,
        reason = "frame counts stay far below 2^52"
    )]
    let phase = (frame.0 as f64 % period) / period;
    #[allow(clippy::cast_possible_truncation, reason = "phase is in [0, 1)")]
    let phase = phase as f32;
    (TAU * phase).sin()
}

/// A 1 ms click at the start of every period.
fn click(every: Duration, amplitude: f32, frame: Frames) -> f32 {
    let period = Frames::from_duration(every).0.max(CLICK_FRAMES + 1);
    if frame.0 % period < CLICK_FRAMES {
        amplitude
    } else {
        0.0
    }
}

/// Voiced bursts with pauses: a tone with a soft attack and release.
fn speechlike(hz: f32, frame: Frames) -> f32 {
    let position = Frames(frame.0 % SYLLABLE.0);
    if position >= VOICED {
        return 0.0;
    }
    #[allow(
        clippy::cast_precision_loss,
        reason = "positions are below one syllable"
    )]
    let envelope = (std::f32::consts::PI * position.0 as f32 / VOICED.0 as f32).sin();
    0.4 * envelope * (sine(hz, frame) + 0.3 * sine(hz * 2.0, frame))
}

/// The clip's sample at `frame`, looping; silence for an empty clip.
fn looped(clip: &[f32], frame: Frames) -> f32 {
    let len = Frames::of_len(clip.len()).0;
    if len == 0 {
        return 0.0;
    }
    let index = usize::try_from(frame.0 % len).unwrap_or_default();
    clip.get(index).copied().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::float_cmp,
        reason = "samples are copied or generated exactly, so they compare exactly"
    )]

    use super::*;

    #[test]
    fn signals_are_a_pure_function_of_the_frame() {
        let signal = TestSignal::microphone();
        let mut first = vec![0.0; 960];
        let mut second = vec![0.0; 960];
        signal.fill(Frames(4_000), &mut first);
        signal.fill(Frames(4_000), &mut second);
        assert_eq!(first, second);
        assert!(first.iter().any(|s| s.abs() > 0.1));
    }

    #[test]
    fn clicks_land_exactly_on_their_period() {
        let signal = TestSignal::Clicks {
            every: Duration::from_millis(500),
            amplitude: 0.9,
        };
        assert_eq!(signal.sample(Frames(0)), 0.9);
        assert_eq!(signal.sample(Frames(47)), 0.9);
        assert_eq!(signal.sample(Frames(48)), 0.0);
        assert_eq!(signal.sample(Frames(24_000)), 0.9);
    }

    #[test]
    fn speechlike_audio_pauses_between_syllables() {
        let signal = TestSignal::system();
        assert_eq!(signal.sample(Frames(10_000)), 0.0);
        assert!(signal.sample(Frames(4_330)).abs() > 0.0);
    }

    #[test]
    fn a_looped_clip_repeats() {
        let signal = TestSignal::Looped(Arc::from(vec![0.1, 0.2, 0.3]));
        assert_eq!(signal.sample(Frames(4)), 0.2);
        assert_eq!(
            TestSignal::Looped(Arc::from(Vec::new())).sample(Frames(9)),
            0.0
        );
    }

    #[test]
    fn a_tone_stays_within_its_amplitude() {
        let signal = TestSignal::Tone {
            hz: 440.0,
            amplitude: 0.5,
        };
        let mut out = vec![0.0; 4_800];
        signal.fill(Frames::ZERO, &mut out);
        assert!(out.iter().all(|s| s.abs() <= 0.5));
    }
}
