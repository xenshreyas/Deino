//! The transcript as the LLM reads it, and how a long one is cut into parts.
//!
//! Every line carries a short reference, `s<n>` for the n-th segment of the
//! meeting, and its time: `[s12 04:31] Maya: we hold the beta date`. The
//! references are global, so a part's notes cite the same `s12` the merge
//! and the validation see (PARITY_PLAN NOT-02: "keep evidence ids through
//! the reduce step").

use deino_domain::{Millis, SegmentId};

/// A meeting longer than this is noted in parts (map-reduce).
pub const LONG_MEETING: Millis = Millis(30 * 60 * 1000);
/// The span of one part of a long meeting (about 20 minutes, the card's window).
pub const PART_SPAN: Millis = Millis(20 * 60 * 1000);
/// A last part shorter than this joins the one before it.
const SHORT_TAIL: Millis = Millis(5 * 60 * 1000);

/// One segment of the transcript, ready for a prompt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TranscriptLine {
    /// Its 1-based position in the meeting's transcript: the `n` of `s<n>`.
    pub reference: u32,
    /// The segment.
    pub segment_id: SegmentId,
    /// Where it starts in the recording.
    pub start: Millis,
    /// Where it ends.
    pub end: Millis,
    /// Who said it, by name when known.
    pub speaker: String,
    /// What was said.
    pub text: String,
}

impl TranscriptLine {
    /// `s12`.
    pub fn reference_label(&self) -> String {
        format!("s{}", self.reference)
    }

    /// `[s12 04:31] Maya: we hold the beta date`.
    pub fn render(&self) -> String {
        format!(
            "[{} {}] {}: {}",
            self.reference_label(),
            clock(self.start),
            self.speaker,
            self.text.trim()
        )
    }
}

/// The lines, one per row.
pub fn render_lines(lines: &[TranscriptLine]) -> String {
    lines
        .iter()
        .map(TranscriptLine::render)
        .collect::<Vec<_>>()
        .join("\n")
}

/// `04:31`, or `1:04:31` from an hour on.
pub fn clock(at: Millis) -> String {
    let seconds = at.get() / 1000;
    let (hours, minutes, seconds) = (seconds / 3600, seconds / 60 % 60, seconds % 60);
    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes:02}:{seconds:02}")
    }
}

/// How to cut a transcript: the longest part, in time and in characters
/// (what the model's context window allows).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PartLimits {
    /// The longest span of one part.
    pub span: Millis,
    /// The most transcript characters one prompt may carry.
    pub chars: usize,
}

/// The transcript in parts: one part when the meeting is short and fits,
/// else consecutive parts of at most `limits` each (never empty).
pub fn parts(lines: &[TranscriptLine], limits: PartLimits) -> Vec<&[TranscriptLine]> {
    if lines.is_empty() {
        return Vec::new();
    }
    if span(lines) <= LONG_MEETING && chars(lines) <= limits.chars {
        return vec![lines];
    }
    let mut cuts = Vec::new();
    let mut start = 0;
    while start < lines.len() {
        let end = part_end(lines, start, limits);
        cuts.push(start..end);
        start = end;
    }
    join_short_tail(&mut cuts, lines, limits.chars);
    cuts.into_iter()
        .filter_map(|range| lines.get(range))
        .collect()
}

/// The end (exclusive) of the part starting at `start`: at least one line.
fn part_end(lines: &[TranscriptLine], start: usize, limits: PartLimits) -> usize {
    let Some(first) = lines.get(start) else {
        return start;
    };
    let mut used = 0;
    let mut end = start;
    for line in lines.iter().skip(start) {
        let size = line_chars(line);
        let too_late = line.start.get().saturating_sub(first.start.get()) >= limits.span.get();
        if end > start && (too_late || used + size > limits.chars) {
            break;
        }
        used += size;
        end += 1;
    }
    end
}

/// Merges a last part shorter than [`SHORT_TAIL`] into the one before, when
/// the result still fits.
fn join_short_tail(
    cuts: &mut Vec<std::ops::Range<usize>>,
    lines: &[TranscriptLine],
    max_chars: usize,
) {
    let [.., before, last] = cuts.as_slice() else {
        return;
    };
    let (before, last) = (before.clone(), last.clone());
    let tail = lines.get(last.clone()).unwrap_or_default();
    let joined = lines.get(before.start..last.end).unwrap_or_default();
    if span(tail) < SHORT_TAIL && chars(joined) <= max_chars {
        cuts.truncate(cuts.len() - 2);
        cuts.push(before.start..last.end);
    }
}

/// From the first line's start to the last line's end.
pub fn span(lines: &[TranscriptLine]) -> Millis {
    match (lines.first(), lines.last()) {
        (Some(first), Some(last)) => Millis(last.end.get().saturating_sub(first.start.get())),
        _ => Millis(0),
    }
}

/// The characters the lines take in a prompt.
fn chars(lines: &[TranscriptLine]) -> usize {
    lines.iter().map(line_chars).sum()
}

fn line_chars(line: &TranscriptLine) -> usize {
    // The reference, time and speaker add about 16 characters to the text.
    line.text.len() + line.speaker.len() + 16
}

#[cfg(test)]
mod transcript_tests;
