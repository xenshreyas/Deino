//! An [`Election`] made concrete: new speakers with IDs, labels and
//! colours, the names to give, the segments to move and the speakers they
//! empty, as one [`TagPlan`].

use std::collections::HashSet;

use deino_domain::{Clock, DisplayName, MeetingId, PaletteColor, PersonId, SegmentId, SpeakerId};

use super::seats::{Elected, Election, Seat, Seated};
use crate::diarization::speaker_labels;
use crate::segments::Speaker;
use crate::speakers::{Seating, SegmentMove};

/// What the meeting app's names decided before voices are matched.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TagPlan {
    /// Unnamed speakers that are now known people.
    pub named: Vec<(SpeakerId, PersonId)>,
    /// New speakers, labels, moved segments and emptied speakers.
    pub seating: Seating,
    /// Speakers holding a name that stands for no known person: voice
    /// matching must not name them otherwise, now or on a later run.
    pub claimed: Vec<SpeakerId>,
}

impl TagPlan {
    /// The meeting's speakers as they will be once the plan is written:
    /// named, relabelled, with the new ones and without the emptied ones.
    pub fn overlay(&self, speakers: Vec<Speaker>) -> Vec<Speaker> {
        let mut speakers: Vec<Speaker> = speakers
            .into_iter()
            .filter(|speaker| !self.seating.emptied.contains(&speaker.id))
            .map(|speaker| self.renamed(speaker))
            .collect();
        speakers.extend(self.seating.created.iter().cloned());
        speakers
    }

    fn renamed(&self, mut speaker: Speaker) -> Speaker {
        if let Some((_, person)) = self.named.iter().find(|(id, _)| *id == speaker.id) {
            speaker.person_id = Some(*person);
        }
        let labelled = self.seating.labelled.iter();
        if let Some((_, label)) = labelled.clone().find(|(id, _)| *id == speaker.id) {
            speaker.label = label.to_string();
        }
        speaker
    }
}

/// The meeting a plan is made for.
#[derive(Clone, Copy, Debug)]
pub(super) struct Track<'a> {
    /// The meeting.
    pub(super) meeting: &'a MeetingId,
    /// Its speakers.
    pub(super) speakers: &'a [Speaker],
    /// Its system-track segments.
    pub(super) segments: &'a [Seated],
}

/// Turns `election` over `track` into a plan; `clock` stamps new speakers'
/// IDs.
pub(super) fn plan(election: &Election, track: &Track<'_>, clock: &dyn Clock) -> TagPlan {
    let mut plan = TagPlan::default();
    let mut seats: Vec<(&DisplayName, SpeakerId)> = Vec::new();
    for elected in &election.elected {
        let speaker = match elected.seat {
            Seat::Speaker(speaker) => {
                name_existing(&mut plan, elected, speaker);
                speaker
            }
            Seat::New => {
                let speaker = new_speaker(track, &plan, elected, clock);
                let id = speaker.id;
                plan.seating.created.push(speaker);
                id
            }
        };
        if elected.person.is_none() {
            plan.claimed.push(speaker);
        }
        seats.push((&elected.name, speaker));
    }
    plan.seating.moved = moves(election, &seats);
    let seated: Vec<SpeakerId> = seats.iter().map(|(_, speaker)| *speaker).collect();
    plan.seating.emptied = emptied(election, track, &seated);
    plan
}

/// Rules 2 and 3: an unnamed speaker takes the person, or else the label.
fn name_existing(plan: &mut TagPlan, elected: &Elected, speaker: SpeakerId) {
    if !elected.takes_name {
        return;
    }
    match elected.person {
        Some(person) => plan.named.push((speaker, person)),
        None => plan.seating.labelled.push((speaker, elected.name.clone())),
    }
}

/// A speaker for a name no voice holds: labelled with the name when no
/// other speaker has it, else the next "Speaker N"; the next palette slot.
fn new_speaker(track: &Track<'_>, plan: &TagPlan, elected: &Elected, clock: &dyn Clock) -> Speaker {
    let created = &plan.seating.created;
    let taken: Vec<String> = track
        .speakers
        .iter()
        .chain(created)
        .map(|speaker| speaker.label.clone())
        .chain(
            plan.seating
                .labelled
                .iter()
                .map(|(_, label)| label.to_string()),
        )
        .collect();
    let free = !taken.iter().any(|label| elected.name.matches(label));
    let label = if free {
        elected.name.to_string()
    } else {
        speaker_labels(1, &taken)
            .into_iter()
            .next()
            .unwrap_or_default()
    };
    Speaker {
        id: SpeakerId::generate(clock),
        meeting_id: track.meeting.clone(),
        label,
        person_id: elected.person,
        is_me: false,
        color: PaletteColor::cycle(track.speakers.len() + created.len()),
    }
}

/// Each moving segment with the ID of its name's speaker.
fn moves(election: &Election, seats: &[(&DisplayName, SpeakerId)]) -> Vec<SegmentMove> {
    election
        .moves
        .iter()
        .filter_map(|change| {
            let (_, to) = seats.iter().find(|(name, _)| *name == &change.name)?;
            Some(SegmentMove {
                segment: change.segment,
                to: *to,
            })
        })
        .collect()
}

/// Unnamed speakers holding no name, every one of whose segments moves away.
fn emptied(election: &Election, track: &Track<'_>, seated: &[SpeakerId]) -> Vec<SpeakerId> {
    let moved: HashSet<SegmentId> = election.moves.iter().map(|m| m.segment).collect();
    let leaving = |speaker: &Speaker| {
        let mut own = track
            .segments
            .iter()
            .filter(|segment| segment.speaker == Some(speaker.id))
            .peekable();
        own.peek().is_some() && own.all(|segment| moved.contains(&segment.id))
    };
    track
        .speakers
        .iter()
        .filter(|speaker| speaker.person_id.is_none() && !speaker.is_me)
        .filter(|speaker| !seated.contains(&speaker.id) && leaving(speaker))
        .map(|speaker| speaker.id)
        .collect()
}
