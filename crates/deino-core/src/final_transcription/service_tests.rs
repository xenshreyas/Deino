use std::sync::Arc;

use deino_domain::{
    ErrorCode, FakeClock, MeetingSource, MeetingStatus, Millis, PaletteColor, SegmentId, SpeakerId,
};
use deino_testkit::at;

use super::*;
use crate::final_transcription::ScriptedTracks;
use crate::meetings::{FakeMeetingRepo, MeetingTitle, NewMeeting};
use crate::segments::{FakeSegmentRepo, Speaker, WordTiming};

const MIC: &str = "/recordings/m/mic.m4a";
const SYSTEM: &str = "/recordings/m/system.m4a";

struct World {
    meetings: Arc<FakeMeetingRepo>,
    segments: Arc<FakeSegmentRepo>,
    tracks: Arc<ScriptedTracks>,
    clock: Arc<FakeClock>,
    service: FinalTranscriptionService,
}

impl World {
    async fn new(audio: MeetingAudio) -> Self {
        let (meetings, segments) = (
            Arc::new(FakeMeetingRepo::default()),
            Arc::new(FakeSegmentRepo::default()),
        );
        let tracks = Arc::new(ScriptedTracks::default());
        let clock = Arc::new(FakeClock::new(at("2026-10-11T10:00:00Z")));
        meetings.create(&new_meeting()).await.unwrap();
        meetings
            .set_audio(&meeting(), &audio, at("2026-10-11T10:00:00Z"))
            .await
            .unwrap();
        let service = FinalTranscriptionService::new(FinalTranscriptionDeps {
            meetings: meetings.clone(),
            segments: segments.clone(),
            writer: segments.clone(),
            speakers: segments.clone(),
            transcriber: tracks.clone(),
            originals: Arc::new(crate::final_transcription::FakeOriginalTranscripts::default()),
            clock: clock.clone(),
        });
        Self {
            meetings,
            segments,
            tracks,
            clock,
            service,
        }
    }

    async fn split() -> Self {
        Self::new(MeetingAudio {
            audio_mic_path: Some(MIC.into()),
            audio_system_path: Some(SYSTEM.into()),
            ..MeetingAudio::default()
        })
        .await
    }

    async fn speaker(&self, label: &str, is_me: bool) -> SpeakerId {
        let id = SpeakerId::generate(self.clock.as_ref());
        let speaker = Speaker {
            id,
            meeting_id: meeting(),
            label: label.into(),
            person_id: None,
            is_me,
            color: PaletteColor::cycle(1),
        };
        self.segments.add_speaker(&speaker).await.unwrap();
        id
    }

    async fn live(&self, channel: Channel, (start, end): (u32, u32), speaker: SpeakerId) {
        let row = NewSegment {
            id: SegmentId::generate(self.clock.as_ref()),
            meeting_id: meeting(),
            channel,
            speaker_id: Some(speaker),
            start: Millis(start),
            end: Millis(end),
            text: "live".into(),
            words: Vec::new(),
        };
        self.segments.append(&[row]).await.unwrap();
    }

    async fn run(&self, repass: bool) -> Result<FinalOutcome, FinalTranscriptionError> {
        let pass = FinalPass {
            repass,
            cloud: None,
            cancel: CancellationToken::new(),
        };
        self.service.run(&meeting(), &pass).await
    }

    async fn transcript(&self) -> Vec<Segment> {
        self.segments.list(&meeting(), Page::ALL).await.unwrap()
    }
}

fn meeting() -> MeetingId {
    MeetingId::new("meeting-a")
}

fn new_meeting() -> NewMeeting {
    NewMeeting {
        id: meeting(),
        title: MeetingTitle::parse("Standup").unwrap(),
        status: MeetingStatus::Processing,
        source: MeetingSource::Live,
        created_at: at("2026-10-11T09:00:00Z"),
        started_at: None,
        folder_path: None,
    }
}

fn heard(start: u32, end: u32, text: &str) -> HeardSpan {
    let word = WordTiming {
        text: text.into(),
        start: Millis(start),
        end: Millis(end),
    };
    HeardSpan {
        start: Millis(start),
        end: Millis(end),
        text: text.into(),
        words: vec![word],
    }
}

#[tokio::test]
async fn a_record_only_meeting_is_transcribed_on_both_channels_after_stop() {
    let world = World::split().await;
    let (me, them) = (
        world.speaker("Me", true).await,
        world.speaker("Them", false).await,
    );
    world
        .tracks
        .say(SYSTEM, vec![heard(500, 1_500, "Hi Sreyas")]);
    world.tracks.say(MIC, vec![heard(2_000, 2_600, "Hello")]);

    let outcome = world.run(false).await.unwrap();

    assert_eq!(
        outcome,
        FinalOutcome::Written {
            segments: 2,
            replaced: 0
        }
    );
    let transcript = world.transcript().await;
    let lines: Vec<(Channel, Option<SpeakerId>, &str)> = transcript
        .iter()
        .map(|s| (s.channel, s.speaker_id, s.text.as_str()))
        .collect();
    assert_eq!(
        lines,
        [
            (Channel::System, Some(them), "Hi Sreyas"),
            (Channel::Mic, Some(me), "Hello")
        ]
    );
}

#[tokio::test]
async fn the_live_transcript_stays_unless_the_re_pass_is_on() {
    let world = World::split().await;
    let me = world.speaker("Me", true).await;
    world.live(Channel::Mic, (0, 1_000), me).await;

    let outcome = world.run(false).await.unwrap();

    assert_eq!(
        outcome,
        FinalOutcome::Skipped(FinalSkip::LiveTranscriptKept)
    );
    assert!(world.tracks.heard().is_empty());
}

#[tokio::test]
async fn a_re_pass_replaces_the_live_lines_and_keeps_their_speakers() {
    let world = World::split().await;
    let (alex, sam) = (
        world.speaker("Alex", false).await,
        world.speaker("Sam", false).await,
    );
    world.live(Channel::System, (0, 2_000), alex).await;
    world.live(Channel::System, (2_000, 6_000), sam).await;
    world.tracks.say(
        SYSTEM,
        vec![
            heard(100, 1_900, "first"),
            heard(2_100, 5_000, "second"),
            heard(9_000, 9_500, "late"),
        ],
    );

    let outcome = world.run(true).await.unwrap();

    assert_eq!(
        outcome,
        FinalOutcome::Written {
            segments: 3,
            replaced: 2
        }
    );
    let speakers: Vec<Option<SpeakerId>> = world
        .transcript()
        .await
        .iter()
        .map(|s| s.speaker_id)
        .collect();
    assert_eq!(
        speakers,
        [Some(alex), Some(sam), None],
        "two remote voices: no lone fallback"
    );
}

#[tokio::test]
async fn a_failed_re_pass_leaves_the_live_transcript_alone() {
    let world = World::split().await;
    let me = world.speaker("Me", true).await;
    world.live(Channel::Mic, (0, 1_000), me).await;
    world.tracks.without_model();

    let error = world.run(true).await.unwrap_err();

    assert!(matches!(
        error,
        FinalTranscriptionError::Repass(TrackError::Unavailable(_))
    ));
    assert_eq!(world.transcript().await[0].text, "live");
}

#[tokio::test]
async fn a_re_pass_never_replaces_the_users_edits() {
    let world = World::split().await;
    let me = world.speaker("Me", true).await;
    world.live(Channel::Mic, (0, 1_000), me).await;
    let id = world.transcript().await[0].id;
    world
        .segments
        .edit_text(&id, "mine", at("2026-10-11T10:05:00Z"))
        .await
        .unwrap();

    let outcome = world.run(true).await.unwrap();

    assert_eq!(outcome, FinalOutcome::Skipped(FinalSkip::EditedByUser));
}

#[tokio::test]
async fn silence_and_missing_audio_are_skipped() {
    let silent = World::split().await;
    let unrecorded = World::new(MeetingAudio::default()).await;

    assert_eq!(
        silent.run(false).await.unwrap(),
        FinalOutcome::Skipped(FinalSkip::NoSpeech)
    );
    assert_eq!(
        unrecorded.run(false).await.unwrap(),
        FinalOutcome::Skipped(FinalSkip::NoAudio)
    );
}

#[tokio::test]
async fn an_older_mixed_recording_is_transcribed_as_one_track() {
    let world = World::new(MeetingAudio {
        audio_path: Some("/m/audio.mp4".into()),
        ..MeetingAudio::default()
    })
    .await;
    world
        .tracks
        .say("/m/audio.mp4", vec![heard(0, 800, "mixed")]);

    world.run(false).await.unwrap();

    assert_eq!(world.transcript().await[0].channel, Channel::Mixed);
}

#[tokio::test]
async fn a_missing_model_is_unavailable_and_a_missing_meeting_not_found() {
    let world = World::split().await;
    world.tracks.without_model();
    let missing = FinalPass {
        repass: false,
        cloud: None,
        cancel: CancellationToken::new(),
    };

    let error = world.run(false).await.unwrap_err();
    let gone = world
        .service
        .run(&MeetingId::new("nope"), &missing)
        .await
        .unwrap_err();

    assert_eq!(error.code(), ErrorCode::Unavailable);
    assert_eq!(gone.code(), ErrorCode::NotFound);
    assert!(world.meetings.state(&meeting()).await.unwrap().is_some());
}
