//! The transcript read model.

use serde::Serialize;

use crate::segments::Segment;

/// One meeting's saved transcript, as the transcript view shows it. Who
/// said each segment (names, colours, "Is this …?") comes from the
/// meeting's participants (`speakers`, CAP-04), by `Segment::speaker_id`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Transcript {
    /// The segments that are not deleted, in spoken order (start, then ID).
    pub segments: Vec<Segment>,
}
