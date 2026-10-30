use std::sync::Mutex;

use super::*;

struct Fixed(Option<MeetingId>);

impl Recorder for Fixed {
    fn recording(&self) -> Option<MeetingId> {
        self.0.clone()
    }
}

struct Wanted(bool);

#[async_trait]
impl FollowWanted for Wanted {
    async fn wanted(&self) -> bool {
        self.0
    }
}

#[test]
fn only_a_track_recorded_in_one_file_is_followed() {
    let dir = tempfile::tempdir().unwrap();
    let audio = dir.path().join("audio");
    std::fs::create_dir_all(&audio).unwrap();
    std::fs::write(audio.join("system.001.m4a.part"), b"").unwrap();
    std::fs::write(audio.join("system.002.m4a.part"), b"").unwrap();

    assert!(
        snapshot(&audio, "mic", dir.path()).unwrap().is_none(),
        "not recorded"
    );
    assert!(
        snapshot(&audio, "system", dir.path()).unwrap().is_none(),
        "two segments"
    );
}

#[test]
fn a_track_without_a_whole_fragment_yet_has_nothing_to_follow() {
    let dir = tempfile::tempdir().unwrap();
    let audio = dir.path().join("audio");
    std::fs::create_dir_all(&audio).unwrap();
    std::fs::write(audio.join("mic.001.m4a.part"), b"\0\0\0\x08free").unwrap();

    assert!(snapshot(&audio, "mic", dir.path()).unwrap().is_none());
}

/// The wanted switch and the recorder decide together.
#[tokio::test]
async fn passes_run_only_while_recording_with_the_live_transcript_off() {
    let meeting = MeetingId::new("meeting-1");
    let cases = [
        (Some(meeting.clone()), true, Some(meeting.clone())),
        (Some(meeting.clone()), false, None),
        (None, true, None),
    ];
    for (recording, wanted, expected) in cases {
        let follower = follower(recording, wanted);
        assert_eq!(follower.recording().await, expected);
    }
}

fn follower(recording: Option<MeetingId>, wanted: bool) -> FollowAlong {
    let forgot = Arc::new(Mutex::new(0));
    let start = chrono::DateTime::from_timestamp(1_700_000_000, 0).unwrap();
    FollowAlong::new(FollowDeps {
        recorder: Arc::new(Fixed(recording)),
        recordings: std::env::temp_dir(),
        transcriber: OfflineTranscriber::new(Arc::new(NoEngine), Arc::new(NoDecoder)),
        memory: PassMemory::new(),
        aligner: Arc::new(deino_core::diarization::NoWordAligner),
        diarizer: Arc::new(deino_core::diarization::FakeTrackDiarizer::default()),
        wanted: Arc::new(Wanted(wanted)),
        forget: Arc::new(move || *forgot.lock().unwrap() += 1),
        clock: Arc::new(deino_domain::FakeClock::new(start)),
    })
}

struct NoEngine;

impl super::super::BatchEngines for NoEngine {
    fn open(
        &self,
    ) -> Result<Arc<dyn deino_stt::BatchTranscriber>, deino_core::final_transcription::TrackError>
    {
        Err(deino_core::final_transcription::TrackError::Unavailable(
            "none".into(),
        ))
    }
}

struct NoDecoder;

impl super::super::TrackDecoder for NoDecoder {
    fn decode(
        &self,
        _path: &Path,
    ) -> Result<Vec<f32>, deino_core::final_transcription::TrackError> {
        Ok(Vec::new())
    }
}
