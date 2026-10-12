//! [`parse_answer`]: the model's text with `[s12]` markers, as runs of text
//! and the segments each cites.
//!
//! A marker is one to a few references in square brackets, `[s12]`,
//! `[s3, s4]`, `[S7 04:31]`. Each resolves through the meeting's
//! [`SegmentIndex`]; references to lines that do not exist (`[s999]`) are
//! dropped with their marker, so a stored citation always names a real
//! segment. Brackets that are not markers (`[redacted]`) stay as text.

use std::sync::LazyLock;

use regex::Regex;

use super::{AnswerCitation, AnswerPart};
use crate::enhance::{MAX_CITATIONS, SegmentIndex};

/// One `[s12]`, `[s3, s4]` or `[s7 04:31]` marker.
static MARKER: LazyLock<Option<Regex>> = LazyLock::new(|| {
    let reference = r"[sS]\d+(?:\s+\d{1,2}:\d{2}(?::\d{2})?)?";
    Regex::new(&format!(
        r"\[\s*({reference}(?:\s*[,;]\s*{reference})*)\s*\]"
    ))
    .ok()
});

/// `text` as runs, each followed by the real segments its markers cite.
pub fn parse_answer(text: &str, index: &SegmentIndex) -> Vec<AnswerPart> {
    let Some(marker) = MARKER.as_ref() else {
        return vec![AnswerPart::plain(text.trim())];
    };
    let mut runs = Runs::default();
    let mut from = 0;
    for found in marker.captures_iter(text) {
        let (Some(whole), Some(list)) = (found.get(0), found.get(1)) else {
            continue;
        };
        runs.text(text.get(from..whole.start()).unwrap_or_default());
        runs.cite(&references(list.as_str()), index);
        from = whole.end();
    }
    runs.text(text.get(from..).unwrap_or_default());
    runs.finish()
}

fn references(list: &str) -> Vec<String> {
    list.split([',', ';'])
        .map(|reference| reference.trim().to_owned())
        .collect()
}

/// The runs built so far.
#[derive(Default)]
struct Runs {
    done: Vec<AnswerPart>,
    current: AnswerPart,
}

impl Runs {
    /// More text: a new run when the current one already cites something.
    /// Blank text between two markers joins nothing.
    fn text(&mut self, text: &str) {
        let cited = !self.current.citations.is_empty();
        if text.is_empty() || (cited && text.trim().is_empty()) {
            return;
        }
        if cited {
            self.done.push(std::mem::take(&mut self.current));
        }
        self.current.text.push_str(text);
    }

    /// A marker: its real segments join the current run's citations.
    fn cite(&mut self, references: &[String], index: &SegmentIndex) {
        let trimmed = self.current.text.trim_end().len();
        self.current.text.truncate(trimmed);
        for evidence in index.resolve(references) {
            let citation = AnswerCitation {
                segment_id: evidence.segment_id,
                start: evidence.start,
            };
            let citations = &mut self.current.citations;
            if citations.len() < MAX_CITATIONS && !citations.contains(&citation) {
                citations.push(citation);
            }
        }
    }

    fn finish(mut self) -> Vec<AnswerPart> {
        if !self.current.text.is_empty() || !self.current.citations.is_empty() {
            self.done.push(self.current);
        }
        if let Some(first) = self.done.first_mut() {
            first.text = first.text.trim_start().to_owned();
        }
        if let Some(last) = self.done.last_mut() {
            last.text = last.text.trim_end().to_owned();
        }
        self.done
    }
}

#[cfg(test)]
mod tests {
    use deino_domain::{Millis, SegmentId};

    use super::*;
    use crate::enhance::TranscriptLine;

    fn index() -> SegmentIndex {
        let lines: Vec<TranscriptLine> = (1..=4)
            .map(|reference| TranscriptLine {
                reference,
                segment_id: uuid::Uuid::from_u128(u128::from(reference))
                    .to_string()
                    .parse::<SegmentId>()
                    .unwrap(),
                start: Millis(reference * 1_000),
                end: Millis(reference * 1_000 + 900),
                speaker: "Maya".into(),
                text: "text".into(),
            })
            .collect();
        SegmentIndex::new(&lines)
    }

    fn starts(part: &AnswerPart) -> Vec<u32> {
        part.citations
            .iter()
            .map(|c| c.start.get() / 1_000)
            .collect()
    }

    #[test]
    fn markers_become_the_citations_of_the_text_before_them() {
        let parts = parse_answer("Beta holds [s2]. Pricing is open [s3, S4 00:04].", &index());

        let texts: Vec<&str> = parts.iter().map(|p| p.text.as_str()).collect();
        assert_eq!(texts, ["Beta holds", ". Pricing is open", "."]);
        assert_eq!(
            (starts(&parts[0]), starts(&parts[1])),
            (vec![2], vec![3, 4])
        );
    }

    #[test]
    fn invented_references_are_dropped_with_their_marker() {
        let parts = parse_answer("A fact [s999] and [s1; s999].", &index());

        assert_eq!(parts[0].text, "A fact and");
        assert_eq!(starts(&parts[0]), [1]);
        assert_eq!(parts[1].text, ".");
    }

    #[test]
    fn other_brackets_and_plain_answers_stay_text() {
        let parts = parse_answer("  The code was [redacted]. ", &index());

        assert_eq!(parts, [AnswerPart::plain("The code was [redacted].")]);
    }

    #[test]
    fn adjacent_markers_keep_at_most_three_distinct_citations() {
        let parts = parse_answer("Many [s1][s1][s2] [s3] [s4]", &index());

        assert_eq!(parts.len(), 1);
        assert_eq!(starts(&parts[0]), [1, 2, 3]);
    }
}
