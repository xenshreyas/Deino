use deino_testkit::Fixtures;
use deino_testkit::fixture::{Channel, MeetingKind};

use super::*;

/// The standup fixture's system track (AAC in M4A, about 53 s).
fn fixture_m4a() -> PathBuf {
    let fixtures = Fixtures::load().unwrap();
    let meeting = fixtures.meeting(MeetingKind::Standup).unwrap();
    let audio = meeting.audio.as_ref().unwrap();
    fixtures.audio_path(audio, Channel::System).unwrap()
}

#[tokio::test]
async fn facts_report_size_and_a_missing_file_as_missing() {
    let ingest = NativeIngest::new();

    let facts = ingest.facts(&fixture_m4a()).await.unwrap();
    let missing = ingest
        .facts(Path::new("/nonexistent/call.mp4"))
        .await
        .unwrap_err();
    let folder = ingest.facts(Path::new("/")).await.unwrap_err();

    assert!(facts.size_bytes > 100_000, "{facts:?}");
    assert!(facts.modified.is_some());
    assert!(matches!(missing, ImportError::Missing), "{missing:?}");
    assert!(matches!(folder, ImportError::UnsupportedFile), "{folder:?}");
}

#[tokio::test]
async fn a_file_that_is_not_media_leaves_no_folder_behind() {
    let dir = tempfile::tempdir().unwrap();
    let fake = dir.path().join("notes.mp3");
    std::fs::write(&fake, "plain text").unwrap();
    let folder = dir.path().join("meeting-a");

    let error = NativeIngest::new()
        .ingest(&fake, MediaKind::Audio, &folder)
        .await
        .unwrap_err();

    assert!(
        matches!(
            error,
            ImportError::UnsupportedFormat | ImportError::Damaged(_)
        ),
        "{error:?}"
    );
    assert!(!folder.exists());
}

#[cfg(target_os = "macos")]
mod native {
    use super::*;

    #[tokio::test]
    async fn an_audio_file_becomes_the_meetings_m4a() {
        let dir = tempfile::tempdir().unwrap();
        let folder = dir.path().join("meeting-a");

        let media = NativeIngest::new()
            .ingest(&fixture_m4a(), MediaKind::Audio, &folder)
            .await
            .unwrap();

        assert_eq!(media.audio, folder.join(AUDIO_FILE));
        assert_eq!(media.video, None);
        assert!((50..60).contains(&media.duration.as_secs()), "{media:?}");
        let reread = deino_audio::read_mono(&media.audio, 16_000).unwrap();
        assert!((16_000 * 50..16_000 * 60).contains(&reread.len()));
    }

    #[tokio::test]
    async fn a_video_is_kept_beside_its_audio() {
        let dir = tempfile::tempdir().unwrap();
        let video = dir.path().join("Screen Recording.MP4");
        std::fs::copy(fixture_m4a(), &video).unwrap();
        let folder = dir.path().join("meeting-b");

        let media = NativeIngest::new()
            .ingest(&video, MediaKind::Video, &folder)
            .await
            .unwrap();

        assert_eq!(media.video, Some(folder.join("video.mp4")));
        assert!(folder.join("video.mp4").exists());
        assert!(video.exists(), "the source stays where it was");
    }

    #[tokio::test]
    async fn discarding_removes_the_folder() {
        let dir = tempfile::tempdir().unwrap();
        let folder = dir.path().join("meeting-c");
        std::fs::create_dir(&folder).unwrap();
        std::fs::write(folder.join(AUDIO_FILE), b"x").unwrap();

        NativeIngest::new().discard(&folder).await;

        assert!(!folder.exists());
    }
}
