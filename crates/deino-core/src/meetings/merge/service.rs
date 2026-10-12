//! [`MeetingMergeService`]: merges two or more meetings into the earliest
//! (Meetily community request #393).

use std::sync::Arc;

use deino_domain::{Clock, MeetingId};

use super::{MeetingMergeRepo, MergeOutcome, MergePlan, PartNotes, combine, timeline};
use crate::meetings::{MeetingError, MeetingWork};
use crate::notes::NotesRepo;

/// What a merge works through.
pub struct MergeDeps {
    /// The meetings.
    pub repo: Arc<dyn MeetingMergeRepo>,
    /// Their notes, which are combined.
    pub notes: Arc<dyn NotesRepo>,
    /// When the merge happened.
    pub clock: Arc<dyn Clock>,
    /// The pipeline, which re-indexes the merged meeting.
    pub work: Arc<dyn MeetingWork>,
}

/// Merges meetings.
pub struct MeetingMergeService {
    deps: MergeDeps,
}

impl MeetingMergeService {
    /// The service over its ports.
    pub fn new(deps: MergeDeps) -> Self {
        Self { deps }
    }

    /// Merges `ids` into the earliest of them: transcripts concatenated with
    /// time offsets, attendees united with the stronger role, tags, folders,
    /// action items and notes combined, and each absorbed recording kept
    /// reachable as a part. The absorbed meetings are deleted and the merged
    /// one re-indexed. Only finished meetings merge, so none has a running
    /// pipeline stage to cancel.
    pub async fn merge(&self, ids: &[MeetingId]) -> Result<MergeOutcome, MeetingError> {
        let (plan, absorbed) = self.plan(ids).await?;
        self.deps.repo.apply_merge(&plan).await?;
        log::info!(target: "meetings", "merged {} meetings into {}", plan.parts.len(), plan.target);
        self.deps.work.reindex(&plan.target).await;
        Ok(MergeOutcome {
            absorbed,
            meeting_id: plan.target,
        })
    }

    /// The merge of `ids`, and the meetings it absorbs in timeline order.
    async fn plan(&self, ids: &[MeetingId]) -> Result<(MergePlan, Vec<MeetingId>), MeetingError> {
        let mut unique = ids.to_vec();
        unique.sort();
        unique.dedup();
        let candidates = self.deps.repo.candidates(&unique).await?;
        if let Some(missing) = unique
            .iter()
            .find(|id| !candidates.iter().any(|c| &c.id == *id))
        {
            return Err(MeetingError::NotFound(missing.clone()));
        }
        let titles: Vec<(MeetingId, String)> = candidates
            .iter()
            .map(|c| (c.id.clone(), c.title.clone()))
            .collect();
        let timeline = timeline(candidates)?;
        let merged_at = self.deps.clock.now();
        let mut ordered = vec![timeline.target.clone()];
        ordered.extend(timeline.parts.iter().map(|part| part.source.clone()));
        let notes = self.part_notes(&ordered, &titles).await?;
        let plan = MergePlan {
            notes: combine(&timeline.target, &notes, merged_at),
            target: timeline.target,
            parts: timeline.parts,
            length: timeline.length,
            merged_at,
        };
        Ok((plan, ordered.split_off(1)))
    }

    /// Each meeting's notes with its title, in timeline order.
    async fn part_notes(
        &self,
        ordered: &[MeetingId],
        titles: &[(MeetingId, String)],
    ) -> Result<Vec<PartNotes>, MeetingError> {
        let mut parts = Vec::with_capacity(ordered.len());
        for id in ordered {
            let title = titles
                .iter()
                .find(|(known, _)| known == id)
                .map(|(_, t)| t.clone());
            parts.push(PartNotes {
                title: title.unwrap_or_default(),
                notes: self.deps.notes.get(id).await?,
            });
        }
        Ok(parts)
    }
}
