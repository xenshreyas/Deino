//! Whether two transcript texts say the same thing.
//!
//! A port of OpenWhispr's `transcriptsOverlap` (`src/helpers/transcriptText.js`,
//! MIT): texts are lower-cased, stripped of punctuation and split into
//! tokens; they overlap when one contains the other, or, with at least
//! [`MIN_TOKENS`] tokens in the shorter, when the share of the shorter's
//! tokens found in the longer, or their longest common token subsequence,
//! is above [`SIMILARITY`] (CAP-02: "normalized similarity > 0.6").
//!
//! Shared by the live bleed holdback (`deino-recording`'s `bleed`) and the
//! after-meeting echo rule ([`super::echo_text`]).

/// The share of matching tokens above which two texts are the same speech.
pub const SIMILARITY: f32 = 0.6;

/// Below this many tokens only containment counts (short phrases like
/// "yeah okay" match too much by chance).
pub const MIN_TOKENS: usize = 3;

/// Longest token sequence compared with the quadratic LCS (longer texts use
/// token coverage only, which already catches long duplicates).
const LCS_LIMIT: usize = 256;

/// Lower-case alphanumeric tokens, punctuation removed.
pub fn tokens(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|token| !token.is_empty())
        .map(str::to_lowercase)
        .collect()
}

/// The highest similarity measure between `a` and `b`, in `[0, 1]`.
pub fn similarity(a: &[String], b: &[String]) -> f32 {
    let shorter = a.len().min(b.len());
    if shorter == 0 {
        return 0.0;
    }
    if contains(a, b) || contains(b, a) {
        return 1.0;
    }
    if shorter < MIN_TOKENS {
        return 0.0;
    }
    let coverage = ratio(common(a, b), shorter);
    let sequence = if a.len().max(b.len()) <= LCS_LIMIT {
        ratio(longest_common_subsequence(a, b), shorter)
    } else {
        0.0
    };
    coverage.max(sequence)
}

/// Whether `a` and `b` are the same speech.
pub fn overlaps(a: &[String], b: &[String]) -> bool {
    similarity(a, b) > SIMILARITY
}

/// The share of `of`'s tokens found (one-for-one) in `within`, in `[0, 1]`.
pub fn coverage(of: &[String], within: &[String]) -> f32 {
    ratio(common(of, within), of.len())
}

/// Whether `needle` appears as a contiguous token run in `haystack`.
fn contains(haystack: &[String], needle: &[String]) -> bool {
    !needle.is_empty()
        && needle.len() <= haystack.len()
        && haystack
            .windows(needle.len())
            .any(|window| window == needle)
}

/// Tokens of `a` matched one-for-one in `b`.
fn common(a: &[String], b: &[String]) -> usize {
    let mut unmatched: Vec<&String> = b.iter().collect();
    a.iter()
        .filter(|token| {
            unmatched
                .iter()
                .position(|candidate| candidate == token)
                .map(|index| unmatched.swap_remove(index))
                .is_some()
        })
        .count()
}

/// The length of the longest common subsequence of tokens.
fn longest_common_subsequence(a: &[String], b: &[String]) -> usize {
    let mut previous = vec![0_usize; b.len() + 1];
    let mut current = vec![0_usize; b.len() + 1];
    for token in a {
        let mut diagonal = 0;
        for (j, other) in b.iter().enumerate() {
            let up = previous.get(j + 1).copied().unwrap_or_default();
            let left = current.get(j).copied().unwrap_or_default();
            let value = if token == other {
                diagonal + 1
            } else {
                up.max(left)
            };
            diagonal = up;
            if let Some(slot) = current.get_mut(j + 1) {
                *slot = value;
            }
        }
        std::mem::swap(&mut previous, &mut current);
    }
    previous.last().copied().unwrap_or_default()
}

#[allow(
    clippy::cast_precision_loss,
    reason = "token counts are far below f32's exact integer range"
)]
fn ratio(part: usize, whole: usize) -> f32 {
    part as f32 / whole.max(1) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn same(a: &str, b: &str) -> bool {
        overlaps(&tokens(a), &tokens(b))
    }

    #[test]
    fn identical_and_contained_texts_overlap() {
        assert!(same("Let's ship it on Friday.", "let's ship it on friday"));
        assert!(same(
            "ship it on Friday",
            "So, let's ship it on Friday, okay?"
        ));
    }

    #[test]
    fn a_bleed_with_a_few_misheard_words_still_overlaps() {
        assert!(same(
            "the quarterly numbers look good for the northeast region",
            "the quarterly number looks good for northeast regions"
        ));
    }

    #[test]
    fn different_speech_does_not_overlap() {
        assert!(!same(
            "the quarterly numbers look good",
            "can you send me the deck after the call"
        ));
    }

    #[test]
    fn short_phrases_only_match_by_containment() {
        assert!(!same("yeah sure", "sure yeah"));
        assert!(same("okay", "okay"));
        assert!(!same("", "anything"));
    }

    #[test]
    fn coverage_is_the_share_of_the_first_texts_tokens() {
        let (short, long) = (tokens("ship it friday"), tokens("we ship it on friday ok"));
        assert!((coverage(&short, &long) - 1.0).abs() < 1e-6);
        assert!((coverage(&long, &short) - 0.5).abs() < 1e-6);
        assert!(coverage(&[], &long).abs() < 1e-6);
    }

    #[test]
    fn the_threshold_is_strictly_above_sixty_percent() {
        // 3 of 5 tokens shared, in order: exactly 0.6, which is not a match.
        assert!((similarity(&tokens("a b c x y"), &tokens("a b c p q")) - 0.6).abs() < 1e-6);
        assert!(!same("a b c x y", "a b c p q"));
    }
}
