//! The [`Detector`] port: one implementation per [`SecretKind`], each
//! reading a [`Scan`] of the text (its words and digit runs, computed once).

use super::detectors::{
    ApiKeyDetector, CardDetector, OneTimeCodeDetector, PasswordDetector, SsnDetector,
};
use super::digits::{DigitRun, digit_runs};
use super::model::{Redaction, SecretKind};
use super::words::{Word, words};

/// A text prepared for the detectors.
pub(crate) struct Scan<'a> {
    /// The text itself.
    pub text: &'a str,
    /// Its words.
    pub words: Vec<Word<'a>>,
    /// Its digit runs, typed or spoken.
    pub runs: Vec<DigitRun>,
}

impl<'a> Scan<'a> {
    /// Splits `text` into words and digit runs.
    pub fn new(text: &'a str) -> Self {
        let words = words(text);
        let runs = digit_runs(text, &words);
        Self { text, words, runs }
    }
}

/// Finds one kind of secret.
///
/// Contract: every returned range lies inside the text, on char
/// boundaries, and has the detector's [`Detector::kind`].
pub(crate) trait Detector: Send + Sync {
    /// The kind it finds.
    fn kind(&self) -> SecretKind;

    /// Every secret of its kind in `scan`.
    fn detect(&self, scan: &Scan<'_>) -> Vec<Redaction>;
}

/// One detector per [`SecretKind`], in [`SecretKind::ALL`] order.
pub(crate) fn standard() -> Vec<Box<dyn Detector>> {
    SecretKind::ALL.into_iter().map(detector_for).collect()
}

fn detector_for(kind: SecretKind) -> Box<dyn Detector> {
    match kind {
        SecretKind::Password => Box::new(PasswordDetector),
        SecretKind::ApiKey => Box::new(ApiKeyDetector),
        SecretKind::CardNumber => Box::new(CardDetector),
        SecretKind::Ssn => Box::new(SsnDetector),
        SecretKind::OneTimeCode => Box::new(OneTimeCodeDetector),
    }
}
