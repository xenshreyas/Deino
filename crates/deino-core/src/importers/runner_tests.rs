//! [`ImportRunner`] and [`ImportJobs`] over the real SQLite repositories,
//! with a scripted source.

use std::sync::Mutex;

use async_trait::async_trait;
use deino_domain::{ActionItemStatus, AttendeeRole, ImportSource};
use serde_json::json;

use super::*;
use crate::action_items::{ActionItemRepo, SqliteActionItemRepo};
use crate::importers::{ImportJobs, ImportWorld, NoImportEvents, SourcePage};
use crate::people::{AttendeeRepo, SqlitePeopleRepo};
use crate::transcript_import::TranscriptImport;

/// Two pages of meetings; fetches what the list left out; can fail one
/// meeting, refuse the key, or cancel mid-run.
#[derive(Default)]
struct Scripted {
    broken: Option<&'static str>,
    unauthorized: bool,
    cancel_after_fetch: Option<CancellationToken>,
    fetched: Mutex<Vec<String>>,
}

fn meeting(title: &str, word: &str) -> TranscriptImport {
    serde_json::from_value(json!({
        "name": title,
        "createdAt": "2026-09-01T10:00:00Z",
        "notes": format!("## Summary\n\n- We agreed on {word}"),
        "transcript": [
            { "speaker": "Ana", "text": format!("Let us talk about {word}"), "timestamp": 0.0, "endTimestamp": 3.0 },
            { "speaker": "Me", "text": "Sounds good", "timestamp": 3.5 }
        ],
        "attendees": [
            { "name": "Ana Ruiz", "email": "ana@acme.com", "isCalendarEventOrganizer": true },
            { "name": "Ben", "isCalendarInvitee": true }
        ],
        "actionItems": [
            { "title": format!("Send the {word} plan"), "assignee": { "email": "ana@acme.com" } },
            { "title": "Book the room", "done": true }
        ]
    }))
    .unwrap()
}

#[async_trait]
impl Importer for Scripted {
    fn source(&self) -> ImportSource {
        ImportSource::Circleback
    }

    async fn page(&self, cursor: Option<&str>) -> Result<SourcePage, ImporterError> {
        if self.unauthorized {
            return Err(ImporterError::Unauthorized(self.source()));
        }
        Ok(match cursor {
            None => SourcePage {
                items: vec![
                    SourceItem::complete("m1", meeting("Pricing review", "zeppelin")),
                    SourceItem::listed("m2"),
                ],
                next: Some("page-2".to_owned()),
            },
            Some(_) => SourcePage {
                // `m2` again: a source that repeats an item across pages.
                items: vec![SourceItem::listed("m2"), SourceItem::listed("m3")],
                next: None,
            },
        })
    }

    async fn fetch(&self, item: SourceItem) -> Result<TranscriptImport, ImporterError> {
        self.fetched.lock().unwrap().push(item.id.clone());
        if let Some(token) = &self.cancel_after_fetch {
            token.cancel();
        }
        if self.broken == Some(item.id.as_str()) {
            return Err(ImporterError::Unreadable(self.source(), "broken"));
        }
        match item.meeting {
            Some(meeting) => Ok(*meeting),
            None => Ok(meeting(&format!("Meeting {}", item.id), "walrus")),
        }
    }
}

async fn run(world: &ImportWorld, source: &Scripted) -> ImportProgress {
    world
        .runner
        .run(source, &CancellationToken::new(), &NoImportEvents)
        .await
}

#[tokio::test]
async fn importing_twice_adds_nothing() {
    let world = ImportWorld::new().await;

    let first = run(&world, &Scripted::default()).await;
    let second = run(&world, &Scripted::default()).await;

    assert_eq!((first.phase, first.total, first.imported), (ImportPhase::Finished, 3, 3));
    assert_eq!((second.total, second.imported, second.skipped), (0, 0, 3));
    assert_eq!(world.meeting_count().await, 3);
    let preview = world.runner.preview(&Scripted::default()).await.unwrap();
    assert_eq!((preview.total, preview.already_imported, preview.new), (3, 3, 0));
}

#[tokio::test]
async fn the_preview_counts_without_importing() {
    let world = ImportWorld::new().await;

    let preview = world.runner.preview(&Scripted::default()).await.unwrap();

    assert_eq!((preview.total, preview.already_imported, preview.new), (3, 0, 3));
    assert_eq!(world.meeting_count().await, 0);
}

#[tokio::test]
async fn imported_meetings_are_searchable() {
    let world = ImportWorld::new().await;
    run(&world, &Scripted::default()).await;

    let hits = world.search("zeppelin").await;

    let (id, _) = world
        .titles()
        .await
        .into_iter()
        .find(|(_, title)| title == "Pricing review")
        .unwrap();
    assert!(!hits.is_empty());
    assert!(hits.iter().all(|hit| hit.meeting_id.as_ref() == Some(&id)));
}

#[tokio::test]
async fn attendees_become_people_and_action_items_are_kept() {
    let world = ImportWorld::new().await;
    run(&world, &Scripted::default()).await;
    let (id, _) = world.titles().await.remove(0);

    let attendees = SqlitePeopleRepo::new(world.database.clone())
        .attendees(&id)
        .await
        .unwrap();
    let items = SqliteActionItemRepo::new(world.database.clone())
        .list_for_meeting(&id)
        .await
        .unwrap();

    let roles: Vec<AttendeeRole> = attendees.iter().map(|a| a.role).collect();
    assert_eq!(roles.len(), 2);
    assert!(roles.contains(&AttendeeRole::Organizer));
    assert_eq!(items.len(), 2);
    let send = items.iter().find(|item| item.title.starts_with("Send")).unwrap();
    let book = items.iter().find(|item| item.title.starts_with("Book")).unwrap();
    assert!(send.assignee_person_id.is_some());
    assert_eq!(book.status, ActionItemStatus::Done);
    // Ana is one person across three meetings: keyed by her address.
    let people: i64 = sqlx::query_scalar("SELECT count(*) FROM people WHERE email = 'ana@acme.com'")
        .fetch_one(world.database.reader())
        .await
        .unwrap();
    assert_eq!(people, 1);
}

#[tokio::test]
async fn a_broken_meeting_is_counted_and_tried_again_next_time() {
    let world = ImportWorld::new().await;
    let broken = Scripted {
        broken: Some("m2"),
        ..Scripted::default()
    };

    let first = run(&world, &broken).await;
    let second = run(&world, &Scripted::default()).await;

    assert_eq!((first.imported, first.failed), (2, 1));
    assert_eq!((second.imported, second.skipped), (1, 2));
}

#[tokio::test]
async fn a_refused_key_stops_the_run_with_its_reason() {
    let world = ImportWorld::new().await;
    let refused = Scripted {
        unauthorized: true,
        ..Scripted::default()
    };

    let progress = run(&world, &refused).await;

    assert_eq!(progress.phase, ImportPhase::Failed);
    assert!(progress.error.unwrap().contains("API key"));
}

#[tokio::test]
async fn a_cancelled_job_keeps_what_it_imported_and_frees_the_slot() {
    let world = ImportWorld::new().await;
    let jobs = ImportJobs::new(world.runner, Arc::new(NoImportEvents));
    let token = jobs.begin(ImportSource::Circleback).unwrap();
    assert!(matches!(
        jobs.begin(ImportSource::Granola),
        Err(ImporterError::Busy)
    ));
    let source = Scripted {
        cancel_after_fetch: Some(token.clone()),
        ..Scripted::default()
    };

    let progress = jobs.run(&source, token).await;

    assert_eq!((progress.phase, progress.imported), (ImportPhase::Cancelled, 1));
    assert_eq!(jobs.status(), progress);
    assert!(!jobs.cancel());
    assert!(jobs.begin(ImportSource::Circleback).is_ok());
}
