//! The redaction check (NOT-09): a local model confirms or rejects the
//! secrets that `deino_core::redaction`'s patterns found from context
//! alone. It sees one short excerpt per candidate, never the transcript.

use askama::Template;

/// The redaction check's rules.
#[derive(Template)]
#[template(path = "redact_check.system.md.jinja", escape = "none")]
pub struct RedactCheckSystem;

/// The candidates to judge.
#[derive(Template)]
#[template(path = "redact_check.user.md.jinja", escape = "none")]
pub struct RedactCheckUser<'a> {
    /// One `n. [kind] …excerpt with «candidate»…` line per candidate.
    pub candidates: &'a str,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prompts::PromptText;

    #[test]
    fn the_rules_say_what_is_not_a_secret() {
        insta::assert_snapshot!(RedactCheckSystem.text(), @r#"
        # General Instructions

        - You check whether short excerpts of a meeting transcript reveal a secret, so it can be hidden from the user's notes.
        - Each excerpt marks one candidate between « and », with the kind of secret a pattern matched.
        - A candidate is a secret when it is the actual value: a password, an API key, a card number, a social security number or a one-time code.
        - It is not a secret when it only talks about one ("the password is on the sticky note", "the code is in the email") or is something else, such as a year, an order number or a room number.
        - When unsure, treat it as a secret.

        # Format Requirements

        - Answer once for every numbered candidate, with its number and `true` when it is a secret.
        "#);
    }

    #[test]
    fn the_candidates_are_listed() {
        let prompt = RedactCheckUser {
            candidates: "1. [password] the wifi password is «hunter2», then",
        };

        insta::assert_snapshot!(prompt.text(), @r"
        <candidates>
        1. [password] the wifi password is «hunter2», then
        </candidates>

        Now decide, for each candidate, whether it is a secret.
        ");
    }
}
