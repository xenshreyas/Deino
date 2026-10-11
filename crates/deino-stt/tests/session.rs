//! The live session lifecycle over the in-process fake transcriber: segments
//! on the recording's timeline, live text, batch fallback after a lost
//! stream, rotation and the final flush.
//!
//! The fake turns every loud 400 ms block into a word named after the block
//! (`w0`, `w1`, ...), so each test knows exactly which words must come out.

#![allow(
    clippy::unwrap_used,
    reason = "test helpers fail loudly, like the tests they serve"
)]

use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};

use deino_stt::clock::FakeMonotonic;
use deino_stt::transcriber::FakeTranscriber;
use deino_stt::{
    FinalSegment, LiveSession, LiveText, Millis, SegmentOrigin, SessionConfig, SessionSink,
};

/// What a sink saw, in order.
#[derive(Debug)]
enum Seen {
    Live(LiveText),
    Segment(FinalSegment),
    Progress(Millis),
}

struct ChannelSink(Mutex<Sender<Seen>>);

impl SessionSink for ChannelSink {
    fn live(&mut self, text: &LiveText) {
        self.0
            .lock()
            .unwrap()
            .send(Seen::Live(text.clone()))
            .unwrap();
    }

    fn segment(&mut self, segment: FinalSegment) {
        self.0.lock().unwrap().send(Seen::Segment(segment)).unwrap();
    }

    fn progress(&mut self, through: Millis) {
        self.0
            .lock()
            .unwrap()
            .send(Seen::Progress(through))
            .unwrap();
    }
}

fn loud(ms: u64) -> Vec<f32> {
    vec![0.5; Millis(ms).to_samples()]
}

fn silent(ms: u64) -> Vec<f32> {
    vec![0.0; Millis(ms).to_samples()]
}

/// Starts a session over `fake`, returning it with the sink's receiver.
fn start(fake: &Arc<FakeTranscriber>, config: SessionConfig) -> (LiveSession, Receiver<Seen>) {
    let (sender, seen) = mpsc::channel();
    let sink = Box::new(ChannelSink(Mutex::new(sender)));
    let clock = Arc::new(FakeMonotonic::default());
    let session = LiveSession::start(fake.clone(), sink, clock, config).unwrap();
    (session, seen)
}

/// Feeds `chunks` in `frame`-long pieces. Pieces aligned with the fake's
/// 400 ms words keep its word boundaries where they would be in one stream.
fn feed_in(session: &LiveSession, frame: Millis, chunks: &[Vec<f32>]) {
    for piece in chunks.concat().chunks(frame.to_samples()) {
        session.feed(piece.to_vec());
    }
}

/// Feeds `chunks` in 80 ms frames, like a capture callback.
fn feed(session: &LiveSession, chunks: &[Vec<f32>]) {
    feed_in(session, Millis(80), chunks);
}

/// Every word's start time, in order.
fn word_starts(segments: &[FinalSegment]) -> Vec<u64> {
    segments
        .iter()
        .flat_map(|segment| segment.words.iter().map(|word| word.start.0))
        .collect()
}

fn segments(seen: &Receiver<Seen>) -> Vec<FinalSegment> {
    seen.try_iter()
        .filter_map(|seen| match seen {
            Seen::Segment(segment) => Some(segment),
            Seen::Live(_) | Seen::Progress(_) => None,
        })
        .collect()
}

fn texts(segments: &[FinalSegment]) -> Vec<&str> {
    segments
        .iter()
        .map(|segment| segment.text.as_str())
        .collect()
}

#[test]
fn continuous_audio_becomes_segments_on_the_recording_timeline() {
    let fake = Arc::new(FakeTranscriber::new());
    let config = SessionConfig {
        start: Millis(10_000),
        ..SessionConfig::default()
    };
    let (session, seen) = start(&fake, config);

    // Two utterances with a 2 s pause, then silence for the decoder to pass.
    feed(
        &session,
        &[loud(800), silent(2000), loud(400), silent(2400)],
    );
    session.finish();

    let segments = segments(&seen);
    assert_eq!(texts(&segments), ["w0 w1", "w7"]);
    assert_eq!(
        (segments[0].start, segments[0].end),
        (Millis(10_000), Millis(10_800))
    );
    assert_eq!(segments[1].words[0].start, Millis(12_800));
    assert!(segments.iter().all(|s| s.origin == SegmentOrigin::Stream));
}

#[test]
fn live_text_grows_with_the_open_segment_and_clears_when_it_closes() {
    let fake = Arc::new(FakeTranscriber::new());
    let (session, seen) = start(&fake, SessionConfig::default());

    feed(&session, &[loud(1200), silent(2000)]);
    session.finish();

    let seen: Vec<Seen> = seen.try_iter().collect();
    let live: Vec<String> = seen
        .iter()
        .filter_map(|seen| match seen {
            Seen::Live(text) => Some(text.committed.clone()),
            Seen::Segment(_) | Seen::Progress(_) => None,
        })
        .collect();
    // However the session batched the frames, the open segment's text only
    // ever grows towards the closed segment's, and clears when it closes.
    let (last, growing) = live.split_last().unwrap();
    assert_eq!(last, "");
    assert!(!growing.is_empty());
    assert!(
        growing
            .iter()
            .all(|text| "w0 w1 w2".starts_with(text.as_str()))
    );
    assert!(growing.windows(2).all(|pair| pair[1].starts_with(&pair[0])));
    let closed: Vec<&str> = seen
        .iter()
        .filter_map(|seen| match seen {
            Seen::Segment(segment) => Some(segment.text.as_str()),
            Seen::Live(_) | Seen::Progress(_) => None,
        })
        .collect();
    assert_eq!(closed, ["w0 w1 w2"]);
}

#[test]
fn a_lost_stream_is_re_transcribed_from_retained_audio_without_losing_words() {
    // The first stream dies about a second in, mid-sentence.
    let fake = Arc::new(FakeTranscriber::new().losing_streams_after(Millis(1000), 1));
    let (session, seen) = start(&fake, SessionConfig::default());

    let speech = [loud(2000), silent(2000), loud(800), silent(2000)];
    feed_in(&session, Millis(400), &speech);
    session.finish();

    let segments = segments(&seen);
    assert_eq!(
        word_starts(&segments),
        [0, 400, 800, 1200, 1600, 4000, 4400]
    );
    assert_eq!(segments[0].origin, SegmentOrigin::BatchFallback);
    assert_eq!(segments.last().unwrap().origin, SegmentOrigin::Stream);
    assert_eq!(
        fake.calls().streams,
        2,
        "a new stream opened after the loss"
    );
}

#[test]
fn the_open_segment_is_flushed_when_the_session_finishes() {
    let fake = Arc::new(FakeTranscriber::new());
    let (session, seen) = start(&fake, SessionConfig::default());

    feed(&session, &[loud(1200)]);
    let report = session.finish();

    assert_eq!(texts(&segments(&seen)), ["w0 w1 w2"]);
    assert_eq!(report.segments, 1);
    assert!(
        fake.calls().batch_samples.is_empty(),
        "no fallback was needed"
    );
}

#[test]
fn a_long_stream_is_rotated_at_a_silence() {
    let fake = Arc::new(FakeTranscriber::new());
    let config = SessionConfig {
        rotate_after: Millis(3000),
        ..SessionConfig::default()
    };
    let (session, seen) = start(&fake, config);

    // Short enough that the second stream never reaches the rotation length.
    let speech = [loud(800), silent(3200), loud(400), silent(1600)];
    feed_in(&session, Millis(400), &speech);
    session.finish();

    let segments = segments(&seen);
    assert_eq!(
        word_starts(&segments),
        [0, 400, 4000],
        "the new stream's times are offset"
    );
    assert_eq!(segments.len(), 2);
    assert_eq!(fake.calls().streams, 2);
}

/// Words delivered so far: the closed segments' and the open one's.
fn delivered_words(live: &LiveText, closed: &[FinalSegment]) -> usize {
    let open = live.committed.split_whitespace().count();
    closed.iter().map(|s| s.words.len()).sum::<usize>() + open
}

#[test]
fn the_progress_watermark_only_rises_and_follows_the_words_it_covers() {
    let fake = Arc::new(FakeTranscriber::new());
    let (session, seen) = start(&fake, SessionConfig::default());

    feed(
        &session,
        &[loud(1200), silent(2000), loud(800), silent(2000)],
    );
    session.finish();

    let mut closed = Vec::new();
    let mut live = LiveText::default();
    let mut last = Millis::ZERO;
    for event in seen.try_iter() {
        match event {
            Seen::Live(text) => live = text,
            Seen::Segment(segment) => closed.push(segment),
            Seen::Progress(through) => {
                assert!(through > last, "{through:?} after {last:?}");
                last = through;
                // Every word ending at or before the watermark is out.
                assert!(delivered_words(&live, &closed) >= word_count(through));
            }
        }
    }
    assert_eq!(
        last,
        Millis(6_000),
        "the final watermark covers all the audio"
    );
}

/// Words of the fed speech (`w0`–`w2` then `w8`, `w9`) ending by `through`.
fn word_count(through: Millis) -> usize {
    [400, 800, 1200, 3600, 4000]
        .iter()
        .filter(|end| **end <= through.0)
        .count()
}

#[test]
fn a_lost_stream_is_counted_in_the_lane_status() {
    let fake = Arc::new(FakeTranscriber::new().losing_streams_after(Millis(1000), 1));
    let (session, seen) = start(&fake, SessionConfig::default());
    assert_eq!(session.status().recoveries, 0);

    feed_in(&session, Millis(400), &[loud(2000), silent(2000)]);
    let recovered = (0..500).any(|_| {
        seen.recv_timeout(std::time::Duration::from_millis(10)).ok();
        session.status().recoveries == 1
    });

    assert!(recovered, "the loss is reported while the lane runs");
    assert!(session.status().streaming, "a new stream opened");
    session.finish();
}
