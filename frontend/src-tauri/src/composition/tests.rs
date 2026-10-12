//! Tests of the composition root: services over a real database, and the
//! pipeline wired end to end over recorded stage services.

use deino_core::meetings::SqliteMeetingRepo;
use deino_core::segments::SqliteSegmentRepo;
use deino_testkit::{FakeClock, at, seed, test_db};

use super::*;

#[tokio::test]
async fn services_read_through_the_database_they_are_built_over() {
    let database = test_db().await;
    seed::meeting(
        &database,
        "meeting-1",
        "Standup",
        at("2026-10-08T09:00:00Z"),
    )
    .await;
    let clock: Arc<dyn Clock> = Arc::new(FakeClock::new(at("2026-10-08T10:00:00Z")));

    let services = Services::build(&database, &clock);

    let meetings = services.meetings.list().await.unwrap();
    assert_eq!(meetings.len(), 1);
}

/// Records `meeting-1` with one transcript segment, still recording.
async fn recording_with_transcript(database: &Database, clock: &FakeClock) {
    use deino_core::meetings::{MeetingLifecycleRepo, MeetingTitle, NewMeeting};
    use deino_core::segments::{NewSegment, SegmentWriter};
    use deino_domain::{Channel, MeetingId, MeetingSource, MeetingStatus, Millis, SegmentId};

    let meeting = NewMeeting {
        id: MeetingId::new("meeting-1"),
        title: MeetingTitle::parse("Standup").unwrap(),
        status: MeetingStatus::Recording,
        source: MeetingSource::Live,
        created_at: at("2026-10-08T09:00:00Z"),
        started_at: None,
        folder_path: None,
    };
    SqliteMeetingRepo::new(database.clone())
        .create(&meeting)
        .await
        .unwrap();
    let segment = NewSegment {
        id: SegmentId::generate(clock),
        meeting_id: meeting.id,
        channel: Channel::Mixed,
        speaker_id: None,
        start: Millis(2_000),
        end: Millis(3_000),
        text: "let's ship it".into(),
        words: Vec::new(),
    };
    SqliteSegmentRepo::new(database.clone())
        .append(&[segment])
        .await
        .unwrap();
}

/// The recorded stage services the test reads back.
struct Recorded {
    enhancer: Arc<deino_pipeline::RecordedEnhancer>,
    titler: Arc<deino_pipeline::RecordedTitler>,
    auto_tags: Arc<deino_pipeline::RecordedAutoTags>,
}

/// Pipeline adapters over fakes, recording what the AI stages were asked.
fn recorded_adapters() -> (PipelineAdapters, Recorded) {
    let recorded = Recorded {
        enhancer: Arc::default(),
        titler: Arc::default(),
        auto_tags: Arc::default(),
    };
    let adapters = PipelineAdapters {
        progress: Arc::new(deino_pipeline::RecordedProgress::default()),
        diarizer: Arc::new(deino_core::diarization::FakeTrackDiarizer::default()),
        ai: StageServices {
            enhancer: recorded.enhancer.clone(),
            titler: recorded.titler.clone(),
            automatic: Arc::new(deino_pipeline::FixedAutomaticNotes(true)),
            adapters: StageAdapters {
                auto_tagger: recorded.auto_tags.clone(),
                auto_export: Arc::new(deino_pipeline::ScriptedAutoExporter::default()),
                action_items: Arc::new(deino_core::action_items::FakeActionItemExtractor::default()),
                insights: Arc::new(deino_pipeline::stages::RecordedInsights::default()),
                automations: Arc::new(deino_pipeline::RecordedAutomations::default()),
                follow_ups: Arc::new(deino_pipeline::stages::RecordedFollowUps::default()),
                identifier: Arc::new(deino_pipeline::RecordedIdentifications::default()),
                search: Arc::new(deino_pipeline::RecordedIndexing::default()),
                entities: Arc::new(deino_pipeline::RecordedEntityUpdates::default()),
            },
        },
        transcript: pipeline::TranscriptStages {
            transcriber: Arc::new(deino_core::final_transcription::ScriptedTracks::default()),
            repass: Arc::new(deino_pipeline::FixedRepass(false)),
            cloud: Arc::new(deino_core::final_transcription::LocalOnly),
            vocab_fixer: Arc::new(deino_pipeline::RecordedFixer::default()),
        },
        recordings: std::path::PathBuf::from("no-recordings"),
        screen_context: Arc::new(deino_pipeline::RecordedScreenContext::default()),
    };
    (adapters, recorded)
}

#[tokio::test]
async fn stopping_a_recording_runs_the_notes_stages_and_readies_the_meeting() {
    use deino_core::meetings::MeetingLifecycleRepo;
    use deino_domain::{MeetingEvent, MeetingId, MeetingStatus};

    let database = test_db().await;
    let fake_clock = Arc::new(FakeClock::new(at("2026-10-08T10:00:00Z")));
    recording_with_transcript(&database, &fake_clock).await;
    let (
        adapters,
        Recorded {
            enhancer,
            titler,
            auto_tags,
        },
    ) = recorded_adapters();
    let clock: Arc<dyn Clock> = fake_clock;
    let wired = Pipeline::build(&database, &clock, adapters);
    wired.pipeline.reconcile().await.unwrap();
    let meeting = MeetingId::new("meeting-1");

    wired
        .lifecycle
        .transition(&meeting, MeetingEvent::StopRecording)
        .await
        .unwrap();
    wired.pipeline.until_idle().await;

    let once = std::slice::from_ref(&meeting);
    assert_eq!(enhancer.meetings(), once);
    assert_eq!(titler.meetings(), once);
    assert_eq!(auto_tags.meetings(), once);
    let state = SqliteMeetingRepo::new(database).state(&meeting).await;
    assert_eq!(state.unwrap().unwrap().status, MeetingStatus::Ready);
}
