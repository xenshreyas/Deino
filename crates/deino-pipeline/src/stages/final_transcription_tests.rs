use std::sync::atomic::{AtomicUsize, Ordering};

use deino_core::final_transcription::{
    CloudPass, CloudSpan, HeardSpan, ScriptedCloud, ScriptedTracks,
};
use deino_core::meetings::{
    FakeMeetingRepo, MeetingAudio, MeetingLifecycleRepo, MeetingTitle, NewMeeting,
};
use deino_core::segments::{FakeSegmentRepo, Page, SegmentReader};
use deino_domain::{ErrorCode, FakeClock, JobStatus, MeetingSource, MeetingStatus, Millis};
use deino_testkit::at;
use tokio_util::sync::CancellationToken;

use super::*;
use crate::RecordedProgress;

const AT: &str = "2026-10-11T09:00:00Z";
const MIC: &str = "/recordings/meeting-a/mic.m4a";

fn meeting() -> MeetingId {
    MeetingId::new("meeting-a")
}

fn ctx(attempt: u32) -> StageCtx {
    StageCtx {
        attempt,
        cancel: CancellationToken::new(),
        clock: Arc::new(FakeClock::new(at(AT))),
        events: Arc::new(RecordedProgress::default()),
    }
}

/// A processing meeting whose microphone track is `mic`.
async fn meetings(mic: Option<&str>) -> Arc<FakeMeetingRepo> {
    let meetings = Arc::new(FakeMeetingRepo::default());
    let new = NewMeeting {
        id: meeting(),
        title: MeetingTitle::parse("Standup").unwrap(),
        status: MeetingStatus::Processing,
        source: MeetingSource::Live,
        created_at: at(AT),
        started_at: None,
        folder_path: None,
    };
    meetings.create(&new).await.unwrap();
    let audio = MeetingAudio {
        audio_mic_path: mic.map(str::to_owned),
        ..MeetingAudio::default()
    };
    meetings
        .set_audio(&meeting(), &audio, at(AT))
        .await
        .unwrap();
    meetings
}

fn said(text: &str) -> HeardSpan {
    HeardSpan {
        start: Millis(0),
        end: Millis(800),
        text: text.into(),
        words: Vec::new(),
    }
}

struct World {
    segments: Arc<FakeSegmentRepo>,
    tracks: Arc<ScriptedTracks>,
    stage: FinalTranscriptionStage,
}

async fn world(mic: Option<&str>, repass: bool) -> World {
    let segments = Arc::new(FakeSegmentRepo::default());
    let tracks = Arc::new(ScriptedTracks::default());
    let ports = StagePorts {
        meetings: meetings(mic).await,
        segments: segments.clone(),
        segment_writer: segments.clone(),
        speakers: segments.clone(),
        transcriber: tracks.clone(),
        repass: Arc::new(crate::stages::FixedRepass(repass)),
        ..StagePorts::fakes()
    };
    World {
        segments,
        tracks,
        stage: FinalTranscriptionStage::new(&ports),
    }
}

#[tokio::test]
async fn a_record_only_meeting_is_transcribed_from_its_tracks() {
    let world = world(Some(MIC), false).await;
    world.tracks.say(MIC, vec![said("Hello Sreyas")]);

    let outcome = world.stage.run(&ctx(1), meeting()).await.unwrap();

    assert_eq!(outcome, StageOutcome::Done);
    let transcript = world.segments.list(&meeting(), Page::ALL).await.unwrap();
    assert_eq!(transcript[0].text, "Hello Sreyas");
    assert_eq!(world.stage.order(), 20);
}

#[tokio::test]
async fn nothing_to_transcribe_is_a_skip_with_its_reason() {
    let unrecorded = world(None, false).await;
    let silent = world(Some(MIC), false).await;

    assert_eq!(
        unrecorded.stage.run(&ctx(1), meeting()).await.unwrap(),
        StageOutcome::Skipped(NO_AUDIO)
    );
    assert_eq!(
        silent.stage.run(&ctx(1), meeting()).await.unwrap(),
        StageOutcome::Skipped(NO_SPEECH)
    );
}

#[tokio::test]
async fn a_missing_model_waits_for_the_user_unless_it_was_only_a_re_pass() {
    let record_only = world(Some(MIC), false).await;
    record_only.tracks.without_model();

    let error = record_only.stage.run(&ctx(1), meeting()).await.unwrap_err();

    assert!(!error.is_retryable());
    assert!(matches!(&error, StageError::Permanent(app) if app.code == ErrorCode::Unavailable));
    let failed = FinalTranscriptionError::Repass(TrackError::Unavailable("no model".into()));
    assert_eq!(
        outcome_of(&failed, 1),
        Ok(StageOutcome::Skipped(REPASS_FAILED))
    );
}

#[test]
fn a_failing_re_pass_is_retried_then_skipped_so_notes_still_come() {
    let crashed = || FinalTranscriptionError::Repass(TrackError::Failed("worker crashed".into()));
    let cancelled = FinalTranscriptionError::Track(TrackError::Cancelled);

    assert!(outcome_of(&crashed(), 1).unwrap_err().is_retryable());
    assert_eq!(
        outcome_of(&crashed(), 3),
        Ok(StageOutcome::Skipped(REPASS_FAILED))
    );
    assert_eq!(outcome_of(&cancelled, 1), Err(StageError::Cancelled));
}

/// A cloud choice that always picks its scripted cloud, counting finished runs.
#[derive(Default)]
struct FixedCloud {
    cloud: Arc<ScriptedCloud>,
    finished: AtomicUsize,
}

#[async_trait]
impl CloudChoice for FixedCloud {
    async fn for_meeting(&self, _meeting: &MeetingId) -> Option<CloudPass> {
        Some(CloudPass {
            transcriber: self.cloud.clone(),
            requested: false,
        })
    }

    fn finished(&self, _meeting: &MeetingId) {
        self.finished.fetch_add(1, Ordering::Relaxed);
    }
}

struct CloudWorld {
    world: World,
    choice: Arc<FixedCloud>,
}

async fn cloud_world() -> CloudWorld {
    let choice = Arc::new(FixedCloud::default());
    let segments = Arc::new(FakeSegmentRepo::default());
    let tracks = Arc::new(ScriptedTracks::default());
    let ports = StagePorts {
        meetings: meetings(Some(MIC)).await,
        segments: segments.clone(),
        segment_writer: segments.clone(),
        speakers: segments.clone(),
        transcriber: tracks.clone(),
        cloud: choice.clone(),
        ..StagePorts::fakes()
    };
    let stage = FinalTranscriptionStage::new(&ports);
    let world = World {
        segments,
        tracks,
        stage,
    };
    CloudWorld { world, choice }
}

impl CloudWorld {
    async fn text(&self) -> Vec<String> {
        let transcript = self.world.segments.list(&meeting(), Page::ALL).await;
        transcript.unwrap().into_iter().map(|s| s.text).collect()
    }

    fn finished(&self) -> usize {
        self.choice.finished.load(Ordering::Relaxed)
    }
}

fn cloud_said(text: &str) -> CloudSpan {
    CloudSpan {
        span: said(text),
        voice: None,
    }
}

#[tokio::test]
async fn the_cloud_transcribes_when_it_is_chosen() {
    let cloud = cloud_world().await;
    cloud
        .choice
        .cloud
        .say(MIC, vec![cloud_said("From the cloud")]);
    cloud.world.tracks.say(MIC, vec![said("From the Mac")]);

    let outcome = cloud.world.stage.run(&ctx(1), meeting()).await.unwrap();

    assert_eq!(outcome, StageOutcome::Done);
    assert_eq!(cloud.text().await, ["From the cloud"]);
    assert!(cloud.world.tracks.heard().is_empty());
    assert_eq!(cloud.finished(), 1);
}

#[tokio::test]
async fn a_refused_key_falls_back_to_the_mac_and_says_why() {
    let cloud = cloud_world().await;
    cloud.choice.cloud.fail_with(|| CloudError::KeyRefused);
    cloud.world.tracks.say(MIC, vec![said("From the Mac")]);

    let outcome = cloud.world.stage.run(&ctx(1), meeting()).await.unwrap();

    let status = "ElevenLabs transcription failed: the API key was refused.";
    assert_eq!(outcome, StageOutcome::Skipped(status));
    assert_eq!(cloud.text().await, ["From the Mac"]);
    assert_eq!(cloud.finished(), 1);
}

#[tokio::test]
async fn a_busy_service_is_retried_then_the_mac_takes_over() {
    let cloud = cloud_world().await;
    cloud.choice.cloud.fail_with(|| CloudError::RateLimited);
    cloud.world.tracks.say(MIC, vec![said("From the Mac")]);

    let first = cloud.world.stage.run(&ctx(1), meeting()).await;
    assert!(first.unwrap_err().is_retryable());
    assert_eq!(cloud.finished(), 0);
    assert!(cloud.text().await.is_empty());

    let last = cloud.world.stage.run(&ctx(3), meeting()).await.unwrap();
    let status = CloudError::RateLimited.status();
    assert_eq!(last, StageOutcome::Skipped(status));
    assert_eq!(cloud.text().await, ["From the Mac"]);
}

#[tokio::test]
async fn a_cancelled_stage_stops() {
    let world = world(Some(MIC), false).await;
    let cancelled = ctx(1);
    cancelled.cancel.cancel();

    assert_eq!(
        world.stage.run(&cancelled, meeting()).await,
        Err(StageError::Cancelled)
    );
}

#[tokio::test]
async fn the_run_reports_its_fraction_once_per_whole_percent() {
    let world = world(Some(MIC), false).await;
    world.tracks.say(MIC, vec![said("Hello Sreyas")]);
    let events = Arc::new(RecordedProgress::default());
    let ctx = StageCtx {
        events: events.clone(),
        ..ctx(1)
    };

    world.stage.run(&ctx, meeting()).await.unwrap();

    // The track says 50% then 100%; the service's own "done" repeats 100%.
    let reported: Vec<(JobStatus, Option<u8>)> = events
        .events()
        .iter()
        .map(|event| (event.status, event.percent()))
        .collect();
    assert_eq!(
        reported,
        [
            (JobStatus::Running, Some(50)),
            (JobStatus::Running, Some(100))
        ]
    );
    assert!(
        events
            .events()
            .iter()
            .all(|event| event.stage.as_str() == ID)
    );
}

#[tokio::test]
async fn a_skipped_run_reports_no_fraction() {
    let world = world(None, false).await;
    let events = Arc::new(RecordedProgress::default());
    let ctx = StageCtx {
        events: events.clone(),
        ..ctx(1)
    };

    world.stage.run(&ctx, meeting()).await.unwrap();

    assert!(events.events().is_empty());
}

/// The re-pass setting, fixed.
struct FixedRepass(bool);

#[async_trait::async_trait]
impl RepassSetting for FixedRepass {
    async fn enabled(&self) -> bool {
        self.0
    }
}

#[tokio::test]
async fn a_re_transcribe_request_is_taken_once_and_the_setting_still_reads() {
    let requests = RequestedRepass::new(Arc::new(FixedRepass(false)));
    let meeting = MeetingId::new("meeting-a");

    requests.request(&meeting);

    assert!(requests.take_request(&meeting));
    assert!(!requests.take_request(&meeting), "a request is used once");
    assert!(!requests.take_request(&MeetingId::new("meeting-b")));
    assert!(!requests.enabled().await);
}
