use std::path::PathBuf;
use std::sync::Mutex;

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
        cancel: CancellationToken::new(),
    }
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
