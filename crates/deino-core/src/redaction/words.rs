//! Words with their byte ranges: what the context-aware detectors read.
//!
//! A word is a maximal run of letters, digits and `_`; everything between
//! two words is their gap. Detectors look at a few words before a match
//! ("the code is …") and at the gaps between digit groups.

use std::ops::Range;

/// One word of a text and where it is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Word<'a> {
    /// The word as written.
    pub text: &'a str,
    /// Its byte range in the text.
    pub range: Range<usize>,
}

impl Word<'_> {
    /// True when the word is `cue`, ignoring ASCII case.
    pub fn is(&self, cue: &str) -> bool {
        self.text.eq_ignore_ascii_case(cue)
    }

    /// True when the word is one of `cues`, ignoring ASCII case.
    pub fn is_any(&self, cues: &[&str]) -> bool {
        cues.iter().any(|cue| self.is(cue))
    }
}

/// Every word of `text`, in order.
pub(super) fn words(text: &str) -> Vec<Word<'_>> {
    let mut found = Vec::new();
    let mut start = None;
    for (index, character) in text.char_indices() {
        match (is_word_char(character), start) {
            (true, None) => start = Some(index),
            (false, Some(from)) => {
                found.push(word(text, from..index));
                start = None;
            }
            _ => {}
        }
    }
    if let Some(from) = start {
        found.push(word(text, from..text.len()));
    }
    found
}

/// The text between the end of `before` and the start of `after`.
pub(super) fn gap<'a>(text: &'a str, before: &Word<'_>, after: &Word<'_>) -> &'a str {
    text.get(before.range.end..after.range.start)
        .unwrap_or_default()
}

/// True when the `window` words before index `at` contain one of `cues`.
pub(super) fn cue_before(words: &[Word<'_>], at: usize, window: usize, cues: &[&str]) -> bool {
    let from = at.saturating_sub(window);
    words
        .get(from..at)
        .is_some_and(|before| before.iter().any(|word| word.is_any(cues)))
}

fn word(text: &str, range: Range<usize>) -> Word<'_> {
    Word {
        text: text.get(range.clone()).unwrap_or_default(),
        range,
    }
}

fn is_word_char(character: char) -> bool {
    character.is_alphanumeric() || character == '_'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words_carry_their_ranges() {
        let text = "the code: 48-29 ok";

        let found = words(text);

        let texts: Vec<&str> = found.iter().map(|word| word.text).collect();
        assert_eq!(texts, ["the", "code", "48", "29", "ok"]);
        assert_eq!(found[2].range, 10..12);
        assert_eq!(gap(text, &found[2], &found[3]), "-");
    }

    #[test]
    fn cues_are_found_only_inside_the_window() {
        let found = words("my code is really 1234");

        assert!(cue_before(&found, 4, 3, &["code"]));
        assert!(!cue_before(&found, 4, 2, &["code"]));
    }
}
