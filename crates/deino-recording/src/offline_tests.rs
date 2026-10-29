use std::path::PathBuf;
use std::sync::Mutex;

use deino_core::final_transcription::TrackProgress;
use deino_domain::Channel;
use deino_stt::transcriber::FakeTranscriber;
use deino_stt::{BatchTranscript, SttError, TimedToken};
use deino_testkit::Fixtures;
use deino_testkit::fixture::{MeetingKind, PcmTrack};
use tokio_util::sync::CancellationToken;

use super::*;

/// A decoder that returns the same samples for any path.
struct Fixed(Vec<f32>);

impl TrackDecoder for Fixed {
    fn decode(&self, _path: &Path) -> Result<Vec<f32>, TrackError> {
        Ok(self.0.clone())
    }
}

/// Engines over one transcriber, counting how often they are opened.
#[derive(Default)]
struct Engines {
    opened: Mutex<usize>,
}

impl BatchEngines for Engines {
    fn open(&self) -> Result<Arc<dyn BatchTranscriber>, TrackError> {
        *self.opened.lock().unwrap() += 1;
        Ok(Arc::new(FakeTranscriber::new()))
    }
}

fn request() -> TrackRequest {
    TrackRequest {
        audio: PathBuf::from("mic.m4a"),
        channel: Channel::Mic,
        echo_reference: None,
        cancel: CancellationToken::new(),
        progress: TrackProgress::default(),
    }
}

/// A request whose progress is kept in the returned list.
fn recorded_request() -> (TrackRequest, Arc<Mutex<Vec<f32>>>) {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&seen);
    let progress = TrackProgress::new(move |fraction| sink.lock().unwrap().push(fraction));
    (
        TrackRequest {
            progress,
            ..request()
        },
        seen,
    )
}

fn transcriber(samples: Vec<f32>) -> (OfflineTranscriber, Arc<Engines>) {
    let engines = Arc::new(Engines::default());
    (
        OfflineTranscriber::new(engines.clone(), Arc::new(Fixed(samples))),
        engines,
    )
}

fn fixture_microphone() -> (PcmTrack, Vec<(u32, u32)>) {
    let fixtures = Fixtures::load().unwrap();
    let meeting = fixtures.meeting(MeetingKind::OneOnOne).unwrap();
    let audio = meeting.audio.as_ref().unwrap();
    let track = PcmTrack::decode(&fixtures.audio_path(audio, Channel::Mic).unwrap()).unwrap();
    assert_eq!(track.sample_rate, 16_000);
    let spoken = meeting
        .segments
        .iter()
        .filter(|s| s.channel == Channel::Mic)
        .map(|s| (s.start_ms.0, s.end_ms.0))
        .collect();
    (track, spoken)
}

#[tokio::test]
async fn a_fixture_track_becomes_timed_utterances_where_its_speech_is() {
    let (track, spoken) = fixture_microphone();
    let (offline, _) = transcriber(track.samples);

    let heard = offline.transcribe(request()).await.unwrap();

    assert!(heard.len() >= 2, "{} utterances", heard.len());
    assert!(heard.windows(2).all(|pair| pair[0].end <= pair[1].start));
    for span in &heard {
        let near_speech = spoken
            .iter()
            .any(|&(start, end)| span.start.0 + 800 >= start && span.end.0 <= end + 800);
        assert!(
            near_speech,
            "{}..{} ms is not where the fixture speaks",
            span.start.0, span.end.0
        );
        assert!(
            span.words
                .iter()
                .all(|w| w.start >= span.start && w.end <= span.end)
        );
    }
}

#[tokio::test]
async fn silence_opens_no_engine_and_a_cancel_stops_between_utterances() {
    let (silent, engines) = transcriber(vec![0.0; 48_000]);
    assert!(silent.transcribe(request()).await.unwrap().is_empty());
    assert_eq!(*engines.opened.lock().unwrap(), 0);

    let (track, _) = fixture_microphone();
    let (offline, _) = transcriber(track.samples);
    let cancelled = TrackRequest {
        cancel: CancellationToken::new(),
        ..request()
    };
    cancelled.cancel.cancel();

    assert!(matches!(
        offline.transcribe(cancelled).await,
        Err(TrackError::Cancelled)
    ));
}

/// A batch engine that stops after its first word on runs over 10 s, as
/// the STT spike saw on some clips.
struct StopsEarly(FakeTranscriber);

impl BatchTranscriber for StopsEarly {
    fn transcribe(&self, pcm: Vec<f32>) -> Result<BatchTranscript, SttError> {
        let long = pcm.len() > CHECKED_UTTERANCE.to_samples();
        let mut transcript = self.0.transcribe(pcm)?;
        if long {
            transcript.tokens.truncate(1);
            transcript.text = transcript
                .tokens
                .iter()
                .map(|t: &TimedToken| t.text.trim())
                .collect();
        }
        Ok(transcript)
    }
}

#[test]
fn a_run_that_stopped_early_is_read_again_in_halves() {
    let loud = vec![0.5; Millis(12_000).to_samples()];
    let engine = StopsEarly(FakeTranscriber::new());

    let heard = checked(&engine, &loud, Millis(0)..Millis(12_000)).unwrap();

    let words: usize = heard.iter().map(|span| span.words.len()).sum();
    assert_eq!(heard.len(), 2);
    assert_eq!(words, 30, "every 400 ms block of both halves");
    assert_eq!(heard[1].start, deino_domain::Millis(6_000));
}

#[tokio::test]
async fn progress_rises_with_each_utterance_and_ends_at_one() {
    let (track, _) = fixture_microphone();
    let (offline, _) = transcriber(track.samples);
    let (request, seen) = recorded_request();

    let heard = offline.transcribe(request).await.unwrap();

    let seen = seen.lock().unwrap().clone();
    assert!(heard.len() >= 2 && seen.len() > heard.len() / 2, "{seen:?}");
    assert!(seen.windows(2).all(|pair| pair[0] <= pair[1]), "{seen:?}");
    assert!(seen.iter().all(|f| (0.0..=1.0).contains(f)));
    assert_eq!(seen.last(), Some(&1.0));
    assert!(seen[0] < 1.0, "the first utterance is not the whole track");
}

#[tokio::test]
async fn a_silent_track_is_done_at_once() {
    let (silent, _) = transcriber(vec![0.0; 48_000]);
    let (request, seen) = recorded_request();

    silent.transcribe(request).await.unwrap();

    assert_eq!(*seen.lock().unwrap(), [1.0]);
}

/// A decoder that returns the same samples for any path, noting each path.
struct Noted(Vec<f32>, Mutex<Vec<PathBuf>>);

impl TrackDecoder for Noted {
    fn decode(&self, path: &Path) -> Result<Vec<f32>, TrackError> {
        self.1.lock().unwrap().push(path.to_path_buf());
        Ok(self.0.clone())
    }
}

#[tokio::test]
async fn the_microphone_is_cancelled_against_its_reference_unless_on_headphones() {
    let (track, _) = fixture_microphone();
    let decoder = Arc::new(Noted(track.samples, Mutex::default()));
    let engines = Arc::new(Engines::default());
    let speakers = OfflineTranscriber::new(engines.clone(), decoder.clone());
    let headphones = OfflineTranscriber::new(engines, decoder.clone())
        .with_headphones(Arc::new(FixedHeadphones(true)));
    let (request, seen) = recorded_request();
    let request = TrackRequest {
        echo_reference: Some(PathBuf::from("system.m4a")),
        ..request
    };

    speakers.transcribe(request.clone()).await.unwrap();
    let echo_pass: Vec<f32> = seen.lock().unwrap().clone();
    headphones.transcribe(request).await.unwrap();

    let paths = decoder.1.lock().unwrap().clone();
    let names: Vec<&str> = paths.iter().filter_map(|p| p.to_str()).collect();
    assert_eq!(names, ["mic.m4a", "system.m4a", "mic.m4a"]);
    assert!(echo_pass.windows(2).all(|pair| pair[0] <= pair[1]));
    assert!(echo_pass.iter().any(|f| *f > 0.0 && *f <= ECHO_SHARE));
    assert_eq!(echo_pass.last(), Some(&1.0));
}
