//! Wiring for the consent tools' chat notice (CAP-13).
//!
//! [`attach_consent`] starts `deino_consent::ChatNotice`, the thread that
//! posts the user's notice once per recorded meeting in the meeting chat,
//! over this Mac's Accessibility ports, the live recorder and its segments
//! ([`AppRecorder`]) and Settings → Consent ([`SettingsNotice`]). The
//! reminder card is UI only (`features/consent`), over the same settings.

use std::sync::Arc;

use deino_consent::{
    ChatConfig, ChatNotice, Monitor, NoticeConfig, NoticeParts, POLL_INTERVAL, Recorder,
    RecorderNow,
};
use deino_core::segments::{SegmentReader, SqliteSegmentRepo};
use deino_core::settings::{ChatMessageSettings, SettingsService};
use deino_db::Database;
use deino_domain::{CallApp, MeetingId};
use deino_recording::{Playhead, RecordingService};
use tauri::{AppHandle, Manager, Runtime};
use tokio::runtime::Handle;

use super::attach::Phase;

super::attach::attach!(
    "consent",
    40,
    Phase::Background(|cx| {
        attach_consent(cx.app, cx.database);
    })
);

/// Starts the chat notice and manages its handle (dropping it stops the
/// thread). Needs the settings; the recorder is looked up on each poll.
pub(crate) fn attach_consent(app: &AppHandle, database: &Database) {
    let Some(settings) = app
        .try_state::<Arc<SettingsService>>()
        .map(|settings| Arc::clone(&settings))
    else {
        return log::error!(target: "consent", "settings are not loaded; the chat notice is off");
    };
    let runtime = tauri::async_runtime::handle().inner().clone();
    let monitor = Monitor::new(NoticeParts {
        platform: deino_consent::platform(),
        recorder: Arc::new(AppRecorder {
            app: app.clone(),
            segments: SqliteSegmentRepo::new(database.clone()),
            runtime: runtime.clone(),
        }),
        config: Arc::new(SettingsNotice { settings, runtime }),
    });
    match ChatNotice::spawn(monitor, POLL_INTERVAL) {
        Ok(notice) => {
            app.manage(notice);
        }
        Err(error) => log::warn!(target: "consent", "could not start the chat notice: {error}"),
    }
}

/// The recorder's state from its playhead, and speech from the segments
/// saved so far. Blocks the notice's own thread (never a runtime thread).
pub(crate) struct AppRecorder<R: Runtime> {
    app: AppHandle<R>,
    segments: SqliteSegmentRepo,
    runtime: Handle,
}

impl<R: Runtime> Recorder for AppRecorder<R> {
    fn now(&self) -> RecorderNow {
        self.app
            .try_state::<Arc<RecordingService>>()
            .map_or(RecorderNow::Idle, |recorder| {
                recorder_now(recorder.playhead())
            })
    }

    fn heard_speech(&self, meeting: &MeetingId) -> bool {
        self.runtime
            .block_on(self.segments.count(meeting))
            .is_ok_and(|count| count > 0)
    }
}

/// What the notice needs from the recorder's playhead.
fn recorder_now(playhead: Playhead) -> RecorderNow {
    match playhead {
        Playhead::Recording { meeting, .. } => RecorderNow::Recording(meeting),
        Playhead::Paused { meeting } => RecorderNow::Paused(meeting),
        Playhead::Idle => RecorderNow::Idle,
    }
}

/// Settings → Consent, read on each poll.
pub(crate) struct SettingsNotice {
    settings: Arc<SettingsService>,
    runtime: Handle,
}

impl NoticeConfig for SettingsNotice {
    fn chat(&self) -> ChatConfig {
        let settings = self.runtime.block_on(self.settings.get());
        chat_config(&settings.consent.chat_message)
    }
}

/// The notice's settings: the message and the apps switched on.
fn chat_config(chat: &ChatMessageSettings) -> ChatConfig {
    let switches = [
        (CallApp::Zoom, chat.zoom),
        (CallApp::Teams, chat.teams),
        (CallApp::Meet, chat.meet),
    ];
    ChatConfig {
        message: chat.message.clone(),
        apps: switches
            .into_iter()
            .filter_map(|(app, on)| on.then_some(app))
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use deino_domain::Millis;

    use super::*;

    #[test]
    fn the_playhead_says_what_the_recorder_does() {
        let meeting = MeetingId::new("meeting");

        assert_eq!(
            recorder_now(Playhead::Recording {
                meeting: meeting.clone(),
                at: Millis(10)
            }),
            RecorderNow::Recording(meeting.clone())
        );
        assert_eq!(
            recorder_now(Playhead::Paused {
                meeting: meeting.clone()
            }),
            RecorderNow::Paused(meeting)
        );
        assert_eq!(recorder_now(Playhead::Idle), RecorderNow::Idle);
    }

    #[test]
    fn only_the_switched_on_apps_get_the_notice() {
        let off = ChatMessageSettings::default();
        let some = ChatMessageSettings {
            zoom: true,
            meet: true,
            ..ChatMessageSettings::default()
        };

        assert!(!chat_config(&off).is_on());
        assert_eq!(chat_config(&some).apps, [CallApp::Zoom, CallApp::Meet]);
        assert_eq!(chat_config(&some).message, some.message);
    }
}
