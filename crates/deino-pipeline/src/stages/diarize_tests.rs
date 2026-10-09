use std::sync::Arc;

use deino_core::diarization::{FakeTrackDiarizer, TrackDiarization, Voice, VoiceTurn};
use deino_core::meetings::{
    FakeMeetingRepo, MeetingAudio, MeetingLifecycleRepo, MeetingTitle, NewMeeting,
};
use deino_core::segments::{FakeSegmentRepo, NewSegment, SegmentWriter, WordTiming};
use deino_domain::{
    Channel, ErrorCode, FakeClock, MeetingSource, MeetingStatus, Millis, SegmentId,
};
use deino_testkit::at;
use tokio_util::sync::CancellationToken;

use super::*;
use crate::RecordedProgress;

const AT: &str = "2026-10-08T09:00:00Z";

fn meeting() -> MeetingId {
    MeetingId::new("meeting-a")
}

fn ctx() -> StageCtx {
    StageCtx {
        attempt: 1,
        cancel: CancellationToken::new(),
        clock: Arc::new(FakeClock::new(at(AT))),
        events: Arc::new(RecordedProgress::default()),
    }
}

/// A processing meeting with only a system track recorded.
async fn meetings(system: Option<&str>) -> Arc<FakeMeetingRepo> {
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
        audio_system_path: system.map(str::to_owned),
        ..MeetingAudio::default()
    };
    meetings
        .set_audio(&meeting(), &audio, at(AT))
        .await
        .unwrap();
    meetings
}

/// Four seconds of remote speech, one word every 500 ms.
fn remote_speech() -> NewSegment {
    let words: Vec<WordTiming> = (0..8_u32)
        .map(|i| WordTiming {
            text: "word".into(),
            start: Millis(i * 500),
            end: Millis(i * 500 + 450),
        })
        .collect();
    NewSegment {
        id: SegmentId::generate(&FakeClock::new(at(AT))),
        meeting_id: meeting(),
        channel: Channel::System,
        speaker_id: None,
        start: Millis(0),
        end: Millis(3_950),
        text: "word ".repeat(8).trim_end().to_owned(),
        words,
    }
}

fn one_voice() -> TrackDiarization {
    TrackDiarization {
        turns: vec![VoiceTurn {
            start: Millis(0),
            end: Millis(4_000),
            voice: 0,
        }],
        voices: vec![Voice {
            centroid: vec![1.0, 0.0],
            speech: Millis(4_000),
        }],
    }
}

async fn stage(
    system: Option<&str>,
    segments: &[NewSegment],
    engine: FakeTrackDiarizer,
) -> DiarizeStage {
    let store = Arc::new(FakeSegmentRepo::default());
    store.append(segments).await.unwrap();
    DiarizeStage::new(&StagePorts {
        segments: store.clone(),
        segment_writer: store.clone(),
        speakers: store,
        meetings: meetings(system).await,
        diarizer: Arc::new(engine),
        ..StagePorts::fakes()
    })
}

#[tokio::test]
async fn a_recorded_call_is_diarized() {
    let stage = stage(
        Some("/rec/system.m4a"),
        &[remote_speech()],
        FakeTrackDiarizer::returning(one_voice()),
    )
    .await;
    assert_eq!(stage.id(), StageId::new("diarize"));
    assert_eq!(stage.order(), 30);
    assert_eq!(stage.run(&ctx(), meeting()).await, Ok(StageOutcome::Done));
}

#[tokio::test]
async fn a_meeting_without_its_track_or_transcript_is_skipped() {
    let no_track = stage(
        None,
        &[remote_speech()],
        FakeTrackDiarizer::returning(one_voice()),
    )
    .await;
    assert_eq!(
        no_track.run(&ctx(), meeting()).await,
        Ok(StageOutcome::Skipped(NO_RECORDING))
    );
    let no_transcript = stage(Some("/rec/system.m4a"), &[], FakeTrackDiarizer::default()).await;
    assert_eq!(
        no_transcript.run(&ctx(), meeting()).await,
        Ok(StageOutcome::Skipped(NO_TRANSCRIPT))
    );
}

#[tokio::test]
async fn missing_models_are_retried_and_unknown_meetings_are_not() {
    let stage = stage(
        Some("/rec/system.m4a"),
        &[remote_speech()],
        FakeTrackDiarizer::default(),
    )
    .await;

    let missing = stage.run(&ctx(), meeting()).await.unwrap_err();
    assert!(missing.is_retryable());
    let StageError::Retryable(error) = missing else {
        unreachable!()
    };
    assert_eq!(error.code, ErrorCode::Unavailable);

    let unknown = stage
        .run(&ctx(), MeetingId::new("meeting-x"))
        .await
        .unwrap_err();
    assert!(!unknown.is_retryable());
}

#[tokio::test]
async fn missing_models_on_the_last_attempt_skip_so_notes_still_run() {
    let stage = stage(
        Some("/rec/system.m4a"),
        &[remote_speech()],
        FakeTrackDiarizer::default(),
    )
    .await;
    let last = StageCtx {
        attempt: RetryPolicy::DEFAULT.max_attempts,
        ..ctx()
    };
    assert_eq!(
        stage.run(&last, meeting()).await,
        Ok(StageOutcome::Skipped(NO_MODELS))
    );
}

#[tokio::test]
async fn a_cancelled_run_stops() {
    let stage = stage(
        Some("/rec/system.m4a"),
        &[remote_speech()],
        FakeTrackDiarizer::returning(one_voice()),
    )
    .await;
    let ctx = ctx();
    ctx.cancel.cancel();
    assert_eq!(stage.run(&ctx, meeting()).await, Err(StageError::Cancelled));
}
