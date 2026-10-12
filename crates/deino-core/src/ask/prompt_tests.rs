//! [`AskContext`]: the material, the history and the request in the prompt.

use chrono::DateTime;
use deino_domain::{AskMessageId, FakeClock, MeetingId, Millis, SegmentId};

use super::*;
use crate::ask::{AnswerPart, RecipeLabel};
use crate::enhance::AttendeeInfo;
use crate::recipes::{LIBRARY, RecipeKind};

fn meeting() -> MeetingId {
    MeetingId::new("meeting-fixture-ask")
}

fn line(reference: u32, text: &str) -> TranscriptLine {
    TranscriptLine {
        reference,
        segment_id: uuid::Uuid::from_u128(u128::from(reference))
            .to_string()
            .parse::<SegmentId>()
            .unwrap(),
        start: Millis(reference * 1_000),
        end: Millis(reference * 1_000 + 900),
        speaker: "Maya".into(),
        text: text.into(),
    }
}

fn material(lines: Vec<TranscriptLine>) -> Material {
    Material {
        meeting: MeetingInfo {
            id: meeting(),
            title: "Renewal".into(),
            date: DateTime::UNIX_EPOCH,
            calendar_event_id: None,
            calendar_title: None,
            description: String::new(),
            attendees: vec![AttendeeInfo {
                name: "Sam".into(),
                title: None,
                is_me: true,
            }],
            series_id: None,
        },
        notes: None,
        transcript: lines,
        transcript_edited_at: None,
        attendee_ids: Vec::new(),
        previous: None,
        vocabulary: Vec::new(),
    }
}

fn message(role: AskRole, text: &str, recipe: Option<&str>) -> AskMessage {
    AskMessage {
        id: AskMessageId::generate(&FakeClock::new(DateTime::UNIX_EPOCH)),
        meeting_id: meeting(),
        role,
        parts: vec![AnswerPart::plain(text)],
        recipe: recipe.map(|name| RecipeLabel {
            slug: "coach-me".into(),
            name: name.into(),
        }),
        created_at: DateTime::UNIX_EPOCH,
    }
}

fn texts(prompt: &Prompt) -> Vec<String> {
    prompt.messages.iter().map(Message::text).collect()
}

#[test]
fn the_question_comes_with_the_meeting_its_notes_and_its_referenced_transcript() {
    let context = AskContext::new(
        &material(vec![line(1, "we hold the beta date")]),
        Some("- beta?".into()),
        Some("  ".into()),
    );

    let prompt = context.prompt(&[], "What did we decide?");

    let user = texts(&prompt).pop().unwrap();
    assert!(user.contains("Attendees: Sam (me)"));
    assert!(user.contains("<user_notes>\n- beta?\n</user_notes>"));
    assert!(!user.contains("<enhanced_notes>"));
    assert!(user.contains("[s1 00:01] Maya: we hold the beta date"));
    assert!(user.ends_with("What did we decide?"));
    assert!(prompt.system.unwrap().contains("[s12]"));
}

#[test]
fn earlier_turns_come_first_without_citations_and_start_with_a_question() {
    let context = AskContext::new(&material(vec![line(1, "x")]), None, None);
    let mut history = vec![message(AskRole::Assistant, "orphan", None)];
    for n in 0..4 {
        history.push(message(AskRole::User, &format!("q{n}"), None));
        history.push(message(AskRole::Assistant, &format!("a{n}"), None));
    }
    history.push(message(AskRole::User, "", Some("Coach me")));
    history.push(message(AskRole::Assistant, "coached", None));

    let messages = texts(&context.prompt(&history, "next"));

    assert_eq!(
        messages[..5],
        ["q2", "a2", "q3", "a3", "(Ran the recipe \"Coach me\".)"]
    );
    assert_eq!(messages.len(), HISTORY_TURNS + 1);
}

#[test]
fn every_single_meeting_built_in_renders_a_prompt_with_its_instructions() {
    let context = AskContext::new(
        &material(vec![line(1, "we hold the beta date")]),
        None,
        None,
    );

    for recipe in LIBRARY.iter().filter(|r| r.kind == RecipeKind::Single) {
        let request = request_text(Some(recipe.prompt), "");
        let user = texts(&context.prompt(&[], &request)).pop().unwrap();

        assert!(
            user.contains(&format!("<recipe>\n{}\n</recipe>", recipe.prompt)),
            "{}",
            recipe.slug
        );
        assert!(user.contains("[s1 00:01]"), "{}", recipe.slug);
    }
}

#[test]
fn what_the_user_adds_to_a_recipe_follows_it() {
    let request = request_text(Some("Write a brief."), " keep it short ");

    assert!(request.ends_with("</recipe>\n\nThe user adds: keep it short"));
    assert_eq!(request_text(None, " plain "), "plain");
}

#[test]
fn a_meeting_without_anything_to_read_is_empty_and_a_long_one_is_cut() {
    assert!(AskContext::new(&material(Vec::new()), None, None).is_empty());
    let long = "word ".repeat(1_000);
    let lines: Vec<TranscriptLine> = (1..=60).map(|n| line(n, &long)).collect();

    let context = AskContext::new(&material(lines), None, None);
    let user = texts(&context.prompt(&[], "q")).pop().unwrap();

    assert!(user.contains(CUT_NOTE));
    assert!(user.len() < MAX_TRANSCRIPT_CHARS + 2_000);
}
