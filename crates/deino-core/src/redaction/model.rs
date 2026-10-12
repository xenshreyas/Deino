//! The redaction vocabulary: what kind of secret a span is ([`SecretKind`]),
//! how sure the detector is ([`Confidence`]) and the span itself
//! ([`Redaction`]).

use std::ops::Range;

/// What a redacted span is replaced with in notes input and exports.
pub const REDACTED: &str = "[redacted]";

/// The kinds of secret Deino redacts (NOT-09).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SecretKind {
    /// A password or passphrase said or typed after a cue ("the password is …").
    Password,
    /// An API key: a known provider prefix, or a long high-entropy token.
    ApiKey,
    /// A payment card number that passes the Luhn check.
    CardNumber,
    /// A US Social Security number.
    Ssn,
    /// A one-time code in context ("the code is 482913").
    OneTimeCode,
}

impl SecretKind {
    /// Every kind, in detection order.
    pub const ALL: [Self; 5] = [
        Self::Password,
        Self::ApiKey,
        Self::CardNumber,
        Self::Ssn,
        Self::OneTimeCode,
    ];

    /// The kind's name in the LLM check's prompt.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Password => "password",
            Self::ApiKey => "api key",
            Self::CardNumber => "card number",
            Self::Ssn => "social security number",
            Self::OneTimeCode => "one-time code",
        }
    }
}

/// How sure a detector is that a span is a secret.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Confidence {
    /// Context-dependent ("the password is on the sticky note"): the
    /// optional LLM check may reject it. Redacted when there is no check.
    Likely,
    /// Self-evident from its shape (a Luhn-valid card, a `sk-ant-` key):
    /// always redacted, never sent to the LLM check.
    Certain,
}

/// One secret found in a text: its byte range, kind and confidence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Redaction {
    /// What it is.
    pub kind: SecretKind,
    /// Where it is, in bytes of the text it was found in (on char
    /// boundaries).
    pub range: Range<usize>,
    /// How sure the detector is.
    pub confidence: Confidence,
}

impl Redaction {
    /// A redaction of `kind` over `range`.
    pub const fn new(kind: SecretKind, range: Range<usize>, confidence: Confidence) -> Self {
        Self {
            kind,
            range,
            confidence,
        }
    }

    /// True when the two spans share at least one byte.
    pub fn overlaps(&self, other: &Self) -> bool {
        self.range.start < other.range.end && other.range.start < self.range.end
    }
}

/// `text` with every span of `redactions` replaced by [`REDACTED`].
/// Overlapping spans are merged first, so each secret is replaced once.
pub fn apply(text: &str, redactions: &[Redaction]) -> String {
    let mut ranges: Vec<Range<usize>> = redactions.iter().map(|r| r.range.clone()).collect();
    ranges.sort_by_key(|range| range.start);
    let mut out = String::with_capacity(text.len());
    let mut cursor = 0;
    for range in merge(ranges) {
        let (Some(before), true) = (text.get(cursor..range.start), range.end <= text.len()) else {
            continue;
        };
        out.push_str(before);
        out.push_str(REDACTED);
        cursor = range.end;
    }
    out.push_str(text.get(cursor..).unwrap_or_default());
    out
}

/// Sorted ranges with every overlapping or touching pair joined.
fn merge(sorted: Vec<Range<usize>>) -> Vec<Range<usize>> {
    let mut merged: Vec<Range<usize>> = Vec::with_capacity(sorted.len());
    for range in sorted {
        match merged.last_mut() {
            Some(last) if range.start <= last.end => last.end = last.end.max(range.end),
            _ => merged.push(range),
        }
    }
    merged
}

#[cfg(test)]
mod tests {
    use super::*;

    fn span(range: Range<usize>) -> Redaction {
        Redaction::new(SecretKind::Password, range, Confidence::Likely)
    }

    #[test]
    fn spans_become_the_marker_and_the_rest_is_kept() {
        let text = "pin 1234 and 5678 end";

        let out = apply(text, &[span(13..17), span(4..8)]);

        assert_eq!(out, "pin [redacted] and [redacted] end");
    }

    #[test]
    fn overlapping_spans_are_replaced_once() {
        let out = apply("abcdefgh", &[span(1..4), span(3..6)]);

        assert_eq!(out, "a[redacted]gh");
    }

    #[test]
    fn no_spans_keep_the_text() {
        assert_eq!(apply("plain", &[]), "plain");
    }
}
