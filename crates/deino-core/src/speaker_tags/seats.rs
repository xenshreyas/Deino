//! Which speaker holds each name the meeting app showed, and which segments
//! move to it: pure rules over one meeting's system track.
//!
//! Every name that won a segment ([`super::align`]) gets one **seat**, the
//! first that applies:
//!
//! 1. the speaker already named as the name's person (by the user, or by an
//!    earlier run);
//! 2. the speaker already labelled with the name (an earlier run that found
//!    no person for it);
//! 3. the unnamed diarized speaker the name wins **more than half** of the
//!    speech of, the largest share first: that voice is the name's, so the
//!    speaker takes the name;
//! 4. a new speaker.
//!
//! Then every won segment moves to its name's seat, unless the user named
//! the speaker it sits on, or it is the user's own: a name overrides
//! diarization and voice matching, never the user. A speaker named here
//! counts as named, so voice matching leaves it alone.

mod tally;

use deino_domain::{DisplayName, PersonId, SegmentId, SpeakerId};
use tally::Tally;

use super::align::Win;
use crate::segments::Speaker;

/// A system-track segment as the rules see it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Seated {
    /// The segment.
    pub id: SegmentId,
    /// The speaker it is attributed to now.
    pub speaker: Option<SpeakerId>,
    /// Its length in milliseconds.
    pub length: u32,
}

/// What the rules look at.
#[derive(Clone, Copy, Debug)]
pub struct Ballot<'a> {
    /// The segments names won.
    pub wins: &'a [Win],
    /// Every segment of the system track.
    pub segments: &'a [Seated],
    /// The meeting's speakers.
    pub speakers: &'a [Speaker],
    /// The names that stand for a known person.
    pub people: &'a [(DisplayName, PersonId)],
}

/// The speaker a name's segments go to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Seat {
    /// An existing speaker.
    Speaker(SpeakerId),
    /// A speaker to create.
    New,
}

/// One name and its seat.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Elected {
    /// The name as the app showed it.
    pub name: DisplayName,
    /// The person it stands for, if any.
    pub person: Option<PersonId>,
    /// Where its segments go.
    pub seat: Seat,
    /// The seat is an unnamed speaker that takes the name (rules 2 and 3).
    pub takes_name: bool,
}

/// A segment moving to its name's seat.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Move {
    /// The segment.
    pub segment: SegmentId,
    /// The speaker it leaves.
    pub from: Option<SpeakerId>,
    /// The name whose seat it joins.
    pub name: DisplayName,
}

/// What the rules decided.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Election {
    /// One seat per name, the name with the most speech first.
    pub elected: Vec<Elected>,
    /// The segments that change speaker.
    pub moves: Vec<Move>,
}

impl Election {
    /// The seat of `name`.
    pub fn seat_of(&self, name: &DisplayName) -> Option<Seat> {
        self.elected
            .iter()
            .find(|elected| &elected.name == name)
            .map(|elected| elected.seat)
    }
}

/// Applies the rules (see the module docs).
pub fn elect(ballot: &Ballot<'_>) -> Election {
    let tally = Tally::count(ballot);
    let mut elected: Vec<Elected> = tally
        .ranked_names()
        .into_iter()
        .map(|name| standing(ballot, name))
        .collect();
    for index in 0..elected.len() {
        if elected[index].seat != Seat::New {
            continue;
        }
        let taken = seated_speakers(&elected);
        let free = |speaker: &Speaker| movable(speaker) && !taken.contains(&speaker.id);
        let candidates: Vec<&Speaker> = ballot.speakers.iter().filter(|s| free(s)).collect();
        if let Some(speaker) = tally.majority_speaker(&candidates, &elected[index].name) {
            elected[index].seat = Seat::Speaker(speaker);
            elected[index].takes_name = true;
        }
    }
    let election = Election {
        elected,
        moves: Vec::new(),
    };
    let moves = moves(ballot, &tally, &election);
    Election { moves, ..election }
}

/// Rules 1 and 2, or a new seat for rule 3 to fill.
fn standing(ballot: &Ballot<'_>, name: DisplayName) -> Elected {
    let person = ballot
        .people
        .iter()
        .find(|(known, _)| known == &name)
        .map(|(_, person)| *person);
    let own = person.and_then(|person| {
        ballot
            .speakers
            .iter()
            .find(|speaker| speaker.person_id == Some(person))
    });
    let labelled = ballot
        .speakers
        .iter()
        .find(|speaker| !speaker.is_me && name.matches(&speaker.label));
    let (seat, takes_name) = match (own, labelled) {
        (Some(speaker), _) => (Seat::Speaker(speaker.id), false),
        (None, Some(speaker)) => (
            Seat::Speaker(speaker.id),
            person.is_some() && speaker.person_id.is_none(),
        ),
        (None, None) => (Seat::New, false),
    };
    Elected {
        name,
        person,
        seat,
        takes_name,
    }
}

/// Speakers already holding a name.
fn seated_speakers(elected: &[Elected]) -> Vec<SpeakerId> {
    elected
        .iter()
        .filter_map(|elected| match elected.seat {
            Seat::Speaker(speaker) => Some(speaker),
            Seat::New => None,
        })
        .collect()
}

/// Every won segment that changes speaker.
fn moves(ballot: &Ballot<'_>, tally: &Tally<'_>, election: &Election) -> Vec<Move> {
    let pinned = |speaker: SpeakerId| {
        !ballot
            .speakers
            .iter()
            .any(|s| s.id == speaker && movable(s))
    };
    ballot
        .wins
        .iter()
        .filter_map(|win| {
            let segment = tally.segment(win.segment)?;
            let seat = election.seat_of(&win.name)?;
            let stays = matches!(seat, Seat::Speaker(id) if segment.speaker == Some(id));
            let kept = segment.speaker.is_some_and(pinned);
            (!stays && !kept).then(|| Move {
                segment: segment.id,
                from: segment.speaker,
                name: win.name.clone(),
            })
        })
        .collect()
}

/// A speaker nobody named and not the user's own voice.
fn movable(speaker: &Speaker) -> bool {
    speaker.person_id.is_none() && !speaker.is_me
}

#[cfg(test)]
#[path = "seats_tests.rs"]
mod tests;
