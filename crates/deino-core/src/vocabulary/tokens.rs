//! Whole words of transcript text, and where their letters are.
//!
//! A [`Token`] is a whitespace-separated run of text. Its *core* is the run
//! without the punctuation around it (`"Sreyas,"` → `Sreyas`, `"(Deino)"` →
//! `Deino`) and without a possessive ending (`"Sreyas's"` → `Sreyas`).
//! Matching compares whole cores and a correction replaces whole cores, so
//! punctuation and possessives survive and a correction never lands inside
//! another word.

use std::ops::Range;

/// Possessive endings kept outside a token's core.
const POSSESSIVES: [&str; 2] = ["'s", "\u{2019}s"];

/// One whitespace-separated word of a text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Token {
    /// The whole run, punctuation included, as a byte range of the text.
    pub span: Range<usize>,
    /// Its core, as a byte range of the text; empty when the run has no
    /// letters or digits (a dash, an ellipsis).
    pub core: Range<usize>,
}

impl Token {
    /// True when the token has letters or digits.
    pub fn is_word(&self) -> bool {
        !self.core.is_empty()
    }

    /// True when nothing follows the core inside the token.
    pub fn ends_bare(&self) -> bool {
        self.core.end == self.span.end
    }

    /// True when nothing precedes the core inside the token.
    pub fn starts_bare(&self) -> bool {
        self.core.start == self.span.start
    }

    /// The core's text.
    pub fn core_of<'a>(&self, text: &'a str) -> &'a str {
        text.get(self.core.clone()).unwrap_or_default()
    }
}

/// The tokens of `text`, in order.
pub(super) fn tokenize(text: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut start = None;
    for (index, ch) in text.char_indices() {
        match (ch.is_whitespace(), start) {
            (true, Some(from)) => {
                tokens.push(token(text, from..index));
                start = None;
            }
            (false, None) => start = Some(index),
            _ => {}
        }
    }
    if let Some(from) = start {
        tokens.push(token(text, from..text.len()));
    }
    tokens
}

/// True when tokens `range` read as one phrase: nothing but spaces between
/// their cores (`"Shreyas Patel"`, not `"Shreyas, Patel"`).
pub(super) fn joined(tokens: &[Token], range: Range<usize>) -> bool {
    let Some(run) = tokens.get(range) else {
        return false;
    };
    let inner_ends = run.iter().rev().skip(1).all(Token::ends_bare);
    let inner_starts = run.iter().skip(1).all(Token::starts_bare);
    run.iter().all(Token::is_word) && inner_ends && inner_starts
}

/// A word as matching compares it: lower case, with typographic apostrophes
/// made plain.
pub(super) fn fold(word: &str) -> String {
    word.chars()
        .flat_map(char::to_lowercase)
        .map(|ch| if ch == '\u{2019}' { '\'' } else { ch })
        .collect()
}

/// A phrase as fuzzy matching compares it: its letters and digits, lower case.
pub(super) fn letters(phrase: &str) -> String {
    phrase
        .chars()
        .filter(|ch| ch.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn token(text: &str, span: Range<usize>) -> Token {
    let run = text.get(span.clone()).unwrap_or_default();
    let first = run.char_indices().find(|(_, ch)| ch.is_alphanumeric());
    let last = run
        .char_indices()
        .rev()
        .find(|(_, ch)| ch.is_alphanumeric());
    let core = match (first, last) {
        (Some((from, _)), Some((to, ch))) => span.start + from..span.start + to + ch.len_utf8(),
        _ => span.start..span.start,
    };
    Token {
        core: without_possessive(text, core),
        span,
    }
}

/// `core` without a trailing `'s`, unless that is all there is.
fn without_possessive(text: &str, core: Range<usize>) -> Range<usize> {
    let word = text.get(core.clone()).unwrap_or_default();
    POSSESSIVES
        .iter()
        .find_map(|ending| {
            let base = word.strip_suffix(ending)?;
            (!base.is_empty()).then(|| core.start..core.start + base.len())
        })
        .unwrap_or(core)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cores(text: &str) -> Vec<&str> {
        tokenize(text).iter().map(|t| t.core_of(text)).collect()
    }

    #[test]
    fn cores_drop_surrounding_punctuation_and_possessives() {
        assert_eq!(
            cores("Hi, (Sreyas)! Sreyas's plan — e.g. done…"),
            ["Hi", "Sreyas", "Sreyas", "plan", "", "e.g", "done"]
        );
        assert_eq!(cores("it\u{2019}s 's"), ["it", "s"]);
    }

    #[test]
    fn tokens_cover_their_runs_with_any_whitespace() {
        let text = "  one\ttwo\n three ";
        let spans: Vec<&str> = tokenize(text)
            .iter()
            .map(|t| &text[t.span.clone()])
            .collect();

        assert_eq!(spans, ["one", "two", "three"]);
        assert!(tokenize("   ").is_empty());
    }

    #[test]
    fn a_phrase_is_joined_only_by_spaces() {
        let tokens = tokenize("Shreyas Patel, Shreyas, Patel");

        assert!(joined(&tokens, 0..2));
        assert!(!joined(&tokens, 2..4));
        assert!(!joined(&tokens, 3..5));
    }

    #[test]
    fn folding_ignores_case_and_apostrophe_style() {
        assert_eq!(fold("O\u{2019}Brien"), "o'brien");
        assert_eq!(letters("O'Brien-Smith 2"), "obriensmith2");
    }
}
