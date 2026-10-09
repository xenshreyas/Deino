//! Mixing the two tracks into one: a sum at −6 dB through a soft limiter.
//!
//! Both tracks are sample-aligned (the sample index is time), so mixing is a
//! per-frame sum. Halving each track (−6 dB) keeps two full-scale signals
//! inside full scale; the limiter only shapes the rare peaks above
//! [`LIMIT_THRESHOLD`] and never clips.

/// Gain applied to the sum: −6 dB.
pub const MIX_GAIN: f32 = 0.5;

/// Level above which the limiter starts to compress.
pub const LIMIT_THRESHOLD: f32 = 0.9;

/// A soft limiter: transparent below [`LIMIT_THRESHOLD`], then a `tanh` knee
/// that approaches but never exceeds full scale.
pub fn limit(sample: f32) -> f32 {
    let magnitude = sample.abs();
    if magnitude <= LIMIT_THRESHOLD {
        return sample;
    }
    let headroom = 1.0 - LIMIT_THRESHOLD;
    let shaped = LIMIT_THRESHOLD + headroom * ((magnitude - LIMIT_THRESHOLD) / headroom).tanh();
    shaped.copysign(sample)
}

/// Mixes one microphone and one system sample.
pub fn mix_sample(microphone: f32, system: f32) -> f32 {
    limit((microphone + system) * MIX_GAIN)
}

/// Mixes two blocks into `out` (cleared first). The shorter block is padded
/// with silence, so `out` is as long as the longer one.
pub fn mix_blocks(microphone: &[f32], system: &[f32], out: &mut Vec<f32>) {
    out.clear();
    let len = microphone.len().max(system.len());
    out.extend((0..len).map(|i| {
        let mic = microphone.get(i).copied().unwrap_or_default();
        let sys = system.get(i).copied().unwrap_or_default();
        mix_sample(mic, sys)
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Equal within float rounding.
    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-6
    }

    #[test]
    fn quiet_audio_passes_through_at_minus_six_db() {
        assert!(close(mix_sample(0.4, 0.2), 0.3));
        assert!(close(mix_sample(-0.5, 0.0), -0.25));
    }

    #[test]
    fn two_full_scale_tracks_never_exceed_full_scale() {
        for (mic, sys) in [(1.0, 1.0), (-1.0, -1.0), (2.0, 2.0), (1.0, 0.95)] {
            let mixed = mix_sample(mic, sys);
            assert!(mixed.abs() <= 1.0, "{mic} + {sys} → {mixed}");
        }
    }

    #[test]
    fn the_limiter_is_continuous_and_monotonic_at_the_knee() {
        let below = limit(LIMIT_THRESHOLD);
        let above = limit(LIMIT_THRESHOLD + 1e-4);
        assert!(above > below);
        assert!(above - below < 1e-3);
        assert!(close(limit(-0.95), -limit(0.95)));
    }

    #[test]
    fn the_shorter_block_is_padded_with_silence() {
        let mut out = Vec::new();
        mix_blocks(&[0.2, 0.2, 0.2], &[0.4], &mut out);
        let expected = [0.3, 0.1, 0.1];
        assert!(
            out.iter().zip(expected).all(|(a, b)| close(*a, b)),
            "{out:?}"
        );
    }
}
