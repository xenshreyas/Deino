//! Editing notes by asking (NOT-06): the rules, and the request with the
//! notes as ID'd blocks and the transcript they cite. The answer is
//! `deino_core::notes::edit::EditAnswer`, a list of block changes.

use askama::Template;

/// `notes.edit`: instructions.
#[derive(Template)]
#[template(path = "notes_edit.system.md.jinja", escape = "none")]
pub struct NotesEditSystem;

/// `notes.edit`: what the user asked, the notes and the transcript.
#[derive(Template)]
#[template(path = "notes_edit.user.md.jinja", escape = "none")]
pub struct NotesEditUser<'a> {
    /// The user's request, in their words.
    pub request: &'a str,
    /// The selected blocks (`b2 to b4`), when the edit is scoped to them.
    pub selection: Option<&'a str>,
    /// The notes: one JSON block per line.
    pub notes: &'a str,
    /// The transcript lines, `[s12 04:31] Speaker: text`.
    pub transcript: &'a str,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prompts::PromptText;

    #[test]
    fn the_request_selection_notes_and_transcript_are_fenced() {
        let prompt = NotesEditUser {
            request: "Niall is spelled Nyal",
            selection: Some("b2 to b3"),
            notes: r#"{"id":"b1","kind":"heading","origin":"ai","text":"Pricing"}"#,
            transcript: "[s1 00:04] Maya: Nyal owns pricing.",
        };

        insta::assert_snapshot!(prompt.text(), @r#"
        <request>
        Niall is spelled Nyal
        </request>

        <selection>
        The user selected b2 to b3.
        </selection>

        <notes>
        {"id":"b1","kind":"heading","origin":"ai","text":"Pricing"}
        </notes>

        <transcript>
        [s1 00:04] Maya: Nyal owns pricing.
        </transcript>

        Now answer with the changes to the notes.
        "#);
    }

    #[test]
    fn without_a_selection_the_whole_notes_are_in_scope() {
        let prompt = NotesEditUser {
            request: "remove the pricing section",
            selection: None,
            notes: "{}",
            transcript: "",
        };

        assert!(!prompt.text().contains("<selection>"));
        assert!(NotesEditSystem.text().contains("insert_after"));
    }
}
