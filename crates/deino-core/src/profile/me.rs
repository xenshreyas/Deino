//! Keeps the "Me" person in step with the profile.
//!
//! Exactly one person row is the user (`people.is_me`). Its name and email
//! follow Settings → Profile, so the user is recognised among calendar
//! attendees and speakers. When the profile's first address already belongs
//! to a person (a calendar attendee seeded before the profile was filled in),
//! that person becomes "Me" rather than a duplicate; merging two people is
//! ORG-04's job.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use deino_db::DbError;
use deino_domain::{Clock, Email, PersonId};

use crate::people::{MeRepo, Person, PersonRepo};
use crate::settings::ProfileSettings;

/// The name "Me" gets until the profile has one.
pub const DEFAULT_ME_NAME: &str = "Me";

/// Writes the "Me" person through the people ports.
pub(super) struct MeSync {
    pub(super) people: Arc<dyn PersonRepo>,
    pub(super) me: Arc<dyn MeRepo>,
    pub(super) clock: Arc<dyn Clock>,
}

impl MeSync {
    /// Makes sure one person is the user, named and addressed as `profile`
    /// says, and returns their ID. Writes nothing when they already match.
    pub(super) async fn sync(&self, profile: &ProfileSettings) -> Result<PersonId, DbError> {
        let email = profile.primary_email();
        let owner = match email {
            Some(email) => self.people.find_by_email(email).await?,
            None => None,
        };
        let adopting = owner.is_some();
        let Some(person) = owner.or(self.me.me().await?) else {
            return self.create(profile).await;
        };
        let wanted = Wanted {
            name: me_name(profile, &person.name),
            email: if adopting {
                person.email.clone()
            } else {
                email.cloned()
            },
        };
        self.reconcile(person, wanted).await
    }

    /// Saves `wanted` onto `person` if it differs, then marks them "Me".
    async fn reconcile(&self, mut person: Person, wanted: Wanted) -> Result<PersonId, DbError> {
        let now = self.clock.now();
        if person.name != wanted.name || person.email != wanted.email {
            person.name = wanted.name;
            person.email = wanted.email;
            person.updated_at = now;
            self.people.update(&person).await?;
        }
        if !person.is_me {
            self.me.make_me(&person.id, now).await?;
            log::debug!(target: "profile", "person {} is now the user", person.id);
        }
        Ok(person.id)
    }

    /// Adds the "Me" person.
    async fn create(&self, profile: &ProfileSettings) -> Result<PersonId, DbError> {
        let now = self.clock.now();
        let person = new_me(profile, PersonId::generate(self.clock.as_ref()), now);
        self.people.create(&person).await?;
        log::debug!(target: "profile", "created the user's person {}", person.id);
        Ok(person.id)
    }
}

/// The name and email the "Me" person should have.
struct Wanted {
    name: String,
    email: Option<Email>,
}

/// The profile's name, else the person's current one, else [`DEFAULT_ME_NAME`].
fn me_name(profile: &ProfileSettings, current: &str) -> String {
    [profile.name.trim(), current.trim()]
        .into_iter()
        .find(|name| !name.is_empty())
        .unwrap_or(DEFAULT_ME_NAME)
        .to_owned()
}

/// A new person who is the user.
fn new_me(profile: &ProfileSettings, id: PersonId, now: DateTime<Utc>) -> Person {
    Person {
        id,
        name: me_name(profile, ""),
        email: profile.primary_email().cloned(),
        company_id: None,
        title: None,
        avatar_path: None,
        notes_md: String::new(),
        summary_md: None,
        summary_updated_at: None,
        is_me: true,
        merged_into: None,
        created_at: now,
        updated_at: now,
    }
}
