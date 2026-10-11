//! A known-good `notes.enhance` answer, built from the ground truth: every
//! line the user typed as theirs, then each decision and fact as an AI line
//! citing the transcript line that says it most closely. Lines are read
//! from the prompts (`[s12 04:31] Speaker: text`), so a long meeting's parts
//! teach the oracle the references its merge answer cites.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex, PoisonError};

use deino_llm::{CompletionRequest, MockProvider, Route};
use deino_testkit::fixture::FixtureMeeting;
use serde_json::{Value, json};

use super::score;

/// The transcript lines seen so far, by reference (`s12`).
type Seen = Arc<Mutex<BTreeMap<String, String>>>;

/// A `MockProvider` answering `notes.enhance` as the oracle.
pub fn provider(meeting: &FixtureMeeting) -> MockProvider {
    let meeting = meeting.clone();
    let seen: Seen = Arc::default();
    MockProvider::new().answer_with(Route::NotesEnhance, move |request| {
        let mut lines = seen.lock().unwrap_or_else(PoisonError::into_inner);
        lines.extend(transcript_lines(request));
        answer(&meeting, &lines).to_string()
    })
}

/// The answer: the user's lines, then the decisions and facts.
fn answer(meeting: &FixtureMeeting, lines: &BTreeMap<String, String>) -> Value {
    let user: Vec<Value> = score::typed_lines(meeting)
        .iter()
        .map(|text| json!({ "kind": "bullet", "text": text, "origin": "user" }))
        .collect();
    let truth = &meeting.ground_truth;
    let decisions = truth
        .decisions
        .iter()
        .map(|text| ai_line(text, text, lines));
    let facts = truth
        .facts
        .iter()
        .map(|fact| ai_line(&fact.answer, &fact.evidence, lines));
    json!({ "sections": [
        { "heading": "Your notes", "blocks": user },
        { "heading": "Decisions", "blocks": decisions.collect::<Vec<_>>() },
        { "heading": "Facts", "blocks": facts.collect::<Vec<_>>() },
    ]})
}

/// An AI bullet saying `text`, citing the line closest to `evidence`.
fn ai_line(text: &str, evidence: &str, lines: &BTreeMap<String, String>) -> Value {
    let cited: Vec<&String> = closest(evidence, lines).into_iter().collect();
    json!({ "kind": "bullet", "text": text, "origin": "ai", "evidence": cited })
}

/// The reference of the line sharing the most words with `evidence`.
fn closest<'a>(evidence: &str, lines: &'a BTreeMap<String, String>) -> Option<&'a String> {
    let wanted = words(evidence);
    lines
        .iter()
        .map(|(reference, text)| (words(text).intersection(&wanted).count(), reference))
        .filter(|(shared, _)| *shared > 0)
        .max_by_key(|(shared, _)| *shared)
        .map(|(_, reference)| reference)
}

fn words(text: &str) -> BTreeSet<String> {
    text.split(|character: char| !character.is_alphanumeric())
        .filter(|word| word.chars().count() >= 4)
        .map(str::to_lowercase)
        .collect()
}

/// `s12` → text for every `[s12 04:31] Speaker: text` line of the request.
fn transcript_lines(request: &CompletionRequest) -> Vec<(String, String)> {
    request
        .messages
        .iter()
        .map(deino_llm::Message::text)
        .flat_map(|message| message.lines().filter_map(parse_line).collect::<Vec<_>>())
        .collect()
}

fn parse_line(line: &str) -> Option<(String, String)> {
    let rest = line.strip_prefix("[s")?;
    let (label, rest) = rest.split_once(']')?;
    let number = label.split_whitespace().next()?;
    if number.is_empty() || !number.chars().all(|character| character.is_ascii_digit()) {
        return None;
    }
    let text = rest.split_once(": ").map_or(rest, |(_, text)| text);
    Some((format!("s{number}"), text.trim().to_owned()))
}
