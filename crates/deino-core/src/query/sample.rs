//! A sample library for tests and demos (feature `test-support`): the eight
//! fixture meetings (`deino_testkit::seed_fixtures`) plus what the fixtures
//! only describe, so every read has something to find.
//!
//! - Shreyas (`shreyas@deino.dev`) is "Me".
//! - Every fixture's ground-truth action items are stored as AI items, with
//!   their assignee, due day and evidence segment; those due a week or more
//!   before [`SAMPLE_NOW`] are done.
//! - Each meeting's AI notes (one enhanced-notes version) list its
//!   decisions and its commitments, each citing the moment it was made.
//! - Tags "Customer" and "Hiring", and the folders Customers › Brightkite.
//!
//! Every ID is a fixture UUID, so output is the same on every run.

use chrono::{DateTime, Days, Utc};
use deino_db::{Database, DbError};
use deino_domain::{
    ActionItemId, ActionItemOrigin, ActionItemStatus, FolderId, IdError, InvalidPaletteColor,
    LocalZone, MeetingId, NOON, PaletteColor, PersonId, SegmentId, TagId, TagSource,
};
use deino_testkit::at;
use deino_testkit::fixture::{
    ExpectedActionItem, FixtureError, FixtureMeeting, Fixtures, SeedError, fixture_uuid,
    person_key, seed_fixtures, segment_key,
};

use super::sample_notes::enhanced_notes;
use crate::action_items::{ActionItem, ActionItemRepo, SqliteActionItemRepo};
use crate::enhance::SqliteEnhancedNotesRepo;
use crate::folders::{Folder, FolderRepo, MeetingFolderRepo, SqliteFolderRepo};
use crate::people::{MeRepo, SqlitePeopleRepo};
use crate::tags::{MeetingTagRepo, SqliteTagRepo, Tag, TagApplication, TagRepo};

/// "Now" in the sample: Friday 9 October 2026, 16:00 UTC.
pub const SAMPLE_NOW: &str = "2026-10-09T16:00:00Z";
/// The user's email in the fixtures.
const ME: &str = "shreyas@deino.dev";
/// The sample's tags and the meetings they are on.
const TAGS: [(&str, &[&str]); 2] = [
    (
        "Customer",
        &[
            "meeting-fixture-sales-discovery",
            "meeting-fixture-customer-support",
        ],
    ),
    ("Hiring", &["meeting-fixture-interview"]),
];

/// Why the sample could not be seeded.
#[derive(Debug, thiserror::Error)]
pub enum SampleError {
    /// The fixture files could not be read.
    #[error("could not load the fixtures")]
    Fixtures(#[from] FixtureError),
    /// The fixtures could not be written.
    #[error("could not seed the fixtures")]
    Seed(#[from] SeedError),
    /// A row could not be written.
    #[error("could not write the sample")]
    Storage(#[from] DbError),
    /// A fixture key made an invalid ID.
    #[error("a sample ID is invalid")]
    Id(#[from] IdError),
    /// A palette slot is out of range.
    #[error("a sample colour is invalid")]
    Color(#[from] InvalidPaletteColor),
    /// The sample notes are not a valid document.
    #[error("the sample notes are invalid")]
    Notes(#[from] deino_doc::DocError),
}

/// Seeds the sample library into `database`, with due days in `zone`.
pub async fn seed_sample_library(
    database: &Database,
    zone: &dyn LocalZone,
) -> Result<(), SampleError> {
    let fixtures = Fixtures::load()?;
    seed_fixtures(database).await?;
    let now = at(SAMPLE_NOW);
    let me: PersonId = fixture_uuid(&person_key(ME)).parse()?;
    SqlitePeopleRepo::new(database.clone())
        .make_me(&me, now)
        .await?;
    let items = SqliteActionItemRepo::new(database.clone());
    let notes = SqliteEnhancedNotesRepo::new(database.clone());
    let done_until = zone.day_of(now) - Days::new(7);
    for meeting in fixtures.meetings() {
        enhanced_notes(&notes, meeting).await?;
        for (index, expected) in meeting.ground_truth.action_items.iter().enumerate() {
            let item = sample_item(meeting, index, expected, zone)?;
            let done = item
                .due_at
                .is_some_and(|due| zone.day_of(due) <= done_until);
            items.create(&item).await?;
            if done {
                items
                    .set_status(&item.id, ActionItemStatus::Done, now)
                    .await?;
            }
        }
    }
    tags(database, now).await?;
    folders(database, now).await
}

/// The fixture's ground-truth item number `index` as an open AI item.
fn sample_item(
    meeting: &FixtureMeeting,
    index: usize,
    expected: &ExpectedActionItem,
    zone: &dyn LocalZone,
) -> Result<ActionItem, SampleError> {
    let source: Option<SegmentId> = meeting
        .segments
        .iter()
        .position(|segment| segment.text.contains(&expected.evidence))
        .map(|position| fixture_uuid(&segment_key(meeting.id.as_str(), position)).parse())
        .transpose()?;
    let assignee: Option<PersonId> = expected
        .assignee
        .as_ref()
        .map(|email| fixture_uuid(&person_key(email.as_str())).parse())
        .transpose()?;
    Ok(ActionItem {
        id: uuid_of::<ActionItemId>(&format!("action-item/{}/{index}", meeting.id))?,
        meeting_id: Some(meeting.id.clone()),
        title: expected.title.clone(),
        description: String::new(),
        assignee_person_id: assignee,
        due_at: expected.due.map(|day| zone.at(day, NOON)),
        status: ActionItemStatus::Open,
        priority: None,
        source_segment_id: source,
        completed_at: None,
        created_by: ActionItemOrigin::Ai,
        created_at: meeting.started_at,
        updated_at: meeting.started_at,
    })
}

async fn tags(database: &Database, now: DateTime<Utc>) -> Result<(), SampleError> {
    let repo = SqliteTagRepo::new(database.clone());
    for (slot, (name, meetings)) in (1..).zip(TAGS) {
        let tag = Tag {
            id: uuid_of::<TagId>(&format!("tag/{name}"))?,
            name: name.to_owned(),
            color: PaletteColor::new(slot)?,
            description: format!("Meetings about {}", name.to_lowercase()),
            auto_apply: false,
            created_at: now,
            updated_at: now,
        };
        repo.create_tag(&tag).await?;
        for meeting in meetings {
            let application = TagApplication {
                meeting_id: MeetingId::new(*meeting),
                tag_id: tag.id,
                applied_by: TagSource::User,
                applied_at: now,
            };
            repo.apply_tag(&application).await?;
        }
    }
    Ok(())
}

/// Customers › Brightkite: the discovery call in Brightkite, the support
/// follow-up in Customers.
async fn folders(database: &Database, now: DateTime<Utc>) -> Result<(), SampleError> {
    let repo = SqliteFolderRepo::new(database.clone());
    let customers = folder("Customers", None, now)?;
    let brightkite = folder("Brightkite", Some(customers.id), now)?;
    repo.create_folder(&customers).await?;
    repo.create_folder(&brightkite).await?;
    let filed = [
        ("meeting-fixture-sales-discovery", brightkite.id),
        ("meeting-fixture-customer-support", customers.id),
    ];
    for (meeting, folder) in filed {
        repo.file_meeting(&MeetingId::new(meeting), &folder, now)
            .await?;
    }
    Ok(())
}

fn folder(name: &str, parent: Option<FolderId>, now: DateTime<Utc>) -> Result<Folder, SampleError> {
    Ok(Folder {
        id: uuid_of::<FolderId>(&format!("folder/{name}"))?,
        parent_id: parent,
        name: name.to_owned(),
        icon: None,
        description: String::new(),
        created_at: now,
        updated_at: now,
    })
}

fn uuid_of<T: std::str::FromStr<Err = IdError>>(key: &str) -> Result<T, IdError> {
    fixture_uuid(key).parse()
}
