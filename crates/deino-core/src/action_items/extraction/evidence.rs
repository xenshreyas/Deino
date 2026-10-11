//! Evidence: the transcript as numbered lines (`[s12 03:41] Maya: …`) for the
//! prompt, and [`Transcript::locate`], which checks the line the model cited
//! against its quote so "open at this moment" never points at the wrong
//! words, or at nothing.

use deino_domain::{Millis, SegmentId};
use deino_llm::extraction::transcript_line;

use super::assignee::normalize;

/// The share of a quote's words a line must contain to be its source.
const QUOTE_MATCH: f64 = 0.6;

/// One transcript line as the prompt shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TranscriptLine {
    /// The segment it is.
    pub segment: SegmentId,
    /// Where it starts in the recording.
    pub start: Millis,
    /// Who said it, as the prompt names them.
    pub speaker: String,
    /// What was said.
    pub text: String,
}

/// A meeting's transcript, numbered for citation.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Transcript {
    /// The lines, in time order; line `n` has the id `s<n>`.
    pub lines: Vec<TranscriptLine>,
}

impl Transcript {
    /// The prompt's transcript: one `[s<n> mm:ss] Speaker: text` per line.
    pub fn render(&self) -> String {
        self.lines
            .iter()
            .enumerate()
            .map(|(index, line)| {
                transcript_line(index, line.start.get(), &line.speaker, &line.text)
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// The segment an item's evidence is in: the cited line when its words
    /// fit the quote (or there is no quote); else the line that holds the
    /// quote; else the cited line if it exists. `None` when the citation is
    /// not a line and the quote is nowhere.
    pub fn locate(&self, cited: Option<&str>, quote: &str) -> Option<SegmentId> {
        let cited = cited.and_then(|id| self.line(id));
        let quote = normalize(quote);
        if quote.is_empty() {
            return cited.map(|line| line.segment);
        }
        if let Some(line) = cited.filter(|line| overlap(&quote, &line.text) >= QUOTE_MATCH) {
            return Some(line.segment);
        }
        self.holding(&quote).or(cited).map(|line| line.segment)
    }

    /// The line an id such as `s12` (or `12`) names.
    fn line(&self, id: &str) -> Option<&TranscriptLine> {
        let id = id.trim();
        let number = id.strip_prefix(['s', 'S']).unwrap_or(id);
        self.lines.get(number.parse::<usize>().ok()?)
    }

    /// The first line that holds most of `quote`'s words.
    fn holding(&self, quote: &str) -> Option<&TranscriptLine> {
        let mut best: Option<(f64, &TranscriptLine)> = None;
        for line in &self.lines {
            let score = overlap(quote, &line.text);
            if score >= QUOTE_MATCH && best.is_none_or(|(top, _)| score > top) {
                best = Some((score, line));
            }
        }
        best.map(|(_, line)| line)
    }
}

/// The share of `quote`'s words (already normalised) found in `text`.
fn overlap(quote: &str, text: &str) -> f64 {
    let text = normalize(text);
    let words: Vec<&str> = text.split(' ').collect();
    let quoted: Vec<&str> = quote.split(' ').collect();
    let found = quoted.iter().filter(|word| words.contains(word)).count();
    #[allow(
        clippy::cast_precision_loss,
        reason = "word counts of one line are far below 2^52"
    )]
    let share = found as f64 / quoted.len().max(1) as f64;
    share
}

#[cfg(test)]
mod tests {
    use deino_domain::FakeClock;
    use deino_testkit::at;

    use super::*;

    fn transcript() -> Transcript {
        let line = |start: u32, speaker: &str, text: &str| TranscriptLine {
            segment: SegmentId::generate(&FakeClock::new(at("2026-10-08T09:00:00Z"))),
            start: Millis(start),
            speaker: speaker.into(),
            text: text.into(),
        };
        Transcript {
            lines: vec![
                line(3_000, "Maya Chen", "Morning. Quick one from me."),
                line(65_000, "Diego", "Can someone add a last modified at field?"),
                line(
                    72_500,
                    "Maya Chen",
                    "Sure, I can have it on the endpoint by end of day tomorrow.",
                ),
            ],
        }
    }

    #[test]
    fn lines_are_numbered_with_their_minute_and_second() {
        let rendered = transcript().render();

        assert_eq!(
            rendered.lines().nth(2),
            Some(
                "[s2 01:12] Maya Chen: Sure, I can have it on the endpoint by end of day tomorrow."
            )
        );
    }

    #[test]
    fn a_citation_that_fits_its_quote_is_kept() {
        let transcript = transcript();

        let found = transcript.locate(
            Some("s2"),
            "I can have it on the endpoint by end of day tomorrow",
        );

        assert_eq!(found, Some(transcript.lines[2].segment));
    }

    #[test]
    fn a_wrong_or_invalid_citation_moves_to_the_line_holding_the_quote() {
        let transcript = transcript();
        let quote = "I can have it on the endpoint";

        assert_eq!(
            transcript.locate(Some("s0"), quote),
            Some(transcript.lines[2].segment)
        );
        assert_eq!(
            transcript.locate(Some("s99"), quote),
            Some(transcript.lines[2].segment)
        );
        assert_eq!(
            transcript.locate(Some("line two"), quote),
            Some(transcript.lines[2].segment)
        );
    }

    #[test]
    fn an_invalid_citation_without_a_quote_found_is_dropped() {
        let transcript = transcript();

        assert_eq!(transcript.locate(Some("s99"), "words nobody said"), None);
        assert_eq!(transcript.locate(None, ""), None);
        assert_eq!(transcript.locate(Some("12"), ""), None);
    }

    #[test]
    fn a_valid_citation_with_a_paraphrased_quote_is_kept() {
        let transcript = transcript();

        assert_eq!(
            transcript.locate(Some("1"), "please add the field"),
            Some(transcript.lines[1].segment)
        );
    }
}
