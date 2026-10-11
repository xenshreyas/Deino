//! Structured answers of the extraction routes: the JSON shape each prompt
//! in [`crate::prompts`] asks for, as typed values for
//! [`Gateway::structured`](crate::Gateway::structured).
//!
//! These are wire types: they hold what the model said, unvalidated. The
//! feature that asked resolves them against its data (AI-05 resolves names
//! to people, phrases to dates and line ids to transcript segments in
//! `deino_core::action_items`), and the eval harness scores them against the
//! fixtures' ground truth. Field docs become the schema's descriptions.

use chrono::{DateTime, FixedOffset};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// How the action item prompt names the user's own voice; the prompt asks
/// for "me" as the assignee when this speaker commits.
pub const ME_SPEAKER: &str = "Me";

/// The action item prompt's meeting block: the title, then the local date
/// with its weekday and offset, which due phrases are counted from
/// ("Thursday 8 October 2026, 08:30 (UTC-07:00)").
pub fn meeting_header(title: &str, held: DateTime<FixedOffset>) -> String {
    format!(
        "Title: {title}\nDate: {}",
        held.format("%A %-d %B %Y, %H:%M (UTC%:z)")
    )
}

/// One attendee line: `- Maya Chen, Staff Engineer`, with `(me)` after the
/// user's name.
pub fn attendee_line(name: &str, is_me: bool, title: Option<&str>) -> String {
    let mine = if is_me { " (me)" } else { "" };
    let title = title.map(|title| format!(", {title}")).unwrap_or_default();
    format!("- {name}{mine}{title}")
}

/// One numbered transcript line, `[s12 03:41] Maya Chen: …`: the id the
/// model cites as `evidence_segment_id`, then minutes and seconds.
pub fn transcript_line(index: usize, start_ms: u32, speaker: &str, text: &str) -> String {
    let seconds = start_ms / 1_000;
    format!(
        "[s{index} {:02}:{:02}] {speaker}: {text}",
        seconds / 60,
        seconds % 60
    )
}

/// `extract.action_items` (AI-05): every commitment made in a meeting.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ActionItemsAnswer {
    /// One entry per commitment; empty when nobody committed to anything.
    pub items: Vec<ExtractedActionItem>,
}

/// One commitment, as the model read it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ExtractedActionItem {
    /// An imperative instruction under 80 characters, starting with a verb.
    pub title: String,
    /// One or two sentences of context.
    #[serde(default)]
    pub description: String,
    /// Who committed: a name as written in the attendees or the transcript,
    /// "me" for the user, or null when nobody took it on.
    #[serde(default)]
    pub assignee: Option<String>,
    /// The words that set the deadline, as said ("by Friday"); null if none.
    #[serde(default)]
    pub due_phrase: Option<String>,
    /// The deadline as a date (YYYY-MM-DD) from the meeting's date; null if none.
    #[serde(default)]
    pub due: Option<String>,
    /// How urgent the speakers made it sound; null when they did not say.
    #[serde(default)]
    pub priority: Option<ExtractedPriority>,
    /// The words of the transcript line where the commitment is made, verbatim.
    #[serde(default)]
    pub evidence_quote: String,
    /// The id of that transcript line, for example "s12".
    #[serde(default)]
    pub evidence_segment_id: Option<String>,
}

/// The priority words the action item prompt allows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ExtractedPriority {
    /// The speakers said it can wait.
    Low,
    /// Neither urgent nor deferrable (models rarely say it).
    Medium,
    /// The speakers called it urgent or blocking.
    High,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json_schema_for;

    #[test]
    fn a_sparse_answer_fills_every_optional_field_with_its_default() {
        let answer: ActionItemsAnswer =
            serde_json::from_str(r#"{"items":[{"title":"Send the deck"}]}"#).unwrap();

        assert_eq!(
            answer.items,
            [ExtractedActionItem {
                title: "Send the deck".into(),
                ..ExtractedActionItem::default()
            }]
        );
    }

    #[test]
    fn prompt_lines_name_the_meeting_people_and_numbered_moments() {
        let offset = FixedOffset::west_opt(7 * 3_600).unwrap();
        let held = chrono::TimeZone::with_ymd_and_hms(&offset, 2026, 10, 8, 8, 31, 4).unwrap();

        assert_eq!(
            meeting_header("Eng standup", held),
            "Title: Eng standup\nDate: Thursday 8 October 2026, 08:31 (UTC-07:00)"
        );
        assert_eq!(
            attendee_line("Shreyas Rao", true, Some("CTO")),
            "- Shreyas Rao (me), CTO"
        );
        assert_eq!(attendee_line("Owen Hart", false, None), "- Owen Hart");
        assert_eq!(
            transcript_line(12, 221_900, "Maya Chen", "On it."),
            "[s12 03:41] Maya Chen: On it."
        );
    }

    #[test]
    fn the_schema_requires_only_the_title_and_documents_each_field() {
        let schema = json_schema_for::<ActionItemsAnswer>();
        let item = &schema["properties"]["items"]["items"];

        assert_eq!(item["required"], serde_json::json!(["title"]));
        let due = item["properties"]["due"]["description"].as_str().unwrap();
        assert!(due.contains("YYYY-MM-DD"), "{due}");
        let priorities: Vec<&serde_json::Value> =
            item["properties"]["priority"]["anyOf"][0]["oneOf"]
                .as_array()
                .unwrap()
                .iter()
                .map(|choice| &choice["const"])
                .collect();
        assert_eq!(priorities, ["low", "medium", "high"]);
    }
}
