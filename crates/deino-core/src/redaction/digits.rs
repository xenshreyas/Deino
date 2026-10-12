//! Digit runs: numbers as they are typed ("4242 4242", "123-45-6789") and
//! as speech-to-text writes them when they are read out ("four two four
//! two", "double four", "four oh two").
//!
//! A run is a sequence of digit words joined only by spaces, hyphens or
//! commas. Each word is a [`DigitGroup`], so detectors can check how a
//! number was grouped (an SSN is 3-2-4) or try card numbers on group
//! boundaries.

use std::ops::Range;

use super::words::{Word, gap};

/// Spoken digits, in value order.
const SPOKEN: [&str; 10] = [
    "zero", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine",
];
/// Words that repeat the next digit ("double four" is 44).
const REPEATS: [(&str, usize); 2] = [("double", 2), ("triple", 3)];

/// One word of a digit run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct DigitGroup {
    /// The digits it stands for.
    pub digits: String,
    /// Its byte range in the text.
    pub range: Range<usize>,
}

/// A run of digit words.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct DigitRun {
    /// Its groups, in order.
    pub groups: Vec<DigitGroup>,
    /// The index of its first word in the text's words.
    pub first_word: usize,
    /// The index of its last word.
    last_word: usize,
}

impl DigitRun {
    /// Every digit of the run.
    pub fn digits(&self) -> String {
        self.groups
            .iter()
            .map(|group| group.digits.as_str())
            .collect()
    }

    /// The byte range from the first group to the last.
    pub fn range(&self) -> Range<usize> {
        let start = self.groups.first().map_or(0, |group| group.range.start);
        let end = self.groups.last().map_or(start, |group| group.range.end);
        start..end
    }

    /// How many digits each group has.
    pub fn shape(&self) -> Vec<usize> {
        self.groups.iter().map(|group| group.digits.len()).collect()
    }
}

/// Every digit run of `text`, whose words are `words`.
pub(super) fn digit_runs(text: &str, words: &[Word<'_>]) -> Vec<DigitRun> {
    let mut runs: Vec<DigitRun> = Vec::new();
    let mut index = 0;
    while index < words.len() {
        let Some((group, used)) = group_at(words, index) else {
            index += 1;
            continue;
        };
        let last_word = index + used - 1;
        match runs.last_mut() {
            Some(run) if continues(text, words, run, index) => {
                run.groups.push(group);
                run.last_word = last_word;
            }
            _ => runs.push(DigitRun {
                groups: vec![group],
                first_word: index,
                last_word,
            }),
        }
        index += used;
    }
    runs
}

/// True when the word at `index` directly follows `run`, separated only
/// by spaces, hyphens or commas.
fn continues(text: &str, words: &[Word<'_>], run: &DigitRun, index: usize) -> bool {
    let (Some(previous), Some(current)) = (words.get(run.last_word), words.get(index)) else {
        return false;
    };
    run.last_word + 1 == index
        && gap(text, previous, current)
            .chars()
            .all(|c| c.is_whitespace() || c == '-' || c == ',')
}

/// The digit group starting at word `index`, and how many words it used.
fn group_at(words: &[Word<'_>], index: usize) -> Option<(DigitGroup, usize)> {
    let word = words.get(index)?;
    if let Some(digits) = digits_of(word) {
        let group = DigitGroup {
            digits,
            range: word.range.clone(),
        };
        return Some((group, 1));
    }
    let times = REPEATS
        .iter()
        .find_map(|(cue, times)| word.is(cue).then_some(*times))?;
    let next = words.get(index + 1)?;
    let digit = digits_of(next).filter(|digit| digit.len() == 1)?;
    let group = DigitGroup {
        digits: digit.repeat(times),
        range: word.range.start..next.range.end,
    };
    Some((group, 2))
}

/// The digits a word stands for: its own ASCII digits, or a spoken digit
/// ("oh" is zero).
fn digits_of(word: &Word<'_>) -> Option<String> {
    if word.text.bytes().all(|byte| byte.is_ascii_digit()) {
        return Some(word.text.to_owned());
    }
    if word.is("oh") {
        return Some("0".to_owned());
    }
    SPOKEN
        .iter()
        .position(|spoken| word.is(spoken))
        .map(|value| value.to_string())
}

#[cfg(test)]
mod tests {
    use super::super::words::words;
    use super::*;

    fn runs(text: &str) -> Vec<(String, Vec<usize>)> {
        digit_runs(text, &words(text))
            .into_iter()
            .map(|run| (run.digits(), run.shape()))
            .collect()
    }

    /// A run's digits and group sizes.
    type Expected<'a> = (&'a str, &'a [usize]);

    #[test]
    fn typed_and_spoken_digits_form_runs() {
        let cases: [(&str, &[Expected<'_>]); 6] = [
            ("4242 4242", &[("42424242", &[4, 4])]),
            ("123-45-6789", &[("123456789", &[3, 2, 4])]),
            ("four two, four two", &[("4242", &[1, 1, 1, 1])]),
            ("double four oh one", &[("4401", &[2, 1, 1])]),
            ("call 555 then 12", &[("555", &[3]), ("12", &[2])]),
            ("in 2024. 1234", &[("2024", &[4]), ("1234", &[4])]),
        ];
        for (text, expected) in cases {
            let expected: Vec<(String, Vec<usize>)> = expected
                .iter()
                .map(|(digits, shape)| ((*digits).to_owned(), shape.to_vec()))
                .collect();
            assert_eq!(runs(text), expected, "{text}");
        }
    }

    #[test]
    fn a_run_spans_its_groups() {
        let text = "card four two 9";
        let run = &digit_runs(text, &words(text))[0];

        assert_eq!(run.range(), 5..15);
        assert_eq!(run.first_word, 1);
    }
}
