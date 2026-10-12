//! `PlacedReader`: one track's placed audio as a gap-free 16 kHz stream.
//!
//! The track writer hands over every block it places on the recording
//! timeline, stamped with its 48 kHz position ([`TimelineSource`]). The
//! reader turns those into a contiguous stream: a hole (a source restart, a
//! late-starting tap, a dropped block) becomes silence, an overlap is
//! trimmed, and the result is resampled once to 16 kHz. Sample `n` of the
//! output is therefore `n / 16 000` s into the recording on both lanes, so
//! the lanes align by index, and so do the segments they produce.

use std::collections::VecDeque;

use deino_audio::capture::resample::{ChunkResampler, ResampleError};
use deino_audio::{Frames, SampleRate, TimelineSource};

/// The rate the lanes run at.
pub const LANE_RATE: SampleRate = SampleRate(16_000);

/// Silence pushed in pieces of this many 48 kHz frames.
const SILENCE: [f32; 480] = [0.0; 480];

/// One track's placed audio, read into a 16 kHz FIFO.
#[derive(Debug)]
pub struct PlacedReader {
    source: TimelineSource,
    resampler: ChunkResampler,
    /// The next 48 kHz timeline position the reader expects.
    next: Frames,
    /// 16 kHz output frames still to discard (the resampler's delay).
    skip: usize,
    block: Vec<f32>,
    fifo: VecDeque<f32>,
}

impl PlacedReader {
    /// A reader of `source` from the start of the recording.
    pub fn new(source: TimelineSource) -> Result<Self, ResampleError> {
        Self::starting_at(source, Frames::ZERO)
    }

    /// A reader of `source` whose first block sits at `start` (48 kHz
    /// frames): a resumed meeting's audio follows its recorded length, and
    /// nothing before it is read or padded.
    pub fn starting_at(source: TimelineSource, start: Frames) -> Result<Self, ResampleError> {
        let resampler = ChunkResampler::new(SampleRate::STORAGE, LANE_RATE)?;
        let skip = LANE_RATE.frames_in(resampler.delay());
        Ok(Self {
            source,
            resampler,
            next: start,
            skip,
            block: Vec::with_capacity(4_800),
            fifo: VecDeque::with_capacity(LANE_RATE.frames_in(std::time::Duration::from_secs(4))),
        })
    }

    /// The timeline position read up to (48 kHz frames).
    pub fn position(&self) -> Frames {
        self.next
    }

    /// The 16 kHz samples waiting to be taken.
    pub fn available(&self) -> usize {
        self.fifo.len()
    }

    /// Reads every placed block that has arrived.
    pub fn drain(&mut self) {
        while let Some(stamp) = self.source.pop_block(&mut self.block) {
            self.pad_to(stamp.at);
            let overlap = self.next.saturating_sub(stamp.at).as_len();
            let block = std::mem::take(&mut self.block);
            let fresh = block.get(overlap..).unwrap_or_default();
            self.push(fresh);
            self.block = block;
        }
    }

    /// Extends the stream with silence up to `position` (48 kHz frames).
    pub fn pad_to(&mut self, position: Frames) {
        while self.next < position {
            let len = (position - self.next).as_len().min(SILENCE.len());
            self.push(SILENCE.get(..len).unwrap_or_default());
        }
    }

    /// Takes the next `out.len()` samples, if that many are waiting.
    pub fn take(&mut self, out: &mut [f32]) -> bool {
        if self.fifo.len() < out.len() {
            return false;
        }
        let len = out.len();
        for (slot, sample) in out.iter_mut().zip(self.fifo.drain(..len)) {
            *slot = sample;
        }
        true
    }

    /// Resamples `samples` (the next stretch of the timeline) into the FIFO.
    fn push(&mut self, samples: &[f32]) {
        self.next += Frames::of_len(samples.len());
        let (fifo, skip) = (&mut self.fifo, &mut self.skip);
        self.resampler.process(samples, |out, _| {
            let dropped = (*skip).min(out.len());
            *skip -= dropped;
            fifo.extend(out.get(dropped..).unwrap_or_default());
        });
    }
}

#[cfg(test)]
mod tests {
    use deino_audio::ring::timeline_ring;

    use super::*;

    fn reader() -> (deino_audio::ring::TimelineSink, PlacedReader) {
        let (sink, source) = timeline_ring();
        (sink, PlacedReader::new(source).unwrap())
    }

    #[test]
    fn a_contiguous_track_becomes_a_third_as_many_samples() {
        let (mut sink, mut reader) = reader();
        for block in 0..100_u64 {
            sink.push(&[0.25; 480], Frames(block * 480));
        }
        reader.drain();
        assert_eq!(reader.position(), Frames(48_000));
        let expected = 16_000 - reader.resampler.delay().as_millis() as usize * 16;
        assert!(
            reader.available().abs_diff(expected) <= 160,
            "{}",
            reader.available()
        );
    }

    #[test]
    fn a_late_track_is_preceded_by_silence() {
        let (mut sink, mut reader) = reader();
        sink.push(&[0.5; 480], Frames(48_000));
        reader.drain();
        assert_eq!(reader.position(), Frames(48_480));
        let mut first = [1.0; 160];
        assert!(reader.take(&mut first));
        assert!(first.iter().all(|s| s.abs() < 1e-6));
    }

    #[test]
    fn an_overlapping_block_is_trimmed() {
        let (mut sink, mut reader) = reader();
        sink.push(&[0.5; 480], Frames(0));
        sink.push(&[0.5; 480], Frames(240));
        reader.drain();
        assert_eq!(reader.position(), Frames(720));
    }

    #[test]
    fn padding_extends_the_stream_to_a_position() {
        let (_sink, mut reader) = reader();
        reader.pad_to(Frames(4_800));
        assert_eq!(reader.position(), Frames(4_800));
    }
}
