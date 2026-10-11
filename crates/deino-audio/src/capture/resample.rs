//! `ChunkResampler`: one streaming sample-rate conversion, allocation-free.
//!
//! Audio is resampled exactly once on its way to each consumer (§3C): the
//! conditioner converts a device's native rate to the 48 kHz of storage, and
//! the live pipeline converts placed 48 kHz audio to the 16 kHz the engine
//! reads. Both use rubato's `FftFixedInOut`, whose fixed input and output
//! chunk sizes make time exact: every `chunk_in` input frames become
//! `chunk_out` output frames, so the caller advances its clock by a whole
//! chunk per emitted block.
//!
//! All buffers are allocated at construction; [`ChunkResampler::process`]
//! never allocates as long as a single input block fits the pending buffer
//! (blocks are 10 ms; the buffer holds a second).

use std::time::Duration;

use rubato::{FftFixedInOut, Resampler, ResamplerConstructionError};

use crate::units::SampleRate;

/// The input chunk aimed for: 10 ms, the size of a device callback.
const CHUNK: Duration = Duration::from_millis(10);

/// Input the pending buffer holds before a block must be split.
const PENDING: Duration = Duration::from_secs(1);

/// Why a resampler could not be built.
#[derive(Debug, thiserror::Error)]
#[error("cannot resample from {from} to {to}")]
pub struct ResampleError {
    /// The input rate.
    pub from: SampleRate,
    /// The output rate.
    pub to: SampleRate,
    /// rubato's reason.
    #[source]
    pub source: ResamplerConstructionError,
}

/// A streaming converter between two fixed rates.
pub struct ChunkResampler {
    inner: Option<FftFixedInOut<f32>>,
    pending: Vec<f32>,
    out: Vec<f32>,
    chunk_in: usize,
    from: SampleRate,
    delay: Duration,
}

impl std::fmt::Debug for ChunkResampler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChunkResampler")
            .field("from", &self.from)
            .field("chunk_in", &self.chunk_in)
            .field("pending", &self.pending.len())
            .finish_non_exhaustive()
    }
}

impl ChunkResampler {
    /// A converter from `from` to `to`; a pass-through when they are equal.
    pub fn new(from: SampleRate, to: SampleRate) -> Result<Self, ResampleError> {
        let pending_len = from.frames_in(PENDING).max(1);
        if from == to {
            return Ok(Self::passthrough(from, pending_len));
        }
        let rates = (rate_of(from), rate_of(to));
        let inner = FftFixedInOut::new(rates.0, rates.1, from.frames_in(CHUNK).max(1), 1)
            .map_err(|source| ResampleError { from, to, source })?;
        let chunk_in = inner.input_frames_next();
        let delay = to.duration_of(inner.output_delay());
        Ok(Self {
            out: vec![0.0; inner.output_frames_max()],
            pending: Vec::with_capacity(pending_len.max(chunk_in * 2)),
            inner: Some(inner),
            chunk_in,
            from,
            delay,
        })
    }

    fn passthrough(from: SampleRate, pending_len: usize) -> Self {
        Self {
            inner: None,
            pending: Vec::with_capacity(pending_len),
            out: Vec::new(),
            chunk_in: 0,
            from,
            delay: Duration::ZERO,
        }
    }

    /// The input rate.
    pub fn input_rate(&self) -> SampleRate {
        self.from
    }

    /// How far output lags input: subtract it from an output block's input
    /// time to stamp it.
    pub fn delay(&self) -> Duration {
        self.delay
    }

    /// Input frames waiting for a whole chunk.
    pub fn pending_frames(&self) -> usize {
        self.pending.len()
    }

    /// Forgets pending input and filter state (after a discontinuity).
    pub fn reset(&mut self) {
        self.pending.clear();
        if let Some(inner) = self.inner.as_mut() {
            inner.reset();
        }
    }

    /// Converts `input`. For every output block it calls `emit(block,
    /// consumed)`, where `consumed` is the input frames the block covers, so
    /// the caller can advance its clock.
    pub fn process(&mut self, input: &[f32], mut emit: impl FnMut(&[f32], usize)) {
        if self.inner.is_none() {
            emit(input, input.len());
            return;
        }
        for piece in input.chunks(self.pending.capacity().max(1)) {
            self.pending.extend_from_slice(piece);
            self.drain_chunks(&mut emit);
        }
    }

    /// Converts every whole chunk waiting in `pending`, keeping the rest.
    fn drain_chunks(&mut self, emit: &mut impl FnMut(&[f32], usize)) {
        let Some(inner) = self.inner.as_mut() else {
            return;
        };
        let mut start = 0;
        while let Some(chunk) = self.pending.get(start..start + self.chunk_in) {
            // A fixed-size chunk into a buffer of `output_frames_max` cannot fail.
            if let Ok((_, written)) =
                inner.process_into_buffer(&[chunk], &mut [&mut self.out], None)
            {
                emit(self.out.get(..written).unwrap_or_default(), self.chunk_in);
            }
            start += self.chunk_in;
        }
        self.pending.copy_within(start.., 0);
        self.pending.truncate(self.pending.len() - start);
    }
}

/// A rate as rubato takes it.
fn rate_of(rate: SampleRate) -> usize {
    usize::try_from(rate.0).unwrap_or(usize::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(resampler: &mut ChunkResampler, input: &[f32], block: usize) -> (Vec<f32>, usize) {
        let mut output = Vec::new();
        let mut consumed = 0;
        for piece in input.chunks(block) {
            resampler.process(piece, |out, used| {
                output.extend_from_slice(out);
                consumed += used;
            });
        }
        (output, consumed)
    }

    #[test]
    fn equal_rates_pass_blocks_through_untouched() {
        let mut resampler = ChunkResampler::new(SampleRate(48_000), SampleRate(48_000)).unwrap();
        let (output, consumed) = run(&mut resampler, &[0.5; 960], 480);
        assert_eq!(output, vec![0.5; 960]);
        assert_eq!(consumed, 960);
        assert_eq!(resampler.delay(), Duration::ZERO);
    }

    #[test]
    fn forty_four_one_becomes_forty_eight_in_exact_ten_ms_chunks() {
        let mut resampler = ChunkResampler::new(SampleRate(44_100), SampleRate(48_000)).unwrap();
        let (output, consumed) = run(&mut resampler, &vec![0.25; 44_100], 512);

        assert_eq!(consumed % 441, 0);
        assert_eq!(output.len() * 441, consumed * 480);
        assert!(resampler.pending_frames() < 441);
        let middle = output[output.len() / 2];
        assert!((middle - 0.25).abs() < 0.01, "level kept: {middle}");
    }

    #[test]
    fn forty_eight_becomes_sixteen_at_a_third_the_samples() {
        let mut resampler = ChunkResampler::new(SampleRate(48_000), SampleRate(16_000)).unwrap();
        let (output, consumed) = run(&mut resampler, &vec![0.1; 48_000], 480);
        assert_eq!(consumed, 48_000);
        assert_eq!(output.len(), 16_000);
    }

    #[test]
    fn a_block_larger_than_the_buffer_is_split_not_grown() {
        let mut resampler = ChunkResampler::new(SampleRate(16_000), SampleRate(48_000)).unwrap();
        let capacity = resampler.pending.capacity();
        let (output, consumed) = run(&mut resampler, &vec![0.0; 40_000], 40_000);
        assert_eq!(resampler.pending.capacity(), capacity);
        assert_eq!(output.len(), consumed * 3);
    }

    #[test]
    fn reset_drops_pending_input() {
        let mut resampler = ChunkResampler::new(SampleRate(44_100), SampleRate(48_000)).unwrap();
        run(&mut resampler, &[0.1; 100], 100);
        assert_eq!(resampler.pending_frames(), 100);
        resampler.reset();
        assert_eq!(resampler.pending_frames(), 0);
    }
}
