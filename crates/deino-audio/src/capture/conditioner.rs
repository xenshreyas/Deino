//! The conditioner: a capture ring at the device rate in, the track's 48 kHz
//! ring out.
//!
//! It drains whole native-rate blocks, resamples them once to 48 kHz
//! ([`ChunkResampler`]), moves their host stamps onto the pause-free
//! recording clock ([`PauseLedger`]) and pushes them into the track writer's
//! [`RtSink`]. A block that does not follow the previous one (a restart, a
//! drop) resets the resampler, so its stamp starts a fresh run and the
//! writer's timeline turns the hole into silence. It runs on its own thread
//! ([`super::track`]), never on the real-time callback.

use std::sync::Arc;
use std::time::Duration;

use super::meter::{PauseLedger, TrackMeter, peak_of};
use super::resample::{ChunkResampler, ResampleError};
use crate::ring::{BlockStamp, RtSink, RtSource};
use crate::timeline::PLACEMENT_TOLERANCE;
use crate::units::{HostTime, SampleRate};

/// What every conditioner of a recording shares with the supervisor.
#[derive(Clone, Debug, Default)]
pub struct CaptureShared {
    /// Paused time the timeline skips.
    pub pause: Arc<PauseLedger>,
}

/// One track's native-rate → 48 kHz path.
#[derive(Debug)]
pub struct Conditioner {
    resampler: ChunkResampler,
    pause: Arc<PauseLedger>,
    meter: Arc<TrackMeter>,
    /// Host time of the first input frame the resampler is still holding.
    next_at: HostTime,
    block: Vec<f32>,
}

impl Conditioner {
    /// A conditioner for input at `rate`.
    pub fn new(
        rate: SampleRate,
        shared: &CaptureShared,
        meter: Arc<TrackMeter>,
    ) -> Result<Self, ResampleError> {
        Ok(Self {
            resampler: ChunkResampler::new(rate, SampleRate::STORAGE)?,
            pause: Arc::clone(&shared.pause),
            meter,
            next_at: HostTime::default(),
            block: Vec::with_capacity(rate.frames_in(Duration::from_millis(100))),
        })
    }

    /// The input rate.
    pub fn rate(&self) -> SampleRate {
        self.resampler.input_rate()
    }

    /// The activity counters it writes.
    pub fn meter(&self) -> &Arc<TrackMeter> {
        &self.meter
    }

    /// Moves every whole block from `input` to `output`; returns how many
    /// blocks it moved.
    pub fn pump(&mut self, input: &mut RtSource, output: &mut RtSink) -> usize {
        let mut moved = 0;
        while let Some(stamp) = input.pop_block(&mut self.block) {
            self.meter
                .record(self.block.len(), stamp.at, peak_of(&self.block));
            self.convert(stamp, output);
            moved += 1;
        }
        moved
    }

    /// Resamples one block and pushes what comes out, stamped on the
    /// recording clock.
    fn convert(&mut self, stamp: BlockStamp, output: &mut RtSink) {
        let at = self.pause.to_timeline(stamp.at);
        self.follow(at);
        let rate = self.resampler.input_rate();
        let delay = self.resampler.delay();
        let mut next_at = self.next_at;
        self.resampler.process(&self.block, |samples, consumed| {
            let stamped = HostTime(next_at.0.saturating_sub(nanos(delay)));
            output.push(samples, stamped);
            next_at = next_at.plus(rate.duration_of(consumed));
        });
        self.next_at = next_at;
    }

    /// Starts a fresh run at `at` unless the block continues the held input.
    fn follow(&mut self, at: HostTime) {
        let held = self.resampler.pending_frames();
        if held == 0 {
            self.next_at = at;
            return;
        }
        let expected = self
            .next_at
            .plus(self.resampler.input_rate().duration_of(held));
        let off = expected.since(at).max(at.since(expected));
        if off > PLACEMENT_TOLERANCE {
            self.resampler.reset();
            self.next_at = at;
        }
    }
}

/// A duration as nanoseconds, saturating.
fn nanos(duration: Duration) -> u64 {
    u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ring::{capture_ring, track_ring};
    use crate::units::Frames;

    fn conditioner(rate: u32, shared: &CaptureShared) -> (Conditioner, Arc<TrackMeter>) {
        let meter = Arc::new(TrackMeter::default());
        let conditioner = Conditioner::new(SampleRate(rate), shared, Arc::clone(&meter)).unwrap();
        (conditioner, meter)
    }

    fn drain(source: &mut RtSource) -> Vec<(BlockStamp, Vec<f32>)> {
        let mut out = Vec::new();
        let mut blocks = Vec::new();
        while let Some(stamp) = source.pop_block(&mut out) {
            blocks.push((stamp, out.clone()));
        }
        blocks
    }

    #[test]
    fn forty_eight_kilohertz_blocks_pass_through_with_their_stamps() {
        let shared = CaptureShared::default();
        let (mut conditioner, meter) = conditioner(48_000, &shared);
        let (mut device, mut input) = capture_ring(48_000);
        let (mut output, mut track) = track_ring();
        device.push(&[0.5; 480], HostTime(1_000));

        assert_eq!(conditioner.pump(&mut input, &mut output), 1);

        let blocks = drain(&mut track);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].0.at, HostTime(1_000));
        assert_eq!(meter.frames(), Frames(480));
        assert!((meter.peak() - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn paused_time_is_removed_from_the_stamps() {
        let shared = CaptureShared::default();
        shared.pause.add(Duration::from_millis(1));
        let (mut conditioner, _) = conditioner(48_000, &shared);
        let (mut device, mut input) = capture_ring(48_000);
        let (mut output, mut track) = track_ring();
        device.push(&[0.5; 480], HostTime(5_000_000));

        conditioner.pump(&mut input, &mut output);

        assert_eq!(drain(&mut track)[0].0.at, HostTime(4_000_000));
    }

    #[test]
    fn a_device_rate_is_resampled_to_storage_and_stamped_continuously() {
        let shared = CaptureShared::default();
        let (mut conditioner, _) = conditioner(44_100, &shared);
        let (mut device, mut input) = capture_ring(44_100);
        let (mut output, mut track) = track_ring();
        let block = Duration::from_millis(10);
        let start = HostTime(1_000_000_000);
        for index in 0..10 {
            device.push(&[0.25; 441], start.plus(block * index));
        }

        conditioner.pump(&mut input, &mut output);

        let blocks = drain(&mut track);
        assert!(blocks.iter().all(|(stamp, _)| stamp.len == 480));
        let step = blocks[1].0.at.since(blocks[0].0.at);
        assert_eq!(step, block);
    }

    #[test]
    fn a_discontinuous_block_restarts_the_run_at_its_own_stamp() {
        let shared = CaptureShared::default();
        let (mut conditioner, _) = conditioner(44_100, &shared);
        let (mut device, mut input) = capture_ring(44_100);
        let (mut output, mut track) = track_ring();
        device.push(&[0.25; 300], HostTime(1_000_000_000));
        device.push(&[0.25; 441], HostTime(3_000_000_000));

        conditioner.pump(&mut input, &mut output);

        let blocks = drain(&mut track);
        let delay = nanos(conditioner.resampler.delay());
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].0.at, HostTime(3_000_000_000 - delay));
    }
}
