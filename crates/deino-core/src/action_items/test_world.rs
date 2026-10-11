//! A small world for action item tests, on a real in-memory database: the
//! user, a colleague and a customer contact, a standup, a customer meeting
//! (tagged, filed two folders deep) and a trashed meeting. "Now" is Friday
//! 9 October 2026, 09:00 in California (UTC-7).

use std::sync::Arc;

use chrono::{DateTime, NaiveDate, Utc};
use deino_db::Database;
use deino_domain::{
    ActionItemId, ActionItemOrigin, ActionItemStatus, AttendeeRole, CompanyId, Email, FakeClock,
    FixedZone, FolderId, LocalZone, MeetingId, NOON, PaletteColor, PersonId, TagId, TagSource,
};
use deino_testkit::{at, seed, test_db};

use super::{
    ActionItem, ActionItemDeps, ActionItemRepo, ActionItemService, ActionItemView, OptionSources,
    RecordedActionItemEvents, SqliteActionItemRepo,
};
use crate::companies::{Company, CompanyRepo, SqliteCompanyRepo};
use crate::folders::{Folder, FolderRepo, MeetingFolderRepo, SqliteFolderRepo};
use crate::meetings::{MeetingTrashRepo, SqliteMeetingRepo};
use crate::people::{Attendee, AttendeeRepo, MeRepo, Person, PersonRepo, SqlitePeopleRepo};
use crate::tags::{MeetingTagRepo, SqliteTagRepo, Tag, TagApplication, TagRepo};

/// "Now": Friday 9 October 2026, 09:00 local.
pub const NOW: &str = "2026-10-09T16:00:00Z";
/// The daily standup.
pub const STANDUP: &str = "meeting-standup";
/// The customer call, tagged "Customer" and filed in Clients › Acme.
pub const ACME_CALL: &str = "meeting-acme";
/// A meeting in the trash.
pub const TRASHED: &str = "meeting-trashed";

/// The seeded database and the IDs tests refer to.
pub struct World {
    pub database: Database,
    pub repo: SqliteActionItemRepo,
    pub clock: Arc<FakeClock>,
    pub zone: Arc<FixedZone>,
    pub me: PersonId,
    pub maya: PersonId,
    pub owen: PersonId,
    pub acme: CompanyId,
    pub customer_tag: TagId,
    pub clients: FolderId,
}

impl World {
    /// Seeds the world.
    pub async fn new() -> Self {
        let database = test_db().await;
        let clock = Arc::new(FakeClock::new(at(NOW)));
        for (id, title, saved) in [
            (STANDUP, "Eng standup", "2026-10-08T15:30:00Z"),
            (ACME_CALL, "Acme renewal", "2026-10-05T16:00:00Z"),
            (TRASHED, "Old sync", "2026-10-01T16:00:00Z"),
        ] {
            seed::meeting(&database, id, title, at(saved)).await;
        }
        SqliteMeetingRepo::new(database.clone())
            .trash(&MeetingId::new(TRASHED), at(NOW))
            .await
            .unwrap();
        let acme = company(&database, &clock).await;
        let people = SqlitePeopleRepo::new(database.clone());
        let me = person(&people, &clock, "Shreyas Rao", None).await;
        people.make_me(&me, at(NOW)).await.unwrap();
        let maya = person(&people, &clock, "Maya Chen", None).await;
        let owen = person(&people, &clock, "Owen Hart", Some(acme)).await;
        attend(&people, ACME_CALL, owen).await;
        let customer_tag = tag(&database, &clock).await;
        let clients = folders(&database, &clock).await;
        Self {
            repo: SqliteActionItemRepo::new(database.clone()),
            database,
            clock,
            zone: Arc::new(FixedZone::hours(-7).unwrap()),
            me,
            maya,
            owen,
            acme,
            customer_tag,
            clients,
        }
    }

    /// Stores an open AI item and returns it.
    pub async fn item(&self, spec: Spec<'_>) -> ActionItem {
        let item = ActionItem {
            id: ActionItemId::generate(self.clock.as_ref()),
            meeting_id: spec.meeting.map(MeetingId::new),
            title: spec.title.into(),
            description: String::new(),
            assignee_person_id: spec.assignee,
            due_at: spec.due.map(|day| self.due(day)),
            status: ActionItemStatus::Open,
            priority: None,
            source_segment_id: None,
            completed_at: None,
            created_by: ActionItemOrigin::Ai,
            created_at: at(NOW),
            updated_at: at(NOW),
        };
        self.repo.create(&item).await.unwrap();
        if spec.done {
            self.repo
                .set_status(&item.id, ActionItemStatus::Done, at(NOW))
                .await
                .unwrap();
        }
        item
    }

    /// The service over this world's repositories, and its recorded events.
    pub fn service(&self) -> (ActionItemService, Arc<RecordedActionItemEvents>) {
        let repo = Arc::new(self.repo.clone());
        let people = Arc::new(SqlitePeopleRepo::new(self.database.clone()));
        let events = Arc::new(RecordedActionItemEvents::default());
        let service = ActionItemService::new(ActionItemDeps {
            items: repo.clone(),
            batch: repo.clone(),
            reader: repo,
            me: people.clone(),
            options: OptionSources {
                people,
                companies: Arc::new(SqliteCompanyRepo::new(self.database.clone())),
                tags: Arc::new(SqliteTagRepo::new(self.database.clone())),
                folders: Arc::new(SqliteFolderRepo::new(self.database.clone())),
            },
            events: events.clone(),
            clock: self.clock.clone(),
            zone: self.zone.clone(),
        });
        (service, events)
    }

    /// Noon, local time, on `day` (`YYYY-MM-DD`): how due days are stored.
    pub fn due(&self, day: &str) -> DateTime<Utc> {
        let day: NaiveDate = day.parse().unwrap();
        self.zone.at(day, NOON)
    }
}

/// A list row with every field set to something plain, for building others.
pub fn view() -> ActionItemView {
    ActionItemView {
        id: ActionItemId::generate(&FakeClock::new(at(NOW))),
        meeting_id: None,
        title: "Item".into(),
        description: String::new(),
        assignee_person_id: None,
        due_at: None,
        status: ActionItemStatus::Open,
        priority: None,
        source_segment_id: None,
        completed_at: None,
        created_by: ActionItemOrigin::User,
        created_at: at(NOW),
        updated_at: at(NOW),
        meeting_title: None,
        meeting_held_at: None,
        assignee_name: None,
        assignee_is_me: false,
        source_start: None,
    }
}

/// What [`World::item`] stores.
#[derive(Clone, Copy, Default)]
pub struct Spec<'a> {
    pub title: &'a str,
    pub meeting: Option<&'a str>,
    pub assignee: Option<PersonId>,
    pub due: Option<&'a str>,
    pub done: bool,
}

impl<'a> Spec<'a> {
    /// An open, unassigned, undated standalone item titled `title`.
    pub fn titled(title: &'a str) -> Self {
        Self {
            title,
            ..Self::default()
        }
    }
}

async fn company(database: &Database, clock: &FakeClock) -> CompanyId {
    let company = Company {
        id: CompanyId::generate(clock),
        name: "Acme".into(),
        domain: None,
        logo_path: None,
        notes_md: String::new(),
        summary_md: None,
        created_at: at(NOW),
        updated_at: at(NOW),
    };
    SqliteCompanyRepo::new(database.clone())
        .create(&company)
        .await
        .unwrap();
    company.id
}

async fn person(
    people: &SqlitePeopleRepo,
    clock: &FakeClock,
    name: &str,
    company: Option<CompanyId>,
) -> PersonId {
    let email = format!(
        "{}@example.com",
        name.split(' ').next().unwrap().to_lowercase()
    );
    let person = Person {
        id: PersonId::generate(clock),
        name: name.into(),
        email: Some(Email::parse(&email).unwrap()),
        company_id: company,
        title: None,
        avatar_path: None,
        notes_md: String::new(),
        summary_md: None,
        summary_updated_at: None,
        is_me: false,
        merged_into: None,
        created_at: at(NOW),
        updated_at: at(NOW),
    };
    people.create(&person).await.unwrap();
    person.id
}

async fn attend(people: &SqlitePeopleRepo, meeting: &str, person: PersonId) {
    let attendee = Attendee {
        meeting_id: MeetingId::new(meeting),
        person_id: person,
        role: AttendeeRole::Invitee,
        rsvp: None,
    };
    people.upsert_attendee(&attendee).await.unwrap();
}

async fn tag(database: &Database, clock: &FakeClock) -> TagId {
    let tags = SqliteTagRepo::new(database.clone());
    let tag = Tag {
        id: TagId::generate(clock),
        name: "Customer".into(),
        color: PaletteColor::new(2).unwrap(),
        description: String::new(),
        auto_apply: false,
        created_at: at(NOW),
        updated_at: at(NOW),
    };
    tags.create_tag(&tag).await.unwrap();
    let application = TagApplication {
        meeting_id: MeetingId::new(ACME_CALL),
        tag_id: tag.id,
        applied_by: TagSource::User,
        applied_at: at(NOW),
    };
    tags.apply_tag(&application).await.unwrap();
    tag.id
}

/// Clients › Acme, with the customer call filed in Acme; returns Clients.
async fn folders(database: &Database, clock: &FakeClock) -> FolderId {
    let folders = SqliteFolderRepo::new(database.clone());
    let folder = |name: &str, parent: Option<FolderId>| Folder {
        id: FolderId::generate(clock),
        parent_id: parent,
        name: name.into(),
        icon: None,
        description: String::new(),
        created_at: at(NOW),
        updated_at: at(NOW),
    };
    let clients = folder("Clients", None);
    let acme = folder("Acme", Some(clients.id));
    folders.create_folder(&clients).await.unwrap();
    folders.create_folder(&acme).await.unwrap();
    folders
        .file_meeting(&MeetingId::new(ACME_CALL), &acme.id, at(NOW))
        .await
        .unwrap();
    clients.id
}
