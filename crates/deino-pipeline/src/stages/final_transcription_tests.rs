use deino_core::final_transcription::{HeardSpan, ScriptedTracks};
use deino_core::meetings::{
    FakeMeetingRepo, MeetingAudio, MeetingLifecycleRepo, MeetingTitle, NewMeeting,
};
use deino_core::segments::{FakeSegmentRepo, Page, SegmentReader};
use deino_domain::{ErrorCode, FakeClock, MeetingSource, MeetingStatus, Millis};
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
