//! [`SegmentEditor`] over the in-memory [`FakeSegmentRepo`]. The fake has
//! no trash, so every meeting it knows is outside it.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_db::DbError;
use deino_domain::{MeetingId, SegmentId};

use super::fake::edit;
use super::{FakeSegmentRepo, Segment, SegmentEditor, SegmentText, TextEdit};
use crate::testing::lock;

impl FakeSegmentRepo {
    /// Names a meeting for [`SegmentEditor::matching`]; an unnamed meeting
    /// is titled by its ID.
    pub fn title_meeting(&self, meeting: &MeetingId, title: &str) {
        lock(&self.titles).push((meeting.clone(), title.to_owned()));
    }

    fn title_of(&self, meeting: &MeetingId) -> String {
        lock(&self.titles)
            .iter()
            .find(|(id, _)| id == meeting)
            .map_or_else(|| meeting.to_string(), |(_, title)| title.clone())
    }
}

/// ASCII case-insensitive containment, as SQLite's `LIKE` compares.
fn contains_ignoring_ascii_case(text: &str, needle: &str) -> bool {
    text.to_ascii_lowercase()
        .contains(&needle.to_ascii_lowercase())
}

#[async_trait]
impl SegmentEditor for FakeSegmentRepo {
    async fn segment(&self, id: &SegmentId) -> Result<Option<Segment>, DbError> {
        Ok(lock(&self.segments)
            .iter()
            .find(|segment| segment.id == *id)
            .cloned())
    }

    async fn edit_many(&self, edits: &[TextEdit], at: DateTime<Utc>) -> Result<(), DbError> {
        for change in edits {
            self.update(&change.id, |segment| edit(segment, &change.text, at));
        }
        Ok(())
    }

    async fn matching(
        &self,
        meeting: Option<&MeetingId>,
        needle: &str,
    ) -> Result<Vec<SegmentText>, DbError> {
        let mut found: Vec<Segment> = lock(&self.segments)
            .iter()
            .filter(|segment| !segment.is_deleted)
            .filter(|segment| meeting.is_none_or(|id| segment.meeting_id == *id))
            .filter(|segment| contains_ignoring_ascii_case(&segment.text, needle))
            .cloned()
            .collect();
        found.sort_by(|a, b| (&a.meeting_id, a.start, a.id).cmp(&(&b.meeting_id, b.start, b.id)));
        Ok(found
            .into_iter()
            .map(|segment| SegmentText {
                meeting_title: self.title_of(&segment.meeting_id),
                id: segment.id,
                meeting_id: segment.meeting_id,
                text: segment.text,
            })
            .collect())
    }
}
