//! Rows the entity tests build on: people, companies, meetings and every
//! kind of link to a person, written straight into a test database.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use deino_db::{Database, test_db};
use deino_domain::{AttendeeRole, Clock, CompanyId, Email, FakeClock, MeetingId, PersonId};
use deino_testkit::{at, seed};

use super::EntityPorts;
use crate::companies::{Company, CompanyDomain, CompanyRepo, SqliteCompanyRepo};
use crate::people::{Person, PersonRepo, SqlitePeopleRepo};

/// "Now" in every entity test.
pub(super) const NOW: &str = "2026-10-11T12:00:00Z";

/// A test database with its fixed clock.
pub(super) struct World {
    pub database: Database,
    pub clock: Arc<FakeClock>,
}

impl World {
    /// An empty, migrated database at [`NOW`].
    pub async fn new() -> Self {
        Self {
            database: test_db().await,
            clock: Arc::new(FakeClock::new(at(NOW))),
        }
    }

    /// Every entity port over the database.
    pub fn ports(&self) -> EntityPorts {
        let clock: Arc<dyn Clock> = self.clock.clone();
        EntityPorts::sqlite(&self.database, clock)
    }

    /// Stores a person; `email` may be `None`.
    pub async fn person(&self, name: &str, email: Option<&str>) -> Person {
        let now = self.clock.now();
        let person = Person {
            id: PersonId::generate(self.clock.as_ref()),
            name: name.into(),
            email: email.map(|email| Email::parse(email).unwrap()),
            company_id: None,
            title: None,
            avatar_path: None,
            notes_md: String::new(),
            summary_md: None,
            summary_updated_at: None,
            is_me: false,
            merged_into: None,
            created_at: now,
            updated_at: now,
        };
        SqlitePeopleRepo::new(self.database.clone())
            .create(&person)
            .await
            .unwrap();
        person
    }

    /// Stores a company.
    pub async fn company(&self, name: &str, domain: Option<&str>) -> Company {
        let company = Company::new(
            CompanyId::generate(self.clock.as_ref()),
            name.into(),
            domain.and_then(CompanyDomain::parse),
            self.clock.now(),
        );
        SqliteCompanyRepo::new(self.database.clone())
            .create(&company)
            .await
            .unwrap();
        company
    }

    /// Marks `person` as the user.
    pub async fn make_me(&self, person: &PersonId) {
        let id = person.to_string();
        self.run("UPDATE people SET is_me = 1 WHERE id = ?1", &[&id])
            .await;
    }

    /// Records `gone` as merged into `into`, without moving any link.
    pub async fn mark_merged(&self, gone: &PersonId, into: &PersonId) {
        let ids = [into.to_string(), gone.to_string()];
        self.run(
            "UPDATE people SET merged_into = ?1 WHERE id = ?2",
            &[&ids[0], &ids[1]],
        )
        .await;
    }

    /// Moves `person` to `company`.
    pub async fn employ(&self, person: &mut Person, company: &Company) {
        person.company_id = Some(company.id);
        SqlitePeopleRepo::new(self.database.clone())
            .update(person)
            .await
            .unwrap();
    }

    /// Stores a meeting that started at `started_at`.
    pub async fn meeting(&self, id: &str, title: &str, started_at: &str) -> MeetingId {
        seed::meeting(&self.database, id, title, at(started_at)).await;
        self.run(
            "UPDATE meetings SET started_at = created_at, duration_s = 1800 WHERE id = ?1",
            &[id],
        )
        .await;
        MeetingId::new(id)
    }

    /// Records `person` as attending `meeting` in `role`.
    pub async fn attend(&self, meeting: &MeetingId, person: &PersonId, role: AttendeeRole) {
        self.run(
            "INSERT INTO meeting_attendees (meeting_id, person_id, role) VALUES (?1, ?2, ?3)",
            &[meeting.as_str(), &person.to_string(), role.as_str()],
        )
        .await;
    }

    /// Gives `meeting` a finished legacy summary.
    pub async fn summarize(&self, meeting: &MeetingId, markdown: &str) {
        let result = serde_json::json!({ "markdown": markdown }).to_string();
        self.run(
            "INSERT INTO summary_processes (meeting_id, status, created_at, updated_at, result)
             VALUES (?1, 'completed', ?2, ?2, ?3)",
            &[meeting.as_str(), NOW, &result],
        )
        .await;
    }

    /// Runs `sql` with text `binds`.
    pub async fn run(&self, sql: &str, binds: &[&str]) {
        let mut query = sqlx::query(sql);
        for bind in binds {
            query = query.bind(*bind);
        }
        query.execute(self.database.writer()).await.unwrap();
    }

    /// The first column of the first row of `sql`, as text.
    pub async fn text(&self, sql: &str, binds: &[&str]) -> Option<String> {
        let mut query = sqlx::query_scalar::<_, Option<String>>(sql);
        for bind in binds {
            query = query.bind(*bind);
        }
        query.fetch_one(self.database.reader()).await.unwrap()
    }

    /// The person with `id`.
    pub async fn stored_person(&self, id: &PersonId) -> Person {
        SqlitePeopleRepo::new(self.database.clone())
            .find(id)
            .await
            .unwrap()
            .unwrap()
    }

    /// The company with `id`.
    pub async fn stored_company(&self, id: &CompanyId) -> Company {
        SqliteCompanyRepo::new(self.database.clone())
            .find(id)
            .await
            .unwrap()
            .unwrap()
    }
}

/// `text` as a UTC time.
pub(super) fn time(text: &str) -> DateTime<Utc> {
    at(text)
}
