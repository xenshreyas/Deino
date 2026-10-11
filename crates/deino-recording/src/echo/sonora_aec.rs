//! `SonoraAec`: WebRTC's AEC3 through `sonora`, a pure-Rust port of the
//! WebRTC audio processing module (BSD-3-Clause; ADR 0026).
//!
//! The processor runs at 16 kHz mono, the rate the lanes transcribe, so the
//! cleaned microphone needs no further conversion. Only echo cancellation
//! (with its mandatory high-pass filter) is on: noise suppression and gain
//! control would reshape the speech the engine was trained on. Every buffer
//! is allocated at construction, so [`EchoCanceller::process`] does not
//! allocate (`tests/aec_allocations.rs` counts).

use sonora::config::{EchoCanceller as AecConfig, HighPassFilter};
use sonora::{AudioProcessing, Config, StreamConfig};

use super::{EchoCanceller, FRAME};

/// The rate the canceller runs at: the lanes' 16 kHz.
const RATE_HZ: u32 = 16_000;

/// WebRTC AEC3 over a 16 kHz mono reference and capture.
pub struct SonoraAec {
    processor: AudioProcessing,
    render_out: Vec<f32>,
    capture_in: Vec<f32>,
}

impl std::fmt::Debug for SonoraAec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SonoraAec").finish_non_exhaustive()
    }
}

impl Default for SonoraAec {
    fn default() -> Self {
        Self::new()
    }
}

impl SonoraAec {
    /// A canceller with AEC3's default tuning.
    pub fn new() -> Self {
        let config = Config {
            echo_canceller: Some(AecConfig::default()),
            high_pass_filter: Some(HighPassFilter::default()),
            ..Config::default()
        };
        let stream = StreamConfig::new(RATE_HZ, 1);
        let processor = AudioProcessing::builder()
            .config(config)
            .capture_config(stream)
            .render_config(stream)
            .build();
        Self {
            processor,
            render_out: vec![0.0; FRAME],
            capture_in: vec![0.0; FRAME],
        }
    }
}

impl EchoCanceller for SonoraAec {
    fn process(&mut self, reference: &[f32], capture: &mut [f32]) {
        if reference.len() != FRAME || capture.len() != FRAME {
            return;
        }
        if self
            .processor
            .process_render_f32(&[reference], &mut [&mut self.render_out])
            .is_err()
        {
            return;
        }
        self.capture_in.copy_from_slice(capture);
        if self
            .processor
            .process_capture_f32(&[&self.capture_in], &mut [capture])
            .is_err()
        {
            // Leave the frame as captured rather than half-processed.
            capture.copy_from_slice(&self.capture_in);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Deterministic speech-like noise: white noise from a linear
    /// congruential generator under a syllable-rate envelope.
    fn voice(len: usize, seed: u32) -> Vec<f32> {
        let mut state = seed.wrapping_mul(2_654_435_761).wrapping_add(1);
        (0..len)
            .map(|i| {
                state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                #[allow(clippy::cast_precision_loss, reason = "test signal values are small")]
                let (noise, t) = ((state >> 8) as f32 / 8_388_608.0 - 1.0, i as f32 / 16_000.0);
                let envelope = 0.5 + 0.5 * (t * 4.0 * std::f32::consts::TAU).sin();
                0.3 * envelope * noise
            })
            .collect()
    }

    fn energy(samples: &[f32]) -> f32 {
        samples.iter().map(|s| s * s).sum::<f32>()
    }

    #[test]
    fn a_delayed_attenuated_echo_of_the_reference_is_largely_removed() {
        let seconds = 8 * 16_000;
        let far = voice(seconds, 1);
        let delay = 640; // 40 ms of speaker-to-mic path
        let mic: Vec<f32> = (0..seconds)
            .map(|i| i.checked_sub(delay).map_or(0.0, |j| far[j] * 0.4))
            .collect();
        let mut aec = SonoraAec::new();
        let mut cleaned = mic.clone();
        for (reference, capture) in far.chunks(FRAME).zip(cleaned.chunks_mut(FRAME)) {
            aec.process(reference, capture);
        }

        // After 4 s of convergence, the echo is down by more than 20 dB.
        let tail = seconds / 2..;
        let before = energy(&mic[tail.clone()]);
        let after = energy(&cleaned[tail]);
        let erle_db = 10.0 * (before / after.max(f32::MIN_POSITIVE)).log10();
        assert!(
            erle_db > 20.0,
            "echo return loss enhancement {erle_db:.1} dB"
        );
    }

    #[test]
    fn near_end_speech_without_echo_survives() {
        let seconds = 4 * 16_000;
        let silence = vec![0.0; seconds];
        let near = voice(seconds, 2);
        let mut aec = SonoraAec::new();
        let mut cleaned = near.clone();
        for (reference, capture) in silence.chunks(FRAME).zip(cleaned.chunks_mut(FRAME)) {
            aec.process(reference, capture);
        }
        let kept = energy(&cleaned[seconds / 2..]) / energy(&near[seconds / 2..]);
        assert!(kept > 0.5, "near-end energy kept: {kept:.2}");
    }
}
