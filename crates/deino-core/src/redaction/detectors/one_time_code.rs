//! One-time codes in context: four to eight digits, typed or read out,
//! a few words after "code", "PIN", "OTP" or "verification" ("the code is
//! 482913", "your PIN is four two four two"). Without a cue a number is
//! left alone; so are years and zip, postal, area or country codes.

use super::super::detector::{Detector, Scan};
use super::super::digits::DigitRun;
use super::super::model::{Confidence, Redaction, SecretKind};
use super::super::words::cue_before;

/// The fewest digits of a code.
const MIN_DIGITS: usize = 4;
/// The most digits of a code.
const MAX_DIGITS: usize = 8;
/// Words that say a number is a one-time code.
const CUES: [&str; 8] = [
    "code",
    "codes",
    "otp",
    "pin",
    "passcode",
    "verification",
    "2fa",
    "mfa",
];
/// How many words before the number a cue may be.
const CUE_WINDOW: usize = 5;
/// Words that make "code" a place or a dialling code, not a secret.
const NOT_SECRET: [&str; 4] = ["zip", "postal", "area", "country"];
/// Four-digit numbers in this range read as years, not codes.
const YEARS: std::ops::RangeInclusive<u32> = 1900..=2099;

/// Finds one-time codes.
pub(crate) struct OneTimeCodeDetector;

impl Detector for OneTimeCodeDetector {
    fn kind(&self) -> SecretKind {
        SecretKind::OneTimeCode
    }

    fn detect(&self, scan: &Scan<'_>) -> Vec<Redaction> {
        scan.runs
            .iter()
            .filter(|run| code_shaped(run) && cued(scan, run))
            .map(|run| Redaction::new(SecretKind::OneTimeCode, run.range(), Confidence::Likely))
            .collect()
    }
}

/// True for four to eight digits that are not a year.
fn code_shaped(run: &DigitRun) -> bool {
    let digits = run.digits();
    let is_year = digits.len() == 4 && digits.parse::<u32>().is_ok_and(|n| YEARS.contains(&n));
    (MIN_DIGITS..=MAX_DIGITS).contains(&digits.len()) && !is_year
}

/// True when a cue comes shortly before the run and is not a zip code's.
fn cued(scan: &Scan<'_>, run: &DigitRun) -> bool {
    let at = run.first_word;
    if !cue_before(&scan.words, at, CUE_WINDOW, &CUES) {
        return false;
    }
    !cue_before(&scan.words, at, CUE_WINDOW + 1, &NOT_SECRET)
}
