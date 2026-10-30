use std::sync::Mutex;

use deino_stt::{BatchTranscript, SttError};

use super::*;

/// An engine that numbers its answers, so a remembered one shows.
#[derive(Default)]
struct Counting {
    runs: Mutex<usize>,
}

impl BatchTranscriber for Counting {
    fn transcribe(&self, _pcm: Vec<f32>) -> Result<BatchTranscript, SttError> {
        let mut runs = self.runs.lock().unwrap();
        *runs += 1;
        Ok(BatchTranscript {
            text: format!("run {runs}"),
            tokens: Vec::new(),
        })
    }
}

fn clip(value: f32) -> Vec<f32> {
    vec![value; 1_600]
}

#[test]
fn the_same_clip_on_the_same_model_is_answered_from_memory_by_any_pass() {
    let engine = Arc::new(Counting::default());
    let memory = PassMemory::new();
    let during = memory.following().engine(engine.clone(), "whisper".into());
    let after = memory.engine(engine.clone(), "whisper".into());

    let first = during.transcribe(clip(0.25)).unwrap();
    let again = after.transcribe(clip(0.25)).unwrap();

    assert_eq!(first.text, "run 1");
    assert_eq!(again.text, "run 1");
    assert_eq!(*engine.runs.lock().unwrap(), 1);
}

#[test]
fn a_different_sample_or_model_is_asked_again() {
    let engine = Arc::new(Counting::default());
    let memory = PassMemory::new();
    let whisper = memory.engine(engine.clone(), "whisper".into());
    let parakeet = memory.engine(engine.clone(), "parakeet".into());
    let mut changed = clip(0.25);
    changed[800] = 0.250_001;

    whisper.transcribe(clip(0.25)).unwrap();
    let other_audio = whisper.transcribe(changed).unwrap();
    let other_model = parakeet.transcribe(clip(0.25)).unwrap();

    assert_eq!(other_audio.text, "run 2");
    assert_eq!(other_model.text, "run 3");
}

#[test]
fn forgetting_starts_a_new_recording_empty() {
    let engine = Arc::new(Counting::default());
    let memory = PassMemory::new();
    let remembered = memory.engine(engine.clone(), "whisper".into());

    remembered.transcribe(clip(0.5)).unwrap();
    memory.forget();
    let fresh = remembered.transcribe(clip(0.5)).unwrap();

    assert_eq!(fresh.text, "run 2");
}

#[test]
fn fingerprints_tell_apart_order_and_length() {
    assert_ne!(fingerprint(&[0.1, 0.2]), fingerprint(&[0.2, 0.1]));
    assert_ne!(fingerprint(&[0.0]), fingerprint(&[0.0, 0.0]));
    assert_eq!(fingerprint(&[0.1, 0.2]), fingerprint(&[0.1, 0.2]));
}

#[tokio::test]
async fn the_pass_after_the_meeting_waits_for_one_during_it() {
    let memory = PassMemory::new();
    let during = memory.exclusive().await;

    let waiting = tokio::time::timeout(std::time::Duration::from_millis(20), memory.shared());
    assert!(waiting.await.is_err(), "held while a pass runs");
    drop(during);

    let _after = memory.shared().await;
}
