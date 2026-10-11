//! The sample's AI notes: one enhanced-notes version per fixture with its
//! decisions, and its commitments citing the moment each was made.

use deino_doc::{Flavor, from_markdown, to_markdown};
use deino_testkit::fixture::{FixtureMeeting, fixture_uuid, segment_key};

use super::SampleError;
use crate::enhance::{EnhancedNotesWriter, NewVersion, SqliteEnhancedNotesRepo};

/// The model the sample's notes say wrote them.
const SAMPLE_MODEL: &str = "sample:fixtures";

/// Stores `meeting`'s sample AI notes as its first version.
pub(super) async fn enhanced_notes(
    repo: &SqliteEnhancedNotesRepo,
    meeting: &FixtureMeeting,
) -> Result<(), SampleError> {
    let doc = from_markdown(&lossless(meeting))?;
    let version = NewVersion {
        meeting_id: meeting.id.clone(),
        markdown: to_markdown(&doc, Flavor::Readable),
        doc,
        template_id: None,
        model: SAMPLE_MODEL.to_owned(),
        basis: None,
        is_user_edited: false,
        at: meeting.started_at,
    };
    repo.insert(&version).await?;
    Ok(())
}

/// The notes as Lossless Markdown, citations as inline tags.
fn lossless(meeting: &FixtureMeeting) -> String {
    let decisions: Vec<String> = meeting
        .ground_truth
        .decisions
        .iter()
        .map(|decision| format!("- {decision}"))
        .collect();
    let commitments: Vec<String> = meeting
        .ground_truth
        .action_items
        .iter()
        .map(|item| format!("- {}{}", item.title, citation(meeting, &item.evidence)))
        .collect();
    format!(
        "## Decisions\n\n{}\n\n## Commitments\n\n{}\n",
        decisions.join("\n"),
        commitments.join("\n")
    )
}

/// A citation of the segment that says `evidence`, or nothing.
fn citation(meeting: &FixtureMeeting, evidence: &str) -> String {
    let found = meeting
        .segments
        .iter()
        .enumerate()
        .find(|(_, segment)| segment.text.contains(evidence));
    found.map_or_else(String::new, |(index, segment)| {
        format!(
            r#" <citation data-segment-id="{}" data-start-ms="{}" />"#,
            fixture_uuid(&segment_key(meeting.id.as_str(), index)),
            segment.start_ms.get()
        )
    })
}
