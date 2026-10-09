//! Eval cases, built from the fixture meetings (`fixtures/meetings/*.json`).
//!
//! `title`: each meeting's transcript should get a short, one-line title
//! about its topic: one that names something the meeting's own calendar
//! title names (the customer, the product, the role being hired).

use deino_llm::prompts::{PromptText, TitleSystem, TitleUser};
use deino_llm::{Prompt, Route};
use deino_testkit::fixture::{FixtureMeeting, Fixtures};
use libtest_mimic::Failed;

use crate::expectations::Expectation;

/// Words too common in titles to show that a title is on topic.
const STOP_WORDS: [&str; 10] = [
    "meeting", "weekly", "sync", "update", "review", "about", "with", "deino", "call", "team",
];

/// One eval case.
#[derive(Clone, Debug)]
pub struct Case {
    /// The route under test.
    pub route: Route,
    /// The fixture it comes from.
    pub name: String,
    /// What the gateway is asked.
    pub prompt: Prompt,
    /// What every good answer satisfies.
    pub expectations: Vec<Expectation>,
    /// The share of live samples that must pass.
    pub required_pass_rate: f64,
    /// A known-good answer, for the smoke trial.
    pub smoke_answer: String,
    /// Text the rendered prompt must contain.
    pub prompt_fragments: Vec<String>,
}

impl Case {
    /// The contract trial: the prompt carries the instructions and the transcript.
    pub fn check_prompt(&self) -> Result<(), Failed> {
        let rendered = format!(
            "{}\n{}",
            self.prompt.system.as_deref().unwrap_or_default(),
            self.prompt
                .messages
                .iter()
                .map(deino_llm::Message::text)
                .collect::<String>()
        );
        match self
            .prompt_fragments
            .iter()
            .find(|fragment| !rendered.contains(fragment.as_str()))
        {
            Some(missing) => Err(format!("the prompt lacks {missing:?}").into()),
            None => Ok(()),
        }
    }
}

/// Every case.
pub fn all() -> Result<Vec<Case>, String> {
    let fixtures =
        Fixtures::load().map_err(|error| format!("could not load the fixtures: {error}"))?;
    Ok(fixtures.meetings().iter().map(title_case).collect())
}

fn title_case(meeting: &FixtureMeeting) -> Case {
    let transcript = transcript(meeting);
    let first_line = meeting
        .segments
        .first()
        .map(|segment| segment.text.clone())
        .unwrap_or_default();
    Case {
        route: Route::Title,
        name: meeting
            .id
            .as_str()
            .trim_start_matches("meeting-fixture-")
            .to_owned(),
        prompt: Prompt::new(
            TitleUser {
                transcript: &transcript,
            }
            .text(),
        )
        .system(TitleSystem.text())
        .max_tokens(40),
        expectations: vec![
            Expectation::SingleLine,
            Expectation::MaxWords(10),
            Expectation::NotContains("<EMPTY>"),
            Expectation::NotContains("\""),
            Expectation::MentionsAny(topic_words(&meeting.title)),
        ],
        required_pass_rate: 0.8,
        smoke_answer: meeting.title.clone(),
        prompt_fragments: vec!["Output only the title".to_owned(), first_line],
    }
}

fn transcript(meeting: &FixtureMeeting) -> String {
    meeting
        .segments
        .iter()
        .map(|segment| format!("{}: {}", segment.speaker, segment.text))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The calendar title's distinctive words, lower case.
fn topic_words(title: &str) -> Vec<String> {
    title
        .split(|character: char| !character.is_alphanumeric())
        .map(str::to_lowercase)
        .filter(|word| word.chars().count() >= 4 && !STOP_WORDS.contains(&word.as_str()))
        .collect()
}
