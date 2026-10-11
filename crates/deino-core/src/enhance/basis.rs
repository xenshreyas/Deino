//! [`EnhanceBasis`]: a fingerprint of what a version was written from, and
//! the regenerate prompts it yields.
//!
//! Granola offers to regenerate when attendees change, transcript lines are
//! deleted, the user's notes are edited or more transcript is captured
//! (`docs/research/granola.md` §3.3). Each version stores the basis of its
//! inputs (`enhanced_notes.basis_json`); comparing it with the current
//! inputs says which of those happened, without keeping a copy of the
//! inputs themselves.

use chrono::{DateTime, Utc};
use deino_domain::{Millis, PersonId};
use serde::{Deserialize, Serialize};

use super::RegenerateReason;
use super::material::Material;
use super::transcript::TranscriptLine;

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0100_0000_01b3;

/// The inputs a version was written from, in brief.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnhanceBasis {
    /// The attendees, sorted.
    pub attendees: Vec<PersonId>,
    /// How many transcript segments there were.
    pub segments: u32,
    /// A digest of those segments' IDs, in time order.
    pub segments_digest: String,
    /// Where the transcript ended.
    pub transcript_end: Millis,
    /// When a transcript segment was last edited, if ever.
    pub transcript_edited_at: Option<DateTime<Utc>>,
    /// When the user's notes were last saved, if ever.
    pub notes_updated_at: Option<DateTime<Utc>>,
}

impl EnhanceBasis {
    /// The basis of `material`.
    pub fn of(material: &Material) -> Self {
        let lines = &material.transcript;
        Self {
            attendees: sorted(&material.attendee_ids),
            segments: u32::try_from(lines.len()).unwrap_or(u32::MAX),
            segments_digest: digest(lines),
            transcript_end: lines.iter().map(|line| line.end).max().unwrap_or_default(),
            transcript_edited_at: material.transcript_edited_at,
            notes_updated_at: material.notes.as_ref().map(|notes| notes.updated_at),
        }
    }

    /// What changed between this basis and `current`, in the order the
    /// banner lists them.
    pub fn reasons(&self, current: &Material) -> Vec<RegenerateReason> {
        let now = Self::of(current);
        let mut reasons = Vec::new();
        if now.attendees != self.attendees {
            reasons.push(RegenerateReason::AttendeesChanged);
        }
        if self.transcript_changed(&now, &current.transcript) {
            reasons.push(RegenerateReason::TranscriptEdited);
        }
        if now.notes_updated_at > self.notes_updated_at {
            reasons.push(RegenerateReason::NotesEdited);
        }
        if now.transcript_end > self.transcript_end {
            reasons.push(RegenerateReason::AudioAppended);
        }
        reasons
    }

    /// Edited, or segments the version saw are gone or replaced. Appended
    /// segments start later, so the version's segments are still the first
    /// `segments` ones unless something was removed.
    fn transcript_changed(&self, now: &Self, lines: &[TranscriptLine]) -> bool {
        if now.transcript_edited_at != self.transcript_edited_at {
            return true;
        }
        let seen = usize::try_from(self.segments).unwrap_or(usize::MAX);
        match lines.get(..seen) {
            Some(prefix) => digest(prefix) != self.segments_digest,
            None => true,
        }
    }
}

fn sorted(ids: &[PersonId]) -> Vec<PersonId> {
    let mut ids = ids.to_vec();
    ids.sort();
    ids.dedup();
    ids
}

/// FNV-1a over the segment IDs, as 16 hex digits.
fn digest(lines: &[TranscriptLine]) -> String {
    let mut hash = FNV_OFFSET;
    for line in lines {
        for byte in line.segment_id.to_string().bytes().chain([b'\n']) {
            hash = (hash ^ u64::from(byte)).wrapping_mul(FNV_PRIME);
        }
    }
    format!("{hash:016x}")
}

#[cfg(test)]
mod basis_tests;
