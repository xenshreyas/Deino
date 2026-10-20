//! [`Outbox`]: finished speaker and roster spans on their way to the sink.
//! A failed save keeps them for the next flush.

use deino_domain::{MeetingId, TagSpan};

use crate::ports::TagSink;

/// The log target.
const TARGET: &str = "deino_speaker_tags";

/// Spans not saved yet, and how many were.
#[derive(Debug, Default)]
pub(crate) struct Outbox {
    spans: Vec<TagSpan>,
    roster: Vec<TagSpan>,
    /// Speaker spans saved so far.
    pub(crate) saved: usize,
    /// Roster spans saved so far.
    pub(crate) saved_roster: usize,
}

impl Outbox {
    /// Adds `spans` and `roster`, then saves everything waiting.
    pub(crate) fn flush(
        &mut self,
        sink: &dyn TagSink,
        meeting: &MeetingId,
        (spans, roster): (Vec<TagSpan>, Vec<TagSpan>),
    ) {
        self.spans.extend(spans);
        self.roster.extend(roster);
        if !self.spans.is_empty() {
            match sink.save(meeting, &self.spans) {
                Ok(()) => self.saved += std::mem::take(&mut self.spans).len(),
                Err(error) => kept(meeting, self.spans.len(), "speaker tags", &error),
            }
        }
        if !self.roster.is_empty() {
            match sink.save_roster(meeting, &self.roster) {
                Ok(()) => self.saved_roster += std::mem::take(&mut self.roster).len(),
                Err(error) => kept(meeting, self.roster.len(), "roster spans", &error),
            }
        }
    }
}

fn kept(meeting: &MeetingId, count: usize, what: &str, error: &dyn std::error::Error) {
    log::warn!(target: TARGET, "meeting {meeting}: {count} {what} kept for the next save: {error}");
}
