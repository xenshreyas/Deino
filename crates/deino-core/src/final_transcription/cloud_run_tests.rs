//! The cloud branch (CAP-14) over fakes: what replaces what, which speakers
//! the lines get, what is kept, and that a failure writes nothing.

use std::sync::{Arc, Mutex};

use deino_domain::{FakeClock, MeetingSource, MeetingStatus, Millis, PaletteColor, SegmentId};
use deino_testkit::at;

use super::*;
use crate::final_transcription::{
    FakeOriginalTranscripts, FinalPass, FinalTranscriptionDeps, HeardSpan, OriginalTranscriptDeps,
    OriginalTranscriptService, OriginalTranscripts, ScriptedCloud, ScriptedTracks, TrackProgress,
};
use crate::meetings::{
    FakeMeetingRepo, MeetingAudio, MeetingLifecycleRepo, MeetingTitle, NewMeeting,
};
use crate::segments::{FakeSegmentRepo, Page, SegmentReader, SegmentWriter, Speaker, SpeakerRepo};

const MIC: &str = "/recordings/m/mic.m4a";
const SYSTEM: &str = "/recordings/m/system.m4a";

struct World {
    segments: Arc<FakeSegmentRepo>,
    originals: Arc<FakeOriginalTranscripts>,
    cloud: Arc<ScriptedCloud>,
    clock: Arc<FakeClock>,
    service: FinalTranscriptionService,
    fractions: Arc<Mutex<Vec<f32>>>,
}

impl World {
    async fn new() -> Self {
        let meetings = Arc::new(FakeMeetingRepo::default());
        let segments = Arc::new(FakeSegmentRepo::default());
        let originals = Arc::new(FakeOriginalTranscripts::default());
        let clock = Arc::new(FakeClock::new(at("2026-10-12T10:00:00Z")));
        meetings.create(&new_meeting()).await.unwrap();
        let audio = MeetingAudio {
            audio_mic_path: Some(MIC.into()),
            audio_system_path: Some(SYSTEM.into()),
            ..MeetingAudio::default()
        };
        meetings
            .set_audio(&meeting(), &audio, at("2026-10-12T10:00:00Z"))
            .await
            .unwrap();
        let service = FinalTranscriptionService::new(FinalTranscriptionDeps {
            meetings,
            segments: segments.clone(),
            writer: segments.clone(),
            speakers: segments.clone(),
            transcriber: Arc::new(ScriptedTracks::default()),
            originals: originals.clone(),
            clock: clock.clone(),
        });
        let cloud = Arc::new(ScriptedCloud::default());
        Self {
            segments,
            originals,
            cloud,
            clock,
            service,
            fractions: Arc::default(),
        }
    }

    /// Progress that keeps every fraction in `fractions`.
    fn progress(&self) -> TrackProgress {
        let fractions = Arc::clone(&self.fractions);
        TrackProgress::new(move |fraction| fractions.lock().unwrap().push(fraction))
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

    async fn run(&self, requested: bool) -> Result<FinalOutcome, FinalTranscriptionError> {
        let pass = FinalPass {
            repass: false,
            requested: false,
            cloud: Some(CloudPass {
                transcriber: self.cloud.clone(),
                requested,
            }),
            cancel: CancellationToken::new(),
            progress: self.progress(),
        };
        self.service.run(&meeting(), &pass).await
    }

    async fn lines(&self) -> Vec<(Channel, Option<String>, String)> {
        let speakers = self.segments.speakers(&meeting()).await.unwrap();
        let label = |id: Option<SpeakerId>| {
            let speaker = speakers.iter().find(|speaker| Some(speaker.id) == id);
            speaker.map(|speaker| speaker.label.clone())
        };
        let segments = self.segments.list(&meeting(), Page::ALL).await.unwrap();
        segments
            .into_iter()
            .map(|s| (s.channel, label(s.speaker_id), s.text))
            .collect()
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
        created_at: at("2026-10-12T09:00:00Z"),
        started_at: None,
        folder_path: None,
    }
}

fn said(start: u32, text: &str, voice: Option<&str>) -> CloudSpan {
    let span = HeardSpan {
        start: Millis(start),
        end: Millis(start + 800),
        text: text.into(),
        words: Vec::new(),
    };
    let voice = voice.map(str::to_owned);
    CloudSpan { span, voice }
}

fn line(channel: Channel, speaker: &str, text: &str) -> (Channel, Option<String>, String) {
    (channel, Some(speaker.to_owned()), text.to_owned())
}

#[tokio::test]
async fn the_cloud_replaces_the_live_transcript_and_keeps_it() {
    let world = World::new().await;
    let (me, them) = (
        world.speaker("Me", true).await,
        world.speaker("Them", false).await,
    );
    world.live(Channel::Mic, (0, 1_000), me).await;
    world.live(Channel::System, (1_000, 3_000), them).await;
    world
        .cloud
        .say(MIC, vec![said(0, "Morning", Some("speaker_0"))]);
    world.cloud.say(
        SYSTEM,
        vec![
            said(1_000, "Hi all", Some("speaker_0")),
            said(2_000, "Hey", Some("speaker_1")),
        ],
    );

    let outcome = world.run(false).await.unwrap();

    assert_eq!(
        outcome,
        FinalOutcome::Written {
            segments: 3,
            replaced: 2
        }
    );
    assert_eq!(
        world.lines().await,
        [
            line(Channel::Mic, "Me", "Morning"),
            line(Channel::System, "Speaker 1", "Hi all"),
            line(Channel::System, "Speaker 2", "Hey"),
        ]
    );
    let sent = world.cloud.sent();
    assert_eq!(sent, [(MIC.into(), false), (SYSTEM.into(), true)]);
    let kept = world.originals.kept(&meeting()).await.unwrap().unwrap();
    assert_eq!(
        kept.iter().map(|l| l.text.as_str()).collect::<Vec<_>>(),
        ["live", "live"]
    );
}

#[tokio::test]
async fn each_upload_moves_the_run_by_its_tracks_share() {
    let world = World::new().await;
    world.cloud.say(MIC, vec![said(0, "Morning", None)]);

    world.run(false).await.unwrap();

    assert_eq!(*world.fractions.lock().unwrap(), [0.5, 1.0]);
}

#[tokio::test]
async fn an_edited_transcript_is_replaced_only_when_the_user_asks() {
    let world = World::new().await;
    let me = world.speaker("Me", true).await;
    world.live(Channel::Mic, (0, 1_000), me).await;
    let segment = world.segments.list(&meeting(), Page::ALL).await.unwrap()[0].id;
    world
        .segments
        .edit_text(&segment, "fixed", at("2026-10-12T10:00:00Z"))
        .await
        .unwrap();
    world.cloud.say(MIC, vec![said(0, "Morning", None)]);

    let automatic = world.run(false).await.unwrap();
    assert_eq!(automatic, FinalOutcome::Skipped(FinalSkip::EditedByUser));
    assert!(world.cloud.sent().is_empty());

    world.run(true).await.unwrap();
    assert_eq!(world.lines().await, [line(Channel::Mic, "Me", "Morning")]);
    let kept = world.originals.kept(&meeting()).await.unwrap().unwrap();
    assert_eq!(kept[0].text, "fixed");
}

#[tokio::test]
async fn a_failure_writes_nothing_and_keeps_nothing() {
    let world = World::new().await;
    let me = world.speaker("Me", true).await;
    world.live(Channel::Mic, (0, 1_000), me).await;
    world.cloud.fail_with(|| CloudError::KeyRefused);

    let error = world.run(false).await.unwrap_err();

    assert!(matches!(
        error,
        FinalTranscriptionError::Cloud(CloudError::KeyRefused)
    ));
    assert_eq!(world.lines().await, [line(Channel::Mic, "Me", "live")]);
    assert_eq!(world.originals.kept(&meeting()).await.unwrap(), None);
    assert!(world.fractions.lock().unwrap().is_empty());
}

#[tokio::test]
async fn silence_writes_nothing() {
    let world = World::new().await;

    let outcome = world.run(false).await.unwrap();

    assert_eq!(outcome, FinalOutcome::Skipped(FinalSkip::NoSpeech));
    assert_eq!(world.cloud.sent().len(), 2);
}

#[tokio::test]
async fn restoring_brings_the_local_transcript_back_with_the_speakers_in_place() {
    let world = World::new().await;
    let me = world.speaker("Me", true).await;
    world.live(Channel::Mic, (0, 1_000), me).await;
    world.cloud.say(MIC, vec![said(0, "Morning", None)]);
    world.run(false).await.unwrap();
    let restore = OriginalTranscriptService::new(OriginalTranscriptDeps {
        originals: world.originals.clone(),
        segments: world.segments.clone(),
        writer: world.segments.clone(),
        speakers: world.segments.clone(),
        clock: world.clock.clone(),
    });

    assert!(restore.has_original(&meeting()).await.unwrap());
    assert!(restore.restore(&meeting()).await.unwrap());

    assert_eq!(world.lines().await, [line(Channel::Mic, "Me", "live")]);
    assert!(!restore.has_original(&meeting()).await.unwrap());
    assert!(!restore.restore(&meeting()).await.unwrap());
}
