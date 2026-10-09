//! The auto-tagger's prompt and answer: pure, so tests check both without
//! a model.
//!
//! The prompt lists every tag the auto-tagger may apply with its
//! description and up to [`MAX_EXAMPLES`] recent meetings that carry it
//! (Circleback's approach: descriptions plus consistent manual tagging),
//! then the meeting's title and summary, or its transcript's opening when
//! there is no summary yet. The answer names tags; only names of offered
//! tags count, so the model can never create one.

use deino_domain::TagId;
use deino_llm::Prompt;
use deino_llm::prompts::{PromptText, TagChoice, TagExample, TaggingSystem, TaggingUser};
use schemars::JsonSchema;
use serde::Deserialize;

use super::{MeetingDigest, Tag};

/// Recent example meetings shown per tag.
pub const MAX_EXAMPLES: u32 = 3;
/// Characters of an example's summary shown in the prompt.
const EXAMPLE_SUMMARY_CHARS: usize = 300;
/// Characters of the meeting's summary or transcript shown in the prompt.
pub const CONTENT_CHARS: usize = 6_000;
/// Room for the answer: a short list of names.
const ANSWER_TOKENS: u32 = 300;

/// The structured answer the `tagging` route returns.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, JsonSchema)]
pub struct TaggingAnswer {
    /// The exact names of the tags that fit the meeting; empty when none does.
    pub tags: Vec<String>,
}

/// A tag the auto-tagger may apply, with its recent examples.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Candidate {
    /// The tag.
    pub tag: Tag,
    /// Recent meetings carrying it, newest first.
    pub examples: Vec<MeetingDigest>,
}

/// What the prompt says about the meeting being tagged.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MeetingContent {
    /// Its title.
    pub title: String,
    /// Its summary, or the opening of its transcript.
    pub text: String,
    /// True when `text` is a summary.
    pub is_summary: bool,
}

/// The `tagging` prompt for `meeting` over `candidates`.
pub fn tagging_prompt(candidates: &[Candidate], meeting: &MeetingContent) -> Prompt {
    let summaries: Vec<Vec<Option<String>>> = candidates
        .iter()
        .map(|candidate| candidate.examples.iter().map(example_summary).collect())
        .collect();
    let tags: Vec<TagChoice<'_>> = candidates
        .iter()
        .zip(&summaries)
        .map(|(candidate, summaries)| choice(candidate, summaries))
        .collect();
    let content: String = meeting.text.chars().take(CONTENT_CHARS).collect();
    let user = TaggingUser {
        tags: &tags,
        title: &meeting.title,
        content_label: if meeting.is_summary {
            "Summary"
        } else {
            "Transcript (opening)"
        },
        content: &content,
    };
    Prompt::new(user.text())
        .system(TaggingSystem.text())
        .max_tokens(ANSWER_TOKENS)
}

fn choice<'a>(candidate: &'a Candidate, summaries: &'a [Option<String>]) -> TagChoice<'a> {
    let examples = candidate
        .examples
        .iter()
        .zip(summaries)
        .map(|(example, summary)| TagExample {
            title: &example.title,
            summary: summary.as_deref(),
        })
        .collect();
    TagChoice {
        name: &candidate.tag.name,
        description: &candidate.tag.description,
        examples,
    }
}

/// An example's summary on one line, cut to [`EXAMPLE_SUMMARY_CHARS`].
fn example_summary(example: &MeetingDigest) -> Option<String> {
    let summary = example.summary.as_deref()?;
    let line = summary.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut cut: String = line.chars().take(EXAMPLE_SUMMARY_CHARS).collect();
    if cut.len() < line.len() {
        cut.push('…');
    }
    (!cut.is_empty()).then_some(cut)
}

/// The offered tags the answer names (ignoring case and surrounding space),
/// each once, in the answer's order. Names of tags not offered are dropped.
pub fn chosen_tags(answer: &TaggingAnswer, candidates: &[Candidate]) -> Vec<TagId> {
    let mut chosen = Vec::new();
    for name in &answer.tags {
        let wanted = name.trim().to_lowercase();
        let found = candidates
            .iter()
            .find(|candidate| candidate.tag.name.to_lowercase() == wanted);
        if let Some(candidate) = found
            && !chosen.contains(&candidate.tag.id)
        {
            chosen.push(candidate.tag.id);
        }
    }
    chosen
}

#[cfg(test)]
mod tests {
    use deino_domain::{FakeClock, MeetingId, PaletteColor};
    use deino_llm::Role;
    use deino_testkit::at;

    use super::*;

    fn tag(name: &str, description: &str) -> Tag {
        Tag {
            id: TagId::generate(&FakeClock::new(at("2026-10-09T09:00:00Z"))),
            name: name.into(),
            color: PaletteColor::cycle(0),
            description: description.into(),
            auto_apply: true,
            created_at: at("2026-10-09T09:00:00Z"),
            updated_at: at("2026-10-09T09:00:00Z"),
        }
    }

    fn digest(title: &str, summary: Option<&str>) -> MeetingDigest {
        MeetingDigest {
            meeting_id: MeetingId::new(format!("meeting-{title}")),
            title: title.into(),
            summary: summary.map(str::to_owned),
        }
    }

    fn candidates() -> Vec<Candidate> {
        vec![
            Candidate {
                tag: tag("Customer", "Calls with paying customers."),
                examples: vec![digest(
                    "Northwind escalation",
                    Some("## Issue\nDuplicate\n jobs."),
                )],
            },
            Candidate {
                tag: tag("Hiring", ""),
                examples: Vec::new(),
            },
        ]
    }

    fn user_text(prompt: &Prompt) -> String {
        let message = prompt.messages.first().unwrap();
        assert_eq!(message.role, Role::User);
        message.text()
    }

    #[test]
    fn the_prompt_shows_tags_examples_on_one_line_and_the_meeting() {
        let meeting = MeetingContent {
            title: "Fernhollow check-in".into(),
            text: "Fernhollow wants more seats.".into(),
            is_summary: false,
        };

        let prompt = tagging_prompt(&candidates(), &meeting);

        let user = user_text(&prompt);
        assert!(
            user.contains("## Customer\nCalls with paying customers."),
            "{user}"
        );
        assert!(
            user.contains("- Northwind escalation: ## Issue Duplicate jobs."),
            "{user}"
        );
        assert!(
            user.contains("Transcript (opening):\nFernhollow wants more seats."),
            "{user}"
        );
        assert!(prompt.system.unwrap().contains("Never invent a tag"));
        assert_eq!(prompt.max_tokens, Some(ANSWER_TOKENS));
    }

    #[test]
    fn long_content_and_example_summaries_are_cut() {
        let long = "word ".repeat(CONTENT_CHARS);
        let mut offered = candidates();
        offered[0].examples = vec![digest("Long", Some(&long))];
        let meeting = MeetingContent {
            title: "T".into(),
            text: long.clone(),
            is_summary: true,
        };

        let user = user_text(&tagging_prompt(&offered, &meeting));

        assert!(user.chars().count() < CONTENT_CHARS + EXAMPLE_SUMMARY_CHARS + 1_000);
        assert!(user.contains('…'));
        assert!(user.contains("Summary:\n"));
    }

    #[test]
    fn only_offered_tags_are_chosen_once_ignoring_case() {
        let offered = candidates();
        let answer = TaggingAnswer {
            tags: vec![
                " customer ".into(),
                "Invented".into(),
                "Customer".into(),
                "HIRING".into(),
            ],
        };

        let chosen = chosen_tags(&answer, &offered);

        assert_eq!(chosen, [offered[0].tag.id, offered[1].tag.id]);
        assert!(chosen_tags(&TaggingAnswer::default(), &offered).is_empty());
    }
}
