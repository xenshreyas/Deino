//! Finding the corrections in a text, left to right.
//!
//! At each word, the longest exact phrase of the [`Lexicon`] that starts
//! there wins (a known name matches too, and is kept as heard). Otherwise the
//! best fuzzy name does, by Jaro-Winkler similarity of the words' letters:
//!
//! - a name of several words needs [`MULTI_WORD_SIMILARITY`] (TRN-05), over
//!   as many words as it has or one more ("Shrey as Patel");
//! - a one-word name needs [`SINGLE_WORD_SIMILARITY`], and only a capitalised
//!   word of at least [`MIN_SINGLE_WORD_LETTERS`] letters qualifies: the
//!   engine capitalises names, so "hunted" never becomes "Hunter";
//! - the heard words must start with the name's letter, have about as many
//!   letters ([`MAX_LETTER_GAP`]), and neither may be a prefix of the other,
//!   so a plural, a following word or a dropped ending never becomes the name.
//!
//! Matched words are consumed, so no two corrections overlap.

use std::ops::Range;

use super::lexicon::{FuzzyName, Lexicon, Outcome};
use super::tokens::{Token, fold, joined, letters};

/// The Jaro-Winkler similarity a misheard name of several words needs.
pub const MULTI_WORD_SIMILARITY: f64 = 0.92;

/// The similarity a misheard one-word name needs: stricter, because one
/// short word has fewer letters to tell a name from a common word.
pub const SINGLE_WORD_SIMILARITY: f64 = 0.95;

/// The fewest letters a word needs before it may be taken for a name.
pub const MIN_SINGLE_WORD_LETTERS: usize = 5;

/// The most letters a misheard name may have more or fewer than the name.
pub const MAX_LETTER_GAP: usize = 2;

/// One correction: tokens `tokens` become `text`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Fix {
    /// The tokens replaced, in order.
    pub tokens: Range<usize>,
    /// What replaces their cores.
    pub text: String,
}

/// A match: how many tokens it covers, and its fix when it changes anything.
type Hit = (usize, Option<Fix>);

/// The corrections `lexicon` makes to `text`, in order, never overlapping.
pub(super) fn find(text: &str, tokens: &[Token], lexicon: &Lexicon) -> Vec<Fix> {
    let folded: Vec<String> = tokens.iter().map(|t| fold(t.core_of(text))).collect();
    let scan = Scan {
        text,
        tokens,
        folded: &folded,
        lexicon,
    };
    let mut fixes = Vec::new();
    let mut at = 0;
    while at < tokens.len() {
        let (covered, fix) = scan
            .exact(at)
            .or_else(|| scan.fuzzy(at))
            .unwrap_or((1, None));
        fixes.extend(fix);
        at += covered.max(1);
    }
    fixes
}

/// One text being scanned.
struct Scan<'a> {
    text: &'a str,
    tokens: &'a [Token],
    folded: &'a [String],
    lexicon: &'a Lexicon,
}

impl Scan<'_> {
    /// The longest exact phrase starting at token `at`.
    fn exact(&self, at: usize) -> Option<Hit> {
        let first = self.folded.get(at)?;
        self.lexicon.phrases(first).iter().find_map(|phrase| {
            let range = at..at + phrase.words.len();
            let same = self.folded.get(range.clone())? == phrase.words.as_slice();
            if !same || !joined(self.tokens, range.clone()) {
                return None;
            }
            Some((phrase.words.len(), self.fix(range, &phrase.outcome)))
        })
    }

    /// What `outcome` makes of tokens `range`, unless they already read so.
    fn fix(&self, range: Range<usize>, outcome: &Outcome) -> Option<Fix> {
        let heard = self.heard(&range);
        let text = match outcome {
            Outcome::Keep => return None,
            Outcome::Spell(text) => text.clone(),
            Outcome::Replace(text) => capitalised_like(text, heard),
        };
        (text != heard).then_some(Fix {
            tokens: range,
            text,
        })
    }

    /// The text from the first core in `range` to the end of the last.
    fn heard(&self, range: &Range<usize>) -> &str {
        let start = self.tokens.get(range.start).map(|t| t.core.start);
        let last = range.end.checked_sub(1).and_then(|i| self.tokens.get(i));
        match (start, last) {
            (Some(start), Some(last)) => self.text.get(start..last.core.end).unwrap_or_default(),
            _ => "",
        }
    }

    /// The most similar fuzzy name starting at token `at`.
    fn fuzzy(&self, at: usize) -> Option<Hit> {
        let letter = letters(self.folded.get(at)?).chars().next()?;
        let best = self
            .lexicon
            .names(letter)
            .iter()
            .flat_map(|name| windows(name).map(move |size| (name, size)))
            .filter_map(|(name, size)| Some((self.score(at, size, name)?, size, name)))
            .max_by(|a, b| a.0.total_cmp(&b.0))?;
        let (_, size, name) = best;
        let fix = Fix {
            tokens: at..at + size,
            text: name.text.clone(),
        };
        Some((size, Some(fix)))
    }

    /// How similar `size` tokens from `at` are to `name`, if similar enough.
    fn score(&self, at: usize, size: usize, name: &FuzzyName) -> Option<f64> {
        let range = at..at + size;
        if !joined(self.tokens, range.clone()) {
            return None;
        }
        let heard: String = self.folded.get(range)?.iter().map(|w| letters(w)).collect();
        if name.words == 1 && !self.may_be_a_name(at, &heard) {
            return None;
        }
        let gap = heard.chars().count().abs_diff(name.key.chars().count());
        if gap > MAX_LETTER_GAP || heard.starts_with(&name.key) || name.key.starts_with(&heard) {
            return None;
        }
        let similarity = strsim::jaro_winkler(&heard, &name.key);
        let bar = if name.words == 1 {
            SINGLE_WORD_SIMILARITY
        } else {
            MULTI_WORD_SIMILARITY
        };
        (similarity >= bar).then_some(similarity)
    }

    /// True when token `at` looks like a name: capitalised and long enough.
    fn may_be_a_name(&self, at: usize, heard: &str) -> bool {
        let capitalised = self
            .tokens
            .get(at)
            .and_then(|token| token.core_of(self.text).chars().next())
            .is_some_and(char::is_uppercase);
        capitalised && heard.chars().count() >= MIN_SINGLE_WORD_LETTERS
    }
}

/// The window sizes tried for `name`: its own length, and one more word for
/// a name of several words split by the engine.
fn windows(name: &FuzzyName) -> impl Iterator<Item = usize> {
    let extra = usize::from(name.words > 1);
    name.words..=name.words + extra
}

/// `text`, with a capital first letter when `heard` has one and `text` does not.
fn capitalised_like(text: &str, heard: &str) -> String {
    let heard_upper = heard.chars().next().is_some_and(char::is_uppercase);
    let mut chars = text.chars();
    match chars.next() {
        Some(first) if heard_upper && first.is_lowercase() => {
            first.to_uppercase().chain(chars).collect()
        }
        _ => text.to_owned(),
    }
}
