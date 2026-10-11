//! What the holdback knows about Them: voice spans, closed and open text,
//! and the two questions it asks of them (was Them active around a Me
//! segment, and did Them say the same words).

use std::collections::VecDeque;

use deino_stt::Millis;

use super::HoldbackPolicy;
use super::overlap;
use crate::vad::VoiceSpan;

/// A Them segment's time and words.
#[derive(Debug)]
struct ThemText {
    start: Millis,
    end: Millis,
    tokens: Vec<String>,
}

/// Them's recent activity, forgotten once it is far behind the Me lane.
#[derive(Debug, Default)]
pub(super) struct ThemEvidence {
    texts: VecDeque<ThemText>,
    open: Option<ThemText>,
    voices: VecDeque<VoiceSpan>,
    voice_open: Option<Millis>,
}

impl ThemEvidence {
    /// Them started speaking at `at`.
    pub(super) fn voice_started(&mut self, at: Millis) {
        self.voice_open = Some(at);
    }

    /// Them spoke during `span`.
    pub(super) fn voice(&mut self, span: VoiceSpan) {
        self.voice_open = None;
        self.voices.push_back(span);
    }

    /// Them's open segment now reads `text` (empty: none is open).
    pub(super) fn live(&mut self, start: Millis, text: &str) {
        let tokens = overlap::tokens(text);
        self.open = (!tokens.is_empty()).then_some(ThemText {
            start,
            end: start,
            tokens,
        });
    }

    /// Them closed a segment.
    pub(super) fn closed(&mut self, start: Millis, end: Millis, text: &str) {
        self.open = None;
        self.texts.push_back(ThemText {
            start,
            end,
            tokens: overlap::tokens(text),
        });
    }

    /// Whether Them was active around `[start, end]`.
    pub(super) fn is_risky(&self, policy: &HoldbackPolicy, start: Millis, end: Millis) -> bool {
        let margin = policy.risk_margin;
        let (from, to) = (start.saturating_sub(margin), end + margin);
        let near = |s: Millis, e: Millis| s <= to && e >= from;
        self.voices.iter().any(|v| near(v.start, v.end))
            || self.voice_open.is_some_and(|since| since <= to)
            || self.texts.iter().any(|t| near(t.start, t.end))
            || self.open.as_ref().is_some_and(|t| t.start <= to)
    }

    /// Whether Them's text near `start` says what `tokens` says.
    pub(super) fn matches(
        &self,
        policy: &HoldbackPolicy,
        start: Millis,
        tokens: &[String],
    ) -> bool {
        let window = policy.window;
        let nearby: Vec<&ThemText> = self
            .texts
            .iter()
            .chain(self.open.as_ref())
            .filter(|t| t.start.0.abs_diff(start.0) <= window.0)
            .collect();
        candidates(&nearby, policy.merge)
            .iter()
            .any(|text| overlap::overlaps(tokens, text))
    }

    /// Forgets activity that ended before `horizon`.
    pub(super) fn forget_before(&mut self, horizon: Millis) {
        while self.texts.front().is_some_and(|t| t.end < horizon) {
            self.texts.pop_front();
        }
        while self.voices.front().is_some_and(|v| v.end < horizon) {
            self.voices.pop_front();
        }
    }
}

/// Each run of up to `merge` consecutive texts, joined (OpenWhispr's
/// `buildMergedCandidates`), so a Me segment spanning two Them segments
/// still matches.
fn candidates(texts: &[&ThemText], merge: usize) -> Vec<Vec<String>> {
    let mut merged = Vec::new();
    for start in 0..texts.len() {
        let mut joined: Vec<String> = Vec::new();
        for text in texts.iter().skip(start).take(merge.max(1)) {
            joined.extend(text.tokens.iter().cloned());
            merged.push(joined.clone());
        }
    }
    merged
}
