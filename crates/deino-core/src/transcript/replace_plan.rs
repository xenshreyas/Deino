//! [`ReplacePlan`]: what a find and replace does to a set of lines, worked
//! out once so the preview and the edit can never disagree.

use deino_domain::MeetingId;

use super::pattern::Pattern;
use super::{MeetingMatches, ReplaceOutcome, ReplacePreview};
use crate::segments::{SegmentText, TextEdit};

/// The edits a find and replace makes, and its counts.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct ReplacePlan {
    /// One edit per line that changes.
    pub(super) edits: Vec<TextEdit>,
    /// The counts the user confirms.
    pub(super) preview: ReplacePreview,
}

impl ReplacePlan {
    /// Plans replacing `pattern` with `with` in `lines` (newest meeting
    /// first). A line the replacement would leave blank is skipped: blanking
    /// a line is a delete, which the user asks for on the line itself.
    pub(super) fn new(lines: Vec<SegmentText>, pattern: Pattern<'_>, with: &str) -> Self {
        let mut plan = Self::default();
        for line in lines {
            let (text, count) = pattern.replace(&line.text, with);
            if count == 0 || text.trim().is_empty() {
                continue;
            }
            plan.count(&line.meeting_id, &line.meeting_title, count);
            plan.edits.push(TextEdit { id: line.id, text });
        }
        plan
    }

    /// The meetings it changes.
    pub(super) fn meetings(&self) -> Vec<MeetingId> {
        let preview = &self.preview.meetings;
        preview.iter().map(|one| one.meeting_id.clone()).collect()
    }

    /// What applying it changed.
    pub(super) fn outcome(&self) -> ReplaceOutcome {
        ReplaceOutcome {
            matches: self.preview.matches,
            segments: self.preview.segments,
            meetings: saturating(self.preview.meetings.len()),
        }
    }

    /// Counts `matches` in one more line of `meeting`.
    fn count(&mut self, meeting: &MeetingId, title: &str, matches: usize) {
        let matches = saturating(matches);
        let preview = &mut self.preview;
        preview.matches = preview.matches.saturating_add(matches);
        preview.segments = preview.segments.saturating_add(1);
        match preview
            .meetings
            .iter_mut()
            .find(|one| one.meeting_id == *meeting)
        {
            Some(one) => one.matches = one.matches.saturating_add(matches),
            None => preview.meetings.push(MeetingMatches {
                meeting_id: meeting.clone(),
                title: title.to_owned(),
                matches,
            }),
        }
    }
}

/// A count as `u32`, saturating (no transcript comes near).
fn saturating(count: usize) -> u32 {
    u32::try_from(count).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use deino_domain::{FakeClock, SegmentId};
    use deino_testkit::at;

    use super::*;

    fn line(meeting: &str, text: &str) -> SegmentText {
        let clock = FakeClock::new(at("2026-10-08T09:00:00Z"));
        SegmentText {
            id: SegmentId::generate(&clock),
            meeting_id: MeetingId::new(meeting),
            meeting_title: format!("{meeting} title"),
            text: text.into(),
        }
    }

    #[test]
    fn counts_matches_lines_and_meetings_in_the_order_given() {
        let lines = vec![
            line("b", "Dino said dino."),
            line("a", "No match here."),
            line("a", "DINO again"),
        ];

        let plan = ReplacePlan::new(lines, Pattern::new("dino", false), "Deino");

        assert_eq!(plan.preview.matches, 3);
        assert_eq!(plan.preview.segments, 2);
        let meetings: Vec<(&str, u32)> = plan
            .preview
            .meetings
            .iter()
            .map(|one| (one.meeting_id.as_str(), one.matches))
            .collect();
        assert_eq!(meetings, [("b", 2), ("a", 1)]);
        assert_eq!(plan.edits[0].text, "Deino said Deino.");
        assert_eq!(plan.outcome().meetings, 2);
    }

    #[test]
    fn a_line_left_blank_is_not_changed() {
        let plan = ReplacePlan::new(vec![line("a", " um ")], Pattern::new("um", false), "");

        assert!(plan.edits.is_empty());
        assert_eq!(plan.preview, ReplacePreview::default());
    }
}
