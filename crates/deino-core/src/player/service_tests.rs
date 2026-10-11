//! The player service over the fake meetings, real files in a scratch
//! recordings folder and a scripted save dialog.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use deino_domain::{ErrorClass, ErrorCode, MeetingId};
use deino_testkit::at;
use tempfile::TempDir;

use super::*;
use crate::export::ScriptedDialog;
use crate::meetings::{FakeMeetingRepo, Meeting, MeetingAudio, MeetingLifecycleRepo};

const MEETING: &str = "meeting-standup";

struct Harness {
    scratch: TempDir,
    meetings: Arc<FakeMeetingRepo>,
    dialog: Arc<ScriptedDialog>,
    service: PlayerService,
}

impl Harness {
    /// A standup whose row names a mixed file, a microphone track and a
    /// system track; `answer` is where the save dialog saves.
    fn new(answer: Option<&str>) -> Self {
        let scratch = tempfile::tempdir().unwrap();
        let meetings = Arc::new(FakeMeetingRepo::with([Meeting {
            id: MeetingId::new(MEETING),
            title: "Standup: Q4/plan".into(),
            created_at: at("2026-10-08T09:00:00Z"),
            updated_at: at("2026-10-08T09:00:00Z"),
        }]));
        let dialog = Arc::new(ScriptedDialog::answering(
            answer.map(|name| scratch.path().join(name)),
        ));
        let service = PlayerService::new(PlayerDeps {
            lifecycle: meetings.clone(),
            meetings: meetings.clone(),
            files: Arc::new(FsMediaFiles::new(scratch.path().join("recordings"))),
            dialog: dialog.clone(),
        });
        Self {
            scratch,
            meetings,
            dialog,
            service,
        }
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.scratch.path().join(relative)
    }

    /// Writes the files that exist and points the row at all of them.
    async fn record(&self, audio: MeetingAudio, existing: &[&str]) {
        for relative in existing {
            let path = self.path(relative);
            tokio::fs::create_dir_all(path.parent().unwrap())
                .await
                .unwrap();
            tokio::fs::write(&path, relative.as_bytes()).await.unwrap();
        }
        let id = MeetingId::new(MEETING);
        let at = at("2026-10-08T10:00:00Z");
        assert!(self.meetings.set_audio(&id, &audio, at).await.unwrap());
    }

    fn text(&self, relative: &str) -> String {
        self.path(relative).to_string_lossy().into_owned()
    }

    async fn media(&self) -> Vec<MediaSource> {
        let media = self.service.media(&MeetingId::new(MEETING)).await.unwrap();
        media.files.iter().map(|file| file.source).collect()
    }
}

fn audio(harness: &Harness) -> MeetingAudio {
    MeetingAudio {
        audio_path: Some(harness.text("recordings/m/audio.mp4")),
        audio_mic_path: Some(harness.text("recordings/m/mic.m4a")),
        audio_system_path: Some(harness.text("recordings/m/system.m4a")),
        video_path: Some(harness.text("elsewhere/screen.mov")),
    }
}

#[tokio::test]
async fn media_lists_only_files_on_disk_inside_the_recordings_folder() {
    let harness = Harness::new(None);
    let existing = [
        "recordings/m/audio.mp4",
        "recordings/m/mic.m4a",
        "elsewhere/screen.mov",
    ];
    harness.record(audio(&harness), &existing).await;

    assert_eq!(
        harness.media().await,
        [MediaSource::Mixed, MediaSource::Microphone]
    );
}

#[tokio::test]
async fn a_meeting_without_a_recording_or_row_has_no_media() {
    let harness = Harness::new(None);

    let unknown = harness
        .service
        .media(&MeetingId::new("meeting-x"))
        .await
        .unwrap();

    assert!(harness.media().await.is_empty());
    assert!(unknown.files.is_empty());
}

#[tokio::test]
async fn a_download_copies_the_file_under_a_named_suggestion() {
    let harness = Harness::new(Some("saved.m4a"));
    harness
        .record(audio(&harness), &["recordings/m/mic.m4a"])
        .await;

    let outcome = harness
        .service
        .download(&MeetingId::new(MEETING), MediaSource::Microphone)
        .await
        .unwrap();

    assert_eq!(outcome, DownloadOutcome::Saved);
    let saved = tokio::fs::read_to_string(harness.path("saved.m4a"))
        .await
        .unwrap();
    assert_eq!(saved, "recordings/m/mic.m4a");
    let request = &harness.dialog.requests()[0];
    assert!(
        request.file_name.ends_with("(Me).m4a"),
        "{}",
        request.file_name
    );
    assert!(!request.file_name.contains('/'), "{}", request.file_name);
    assert_eq!(request.extension, "m4a");
}

#[tokio::test]
async fn closing_the_dialog_saves_nothing() {
    let harness = Harness::new(None);
    harness
        .record(audio(&harness), &["recordings/m/audio.mp4"])
        .await;

    let outcome = harness
        .service
        .download(&MeetingId::new(MEETING), MediaSource::Mixed)
        .await
        .unwrap();

    assert_eq!(outcome, DownloadOutcome::Cancelled);
}

#[tokio::test]
async fn downloading_a_file_that_is_not_there_is_not_found() {
    let harness = Harness::new(Some("x.mov"));
    harness
        .record(audio(&harness), &["elsewhere/screen.mov"])
        .await;

    let error = harness
        .service
        .download(&MeetingId::new(MEETING), MediaSource::Video)
        .await
        .unwrap_err();

    assert_eq!(error.code(), ErrorCode::NotFound);
    assert!(harness.dialog.requests().is_empty());
    assert!(!Path::new(&harness.path("x.mov")).exists());
}
