//! Who a name from the meeting app stands for.
//!
//! In order: an invited attendee with that name (ignoring case and
//! spacing); an invited attendee whose first name it is, when the app shows
//! a single word ("Alex") and only one attendee has that first name; else the
//! one person in People with exactly that name. Otherwise the name stays a
//! label: Deino never creates people from what a window shows. The user is
//! never a match: their own tile does not speak on the system track.

use deino_db::DbError;
use deino_domain::{DisplayName, MeetingId, PersonId};

use crate::people::{AttendeeRepo, Person, PersonRepo};

/// How many People search results are checked for an exact name.
const SEARCH_LIMIT: u32 = 20;

/// The ports a name is resolved through.
pub(super) struct Directory<'a> {
    /// The meeting's invitees.
    pub(super) attendees: &'a dyn AttendeeRepo,
    /// Everyone the user knows.
    pub(super) people: &'a dyn PersonRepo,
}

/// The names in `names` that stand for a known person.
pub(super) async fn resolve(
    directory: &Directory<'_>,
    meeting: &MeetingId,
    names: &[DisplayName],
) -> Result<Vec<(DisplayName, PersonId)>, DbError> {
    let invited = invitees(directory, meeting).await?;
    let mut resolved = Vec::new();
    for name in names {
        let person = match by_attendee(name, &invited) {
            Some(person) => Some(person),
            None => by_people(directory.people, name).await?,
        };
        resolved.extend(person.map(|person| (name.clone(), person)));
    }
    Ok(resolved)
}

/// The meeting's attendees as people, without the user and merged people.
async fn invitees(directory: &Directory<'_>, meeting: &MeetingId) -> Result<Vec<Person>, DbError> {
    let mut people = Vec::new();
    for attendee in directory.attendees.attendees(meeting).await? {
        let person = directory.people.find(&attendee.person_id).await?;
        people.extend(person.filter(eligible));
    }
    Ok(people)
}

/// The invitee `name` stands for: their full name, or their unique first name.
pub(super) fn by_attendee(name: &DisplayName, invited: &[Person]) -> Option<PersonId> {
    if let Some(person) = invited.iter().find(|person| name.matches(&person.name)) {
        return Some(person.id);
    }
    if name.as_str().contains(' ') {
        return None;
    }
    let mut first_names = invited.iter().filter(|person| {
        person
            .name
            .split_whitespace()
            .next()
            .is_some_and(|first| name.matches(first))
    });
    match (first_names.next(), first_names.next()) {
        (Some(person), None) => Some(person.id),
        _ => None,
    }
}

/// The one person in People named exactly `name`.
async fn by_people(
    people: &dyn PersonRepo,
    name: &DisplayName,
) -> Result<Option<PersonId>, DbError> {
    let found = people.search(name.as_str(), SEARCH_LIMIT).await?;
    let mut exact = found
        .iter()
        .filter(|person| eligible(person) && name.matches(&person.name));
    Ok(match (exact.next(), exact.next()) {
        (Some(person), None) => Some(person.id),
        _ => None,
    })
}

/// Anyone but the user and people merged into someone else.
fn eligible(person: &Person) -> bool {
    !person.is_me && person.merged_into.is_none()
}
