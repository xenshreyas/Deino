//! The `followup.email` route's wire types (NOT-07): the two structured
//! answers a suggested follow-up email takes.
//!
//! 1. [`FollowUpDecision`]: whether the user should send a follow-up at all.
//!    Granola stays quiet when a colleague owns the follow-up, the
//!    conversation moved to Slack or another chat, or a blocker comes before
//!    any next step (`docs/research/granola.md` §7.2).
//! 2. [`FollowUpDraftAnswer`]: the subject and body, with `[placeholders]`
//!    for what only the user knows.
//!
//! These are wire types: `deino_core::followups` acts on them and the eval
//! harness scores them. [`placeholders`] finds the bracketed gaps.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// What the model decided about a follow-up email.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FollowUpVerdict {
    /// The user should follow up with the external attendees by email.
    Send,
    /// A colleague said they would send the follow-up.
    ColleagueOwns,
    /// The conversation continues in Slack or another chat, not email.
    MovedToChat,
    /// A blocker must be resolved before there is anything to send.
    BlockerFirst,
    /// Nothing was agreed that an email would move forward.
    NothingToFollowUp,
}

/// `followup.email`, first call: should the user send a follow-up?
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct FollowUpDecision {
    /// One sentence on what in the meeting decides it.
    pub reason: String,
    /// The decision.
    pub verdict: FollowUpVerdict,
}

/// `followup.email`, second call: the draft.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct FollowUpDraftAnswer {
    /// The subject line, at most 80 characters.
    pub subject: String,
    /// The body in plain text, paragraphs separated by blank lines, with a
    /// `[placeholder]` wherever only the user knows what goes there.
    pub body: String,
}

/// The `[placeholders]` of `text`, in order: bracketed runs of 1–60
/// characters on one line, without nested brackets. Markdown links
/// (`[text](url)`) are not placeholders.
pub fn placeholders(text: &str) -> Vec<&str> {
    let mut found = Vec::new();
    let mut rest = text;
    while let Some(open) = rest.find('[') {
        let after = rest.get(open + 1..).unwrap_or_default();
        let Some(close) = after.find(['[', ']', '\n']) else {
            break;
        };
        let inner = after.get(..close).unwrap_or_default();
        let closed = after.get(close..).is_some_and(|tail| tail.starts_with(']'));
        let is_link = after
            .get(close + 1..)
            .is_some_and(|tail| tail.starts_with('('));
        if closed && !is_link && (1..=60).contains(&inner.chars().count()) {
            found.push(inner);
        }
        rest = after.get(close..).unwrap_or_default();
        rest = rest.strip_prefix(']').unwrap_or(rest);
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verdicts_serialize_in_snake_case() {
        let decision: FollowUpDecision =
            serde_json::from_str(r#"{"reason": "Priya sends it.", "verdict": "colleague_owns"}"#)
                .unwrap();

        assert_eq!(decision.verdict, FollowUpVerdict::ColleagueOwns);
        assert_eq!(
            serde_json::to_value(FollowUpVerdict::MovedToChat).unwrap(),
            "moved_to_chat"
        );
    }

    #[test]
    fn placeholders_are_short_bracketed_runs_on_one_line() {
        let body = "Hi [first name],\nThe pilot starts [start date]. See [the deck](https://x.y).\n[unclosed\n[] [[nested]]";

        assert_eq!(placeholders(body), ["first name", "start date", "nested"]);
        assert!(placeholders("No gaps here.").is_empty());
        assert!(placeholders(&format!("[{}]", "x".repeat(61))).is_empty());
    }
}
