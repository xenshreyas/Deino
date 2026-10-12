//! Passwords said or typed after a cue: "the password is hunter2",
//! "wifi passphrase: blue horse battery", "pw = Tr0ub4dor&3".
//!
//! The cue must be linked to what follows ("is", "was", "'s", a colon or an
//! equals sign), so "password reset" or "passwords are rotated" are left
//! alone. The secret runs to the end of the clause, at most
//! [`MAX_WORDS`] words. Whether it really is a password ("the password is on
//! the sticky note") is the LLM check's call, so every match is likely.

use std::ops::Range;
use std::sync::LazyLock;

use regex::Regex;

use super::super::detector::{Detector, Scan};
use super::super::model::{Confidence, Redaction, SecretKind};
use super::super::words::{Word, gap};

/// Words that announce a password.
const CUES: [&str; 6] = ["password", "passphrase", "passwd", "pwd", "pw", "passcode"];
/// Words between the cue and the secret ("is", "'s", "will be", "is now").
const LINKS: [&str; 7] = ["is", "was", "s", "be", "will", "now", "its"];
/// The most link words skipped after a cue.
const MAX_LINKS: usize = 3;
/// The most words a password runs to.
const MAX_WORDS: usize = 4;
/// Words that end the clause the password is in.
const STOPS: [&str; 7] = ["and", "but", "so", "then", "which", "because", "or"];
/// Characters that end a clause when they end a word.
const CLAUSE_END: [char; 7] = ['.', ',', ';', '!', '?', ')', '"'];

/// Whitespace-separated chunks: a password keeps its punctuation.
static CHUNKS: LazyLock<Option<Regex>> = LazyLock::new(|| Regex::new(r"\S+").ok());

/// Finds passwords after a cue.
pub(crate) struct PasswordDetector;

impl Detector for PasswordDetector {
    fn kind(&self) -> SecretKind {
        SecretKind::Password
    }

    fn detect(&self, scan: &Scan<'_>) -> Vec<Redaction> {
        (0..scan.words.len())
            .filter(|&at| scan.words.get(at).is_some_and(|word| word.is_any(&CUES)))
            .filter_map(|at| secret_after(scan, at))
            .map(|range| Redaction::new(SecretKind::Password, range, Confidence::Likely))
            .collect()
    }
}

/// The secret linked to the cue at word `cue`, if any.
fn secret_after(scan: &Scan<'_>, cue: usize) -> Option<Range<usize>> {
    let words = &scan.words;
    let first = words.get(cue + 1)?;
    let links = words
        .iter()
        .skip(cue + 1)
        .take(MAX_LINKS)
        .take_while(|word| word.is_any(&LINKS))
        .count();
    let typed_link = gap(scan.text, words.get(cue)?, first).contains([':', '=']);
    if links == 0 && !typed_link {
        return None;
    }
    let secret: &Word<'_> = words.get(cue + 1 + links)?;
    let end = clause_end(scan.text, secret.range.start);
    (end > secret.range.start).then_some(secret.range.start..end)
}

/// Where the password starting at byte `start` ends: after at most
/// [`MAX_WORDS`] chunks, before a clause word, or at clause punctuation.
fn clause_end(text: &str, start: usize) -> usize {
    let (Some(rest), Some(chunks)) = (text.get(start..), CHUNKS.as_ref()) else {
        return start;
    };
    let mut end = start;
    for (taken, chunk) in chunks.find_iter(rest).enumerate() {
        let word = chunk.as_str();
        if taken == MAX_WORDS || STOPS.iter().any(|stop| word.eq_ignore_ascii_case(stop)) {
            break;
        }
        let kept = word.trim_end_matches(CLAUSE_END);
        if !kept.is_empty() {
            end = start + chunk.start() + kept.len();
        }
        if kept.len() < word.len() {
            break;
        }
    }
    end
}

#[cfg(test)]
mod tests {
    use super::*;

    fn secrets(text: &str) -> Vec<&str> {
        PasswordDetector
            .detect(&Scan::new(text))
            .into_iter()
            .filter_map(|found| text.get(found.range))
            .collect()
    }

    #[test]
    fn the_clause_after_a_linked_cue_is_the_secret() {
        let cases: [(&str, &[&str]); 7] = [
            ("the password is hunter2, then log in", &["hunter2"]),
            (
                "wifi passphrase: blue horse battery",
                &["blue horse battery"],
            ),
            ("pw = Tr0ub4dor&3 ok", &["Tr0ub4dor&3 ok"]),
            ("the password's correct horse and more", &["correct horse"]),
            ("password reset is tomorrow", &[]),
            ("passwords rotate monthly", &[]),
            ("the password is.", &[]),
        ];
        for (text, expected) in cases {
            assert_eq!(secrets(text), expected, "{text}");
        }
    }
}
