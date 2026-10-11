//! [`RelatedService`]: the meetings related to one meeting.
//!
//! Granola links a recurring meeting's notes to its earlier instances by the
//! recurring event, or by an exact title (§6.5). An exact title alone would
//! relate every "Weekly sync" of every team, so Deino also asks for the same
//! people: the attendees, the user left out, must be the same non-empty set.

use std::collections::BTreeSet;
use std::sync::Arc;

use deino_domain::{MeetingId, PersonId};

use super::{
    LinkError, RelatedBy, RelatedCandidate, RelatedMeeting, RelatedMeetingRepo, RelatedMeetings,
    RelationKey,
};
use crate::people::MeRepo;

/// The most related meetings listed besides "Previous in series".
pub const RELATED_LIMIT: usize = 5;

/// Finds the meetings related to a meeting.
pub struct RelatedService {
    repo: Arc<dyn RelatedMeetingRepo>,
    me: Arc<dyn MeRepo>,
}

impl RelatedService {
    /// A service over the related-meetings repository and the "Me" person.
    pub fn new(repo: Arc<dyn RelatedMeetingRepo>, me: Arc<dyn MeRepo>) -> Self {
        Self { repo, me }
    }

    /// The meetings related to `meeting`, with "Previous in series".
    pub async fn related(&self, meeting: &MeetingId) -> Result<RelatedMeetings, LinkError> {
        let key = self
            .repo
            .key(meeting)
            .await?
            .ok_or_else(|| LinkError::MeetingNotFound(meeting.clone()))?;
        let candidates = self.repo.candidates(&key).await?;
        let me = self.me.me().await?.map(|person| person.id);
        Ok(relate(&key, candidates, me.as_ref()))
    }
}

/// Judges `candidates` (newest first) against `key`: the latest one before
/// the meeting is "Previous in series", the rest are listed newest first.
pub(super) fn relate(
    key: &RelationKey,
    candidates: Vec<RelatedCandidate>,
    me: Option<&PersonId>,
) -> RelatedMeetings {
    let mut related: Vec<RelatedMeeting> = candidates
        .into_iter()
        .filter_map(|candidate| {
            let by = relation(key, &candidate, me)?;
            Some(RelatedMeeting {
                id: candidate.id,
                title: candidate.title,
                happened_at: candidate.happened_at,
                by,
            })
        })
        .collect();
    let previous = related
        .iter()
        .position(|meeting| meeting.happened_at < key.happened_at)
        .map(|index| related.remove(index));
    related.truncate(RELATED_LIMIT);
    RelatedMeetings {
        previous,
        others: related,
    }
}

/// Why `candidate` is related to the key's meeting, if it is.
fn relation(
    key: &RelationKey,
    candidate: &RelatedCandidate,
    me: Option<&PersonId>,
) -> Option<RelatedBy> {
    if key.series_id.is_some() && candidate.series_id == key.series_id {
        return Some(RelatedBy::Series);
    }
    let people = others(&key.attendees, me);
    let same_people = !people.is_empty() && people == others(&candidate.attendees, me);
    (candidate.title == key.title && same_people).then_some(RelatedBy::TitleAndPeople)
}

/// `attendees` without the user.
fn others<'a>(attendees: &'a BTreeSet<PersonId>, me: Option<&PersonId>) -> BTreeSet<&'a PersonId> {
    attendees
        .iter()
        .filter(|person| Some(*person) != me)
        .collect()
}
