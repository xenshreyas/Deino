//! `StreamPerf`: how fast a live lane keeps up, logged without any text.
//!
//! Ported in spirit from Handy's `StreamPerf` (`managers/transcription.rs`,
//! MIT) and extended with the latency CAP-01 is accepted on:
//!
//! - **RTF**: compute time over audio time (below 1 keeps up; the spike
//!   measured about 0.06 on an M4 Max with the live window);
//! - **backlog**: audio queued for the lane plus audio the decoder holds,
//!   and **queue lag** alone: the decoder's own buffer is a sawtooth between
//!   R and C+R by design, so only our queue signals falling behind (the
//!   spike's rule: alarm above twice the chunk);
//! - **first-text latency**: from the moment a segment's first word was
//!   spoken to the moment its text reached the app (the CAP-01 target is
//!   about 1.5 s at p50), and **token latency** for every word.
//!
//! Wall time for a timeline position is estimated from the first feed (the
//! lane receives audio in real time), so latency is measured from when the
//! audio reached the lane. Logs carry numbers only (§3A).

use std::collections::VecDeque;
use std::time::Duration;

use serde::Serialize;

use crate::pcm::Millis;

/// Latency samples kept for percentiles (the most recent ones).
const LATENCY_SAMPLES: usize = 4096;

/// Queue lag past twice the live chunk (2 × 560 ms) means the lane is
/// falling behind; it is logged as a warning.
const QUEUE_LAG_ALARM: Millis = Millis(1120);

/// Log target for perf lines (`RUST_LOG=deino_stt::perf=info`).
pub const TARGET: &str = "deino_stt::perf";

/// One feed, as the lane measured it.
#[derive(Clone, Copy, Debug)]
pub struct FeedSample<'a> {
    /// Monotonic time when the feed returned.
    pub now: Duration,
    /// Timeline position at the end of the fed audio.
    pub fed_to: Millis,
    /// Audio in the feed.
    pub audio: Millis,
    /// Time the feed took.
    pub compute: Duration,
    /// Audio the decoder still holds.
    pub pending: Millis,
    /// Audio waiting in the lane's queue.
    pub queued: Millis,
    /// End times (timeline ms) of the words this feed committed.
    pub token_ends: &'a [i64],
    /// Start times (timeline ms) of the segments this feed opened.
    pub onsets: &'a [i64],
}

/// The numbers a lane reports when it ends (and the benchmark prints).
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct PerfReport {
    /// Audio fed, in seconds.
    pub audio_s: f64,
    /// Compute time over audio time.
    pub rtf: f64,
    /// Feeds made.
    pub feeds: u64,
    /// Segments emitted.
    pub segments: u64,
    /// Streams lost and re-transcribed.
    pub recoveries: u32,
    /// The largest backlog (queued plus pending) seen, in ms.
    pub backlog_max_ms: u64,
    /// The largest queue lag seen, in ms.
    pub queue_lag_max_ms: u64,
    /// Speech onset to first text, p50 and p95, in ms.
    pub first_text_p50_ms: Option<u64>,
    /// See `first_text_p50_ms`.
    pub first_text_p95_ms: Option<u64>,
    /// Word end to its text, p50 and p95, in ms.
    pub token_p50_ms: Option<u64>,
    /// See `token_p50_ms`.
    pub token_p95_ms: Option<u64>,
}

/// Running latency samples.
#[derive(Debug, Default)]
struct Latencies(VecDeque<u64>);

impl Latencies {
    fn record(&mut self, latency: Millis) {
        if self.0.len() == LATENCY_SAMPLES {
            self.0.pop_front();
        }
        self.0.push_back(latency.0);
    }

    /// The `percent`-th percentile, nearest-rank.
    fn percentile(&self, percent: usize) -> Option<u64> {
        let mut sorted: Vec<u64> = self.0.iter().copied().collect();
        sorted.sort_unstable();
        let rank = (sorted.len() * percent).div_ceil(100).max(1);
        sorted.get(rank - 1).copied()
    }
}

/// Per-lane stream metrics.
#[derive(Debug)]
pub struct StreamPerf {
    label: &'static str,
    interval: Duration,
    last_log: Duration,
    /// Wall time of timeline position zero, set at the first feed.
    origin: Option<Duration>,
    audio: Millis,
    compute: Duration,
    feeds: u64,
    segments: u64,
    recoveries: u32,
    backlog: Millis,
    backlog_max: Millis,
    queue_lag_max: Millis,
    first_text: Latencies,
    tokens: Latencies,
}

impl StreamPerf {
    /// Metrics for lane `label`, logged every `interval` from `now`.
    pub fn new(label: &'static str, interval: Duration, now: Duration) -> Self {
        Self {
            label,
            interval,
            last_log: now,
            origin: None,
            audio: Millis::ZERO,
            compute: Duration::ZERO,
            feeds: 0,
            segments: 0,
            recoveries: 0,
            backlog: Millis::ZERO,
            backlog_max: Millis::ZERO,
            queue_lag_max: Millis::ZERO,
            first_text: Latencies::default(),
            tokens: Latencies::default(),
        }
    }

    /// Records one feed and logs when the interval has passed.
    pub fn feed(&mut self, sample: &FeedSample<'_>) {
        let origin = *self
            .origin
            .get_or_insert_with(|| sample.now.saturating_sub(duration(sample.fed_to)));
        self.audio += sample.audio;
        self.compute += sample.compute;
        self.feeds += 1;
        self.track_backlog(sample);
        let latency = |at: i64| {
            let spoken = origin + duration(Millis::from_engine(at));
            Millis(u64::try_from(sample.now.saturating_sub(spoken).as_millis()).unwrap_or(u64::MAX))
        };
        for &end in sample.token_ends {
            self.tokens.record(latency(end));
        }
        for &onset in sample.onsets {
            self.first_text.record(latency(onset));
        }
        if sample.now.saturating_sub(self.last_log) >= self.interval {
            self.last_log = sample.now;
            self.log("stt perf");
        }
    }

    fn track_backlog(&mut self, sample: &FeedSample<'_>) {
        self.backlog = sample.pending + sample.queued;
        self.backlog_max = self.backlog_max.max(self.backlog);
        if sample.queued > QUEUE_LAG_ALARM && sample.queued > self.queue_lag_max {
            log::warn!(
                target: TARGET,
                "lane {} is falling behind: {} ms of audio queued",
                self.label,
                sample.queued.0
            );
        }
        self.queue_lag_max = self.queue_lag_max.max(sample.queued);
    }

    /// Records a segment the lane emitted.
    pub fn segment(&mut self) {
        self.segments += 1;
    }

    /// Records a lost stream that was re-transcribed.
    pub fn recovery(&mut self) {
        self.recoveries += 1;
    }

    /// The numbers so far.
    pub fn report(&self) -> PerfReport {
        PerfReport {
            audio_s: self.audio.as_secs_f64(),
            rtf: self.rtf(),
            feeds: self.feeds,
            segments: self.segments,
            recoveries: self.recoveries,
            backlog_max_ms: self.backlog_max.0,
            queue_lag_max_ms: self.queue_lag_max.0,
            first_text_p50_ms: self.first_text.percentile(50),
            first_text_p95_ms: self.first_text.percentile(95),
            token_p50_ms: self.tokens.percentile(50),
            token_p95_ms: self.tokens.percentile(95),
        }
    }

    /// Logs the totals; called when the lane ends.
    pub fn log_summary(&self) {
        self.log("stt perf (lane ended)");
    }

    fn rtf(&self) -> f64 {
        let audio = self.audio.as_secs_f64();
        if audio > 0.0 {
            self.compute.as_secs_f64() / audio
        } else {
            0.0
        }
    }

    fn log(&self, what: &str) {
        let report = self.report();
        let ms = |value: Option<u64>| value.map_or_else(|| "-".to_string(), |ms| ms.to_string());
        log::info!(
            target: TARGET,
            "{what}: lane={} audio_s={:.1} rtf={:.3} backlog_ms={} backlog_max_ms={} queue_lag_max_ms={} first_text_ms p50={} p95={} token_ms p50={} p95={} feeds={} segments={} recoveries={}",
            self.label,
            report.audio_s,
            report.rtf,
            self.backlog.0,
            report.backlog_max_ms,
            report.queue_lag_max_ms,
            ms(report.first_text_p50_ms),
            ms(report.first_text_p95_ms),
            ms(report.token_p50_ms),
            ms(report.token_p95_ms),
            report.feeds,
            report.segments,
            report.recoveries,
        );
    }
}

fn duration(millis: Millis) -> Duration {
    Duration::from_millis(millis.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample<'a>(
        now_ms: u64,
        fed_to: u64,
        token_ends: &'a [i64],
        onsets: &'a [i64],
    ) -> FeedSample<'a> {
        FeedSample {
            now: Duration::from_millis(now_ms),
            fed_to: Millis(fed_to),
            audio: Millis(500),
            compute: Duration::from_millis(50),
            pending: Millis(200),
            queued: Millis(100),
            token_ends,
            onsets,
        }
    }

    #[test]
    fn latency_is_measured_from_when_the_words_were_spoken() {
        // The lane's first feed (audio 0..500 ms) arrives at wall time 10 s.
        let mut perf = StreamPerf::new("test", Duration::from_mins(1), Duration::ZERO);
        perf.feed(&sample(10_000, 500, &[], &[]));
        // At 11 s the decoder commits a word spoken 400..800 ms into the
        // recording, which opened a segment.
        perf.feed(&sample(11_000, 1000, &[800], &[400]));

        let report = perf.report();

        assert_eq!(report.first_text_p50_ms, Some(1100));
        assert_eq!(report.token_p50_ms, Some(700));
        assert!((report.rtf - 0.1).abs() < 1e-9);
        assert_eq!(report.backlog_max_ms, 300);
    }

    #[test]
    fn percentiles_use_the_nearest_rank() {
        let mut latencies = Latencies::default();
        for ms in 1..=20 {
            latencies.record(Millis(ms * 100));
        }

        assert_eq!(latencies.percentile(50), Some(1000));
        assert_eq!(latencies.percentile(95), Some(1900));
        assert_eq!(Latencies::default().percentile(50), None);
    }
}
