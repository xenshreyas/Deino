//! The `notes.edit` prompt: the request, the selection, the notes as one
//! JSON block per line, and the transcript the citations point at.

use deino_llm::Prompt;
use deino_llm::prompts::PromptText;
use deino_llm::prompts::notes_edit::{NotesEditSystem, NotesEditUser};
use serde::Serialize;

use super::model::TextKind;
use super::scope::block_id;

/// The most of the transcript a prompt carries, in characters (about 15k
/// tokens); a longer meeting's tail is cut.
pub const MAX_TRANSCRIPT_CHARS: usize = 60_000;
/// The answer cap: a list of changes is short.
const EDIT_MAX_TOKENS: u32 = 4_096;

/// One block as the model sees it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PromptBlock {
    /// `b3`.
    pub id: String,
    /// How it reads.
    pub kind: TextKind,
    /// `user`, `ai` or `mixed`.
    pub origin: &'static str,
    /// Its text, references included.
    pub text: String,
    /// True when its text may not be replaced.
    #[serde(skip_serializing_if = "is_false")]
    pub locked: bool,
}

/// For `skip_serializing_if`, which passes a reference.
fn is_false(value: &bool) -> bool {
    !*value
}

impl PromptBlock {
    /// Block `position`.
    pub fn new(position: usize, kind: TextKind, origin: &'static str, text: String) -> Self {
        Self {
            id: block_id(position),
            kind,
            origin,
            text,
            locked: false,
        }
    }
}

/// What the prompt is built from.
pub struct EditPrompt<'a> {
    /// The request, redacted.
    pub instruction: &'a str,
    /// The selection (`b2 to b4`), if any.
    pub selection: Option<&'a str>,
    /// The notes' blocks, redacted.
    pub blocks: &'a [PromptBlock],
    /// The transcript lines, rendered and redacted.
    pub transcript: &'a str,
}

impl EditPrompt<'_> {
    /// The prompt.
    pub fn render(&self) -> Prompt {
        let notes: Vec<String> = self
            .blocks
            .iter()
            .filter_map(|block| serde_json::to_string(block).ok())
            .collect();
        let notes = notes.join("\n");
        let transcript = cut(self.transcript, MAX_TRANSCRIPT_CHARS);
        let user = NotesEditUser {
            request: self.instruction.trim(),
            selection: self.selection,
            notes: &notes,
            transcript: if transcript.is_empty() {
                "No transcript."
            } else {
                transcript
            },
        };
        Prompt::new(user.text())
            .system(NotesEditSystem.text())
            .max_tokens(EDIT_MAX_TOKENS)
    }
}

/// `text` cut to at most `max` characters, at a line end when it can be.
fn cut(text: &str, max: usize) -> &str {
    let Some((end, _)) = text.char_indices().nth(max) else {
        return text;
    };
    let head = text.get(..end).unwrap_or(text);
    head.rfind('\n')
        .map_or(head, |line_end| head.get(..line_end).unwrap_or(head))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_are_one_json_object_per_line() {
        let mut locked = PromptBlock::new(1, TextKind::Bullet, "user", "pw [redacted]".into());
        locked.locked = true;
        let blocks = [
            PromptBlock::new(0, TextKind::Heading, "ai", "Pricing".into()),
            locked,
        ];
        let prompt = EditPrompt {
            instruction: " make it shorter ",
            selection: None,
            blocks: &blocks,
            transcript: "",
        }
        .render();

        insta::assert_snapshot!(prompt.messages.first().map(|message| message.text()).unwrap_or_default(), @r#"
        <request>
        make it shorter
        </request>

        <notes>
        {"id":"b1","kind":"heading","origin":"ai","text":"Pricing"}
        {"id":"b2","kind":"bullet","origin":"user","text":"pw [redacted]","locked":true}
        </notes>

        <transcript>
        No transcript.
        </transcript>

        Now answer with the changes to the notes.
        "#);
    }

    #[test]
    fn a_long_transcript_is_cut_at_a_line_end() {
        assert_eq!(cut("one\ntwo\nthree", 9), "one\ntwo");
        assert_eq!(cut("short", 9), "short");
    }
}
