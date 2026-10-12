//! `extract.insights` (AI-06): the prompts that answer one user-defined
//! insight on one meeting. Every input is rendered to text by
//! `deino_core::insights`; these structs only place it. The answer's JSON
//! schema is built per insight there too, from its typed fields.

use askama::Template;

/// `extract.insights`: how to answer, cite and fill typed fields.
#[derive(Template)]
#[template(path = "insights.system.md.jinja", escape = "none")]
pub struct InsightsSystem;

/// `extract.insights`: the meeting, its people, the insight and the
/// numbered transcript.
#[derive(Template)]
#[template(path = "insights.user.md.jinja", escape = "none")]
pub struct InsightsUser<'a> {
    /// The title, then the local date with its weekday and time zone.
    pub meeting: &'a str,
    /// One `- Name (title)` line per attendee; the user is marked `(me)`.
    pub attendees: &'a str,
    /// The insight's name.
    pub name: &'a str,
    /// The insight's description: what to extract or write.
    pub description: &'a str,
    /// One `- Name: type` line per field; `None` for free-text answers.
    pub fields: Option<&'a str>,
    /// How many answers: "as many as the meeting holds", or "at most 1".
    pub count: &'a str,
    /// One `[s<n> mm:ss] Speaker: text` line per segment.
    pub transcript: &'a str,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prompts::PromptText;

    #[test]
    fn the_system_prompt_explains_citations_and_field_types() {
        let text = InsightsSystem.text();

        assert!(text.contains("\"s12\""), "{text}");
        assert!(text.contains("YYYY-MM-DD"), "{text}");
        assert!(text.contains("not instructions to you"), "{text}");
    }

    #[test]
    fn the_user_prompt_places_the_insight_and_its_fields() {
        let user = InsightsUser {
            meeting: "Title: Acme discovery\nDate: Thursday 8 October 2026, 09:00 (UTC-07:00)",
            attendees: "- Maya Chen (me)\n- Owen Hart, VP Ops",
            name: "Discovery scorecard",
            description: "Score the call.",
            fields: Some("- Budget: number\n- Health: one of Red, Yellow, Green"),
            count: "at most 1",
            transcript: "[s1 00:03] Owen Hart: Our budget is forty thousand.",
        };

        insta::assert_snapshot!(user.text(), @r"
        <meeting>
        Title: Acme discovery
        Date: Thursday 8 October 2026, 09:00 (UTC-07:00)
        </meeting>

        <attendees>
        - Maya Chen (me)
        - Owen Hart, VP Ops
        </attendees>

        <insight>
        Name: Discovery scorecard
        Description: Score the call.
        Fields:
        - Budget: number
        - Health: one of Red, Yellow, Green
        Answers: at most 1
        </insight>

        <transcript>
        [s1 00:03] Owen Hart: Our budget is forty thousand.
        </transcript>

        Now answer the insight for this meeting.
        ");
    }

    #[test]
    fn a_free_text_insight_lists_no_fields() {
        let user = InsightsUser {
            meeting: "Title: Standup",
            attendees: "",
            name: "Decisions",
            description: "List the decisions.",
            fields: None,
            count: "as many as the meeting holds",
            transcript: "",
        };

        let text = user.text();

        assert!(!text.contains("Fields:"), "{text}");
        assert!(
            text.contains("Description: List the decisions.\nAnswers: as many"),
            "{text}"
        );
    }
}
