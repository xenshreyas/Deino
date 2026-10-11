//! [`Pattern`]: the literal text find and replace looks for.
//!
//! Matching is literal (no wildcards) and non-overlapping, left to right.
//! Without "match case" it ignores the case of ASCII letters only, as
//! SQLite's `LIKE` does, so the database's pre-filter and this exact match
//! agree; Deino transcribes English, where that covers every letter.

use std::ops::Range;

/// A literal needle and how to compare it.
#[derive(Clone, Copy, Debug)]
pub(super) struct Pattern<'a> {
    needle: &'a str,
    match_case: bool,
}

impl<'a> Pattern<'a> {
    /// A pattern for `needle`, which must not be empty.
    pub(super) fn new(needle: &'a str, match_case: bool) -> Self {
        Self { needle, match_case }
    }

    /// Where `text` contains the needle, left to right, not overlapping.
    pub(super) fn ranges(&self, text: &str) -> Vec<Range<usize>> {
        let mut found = Vec::new();
        let mut from = 0;
        while let Some(start) = self.find_from(text, from) {
            let end = start + self.needle.len();
            found.push(start..end);
            from = end;
        }
        found
    }

    /// `text` with every match replaced by `with`, and how many there were.
    pub(super) fn replace(&self, text: &str, with: &str) -> (String, usize) {
        let ranges = self.ranges(text);
        let mut replaced = String::with_capacity(text.len());
        let mut cursor = 0;
        // Every range starts and ends on a character boundary, so `get` is `Some`.
        for range in &ranges {
            replaced.push_str(text.get(cursor..range.start).unwrap_or_default());
            replaced.push_str(with);
            cursor = range.end;
        }
        replaced.push_str(text.get(cursor..).unwrap_or_default());
        (replaced, ranges.len())
    }

    /// The first match starting at or after byte `from`.
    fn find_from(&self, text: &str, from: usize) -> Option<usize> {
        if self.needle.is_empty() {
            return None;
        }
        let rest = text.get(from..)?;
        if self.match_case {
            return rest.find(self.needle).map(|offset| from + offset);
        }
        rest.char_indices()
            .map(|(offset, _)| offset)
            .find(|&offset| {
                rest.get(offset..offset + self.needle.len())
                    .is_some_and(|candidate| candidate.eq_ignore_ascii_case(self.needle))
            })
            .map(|offset| from + offset)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_ignore_ascii_case_unless_asked_not_to() {
        let text = "Ship it. SHIP the ship!";

        assert_eq!(Pattern::new("ship", false).ranges(text).len(), 3);
        assert_eq!(Pattern::new("ship", true).ranges(text), [19..23]);
    }

    #[test]
    fn matches_do_not_overlap_and_are_literal() {
        assert_eq!(Pattern::new("aa", false).ranges("aaaa"), [0..2, 2..4]);
        assert_eq!(Pattern::new("a.b", false).ranges("axb a.b"), [4..7]);
    }

    #[test]
    fn replacing_keeps_the_text_around_each_match() {
        let (text, count) = Pattern::new("deino", false).replace("Deino, deino and DEINO.", "Dino");

        assert_eq!(text, "Dino, Dino and Dino.");
        assert_eq!(count, 3);
    }

    #[test]
    fn multibyte_text_is_matched_on_character_boundaries() {
        let (text, count) = Pattern::new("cafe", false).replace("Café, CAFE — café", "bar");

        assert_eq!(text, "Café, bar — café");
        assert_eq!(count, 1);
    }

    #[test]
    fn an_empty_needle_matches_nothing() {
        assert!(Pattern::new("", false).ranges("anything").is_empty());
    }
}
