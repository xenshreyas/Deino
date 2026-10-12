//! [`Redactor`]: the detectors, then the optional check, then `[redacted]`;
//! and [`TextRedactor`], the port the notes input and exports call.

use std::ops::Range;
use std::sync::Arc;

use async_trait::async_trait;

use super::check::{Candidate, NoCheck, SpanCheck, excerpt};
use super::detector::{Detector, Scan, standard};
use super::model::{Confidence, REDACTED, Redaction, apply};

/// Replaces secrets in texts before they reach notes or exports.
///
/// Contract: returns as many texts as it was given, in order; a text
/// without secrets comes back unchanged.
#[async_trait]
pub trait TextRedactor: Send + Sync {
    /// `texts` with every secret replaced by `[redacted]`.
    async fn redact(&self, texts: Vec<String>) -> Vec<String>;
}

/// Redaction off: every text comes back as it was.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoRedaction;

#[async_trait]
impl TextRedactor for NoRedaction {
    async fn redact(&self, texts: Vec<String>) -> Vec<String> {
        texts
    }
}

/// Every detector of [`SecretKind::ALL`](super::SecretKind::ALL), and the
/// check that may reject the likely ones.
pub struct Redactor {
    detectors: Vec<Box<dyn Detector>>,
    check: Arc<dyn SpanCheck>,
}

impl Redactor {
    /// The patterns alone: every candidate is redacted.
    pub fn regex_only() -> Self {
        Self::with_check(Arc::new(NoCheck))
    }

    /// The patterns, with `check` judging the likely candidates.
    pub fn with_check(check: Arc<dyn SpanCheck>) -> Self {
        Self {
            detectors: standard(),
            check,
        }
    }

    /// Every candidate secret in `text`, before the check. A span touching
    /// an earlier `[redacted]` is left out, so redacting twice changes
    /// nothing more.
    pub fn candidates(&self, text: &str) -> Vec<Redaction> {
        let scan = Scan::new(text);
        let markers: Vec<Range<usize>> = text
            .match_indices(REDACTED)
            .map(|(start, marker)| start..start + marker.len())
            .collect();
        self.detectors
            .iter()
            .flat_map(|detector| {
                let kind = detector.kind();
                detector
                    .detect(&scan)
                    .into_iter()
                    .filter(move |span| span.kind == kind)
            })
            .filter(|span| !markers.iter().any(|marker| overlaps(marker, &span.range)))
            .collect()
    }

    /// The likely candidates the check should see: those no certain span
    /// already covers.
    fn to_check(found: &[Vec<Redaction>]) -> Vec<(usize, usize)> {
        let mut asks = Vec::new();
        for (text, spans) in found.iter().enumerate() {
            let certain: Vec<&Redaction> = spans
                .iter()
                .filter(|span| span.confidence == Confidence::Certain)
                .collect();
            let open = spans.iter().enumerate().filter(|(_, span)| {
                span.confidence == Confidence::Likely && !certain.iter().any(|c| c.overlaps(span))
            });
            asks.extend(open.map(|(index, _)| (text, index)));
        }
        asks
    }

    /// Removes the candidates the check rejected.
    async fn judge(&self, texts: &[String], found: &mut [Vec<Redaction>]) {
        let asks = Self::to_check(found);
        if asks.is_empty() {
            return;
        }
        let candidates: Vec<Candidate> = asks
            .iter()
            .filter_map(|&(text, index)| candidate(texts.get(text)?, found.get(text)?.get(index)?))
            .collect();
        let verdicts = self.check.confirm(&candidates).await;
        let mut rejected: Vec<(usize, usize)> = asks
            .into_iter()
            .zip(verdicts.into_iter().chain(std::iter::repeat(true)))
            .filter_map(|(ask, keep)| (!keep).then_some(ask))
            .collect();
        rejected.sort_unstable_by(|a, b| b.cmp(a));
        for (text, index) in rejected {
            if let Some(spans) = found.get_mut(text).filter(|spans| index < spans.len()) {
                spans.remove(index);
            }
        }
        log::debug!(target: "redaction", "{} candidates checked", candidates.len());
    }
}

fn overlaps(a: &Range<usize>, b: &Range<usize>) -> bool {
    a.start < b.end && b.start < a.end
}

fn candidate(text: &str, span: &Redaction) -> Option<Candidate> {
    text.get(span.range.clone())?;
    Some(Candidate {
        kind: span.kind,
        excerpt: excerpt(text, span.range.clone()),
    })
}

#[async_trait]
impl TextRedactor for Redactor {
    async fn redact(&self, texts: Vec<String>) -> Vec<String> {
        let mut found: Vec<Vec<Redaction>> =
            texts.iter().map(|text| self.candidates(text)).collect();
        self.judge(&texts, &mut found).await;
        texts
            .iter()
            .zip(&found)
            .map(|(text, spans)| apply(text, spans))
            .collect()
    }
}

/// Whether the user turned redaction on (Settings → Privacy).
#[async_trait]
pub trait RedactionSetting: Send + Sync {
    /// True when notes input and exports are redacted.
    async fn enabled(&self) -> bool;
}

/// [`TextRedactor`] that redacts only while the setting is on, read at
/// every call so a change applies to the next notes or export.
pub struct SettingRedactor {
    setting: Arc<dyn RedactionSetting>,
    redactor: Arc<dyn TextRedactor>,
}

impl SettingRedactor {
    /// `redactor`, switched by `setting`.
    pub fn new(setting: Arc<dyn RedactionSetting>, redactor: Arc<dyn TextRedactor>) -> Self {
        Self { setting, redactor }
    }
}

#[async_trait]
impl TextRedactor for SettingRedactor {
    async fn redact(&self, texts: Vec<String>) -> Vec<String> {
        if self.setting.enabled().await {
            self.redactor.redact(texts).await
        } else {
            texts
        }
    }
}
