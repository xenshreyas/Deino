//! Splitting a long transcript into overlapping chunks that fit a local
//! model's context window (ported from the legacy `summary/processor.rs`).
//!
//! Sizes are estimated from characters (about 0.35 tokens per character),
//! and each chunk ends at a sentence or word boundary when one exists past
//! the overlap, so no word is cut and no text is lost between chunks.

/// Estimated tokens per character of English text.
const TOKENS_PER_CHAR: f64 = 0.35;

/// A rough token count of `text`.
pub fn rough_token_count(text: &str) -> usize {
    to_usize((usize_to_f64(text.chars().count()) * TOKENS_PER_CHAR).ceil())
}

fn chars_for(tokens: usize) -> usize {
    to_usize((usize_to_f64(tokens) / TOKENS_PER_CHAR).ceil())
}

#[allow(
    clippy::cast_precision_loss,
    reason = "transcript lengths are far below 2^52 characters"
)]
fn usize_to_f64(value: usize) -> f64 {
    value as f64
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a rounded, non-negative size estimate"
)]
fn to_usize(value: f64) -> usize {
    value.max(0.0) as usize
}

/// Splits `text` into chunks of about `chunk_tokens` tokens that overlap by
/// about `overlap_tokens`. Short text is one chunk; empty text is none.
pub fn chunk_text(text: &str, chunk_tokens: usize, overlap_tokens: usize) -> Vec<String> {
    if text.is_empty() || chunk_tokens == 0 {
        return Vec::new();
    }
    let window = chars_for(chunk_tokens);
    let overlap = chars_for(overlap_tokens);
    let bounds: Vec<usize> = text
        .char_indices()
        .map(|(at, _)| at)
        .chain([text.len()])
        .collect();
    let total = bounds.len() - 1;
    if total <= window {
        return vec![text.to_owned()];
    }
    let mut chunks = Vec::new();
    let mut start = 0;
    loop {
        let end = chunk_end(text, &bounds, (start, (start + window).min(total)), overlap);
        let piece = byte(&bounds, start)
            .zip(byte(&bounds, end))
            .and_then(|(from, to)| text.get(from..to));
        chunks.extend(piece.map(str::to_owned));
        if end >= total {
            return chunks;
        }
        start = end.saturating_sub(overlap).max(start + 1);
    }
}

/// Where a chunk spanning chars `start..end` should end: the last sentence
/// end (`. `), else the last space, past the overlap; else `end` itself.
fn chunk_end(text: &str, bounds: &[usize], (start, end): (usize, usize), overlap: usize) -> usize {
    if end + 1 >= bounds.len() {
        return end;
    }
    let Some(slice) = byte(bounds, start)
        .zip(byte(bounds, end))
        .and_then(|(from, to)| text.get(from..to))
    else {
        return end;
    };
    let past_overlap = |cut: usize| {
        slice
            .get(..cut)
            .map(|head| head.chars().count())
            .filter(|chars| *chars > overlap)
    };
    let sentence = slice.rfind(". ").map(|at| at + 2).and_then(past_overlap);
    let word = slice.rfind(' ').map(|at| at + 1).and_then(past_overlap);
    sentence.or(word).map_or(end, |chars| start + chars)
}

fn byte(bounds: &[usize], char_index: usize) -> Option<usize> {
    bounds.get(char_index).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_after_an_early_sentence_boundary_is_kept() {
        let text = "Intro. LOST_MARKER trailing content ensures chunking";

        let chunks = chunk_text(text, 10, 1);

        assert!(
            chunks.iter().any(|chunk| chunk.contains("LOST_MARKER")),
            "{chunks:?}"
        );
    }

    #[test]
    fn chunks_end_on_character_boundaries() {
        assert_eq!(chunk_text("é ab", 1, 0), ["é ", "ab"]);
    }

    #[test]
    fn chunking_progresses_when_the_overlap_fills_the_window() {
        assert_eq!(chunk_text("abcd", 1, 1), ["abc", "bcd"]);
    }

    #[test]
    fn short_and_empty_texts() {
        assert_eq!(chunk_text("short", 100, 10), ["short"]);
        assert!(chunk_text("", 100, 10).is_empty());
        assert_eq!(rough_token_count("abcdefghij"), 4);
    }
}
