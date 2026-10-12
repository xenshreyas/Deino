//! US Social Security numbers: digits grouped 3-2-4 (`123-45-6789`), or
//! nine digits read out after "social security" or "SSN". Numbers the SSA
//! never issues (area 000, 666 or 9xx; group 00; serial 0000) are skipped.

use super::super::detector::{Detector, Scan};
use super::super::digits::DigitRun;
use super::super::model::{Confidence, Redaction, SecretKind};
use super::super::words::cue_before;

/// How an SSN is written.
const SHAPE: [usize; 3] = [3, 2, 4];
/// Digits in an SSN.
const DIGITS: usize = 9;
/// Words that say a number is an SSN.
const CUES: [&str; 3] = ["social", "ssn", "security"];
/// How many words before the number a cue may be.
const CUE_WINDOW: usize = 6;

/// Finds Social Security numbers.
pub(crate) struct SsnDetector;

impl Detector for SsnDetector {
    fn kind(&self) -> SecretKind {
        SecretKind::Ssn
    }

    fn detect(&self, scan: &Scan<'_>) -> Vec<Redaction> {
        scan.runs
            .iter()
            .filter_map(|run| confidence(scan, run).map(|sure| (run, sure)))
            .map(|(run, sure)| Redaction::new(SecretKind::Ssn, run.range(), sure))
            .collect()
    }
}

/// How sure the run is an SSN, or `None` when it is not one.
fn confidence(scan: &Scan<'_>, run: &DigitRun) -> Option<Confidence> {
    let digits = run.digits();
    if digits.len() != DIGITS || !issuable(&digits) {
        return None;
    }
    if run.shape() == SHAPE {
        return Some(Confidence::Certain);
    }
    cue_before(&scan.words, run.first_word, CUE_WINDOW, &CUES).then_some(Confidence::Likely)
}

/// True when nine digits could be an issued SSN.
fn issuable(digits: &str) -> bool {
    let (Some(area), Some(group), Some(serial)) =
        (digits.get(..3), digits.get(3..5), digits.get(5..))
    else {
        return false;
    };
    area != "000" && area != "666" && !area.starts_with('9') && group != "00" && serial != "0000"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unissued_numbers_are_not_ssns() {
        let cases = [
            ("123456789", true),
            ("000456789", false),
            ("666456789", false),
            ("912456789", false),
            ("123006789", false),
            ("123450000", false),
        ];
        for (digits, valid) in cases {
            assert_eq!(issuable(digits), valid, "{digits}");
        }
    }
}
