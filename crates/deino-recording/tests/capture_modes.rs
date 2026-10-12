//! End to end with the one-on-one fixture: the two ways a recording's
//! transcript is shaped beyond what the engines hear.
//!
//! - **Live transcription off (CAP-08):** no engine opens while recording,
//!   nothing is saved live, and after stop the `final_transcription` rules
//!   transcribe both tracks into Me and Them lines where the fixture speaks.
//! - **Live vocabulary (TRN-05):** every line saved and shown while
//!   recording is corrected, and every original is kept for undo.
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::print_stdout,
    reason = "an integration-test crate: helpers fail the test by panicking, and report numbers"
)]

#[path = "support/rig.rs"]
mod rig;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use chrono::DateTime;
use deino_audio::TestSignal;
use deino_audio::capture::OutputRoute;
use deino_core::final_transcription::{
    FinalOutcome, FinalPass, FinalTranscriptionDeps, FinalTranscriptionService, TrackError,
};
use deino_core::meetings::{MeetingAudio, MeetingLifecycleRepo, SqliteMeetingRepo};
use deino_core::segments::{Page, Segment, SegmentReader, SpeakerRepo, SqliteSegmentRepo};
use deino_core::vocabulary::{Corrector, Lexicon, RecordedLiveVocabulary, VocabTerm};
use deino_domain::{Channel, Clock, FakeClock, MeetingId, MeetingStatus, VocabKind, VocabTermId};
use deino_recording::engine::{EngineError, LaneEngines, LaneRequest, LaneTranscriber};
use deino_recording::{BatchEngines, OfflineTranscriber, Preferences, StartRequest, TrackDecoder};
use deino_stt::BatchTranscriber;
use deino_stt::transcriber::FakeTranscriber;
use deino_testkit::Fixtures;
use deino_testkit::fixture::{MeetingKind, PcmTrack, Segment as TruthSegment};
use rig::{FastClock, FixtureCapture, RigOptions, rig_over};
use tokio_util::sync::CancellationToken;

/// Recording time per real second.
const SPEED: u32 = 8;

/// How far an utterance may sit from the fixture line it transcribes.
const SLACK_MS: u32 = 800;

struct Fixture {
    mic: PcmTrack,
    system: PcmTrack,
    truth: Vec<TruthSegment>,
    length: Duration,
}

fn one_on_one() -> Fixture {
    let fixtures = Fixtures::load().unwrap();
    let meeting = fixtures.meeting(MeetingKind::OneOnOne).unwrap();
    let audio = meeting.audio.as_ref().unwrap();
    let decode = |channel| PcmTrack::decode(&fixtures.audio_path(audio, channel).unwrap()).unwrap();
    Fixture {
        mic: decode(Channel::Mic),
        system: decode(Channel::System),
        truth: meeting
            .segments
            .iter()
            .filter(|s| s.end_ms.0 <= audio.clip_end_ms.0)
            .cloned()
            .collect(),
        length: Duration::from_millis(u64::from(audio.clip_end_ms.0)),
    }
}

/// Lane engines that refuse to open, so a recording that opens one fails.
struct NoEngines;

impl LaneEngines for NoEngines {
    fn open(&self, request: LaneRequest) -> Result<Box<dyn LaneTranscriber>, EngineError> {
        let refused = std::io::Error::other("a record-only recording opened an engine");
        Err(EngineError {
            lane: request.lane,
            source: Box::new(refused),
        })
    }
}

/// Records the fixture (on headphones) with `options`, then stops.
async fn record(fixture: &Fixture, options: RigOptions) -> (rig::Rig, MeetingId) {
    let capture = FixtureCapture {
        clock: Arc::new(FastClock::new(SPEED)),
        microphone: TestSignal::from_pcm(&fixture.mic.samples, fixture.mic.sample_rate),
        system: TestSignal::from_pcm(&fixture.system.samples, fixture.system.sample_rate),
        route: OutputRoute::Headphones,
    };
    let rig = rig_over(capture, options).await;
    let meeting = rig
        .service
        .start(StartRequest::titled(Some("Weekly one-on-one".into())))
        .await
        .unwrap();
    let (length, clock) = (
        fixture.length + Duration::from_millis(1_500),
        Arc::clone(&rig.clock),
    );
    tokio::task::spawn_blocking(move || clock.wait_for(length))
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

/// The fixture's tracks by file name, as the finalized files would decode.
struct FixtureTracks {
    mic: Vec<f32>,
    system: Vec<f32>,
}

impl TrackDecoder for FixtureTracks {
    fn decode(&self, path: &Path) -> Result<Vec<f32>, TrackError> {
        let mic = path.file_name().is_some_and(|name| name == "mic.m4a");
        Ok(if mic {
            self.mic.clone()
        } else {
            self.system.clone()
        })
    }
}

/// The scripted batch engine.
struct Scripted;

impl BatchEngines for Scripted {
    fn open(&self) -> Result<Arc<dyn BatchTranscriber>, TrackError> {
        Ok(Arc::new(FakeTranscriber::new()))
    }
}

/// Runs the `final_transcription` rules over the rig's database.
async fn transcribe_after(rig: &rig::Rig, meeting: &MeetingId, fixture: &Fixture) -> FinalOutcome {
    let meetings = Arc::new(SqliteMeetingRepo::new(rig.database.clone()));
    let folder = PathBuf::from(rig.recordings.path());
    let audio = MeetingAudio {
        audio_mic_path: Some(folder.join("mic.m4a").display().to_string()),
        audio_system_path: Some(folder.join("system.m4a").display().to_string()),
        ..MeetingAudio::default()
    };
    meetings
        .set_audio(meeting, &audio, rig.wall.now())
        .await
        .unwrap();
    let tracks = FixtureTracks {
        mic: fixture.mic.samples.clone(),
        system: fixture.system.samples.clone(),
    };
    let segments = Arc::new(SqliteSegmentRepo::new(rig.database.clone()));
    let service = FinalTranscriptionService::new(FinalTranscriptionDeps {
        meetings,
        segments: segments.clone(),
        writer: segments.clone(),
        speakers: segments,
        transcriber: Arc::new(OfflineTranscriber::new(
            Arc::new(Scripted),
            Arc::new(tracks),
        )),
        clock: rig.wall.clone(),
    });
    let pass = FinalPass {
        repass: false,
        cancel: CancellationToken::new(),
    };
    service.run(meeting, &pass).await.unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn with_live_transcription_off_segments_appear_after_stop() {
    let fixture = one_on_one();
    let options = RigOptions {
        engines: Arc::new(NoEngines),
        preferences: Preferences {
            record_only: true,
            ..Preferences::default()
        },
        ..RigOptions::default()
    };
    let (rig, meeting) = record(&fixture, options).await;
    assert!(
        stored(&rig, &meeting).await.is_empty(),
        "nothing is transcribed live"
    );
    assert!(rig.events.segments().is_empty());
    let state = SqliteMeetingRepo::new(rig.database.clone())
        .state(&meeting)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(state.status, MeetingStatus::Processing);

    let outcome = transcribe_after(&rig, &meeting, &fixture).await;

    let segments = stored(&rig, &meeting).await;
    println!("{outcome:?}");
    assert!(matches!(outcome, FinalOutcome::Written { replaced: 0, .. }));
    assert_lines_where_the_fixture_speaks(&segments, &fixture);
    let speakers = SqliteSegmentRepo::new(rig.database.clone())
        .speakers(&meeting)
        .await
        .unwrap();
    let me = speakers.iter().find(|s| s.is_me).unwrap().id;
    let them = speakers.iter().find(|s| !s.is_me).unwrap().id;
    for segment in &segments {
        let expected = if segment.channel == Channel::Mic {
            me
        } else {
            them
        };
        assert_eq!(segment.speaker_id, Some(expected));
    }
}

/// Both channels have lines; each sits where the fixture speaks on its
/// channel, and most fixture lines are heard.
fn assert_lines_where_the_fixture_speaks(segments: &[Segment], fixture: &Fixture) {
    let near = |s: &Segment, t: &TruthSegment| {
        t.channel == s.channel
            && s.start.0 <= t.end_ms.0 + SLACK_MS
            && s.end.0 + SLACK_MS >= t.start_ms.0
    };
    for channel in [Channel::Mic, Channel::System] {
        assert!(
            segments.iter().any(|s| s.channel == channel),
            "{channel:?} has no lines"
        );
    }
    for segment in segments {
        assert!(
            fixture.truth.iter().any(|t| near(segment, t)),
            "a stray line at {} ms",
            segment.start.0
        );
    }
    let heard = fixture
        .truth
        .iter()
        .filter(|t| segments.iter().any(|s| near(s, t)))
        .count();
    println!("{heard} of {} fixture lines heard", fixture.truth.len());
    assert!(heard * 10 >= fixture.truth.len() * 9);
}

/// Every word the scripted engine can say ("w0", "w1", ...) sounds like
/// "Shreyas".
fn everything_is_shreyas() -> Corrector {
    let term = VocabTerm {
        id: VocabTermId::generate(&FakeClock::new(DateTime::UNIX_EPOCH)),
        kind: VocabKind::Term,
        text: "Shreyas".into(),
        sounds_like: (0..1_000).map(|n| format!("w{n}")).collect(),
        created_at: DateTime::UNIX_EPOCH,
        updated_at: DateTime::UNIX_EPOCH,
    };
    Corrector::new(Lexicon::new(&[term], &[]))
}

#[tokio::test(flavor = "multi_thread")]
async fn live_lines_are_saved_and_shown_corrected_with_their_originals_kept() {
    let fixture = one_on_one();
    let vocabulary = Arc::new(RecordedLiveVocabulary::new(everything_is_shreyas()));
    let options = RigOptions {
        vocabulary: vocabulary.clone(),
        ..RigOptions::default()
    };

    let (rig, meeting) = record(&fixture, options).await;

    let segments = stored(&rig, &meeting).await;
    assert!(!segments.is_empty());
    let corrected = |text: &str| {
        text.split_whitespace()
            .all(|word| word.trim_matches('.') == "Shreyas")
    };
    assert!(
        segments.iter().all(|s| corrected(&s.text)),
        "every saved line is corrected"
    );
    assert!(
        rig.events.segments().iter().all(|s| corrected(&s.text)),
        "every shown line is corrected"
    );
    let kept = vocabulary.kept();
    assert!(segments.iter().all(|s| {
        kept.iter()
            .any(|k| k.segment_id == s.id && k.original_text.contains('w'))
    }));
}
