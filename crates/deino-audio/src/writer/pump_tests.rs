//! Pump tests with a fake encoder and a fake host clock: placement, backlog,
//! stalls and drops, all without threads or sleeping.

#![allow(
    clippy::float_cmp,
    reason = "samples are copied or generated exactly, so they compare exactly"
)]

use std::sync::mpsc::Receiver;

use super::*;
use crate::clock::FakeHostClock;
use crate::encoder::{EncoderFactory, FakeEncoderFactory, SegmentOutput};
use crate::health::health_channel;
use crate::ring::{RtSink, TimelineSource, timeline_ring, track_ring};
use crate::sink::MemorySink;

/// 10 ms of 48 kHz audio.
const BLOCK: Duration = Duration::from_millis(10);

struct Rig {
    pump: TrackPump,
    sink: RtSink,
    clock: Arc<FakeHostClock>,
    encoders: FakeEncoderFactory,
    events: Receiver<HealthEvent>,
    placed: TimelineSource,
}

fn rig(track: Track) -> Rig {
    let clock = Arc::new(FakeHostClock::new(HostTime(1_000_000_000)));
    let encoders = FakeEncoderFactory::default();
    let output = SegmentOutput {
        sink: Arc::new(MemorySink::default()),
        ledger: Arc::default(),
    };
    let (sink, source) = track_ring();
    let (placed_sink, placed) = timeline_ring();
    let (health, events) = health_channel();
    let pump = TrackPump::new(PumpParts {
        track,
        source,
        placed: placed_sink,
        epoch: clock.now(),
        encoder: encoders.open(track, output).unwrap(),
        clock: clock.clone(),
        health,
        appended: Arc::default(),
    });
    Rig {
        pump,
        sink,
        clock,
        encoders,
        events,
        placed,
    }
}

/// Captures one 10 ms block of `value` now, then advances the clock.
fn capture(rig: &mut Rig, value: f32) {
    assert!(rig.sink.push(&[value; 480], rig.clock.now()));
    rig.clock.advance(BLOCK);
}

#[test]
fn blocks_reach_the_encoder_in_timeline_order() {
    let mut rig = rig(Track::Microphone);
    capture(&mut rig, 0.1);
    capture(&mut rig, 0.2);

    rig.pump.poll().unwrap();

    let samples = rig.encoders.samples(Track::Microphone);
    assert_eq!(samples.len(), 960);
    assert_eq!(samples[0], 0.1);
    assert_eq!(samples[480], 0.2);
}

#[test]
fn a_late_starting_track_begins_with_silence_from_the_epoch() {
    let mut rig = rig(Track::System);
    rig.clock.advance(Duration::from_millis(300));
    capture(&mut rig, 0.5);

    rig.pump.poll().unwrap();

    let samples = rig.encoders.samples(Track::System);
    assert_eq!(samples.len(), 14_400 + 480);
    assert!(samples[..14_400].iter().all(|s| *s == 0.0));
    assert!(samples[14_400..].iter().all(|s| *s == 0.5));
}

#[test]
fn placed_audio_reaches_the_timeline_ring_at_its_timeline_position() {
    let mut rig = rig(Track::System);
    rig.clock.advance(Duration::from_millis(300));
    capture(&mut rig, 0.5);
    capture(&mut rig, 0.25);

    rig.pump.poll().unwrap();

    let mut block = Vec::new();
    let first = rig.placed.pop_block(&mut block).unwrap();
    assert_eq!((first.at, first.len), (Frames(14_400), 480));
    assert!(block.iter().all(|s| *s == 0.5));
    let second = rig.placed.pop_block(&mut block).unwrap();
    assert_eq!(second.at, Frames(14_880));
}

#[test]
fn a_refusing_encoder_queues_audio_reports_a_stall_and_recovers() {
    let mut rig = rig(Track::Microphone);
    rig.encoders.set_ready(false);
    for _ in 0..300 {
        capture(&mut rig, 0.3);
        rig.pump.poll().unwrap();
    }
    assert!(rig.encoders.samples(Track::Microphone).is_empty());

    rig.encoders.set_ready(true);
    rig.pump.poll().unwrap();

    assert_eq!(rig.encoders.samples(Track::Microphone).len(), 300 * 480);
    let events: Vec<_> = rig.events.try_iter().collect();
    assert!(matches!(
        events[0],
        HealthEvent::Stalled {
            track: Track::Microphone,
            ..
        }
    ));
    assert_eq!(
        events[1],
        HealthEvent::Resumed {
            track: Track::Microphone
        }
    );
}

#[test]
fn frames_the_ring_dropped_are_reported_once() {
    let mut rig = rig(Track::System);
    let two_seconds = vec![0.0; 96_000];
    assert!(rig.sink.push(&two_seconds, rig.clock.now()));
    assert!(!rig.sink.push(&[0.0; 480], rig.clock.now()));

    rig.pump.poll().unwrap();
    rig.pump.poll().unwrap();

    let events: Vec<_> = rig.events.try_iter().collect();
    assert_eq!(
        events,
        vec![HealthEvent::FramesDropped {
            track: Track::System,
            frames: Frames(480),
        }]
    );
}

#[test]
fn padding_to_the_stop_time_fills_the_tail_and_finish_reports_the_length() {
    let mut rig = rig(Track::Microphone);
    capture(&mut rig, 0.7);
    rig.clock.advance(Duration::from_millis(990));

    rig.pump.pad_to(rig.clock.now());
    rig.pump.feed().unwrap();
    assert!(rig.pump.is_drained());
    let report = rig.pump.finish().unwrap();

    assert_eq!(report.frames, Frames(48_000));
    assert_eq!(report.dropped, Frames::ZERO);
    assert!(rig.encoders.finished(Track::Microphone));
}

#[test]
fn abandoning_the_backlog_counts_the_loss() {
    let mut rig = rig(Track::Microphone);
    rig.encoders.set_ready(false);
    capture(&mut rig, 0.3);
    rig.pump.poll().unwrap();

    rig.pump.abandon_backlog();

    assert!(rig.pump.is_drained());
    assert_eq!(rig.pump.finish().unwrap().dropped, Frames(480));
}
