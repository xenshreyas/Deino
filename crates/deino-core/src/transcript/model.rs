//! The transcript read model: segments plus the speakers who said them.

use deino_domain::{PaletteColor, PersonId, SpeakerId};
use serde::Serialize;

use crate::segments::Segment;

/// One meeting's saved transcript, as the transcript view shows it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Transcript {
    /// The segments that are not deleted, in spoken order (start, then ID).
    pub segments: Vec<Segment>,
    /// The meeting's speakers, by label; a segment's `speakerId` names one.
    pub speakers: Vec<TranscriptSpeaker>,
}

/// A voice of the transcript with the name the view shows for it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptSpeaker {
    /// The speaker's ID.
    pub id: SpeakerId,
    /// The identified person's name, else the speaker's label ("Speaker 2").
    pub name: String,
    /// The person the voice belongs to, once identified (the merge target
    /// for a merged person).
    pub person_id: Option<PersonId>,
    /// True for the user's own voice: its bubbles sit on the right.
    pub is_me: bool,
    /// The speaker's palette slot for its name and avatar.
    pub color: PaletteColor,
}
