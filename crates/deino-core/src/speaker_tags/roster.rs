//! The roster as evidence of who spoke, pure: when the call window listed
//! exactly one remote person at a moment, the remote voice then is theirs.
//!
//! Zoom hides its participant tiles while the user shares their screen, so
//! the roster (`speaker_tag_roster`) often covers only part of a call. Three
//! rules keep partial coverage honest:
//!
//! 1. [`without_me`]: the user is never a remote voice, though the window
//!    lists them when the adapter could not tell which tile is theirs.
//! 2. [`covers_call`]: the one-to-one rule ([`super::one_to_one`]) needs the
//!    roster to have watched most of the call; a roster that saw one person
//!    for a few minutes says nothing about the rest.
//! 3. [`listed_alone`]: a system-track segment whose middle falls where the
//!    roster listed exactly one remote person is won by that person.

use deino_domain::{DisplayName, Millis, TagSpan};

use super::align::{Timed, Win};

/// The share of the system track's speech the roster must have watched for
/// the one-to-one rule, in percent.
pub const ONE_TO_ONE_COVERAGE_PERCENT: u64 = 80;

/// `roster` without the spans that list the user: the name `me`, or a
/// longer name starting with it ("Shreyas" lists as "Shreyas Goyal").
pub fn without_me(roster: Vec<TagSpan>, me: Option<&str>) -> Vec<TagSpan> {
    let Some(me) = me.and_then(|name| DisplayName::parse(name).ok()) else {
        return roster;
    };
    roster
        .into_iter()
        .filter(|span| !is_me(&span.name, &me))
        .collect()
}

fn is_me(name: &DisplayName, me: &DisplayName) -> bool {
    let (name, me) = (name.key(), me.key());
    name == me
        || name
            .strip_prefix(&me)
            .is_some_and(|rest| rest.starts_with(' '))
}

/// Whether `roster` watched at least [`ONE_TO_ONE_COVERAGE_PERCENT`] of the
/// speech in `segments`.
pub fn covers_call(roster: &[TagSpan], segments: &[Timed]) -> bool {
    let (watched, total) = segments
        .iter()
        .fold((0_u64, 0_u64), |(watched, total), segment| {
            let length = u64::from(segment.length());
            let seen = listed_at(roster, middle(segment)).next().is_some();
            (watched + if seen { length } else { 0 }, total + length)
        });
    total > 0 && watched * 100 >= total * ONE_TO_ONE_COVERAGE_PERCENT
}

/// Every segment of `segments` whose middle falls where `roster` listed
/// exactly one remote person, won by that person.
pub fn listed_alone(roster: &[TagSpan], segments: &[Timed]) -> Vec<Win> {
    segments
        .iter()
        .filter(|segment| segment.length() > 0)
        .filter_map(|segment| {
            let mut names = listed_at(roster, middle(segment)).map(|span| &span.name);
            let first = names.next()?;
            names.all(|name| name.key() == first.key()).then(|| Win {
                segment: segment.id,
                name: first.clone(),
                covered: Millis(segment.length()),
            })
        })
        .collect()
}

fn listed_at(roster: &[TagSpan], at: Millis) -> impl Iterator<Item = &TagSpan> {
    roster
        .iter()
        .filter(move |span| span.start <= at && at < span.end)
}

fn middle(segment: &Timed) -> Millis {
    Millis(segment.start.0 + segment.length() / 2)
}

#[cfg(test)]
#[path = "roster_tests.rs"]
mod tests;
