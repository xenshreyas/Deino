//! End to end: the one-on-one fixture's two tracks played through mock
//! capture, the conditioners, the track writers, the live pipeline (echo
//! cancellation, VAD, two lanes on the scripted fake engine) and the
//! transcript router, into a real SQLite database.
//!
//! - **Headphones:** every Me and Them line of the fixture lands on its own
//!   channel, at its time, with the lane's speaker.
//! - **Speakers:** the microphone also hears the call (the system track,
//!   40 ms late, at −9 dB): fewer than 5 % of the stored lines are
//!   duplicates of Them (CAP-02).
//!
//! The recording is paced by sample count, not wall time
//! (`support/paced.rs`): it runs as fast as the lanes transcribe, so the
//! result depends on the audio alone, however loaded the machine is.
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::print_stdout,
    clippy::cast_precision_loss,
    reason = "an integration-test crate: helpers fail the test by panicking, and report numbers"
)]

#[path = "support/paced.rs"]
mod paced;
#[path = "support/rig.rs"]
mod rig;

use std::sync::Arc;
use std::time::Duration;

use deino_audio::TestSignal;
use deino_audio::capture::OutputRoute;
use deino_core::meetings::MeetingLifecycleRepo;
use deino_core::segments::{Page, Segment, SegmentReader, SpeakerRepo, SqliteSegmentRepo};
use deino_domain::{Channel, MeetingId, MeetingStatus};
use deino_recording::{SegmentUpdateKind, StartRequest};
use deino_testkit::Fixtures;
use deino_testkit::fixture::{MeetingKind, PcmTrack, Segment as TruthSegment};
use paced::paced;
use rig::rig_on;

/// How far a stored segment may sit from the fixture's (the fake engine
/// works in 400 ms blocks, and a word ends where its block ends).
const SLACK_MS: u32 = 800;

/// The fixture: its tracks (padded to one length, so they loop together)
/// and the segments spoken in its clip.
struct Fixture {
    mic: PcmTrack,
    system: PcmTrack,
    truth: Vec<TruthSegment>,
    length: Duration,
}

impl Fixture {
    /// The fixture played `loops` times back to back.
    fn looped(mut self, loops: u32) -> Self {
        let period = self.mic.duration().0;
        self.truth = (0..loops)
            .flat_map(|k| {
                self.truth.iter().map(move |t| TruthSegment {
                    start_ms: deino_domain::Millis(t.start_ms.0 + k * period),
                    end_ms: deino_domain::Millis(t.end_ms.0 + k * period),
                    ..t.clone()
                })
            })
            .collect();
        self.length = Duration::from_millis(u64::from(period)) * (loops - 1) + self.length;
        self
    }
}

fn one_on_one() -> Fixture {
    let fixtures = Fixtures::load().unwrap();
    let meeting = fixtures.meeting(MeetingKind::OneOnOne).unwrap();
    let audio = meeting.audio.as_ref().unwrap();
    let path = |channel| fixtures.audio_path(audio, channel).unwrap();
    let truth = meeting
        .segments
        .iter()
        .filter(|segment| segment.end_ms.0 <= audio.clip_end_ms.0)
        .cloned()
        .collect();
    let (mut mic, mut system) = (
        PcmTrack::decode(&path(Channel::Mic)).unwrap(),
        PcmTrack::decode(&path(Channel::System)).unwrap(),
    );
    assert_eq!(mic.sample_rate, system.sample_rate);
    let len = mic.samples.len().max(system.samples.len());
    mic.samples.resize(len, 0.0);
    system.samples.resize(len, 0.0);
    Fixture {
        mic,
        system,
        truth,
        length: Duration::from_millis(u64::from(audio.clip_end_ms.0)),
    }
}

/// The microphone hearing the call through speakers: `mic + gain × system`
/// delayed by `delay_ms`.
fn with_bleed(mic: &PcmTrack, system: &PcmTrack, delay_ms: u32, gain: f32) -> Vec<f32> {
    let delay = usize::try_from(delay_ms * mic.sample_rate / 1_000).unwrap();
    (0..mic.samples.len())
        .map(|i| {
            let echo = i
                .checked_sub(delay)
                .and_then(|j| system.samples.get(j))
                .copied()
                .unwrap_or(0.0);
            (mic.samples[i] + gain * echo).clamp(-1.0, 1.0)
        })
        .collect()
}

/// Records `microphone` and the fixture's system track on `route` for the
/// fixture's length.
async fn record(
    fixture: &Fixture,
    microphone: TestSignal,
    route: OutputRoute,
) -> (rig::Rig, MeetingId) {
    let system = TestSignal::from_pcm(&fixture.system.samples, fixture.system.sample_rate);
    let (harness, options) = paced(microphone, system, route);
    let rig = rig_on(harness, options).await;
    let meeting = rig
        .service
        .start(StartRequest::titled(Some("Weekly one-on-one".into())))
        .await
        .unwrap();
    tokio::task::spawn_blocking({
        let (length, clock) = (
            fixture.length + Duration::from_millis(1_500),
            Arc::clone(&rig.clock),
        );
        move || clock.wait_for(length)
    })
    .await
    .unwrap();
    assert_eq!(rig.service.stop().await.unwrap(), meeting);
    (rig, meeting)
}

async fn stored(rig: &rig::Rig, meeting: &MeetingId) -> Vec<Segment> {
    SqliteSegmentRepo::new(rig.database.clone())
        .list(meeting, Page::ALL)
        .await
        .unwrap()
}

/// Whether the stored segment covers the fixture segment's time.
fn covers(segment: &Segment, truth: &TruthSegment) -> bool {
    segment.start.0 <= truth.end_ms.0 + SLACK_MS && segment.end.0 + SLACK_MS >= truth.start_ms.0
}

/// Stored segments on `channel` that no fixture segment of that channel explains.
fn unexplained<'a>(
    segments: &'a [Segment],
    truth: &[TruthSegment],
    channel: Channel,
) -> Vec<&'a Segment> {
    segments
        .iter()
        .filter(|s| s.channel == channel)
        .filter(|s| !truth.iter().any(|t| t.channel == channel && covers(s, t)))
        .collect()
}

/// At least 90 % of `channel`'s fixture lines are heard, and nothing else.
fn assert_channel_heard(segments: &[Segment], fixture: &Fixture, channel: Channel) {
    let expected: Vec<&TruthSegment> = fixture
        .truth
        .iter()
        .filter(|t| t.channel == channel)
        .collect();
    let heard = expected
        .iter()
        .filter(|t| {
            segments
                .iter()
                .any(|s| s.channel == channel && covers(s, t))
        })
        .count();
    println!(
        "{channel:?}: {heard} of {} fixture lines heard",
        expected.len()
    );
    assert!(
        heard * 10 >= expected.len() * 9,
        "{channel:?}: {heard} of {}",
        expected.len()
    );
    let stray: Vec<(u32, u32, &str)> = unexplained(segments, &fixture.truth, channel)
        .iter()
        .map(|s| (s.start.0, s.end.0, s.text.as_str()))
        .collect();
    assert!(stray.is_empty(), "{channel:?} has stray lines: {stray:?}");
}

/// Me lines carry the Me speaker; the meeting is processing with its length.
async fn assert_meeting_closed(
    rig: &rig::Rig,
    meeting: &MeetingId,
    segments: &[Segment],
    fixture: &Fixture,
) {
    let speakers = SqliteSegmentRepo::new(rig.database.clone())
        .speakers(meeting)
        .await
        .unwrap();
    let me = speakers.iter().find(|s| s.is_me).unwrap();
    assert!(
        segments
            .iter()
            .filter(|s| s.channel == Channel::Mic)
            .all(|s| s.speaker_id == Some(me.id))
    );
    let state = deino_core::meetings::SqliteMeetingRepo::new(rig.database.clone())
        .state(meeting)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(state.status, MeetingStatus::Processing);
    let duration = state.duration.unwrap().0;
    assert!(
        duration.abs_diff(u32::try_from(fixture.length.as_secs()).unwrap()) <= 3,
        "{duration} s"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn on_headphones_every_line_lands_on_its_own_channel_at_its_time() {
    let fixture = one_on_one();
    let mic = TestSignal::from_pcm(&fixture.mic.samples, fixture.mic.sample_rate);
    let (rig, meeting) = record(&fixture, mic, OutputRoute::Headphones).await;
    let segments = stored(&rig, &meeting).await;

    for channel in [Channel::Mic, Channel::System] {
        assert_channel_heard(&segments, &fixture, channel);
    }
    assert_meeting_closed(&rig, &meeting, &segments, &fixture).await;
    let manifest = rig
        .recordings
        .path()
        .join(meeting.as_str())
        .join("recording.json");
    assert!(manifest.exists());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn on_speakers_fewer_than_five_percent_of_lines_are_echo() {
    let fixture = one_on_one();
    let bled = with_bleed(&fixture.mic, &fixture.system, 40, 0.35);
    let mic = TestSignal::from_pcm(&bled, fixture.mic.sample_rate);
    let (rig, meeting) = record(&fixture, mic, OutputRoute::Speakers).await;
    let segments = stored(&rig, &meeting).await;

    assert_few_echoes(&rig, &segments, &fixture);
}

/// CAP-02's acceptance on a 10-minute speakers-mode recording (the fixture
/// looped 11 times).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "ten minutes of audio; run with --ignored"]
async fn on_speakers_for_ten_minutes_fewer_than_five_percent_of_lines_are_echo() {
    let fixture = one_on_one().looped(11);
    let bled = with_bleed(&fixture.mic, &fixture.system, 40, 0.35);
    let mic = TestSignal::from_pcm(&bled, fixture.mic.sample_rate);
    let (rig, meeting) = record(&fixture, mic, OutputRoute::Speakers).await;
    let segments = stored(&rig, &meeting).await;

    assert_few_echoes(&rig, &segments, &fixture);
}

/// Fewer than 5 % of stored lines are Me lines that only Them's speech explains.
fn assert_few_echoes(rig: &rig::Rig, segments: &[Segment], fixture: &Fixture) {
    for echo in unexplained(segments, &fixture.truth, Channel::Mic) {
        let near: Vec<(u32, u32, &str)> = segments
            .iter()
            .filter(|s| {
                s.channel == Channel::System
                    && s.start.0 <= echo.end.0 + 3_000
                    && s.end.0 + 3_000 >= echo.start.0
            })
            .map(|s| (s.start.0, s.end.0, s.text.as_str()))
            .collect();
        println!(
            "echo {}-{} {:?}; Them nearby: {near:?}",
            echo.start.0, echo.end.0, echo.text
        );
    }
    let echoes = unexplained(segments, &fixture.truth, Channel::Mic).len();
    let share = echoes as f64 / segments.len().max(1) as f64;
    let held_back = rig
        .events
        .segments()
        .iter()
        .filter(|event| event.state == SegmentUpdateKind::Discarded)
        .count();
    println!(
        "speakers: {echoes} echo lines of {} stored ({:.1} %); {held_back} discarded by the holdback",
        segments.len(),
        share * 100.0
    );
    assert!(
        share < 0.05,
        "{echoes} of {} lines are echo",
        segments.len()
    );
    let them = segments
        .iter()
        .filter(|s| s.channel == Channel::System)
        .count();
    assert!(them > 0, "the call is still transcribed");
}
