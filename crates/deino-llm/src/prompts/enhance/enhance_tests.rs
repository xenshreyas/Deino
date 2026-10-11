//! Snapshots of the `notes.enhance` and notes-title prompts: a wording
//! change shows up in review.

use super::*;
use crate::prompts::PromptText;

const MEETING: &str = "Title: Maya 1:1\nDate: Wednesday 7 October 2026, 15:00 UTC\nAttendees (2): Shreyas (me), Maya Chen (Staff engineer)";
const TEMPLATE: &str =
    "1:1. A weekly one-on-one.\n1. Overview: the state of things. Format: bullet blocks.";
const TRANSCRIPT: &str =
    "[s1 00:04] Shreyas: How are you holding up?\n[s2 00:09] Maya: Tired, honestly.";

#[test]
fn the_system_prompt_puts_the_users_lines_first() {
    insta::assert_snapshot!(EnhanceSystem.text());
}

#[test]
fn one_pass_carries_every_input() {
    let prompt = EnhanceUser {
        meeting: MEETING,
        template: TEMPLATE,
        previous: Some("Maya 1:1 on 30 September:\n- beta on track"),
        vocabulary: Some("Fernhollow, PagerDuty"),
        notes: Some("- v tired post fernhollow\n- [ ] update pagerduty"),
        transcript: TRANSCRIPT,
    };

    insta::assert_snapshot!(prompt.text());
}

#[test]
fn one_pass_without_notes_says_so() {
    let prompt = EnhanceUser {
        meeting: MEETING,
        template: TEMPLATE,
        previous: None,
        vocabulary: None,
        notes: None,
        transcript: TRANSCRIPT,
    };

    insta::assert_snapshot!(prompt.text());
}

#[test]
fn a_part_names_its_place_in_the_meeting() {
    insta::assert_snapshot!(EnhancePartSystem.text());
    let prompt = EnhancePartUser {
        meeting: MEETING,
        template: TEMPLATE,
        notes: Some("- v tired post fernhollow"),
        part: 2,
        parts: 4,
        span: "20:00–40:00",
        transcript: TRANSCRIPT,
    };

    insta::assert_snapshot!(prompt.text());
}

#[test]
fn the_merge_keeps_the_parts_references() {
    let prompt = EnhanceMergeUser {
        meeting: MEETING,
        template: TEMPLATE,
        previous: None,
        vocabulary: None,
        notes: Some("- v tired post fernhollow"),
        partials: "<part number=\"1\" span=\"00:00–20:00\">\n### Overview\n- Maya is tired after the incident [s2]\n</part>",
    };

    insta::assert_snapshot!(prompt.text());
}

#[test]
fn a_title_comes_from_the_notes() {
    insta::assert_snapshot!(
        TitleNotesUser {
            notes: "### Overview\n- Maya is off on-call for two weeks"
        }
        .text()
    );
}
