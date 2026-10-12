//! Who a follow-up goes to, and whether a meeting gets one at all: at least
//! one external attendee and at most [`MAX_PEOPLE`] people
//! (`docs/research/granola.md` §7.2). Internal or external is
//! `profile::InternalDomains`, the rule calendar sync, templates and
//! briefs share, over ORG-04's people.

use deino_domain::{Email, PersonId};

use crate::profile::InternalDomains;

/// The most people a meeting may have for a follow-up to be suggested.
pub const MAX_PEOPLE: usize = 10;

/// Someone who took part: an attendee or a named speaker.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Participant {
    /// The person.
    pub id: PersonId,
    /// Their name.
    pub name: String,
    /// Their address, when known.
    pub email: Option<Email>,
    /// Their company's name, when known.
    pub company: Option<String>,
    /// True for the user.
    pub is_me: bool,
}

impl Participant {
    /// True when the person is outside the user's organisation: an address
    /// that is not the user's and not at one of their work domains.
    pub fn is_external(&self, domains: &InternalDomains) -> bool {
        !self.is_me
            && self
                .email
                .as_ref()
                .is_some_and(|email| !domains.is_internal(email))
    }
}

/// Why a meeting gets no follow-up before any model is asked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ineligible {
    /// The user listed no address in Settings → Profile, so internal and
    /// external cannot be told apart.
    NoProfile,
    /// Nobody is known to have attended.
    NoAttendees,
    /// Everyone is from the user's organisation.
    Internal,
    /// More than [`MAX_PEOPLE`] people took part.
    TooManyPeople,
}

impl Ineligible {
    /// The pipeline's skip reason, shown in the stage's status.
    pub fn reason(self) -> &'static str {
        match self {
            Self::NoProfile => "Add your work email in Settings → Profile to get follow-ups.",
            Self::NoAttendees => "No attendees are known.",
            Self::Internal => "Everyone was from your organisation.",
            Self::TooManyPeople => "More than ten people took part.",
        }
    }
}

/// The external attendees' addresses (in order, without repeats) when the
/// meeting gets a follow-up; otherwise why not.
pub fn judge(people: &[Participant], domains: &InternalDomains) -> Result<Vec<Email>, Ineligible> {
    if domains.own().next().is_none() {
        return Err(Ineligible::NoProfile);
    }
    if people.is_empty() {
        return Err(Ineligible::NoAttendees);
    }
    if people.len() > MAX_PEOPLE {
        return Err(Ineligible::TooManyPeople);
    }
    let mut external: Vec<Email> = Vec::new();
    for email in people
        .iter()
        .filter(|person| person.is_external(domains))
        .filter_map(|person| person.email.clone())
    {
        if !external.contains(&email) {
            external.push(email);
        }
    }
    if external.is_empty() {
        return Err(Ineligible::Internal);
    }
    Ok(external)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn person(name: &str, email: Option<&str>, is_me: bool) -> Participant {
        Participant {
            id: PersonId::generate(&deino_domain::FakeClock::new(chrono::DateTime::UNIX_EPOCH)),
            name: name.to_owned(),
            email: email.map(|email| Email::parse(email).unwrap()),
            company: None,
            is_me,
        }
    }

    fn deino() -> InternalDomains {
        InternalDomains::from_emails(&[Email::parse("shreyas@deino.dev").unwrap()])
    }

    fn me() -> Participant {
        person("Shreyas", Some("shreyas@deino.dev"), true)
    }

    #[test]
    fn one_external_guest_is_enough_and_only_guests_are_recipients() {
        let people = [
            me(),
            person("Lena", Some("lena@deino.dev"), false),
            person("Marcus", Some("marcus@brightkite.example"), false),
            person("Marcus again", Some("marcus@brightkite.example"), false),
            person("Hannah", Some("hannah@brightkite.example"), false),
        ];

        let to = judge(&people, &deino()).unwrap();

        let to: Vec<&str> = to.iter().map(Email::as_str).collect();
        assert_eq!(
            to,
            ["marcus@brightkite.example", "hannah@brightkite.example"]
        );
    }

    #[test]
    fn colleagues_only_and_people_without_addresses_are_internal() {
        let people = [
            me(),
            person("Maya", Some("maya@deino.dev"), false),
            person("Someone", None, false),
        ];

        assert_eq!(judge(&people, &deino()), Err(Ineligible::Internal));
    }

    #[test]
    fn ten_people_qualify_and_eleven_do_not() {
        let mut people = vec![me(), person("Guest", Some("g@acme.example"), false)];
        people.extend((0..8).map(|n| person(&format!("P{n}"), None, false)));
        assert!(judge(&people, &deino()).is_ok());

        people.push(person("One more", None, false));
        assert_eq!(judge(&people, &deino()), Err(Ineligible::TooManyPeople));
    }

    #[test]
    fn without_a_profile_or_attendees_nothing_can_be_judged() {
        let nobody = InternalDomains::default();

        assert_eq!(judge(&[me()], &nobody), Err(Ineligible::NoProfile));
        assert_eq!(judge(&[], &deino()), Err(Ineligible::NoAttendees));
    }
}
