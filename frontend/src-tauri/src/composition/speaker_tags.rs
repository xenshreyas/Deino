//! Wiring for speaker tags (CAP-05).
//!
//! - [`attach_speaker_tags`] manages `Arc<SpeakerTags>` over this Mac's
//!   Accessibility ports, the recorder's playhead ([`RecorderClock`]) and the
//!   `speaker_tag_spans` table ([`DatabaseSink`]). The recorder starts a
//!   watch for every recording through `features::speaker_tags::WatchOnStart`.
//! - [`tag_priors`] gives `identify_speakers` the spans as priors.
//! - `capture`: the opt-in dumps of call windows the watch cannot read
//!   ([`LogFolderCapture`]), written to the log folder.

mod capture;

use std::sync::Arc;

use deino_core::people::SqlitePeopleRepo;
use deino_core::segments::SqliteSegmentRepo;
use deino_core::settings::SettingsService;
use deino_core::speaker_tags::{
    SpeakerTagPriors, SpeakerTagWriter, SqliteSpeakerTagRepo, TagPriorPorts, TagPriors,
};
use deino_db::Database;
use deino_domain::{Clock, MeetingId, TagSpan};
use deino_recording::{Playhead, RecordingService};
use deino_speaker_tags::{
    NoCapture, Position, RecordingClock, SinkError, SpeakerTags, SpeakerTagsParts, TagSink,
    WindowCapture,
};
use tauri::{AppHandle, Manager, Runtime};
use tokio::runtime::Handle;

use self::capture::LogFolderCapture;
use super::attach::Phase;

super::attach::attach!(
    "speaker-tags",
    10,
    Phase::Background(|cx| {
        attach_speaker_tags(cx.app, cx.database, cx.clock);
    })
);

/// Builds and manages `Arc<SpeakerTags>`.
pub(crate) fn attach_speaker_tags(app: &AppHandle, database: &Database, clock: &Arc<dyn Clock>) {
    let runtime = tauri::async_runtime::handle().inner().clone();
    let tags = SpeakerTags::new(SpeakerTagsParts {
        platform: deino_speaker_tags::platform(),
        clock: Arc::new(RecorderClock { app: app.clone() }),
        sink: Arc::new(DatabaseSink {
            repo: SqliteSpeakerTagRepo::new(database.clone()),
            runtime: runtime.clone(),
        }),
        capture: window_capture(app, runtime, clock),
    });
    app.manage(Arc::new(tags));
}

/// The call-window dumps over the settings and the log folder; none
/// without settings.
fn window_capture(
    app: &AppHandle,
    runtime: Handle,
    clock: &Arc<dyn Clock>,
) -> Arc<dyn WindowCapture> {
    let Some(settings) = app.try_state::<Arc<SettingsService>>() else {
        log::warn!(target: "speaker_tags", "settings are not loaded; call windows are never saved");
        return Arc::new(NoCapture);
    };
    Arc::new(LogFolderCapture {
        settings: Arc::clone(&settings),
        runtime,
        clock: Arc::clone(clock),
        folder: crate::logging::log_dir(),
    })
}

/// The speaker tags of a meeting as priors for `identify_speakers`.
pub(crate) fn tag_priors(database: &Database, clock: Arc<dyn Clock>) -> Arc<dyn TagPriors> {
    let people = Arc::new(SqlitePeopleRepo::new(database.clone()));
    Arc::new(SpeakerTagPriors::new(TagPriorPorts {
        tags: Arc::new(SqliteSpeakerTagRepo::new(database.clone())),
        segments: Arc::new(SqliteSegmentRepo::new(database.clone())),
        attendees: people.clone(),
        me: people.clone(),
        people,
        clock,
    }))
}

/// The recording clock over the live recorder's playhead, looked up on each
/// poll (the recorder is managed after speaker tags).
pub(crate) struct RecorderClock<R: Runtime> {
    app: AppHandle<R>,
}

impl<R: Runtime> RecordingClock for RecorderClock<R> {
    fn position(&self, meeting: &MeetingId) -> Position {
        let Some(recorder) = self.app.try_state::<Arc<RecordingService>>() else {
            return Position::Over;
        };
        position(&recorder.playhead(), meeting)
    }
}

/// Where `meeting` is, from the recorder's playhead: any other meeting
/// recording, or none, means its recording is over.
fn position(playhead: &Playhead, meeting: &MeetingId) -> Position {
    match playhead {
        Playhead::Recording { meeting: now, at } if now == meeting => Position::At(*at),
        Playhead::Paused { meeting: now } if now == meeting => Position::Paused,
        Playhead::Recording { .. } | Playhead::Paused { .. } | Playhead::Idle => Position::Over,
    }
}

/// Stores spans in `speaker_tag_spans`, blocking the watcher's thread (never
/// a runtime thread) on the write.
pub(crate) struct DatabaseSink {
    repo: SqliteSpeakerTagRepo,
    runtime: Handle,
}

impl TagSink for DatabaseSink {
    fn save(&self, meeting: &MeetingId, spans: &[TagSpan]) -> Result<(), SinkError> {
        self.runtime
            .block_on(self.repo.append(meeting, spans))
            .map_err(SinkError::new)
    }

    fn save_roster(&self, meeting: &MeetingId, spans: &[TagSpan]) -> Result<(), SinkError> {
        self.runtime
            .block_on(self.repo.append_roster(meeting, spans))
            .map_err(SinkError::new)
    }
}

#[cfg(test)]
mod tests {
    use deino_core::speaker_tags::SpeakerTagReader;
    use deino_domain::{CallApp, DisplayName, Millis};
    use deino_testkit::{at, seed, test_db};

    use super::*;

    #[test]
    fn only_the_watched_meetings_recording_is_its_clock() {
        let (mine, other) = (MeetingId::new("mine"), MeetingId::new("other"));
        let recording = |meeting: &MeetingId| Playhead::Recording {
            meeting: meeting.clone(),
            at: Millis(1_500),
        };

        assert_eq!(
            position(&recording(&mine), &mine),
            Position::At(Millis(1_500))
        );
        assert_eq!(
            position(
                &Playhead::Paused {
                    meeting: mine.clone()
                },
                &mine
            ),
            Position::Paused
        );
        assert_eq!(position(&recording(&other), &mine), Position::Over);
        assert_eq!(position(&Playhead::Idle, &mine), Position::Over);
    }

    #[test]
    fn the_sink_writes_spans_from_a_plain_thread() {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let database = runtime.block_on(async {
            let database = test_db().await;
            seed::meeting(&database, "meeting", "Weekly", at("2026-10-11T09:00:00Z")).await;
            database
        });
        let sink = DatabaseSink {
            repo: SqliteSpeakerTagRepo::new(database.clone()),
            runtime: runtime.handle().clone(),
        };
        let span = TagSpan {
            start: Millis(0),
            end: Millis(900),
            name: DisplayName::parse("Alex Chen").unwrap(),
            app: CallApp::Zoom,
        };
        let meeting = MeetingId::new("meeting");

        std::thread::scope(|scope| {
            scope.spawn(|| {
                sink.save(&meeting, std::slice::from_ref(&span)).unwrap();
                sink.save_roster(&meeting, std::slice::from_ref(&span))
                    .unwrap();
            });
        });

        let repo = SqliteSpeakerTagRepo::new(database);
        let stored = runtime.block_on(repo.spans(&meeting)).unwrap();
        let roster = runtime.block_on(repo.roster(&meeting)).unwrap();
        assert_eq!(stored, std::slice::from_ref(&span));
        assert_eq!(roster, [span]);
    }
}
