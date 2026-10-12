//! The template gallery: twelve ready-made insights to start from
//! (Circleback §10.3's recipes), each a complete [`InsightInput`] the user
//! can create as is or edit first.
//!
//! Descriptions follow Circleback's advice: say what to extract or write,
//! and show the shape of a good answer.

use deino_domain::{Condition, ConditionGroups};
use serde::Serialize;

use super::{ApplyWhen, Cardinality, FieldKind, InsightField, InsightInput};

mod sales;

use sales::{
    competitor_mentions, customer_feedback, deal_summary, discovery_scorecard, health_signals,
    objections,
};

/// A gallery entry.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct InsightTemplate {
    /// A stable key (`discovery_scorecard`).
    pub key: String,
    /// One line for the gallery card.
    pub summary: String,
    /// The insight it creates.
    pub input: InsightInput,
}

/// Every template, in gallery order.
pub fn gallery() -> Vec<InsightTemplate> {
    vec![
        discovery_scorecard(),
        customer_feedback(),
        competitor_mentions(),
        health_signals(),
        executive_summary(),
        interview_debrief(),
        questions_asked(),
        objections(),
        risks(),
        decisions(),
        follow_up_email(),
        deal_summary(),
    ]
}

/// The template with `key`.
pub fn template(key: &str) -> Option<InsightTemplate> {
    gallery().into_iter().find(|template| template.key == key)
}

/// The parts of one template, before it becomes an [`InsightTemplate`].
struct Spec<'a> {
    key: &'a str,
    name: &'a str,
    summary: &'a str,
    description: &'a str,
    fields: Vec<InsightField>,
    cardinality: Cardinality,
    apply_when: ApplyWhen,
}

impl Spec<'_> {
    fn build(self) -> InsightTemplate {
        InsightTemplate {
            key: self.key.to_owned(),
            summary: self.summary.to_owned(),
            input: InsightInput {
                name: self.name.to_owned(),
                description: self.description.to_owned(),
                fields: self.fields,
                cardinality: self.cardinality,
                apply_when: self.apply_when,
            },
        }
    }
}

fn field(name: &str, kind: FieldKind) -> InsightField {
    InsightField {
        name: name.to_owned(),
        kind,
    }
}

fn choice(name: &str, options: &[&str]) -> InsightField {
    let options = options.iter().map(|option| (*option).to_owned()).collect();
    field(name, FieldKind::Enum { options })
}

const ONE: Cardinality = Cardinality::Fixed { count: 1 };

/// Every meeting.
fn always() -> ApplyWhen {
    ApplyWhen::Automatic {
        conditions: ConditionGroups::every_meeting(),
    }
}

/// Meetings with someone from outside the user's organisation.
fn external() -> ApplyWhen {
    ApplyWhen::Automatic {
        conditions: ConditionGroups(vec![vec![Condition::External { external: true }]]),
    }
}

fn executive_summary() -> InsightTemplate {
    Spec {
        key: "executive_summary",
        name: "Executive summary",
        summary: "Three to five sentences an executive can read in thirty seconds.",
        description: "Write a three to five sentence summary of the meeting for an executive who was not there: the purpose, what was decided, what is at risk, and what happens next. Plain prose, no bullet points.",
        fields: Vec::new(),
        cardinality: ONE,
        apply_when: ApplyWhen::Manual,
    }
    .build()
}

fn interview_debrief() -> InsightTemplate {
    Spec {
        key: "interview_debrief",
        name: "Interview debrief",
        summary: "Strengths, concerns and a hiring recommendation for a candidate.",
        description: "Debrief this job interview: the candidate, the role, their strongest evidence, the concerns it raised, a recommendation, and a 1–5 score (5: an outstanding hire). Judge only from what was said.",
        fields: vec![
            field("Candidate", FieldKind::Text),
            field("Role", FieldKind::Text),
            field("Strengths", FieldKind::Text),
            field("Concerns", FieldKind::Text),
            choice("Recommendation", &["Strong yes", "Yes", "No", "Strong no"]),
            field("Score", FieldKind::Number),
        ],
        cardinality: ONE,
        apply_when: ApplyWhen::Automatic {
            conditions: ConditionGroups(vec![vec![Condition::TitleContains {
                text: "interview".to_owned(),
            }]]),
        },
    }
    .build()
}

fn questions_asked() -> InsightTemplate {
    Spec {
        key: "questions_asked",
        name: "Questions asked",
        summary: "Every substantive question, one answer each.",
        description: "List every substantive question someone asked, one answer per question, worded as a clear question. Skip small talk and questions that were only rhetorical. Example: \"Does the mobile app work offline?\"",
        fields: Vec::new(),
        cardinality: Cardinality::Auto,
        apply_when: ApplyWhen::Manual,
    }
    .build()
}

fn risks() -> InsightTemplate {
    Spec {
        key: "risks",
        name: "Risks",
        summary: "Risks to the work, with severity and an owner.",
        description: "List the risks raised in the meeting: anything that could delay, derail or hurt the work. One answer per risk, with how severe the speakers made it sound and who is watching it, if anyone.",
        fields: vec![
            field("Risk", FieldKind::Text),
            choice("Severity", &["Low", "Medium", "High"]),
            field("Owner", FieldKind::Text),
        ],
        cardinality: Cardinality::Auto,
        apply_when: always(),
    }
    .build()
}

fn decisions() -> InsightTemplate {
    Spec {
        key: "decisions",
        name: "Decisions",
        summary: "What was decided, one decision per answer.",
        description: "List the decisions the meeting made: things the participants agreed on or settled, not ideas still open. One answer per decision, as one clear sentence. Example: \"The pilot covers one region for 60 days.\"",
        fields: Vec::new(),
        cardinality: Cardinality::Auto,
        apply_when: always(),
    }
    .build()
}

fn follow_up_email() -> InsightTemplate {
    Spec {
        key: "follow_up_email",
        name: "Follow-up email",
        summary: "A ready-to-send recap email to the other attendees.",
        description: "Write a short follow-up email from me to the other attendees: thank them, recap what was agreed, list the next steps with owners and dates, and close warmly. Plain text with a greeting and a sign-off; no subject line.",
        fields: Vec::new(),
        cardinality: ONE,
        apply_when: ApplyWhen::Manual,
    }
    .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_gallery_has_twelve_valid_templates_with_unique_keys_and_names() {
        let templates = gallery();

        assert_eq!(templates.len(), 12);
        for template in &templates {
            let valid = template.input.clone().validated().unwrap();
            assert_eq!(
                valid, template.input,
                "{} is already canonical",
                template.key
            );
            assert_eq!(super::template(&template.key).as_ref(), Some(template));
        }
        let mut keys: Vec<&str> = templates.iter().map(|t| t.key.as_str()).collect();
        let mut names: Vec<&str> = templates.iter().map(|t| t.input.name.as_str()).collect();
        keys.dedup();
        names.sort_unstable();
        names.dedup();
        assert_eq!((keys.len(), names.len()), (12, 12));
        assert_eq!(template("nope"), None);
    }
}
