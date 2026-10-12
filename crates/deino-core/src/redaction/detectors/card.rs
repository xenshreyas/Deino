//! Payment card numbers: 13 to 19 digits, on a card network's prefix, that
//! pass the Luhn check. Tried on digit-group boundaries, so a card read out
//! digit by digit, or followed by its expiry, is still found; an order
//! number that fails Luhn is not.

use std::ops::Range;

use super::super::detector::{Detector, Scan};
use super::super::digits::DigitRun;
use super::super::model::{Confidence, Redaction, SecretKind};

/// The shortest card number (some Visa and Maestro).
const MIN_DIGITS: usize = 13;
/// The longest card number (ISO/IEC 7812).
const MAX_DIGITS: usize = 19;

/// Finds card numbers.
pub(crate) struct CardDetector;

impl Detector for CardDetector {
    fn kind(&self) -> SecretKind {
        SecretKind::CardNumber
    }

    fn detect(&self, scan: &Scan<'_>) -> Vec<Redaction> {
        scan.runs
            .iter()
            .flat_map(cards_in)
            .map(|range| Redaction::new(SecretKind::CardNumber, range, Confidence::Certain))
            .collect()
    }
}

/// The byte ranges of the card numbers in `run`, longest first from each
/// group, never overlapping.
fn cards_in(run: &DigitRun) -> Vec<Range<usize>> {
    let mut found = Vec::new();
    let mut start = 0;
    while let Some(first) = run.groups.get(start) {
        let Some(end) = longest_card_from(run, start) else {
            start += 1;
            continue;
        };
        let last = run
            .groups
            .get(end)
            .map_or(first.range.end, |group| group.range.end);
        found.push(first.range.start..last);
        start = end + 1;
    }
    found
}

/// The index of the last group of the longest card number that starts at
/// group `start`.
fn longest_card_from(run: &DigitRun, start: usize) -> Option<usize> {
    let mut digits = String::new();
    let mut best = None;
    for (index, group) in run.groups.iter().enumerate().skip(start) {
        digits.push_str(&group.digits);
        if digits.len() > MAX_DIGITS {
            break;
        }
        if digits.len() >= MIN_DIGITS && is_card(&digits) {
            best = Some(index);
        }
    }
    best
}

/// True for a number on a card network's prefix that passes Luhn.
fn is_card(digits: &str) -> bool {
    on_network(digits) && luhn(digits)
}

/// Visa (4), Mastercard (51–55, 2221–2720), Amex, Diners and JCB (3),
/// Discover, Union Pay and Maestro (5, 6). Numbers starting with 0, 1, 7,
/// 8 or 9 are not cards.
fn on_network(digits: &str) -> bool {
    match digits.as_bytes().first() {
        Some(b'3'..=b'6') => true,
        Some(b'2') => digits
            .get(..4)
            .and_then(|prefix| prefix.parse::<u16>().ok())
            .is_some_and(|prefix| (2221..=2720).contains(&prefix)),
        _ => false,
    }
}

/// The Luhn (mod 10) check over ASCII digits; false for anything else.
pub(crate) fn luhn(digits: &str) -> bool {
    let mut sum = 0;
    for (position, byte) in digits.bytes().rev().enumerate() {
        if !byte.is_ascii_digit() {
            return false;
        }
        let digit = u32::from(byte - b'0');
        sum += match (position % 2 == 1, digit * 2) {
            (true, doubled) if doubled > 9 => doubled - 9,
            (true, doubled) => doubled,
            (false, _) => digit,
        };
    }
    !digits.is_empty() && sum % 10 == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn luhn_accepts_valid_numbers_only() {
        let cases = [
            ("4242424242424242", true),
            ("378282246310005", true),
            ("5555555555554444", true),
            ("4242424242424241", false),
            ("12345678901234", false),
            ("42a2", false),
            ("", false),
        ];
        for (digits, valid) in cases {
            assert_eq!(luhn(digits), valid, "{digits}");
        }
    }

    #[test]
    fn networks_are_recognised_by_prefix() {
        let cases = [
            ("4242424242424242", true),
            ("2223003122003222", true),
            ("2000000000000000", false),
            ("1234567812345670", false),
            ("9999999999999995", false),
        ];
        for (digits, valid) in cases {
            assert_eq!(on_network(digits), valid, "{digits}");
        }
    }
}
