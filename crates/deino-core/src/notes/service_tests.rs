//! The notes rules over in-memory ports.

use std::sync::Arc;

use chrono::TimeDelta;
use deino_doc::{Block, Doc, ImageAttrs, Inline, Mark};
use deino_domain::{FakeClock, MeetingId, MeetingSource, MeetingStatus};
use deino_testkit::at;

use super::*;
use crate::meetings::{FakeMeetingRepo, MeetingLifecycleRepo, MeetingTitle, NewMeeting};

const NOW: &str = "2026-10-08T09:00:00Z";
const PNG: &[u8] = b"\x89PNG\r\n\x1a\n-the-rest";

struct Harness {
    service: NoteService,
    notes: Arc<FakeNotesRepo>,
    meetings: Arc<FakeMeetingRepo>,
    images: Arc<MemoryNoteImages>,
    events: Arc<RecordedNotesEvents>,
    clock: Arc<FakeClock>,
}

async fn harness() -> Harness {
    let notes = Arc::new(FakeNotesRepo::default());
    let meetings = Arc::new(FakeMeetingRepo::default());
    for (id, status) in [
        ("meeting-a", MeetingStatus::Ready),
        ("meeting-draft", MeetingStatus::Scheduled),
    ] {
        meetings.create(&new_meeting(id, status)).await.unwrap();
    }
    let images = Arc::new(MemoryNoteImages::default());
    let events = Arc::new(RecordedNotesEvents::default());
    let clock = Arc::new(FakeClock::new(at(NOW)));
    let ports = NotePorts {
        notes: notes.clone(),
        meetings: meetings.clone(),
        images: images.clone(),
        events: events.clone(),
    };
    let service = NoteService::new(ports, clock.clone());
    Harness {
        service,
        notes,
        meetings,
        images,
        events,
        clock,
    }
}

fn new_meeting(id: &str, status: MeetingStatus) -> NewMeeting {
    NewMeeting {
        id: MeetingId::new(id),
        title: MeetingTitle::parse("Standup").unwrap(),
        status,
        source: MeetingSource::Live,
        created_at: at(NOW),
        started_at: None,
        folder_path: None,
    }
}

fn doc(text: &str) -> Doc {
    Doc {
        content: vec![Block::paragraph(vec![Inline::text(text, vec![])])],
    }
}

fn image(src: &str) -> Doc {
    Doc {
        content: vec![Block::Image {
            attrs: ImageAttrs {
                src: src.into(),
                alt: None,
                title: None,
            },
        }],
    }
}

fn id(text: &str) -> MeetingId {
    MeetingId::new(text)
}

#[tokio::test]
async fn a_meeting_without_notes_opens_an_empty_document() {
    let h = harness().await;

    let opened = h.service.get(&id("meeting-a")).await.unwrap();

    assert_eq!(opened.doc, Doc::empty());
    assert_eq!(opened.updated_at, None);
    assert_eq!(opened.folder, "/meetings/meeting-a");
}

#[tokio::test]
async fn saving_stores_the_normalized_doc_and_its_readable_markdown() {
    let h = harness().await;
    let messy = Doc {
        content: vec![Block::paragraph(vec![
            Inline::text("a", vec![Mark::Italic, Mark::Bold]),
            Inline::text("b", vec![Mark::Bold, Mark::Italic]),
        ])],
    };

    let saved = h.service.save(&id("meeting-a"), messy).await.unwrap();

    let stored = h.notes.get(&id("meeting-a")).await.unwrap().unwrap();
    assert_eq!(stored.doc.content[0].inline_content().len(), 1);
    assert_eq!(stored.markdown, "***ab***\n");
    assert_eq!(saved.updated_at, at(NOW));
    assert_eq!(h.events.announced(), [saved]);
    assert_eq!(
        h.service
            .markdown(&id("meeting-a"))
            .await
            .unwrap()
            .as_deref(),
        Some("***ab***\n")
    );
}

#[tokio::test]
async fn every_autosave_replaces_the_last_one() {
    let h = harness().await;
    h.service
        .save(&id("meeting-a"), doc("draft"))
        .await
        .unwrap();
    h.clock.advance(TimeDelta::seconds(1));

    h.service
        .save(&id("meeting-a"), doc("draft, longer"))
        .await
        .unwrap();

    let opened = h.service.get(&id("meeting-a")).await.unwrap();
    assert_eq!(opened.doc, doc("draft, longer"));
    assert_eq!(opened.updated_at, Some(at(NOW) + TimeDelta::seconds(1)));
}

#[tokio::test]
async fn bad_documents_are_refused_and_nothing_is_stored() {
    let h = harness().await;
    let empty_text = Doc {
        content: vec![Block::paragraph(vec![Inline::text("", vec![])])],
    };
    let huge = doc(&"x".repeat(MAX_DOC_BYTES));

    let errors = [
        h.service
            .save(&id("meeting-a"), empty_text)
            .await
            .unwrap_err(),
        h.service
            .save(&id("meeting-a"), image("/etc/passwd"))
            .await
            .unwrap_err(),
        h.service.save(&id("meeting-a"), huge).await.unwrap_err(),
        h.service
            .save(&id("meeting-x"), doc("hi"))
            .await
            .unwrap_err(),
    ];

    assert!(matches!(errors[0], NotesError::InvalidDoc(_)));
    assert!(matches!(errors[1], NotesError::ForeignImage));
    assert!(matches!(errors[2], NotesError::TooLarge));
    assert!(matches!(errors[3], NotesError::NotFound(_)));
    assert_eq!(h.notes.get(&id("meeting-a")).await.unwrap(), None);
    assert!(h.events.announced().is_empty());
}

#[tokio::test]
async fn images_are_checked_by_content_and_named_by_the_service() {
    let h = harness().await;

    let stored = h.service.save_image(&id("meeting-a"), PNG).await.unwrap();

    let name = ImageName::from_src(&stored.src).unwrap();
    assert_eq!(h.images.names(&id("meeting-a")), [name.to_string()]);
    assert_eq!(
        name.to_string().rsplit_once('.').map(|(_, ext)| ext),
        Some("png")
    );
    h.service
        .save(&id("meeting-a"), image(&stored.src))
        .await
        .unwrap();
}

#[tokio::test]
async fn unsupported_or_oversized_images_are_refused() {
    let h = harness().await;
    let mut oversized = PNG.to_vec();
    oversized.resize(MAX_IMAGE_BYTES + 1, 0);

    let svg = h
        .service
        .save_image(&id("meeting-a"), b"<svg/>")
        .await
        .unwrap_err();
    let big = h
        .service
        .save_image(&id("meeting-a"), &oversized)
        .await
        .unwrap_err();

    assert!(matches!(svg, NotesError::UnsupportedImage));
    assert!(matches!(big, NotesError::ImageTooLarge));
    assert!(h.images.names(&id("meeting-a")).is_empty());
}

#[tokio::test]
async fn scheduled_meetings_hold_notes_before_a_recording() {
    let h = harness().await;

    let meeting = h.service.create_scheduled("  Board prep ").await.unwrap();

    assert_eq!(meeting.title, "Board prep");
    assert!(meeting.id.as_str().starts_with("meeting-"));
    let state = h.meetings.state(&meeting.id).await.unwrap().unwrap();
    assert_eq!(state.status, MeetingStatus::Scheduled);
    assert!(h.service.create_scheduled(" ").await.is_err());
}
