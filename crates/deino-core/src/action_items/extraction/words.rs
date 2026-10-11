//! [`Words`]: a phrase as lower-case words, and the number and month words
//! the due-date and name rules read.

/// Ordinal words up to "twentieth", by day of the month.
const ORDINALS: [&str; 20] = [
    "first",
    "second",
    "third",
    "fourth",
    "fifth",
    "sixth",
    "seventh",
    "eighth",
    "ninth",
    "tenth",
    "eleventh",
    "twelfth",
    "thirteenth",
    "fourteenth",
    "fifteenth",
    "sixteenth",
    "seventeenth",
    "eighteenth",
    "nineteenth",
    "twentieth",
];

/// Counting words up to twelve.
const NUMBERS: [&str; 12] = [
    "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten", "eleven",
    "twelve",
];

/// Month names and their usual abbreviations, by month number.
const MONTHS: [(&str, &str); 12] = [
    ("january", "jan"),
    ("february", "feb"),
    ("march", "mar"),
    ("april", "apr"),
    ("may", "may"),
    ("june", "jun"),
    ("july", "jul"),
    ("august", "aug"),
    ("september", "sep"),
    ("october", "oct"),
    ("november", "nov"),
    ("december", "dec"),
];

/// A phrase split into lower-case words, without punctuation. Hyphens split
/// words, and "twenty third" is joined back into "twenty-third".
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Words(Vec<String>);

impl Words {
    /// The words of `phrase`.
    pub fn of(phrase: &str) -> Self {
        let cleaned: String = phrase
            .to_lowercase()
            .chars()
            .map(|character| {
                if character.is_alphanumeric() || character == '\'' {
                    character
                } else {
                    ' '
                }
            })
            .collect();
        let mut words: Vec<String> = Vec::new();
        for word in cleaned.split_whitespace() {
            match words.last_mut() {
                Some(tens) if is_tens(tens) && ordinal_unit(word).is_some() => {
                    tens.push('-');
                    tens.push_str(word);
                }
                _ => words.push(word.to_owned()),
            }
        }
        Self(words)
    }

    /// The word at `index`, if there is one.
    pub fn get(&self, index: usize) -> Option<&str> {
        self.0.get(index).map(String::as_str)
    }

    /// True when `word` is one of the words.
    pub fn has(&self, word: &str) -> bool {
        self.0.iter().any(|candidate| candidate == word)
    }

    /// True when `run` appears as consecutive words.
    pub fn has_run(&self, run: &[&str]) -> bool {
        self.0
            .windows(run.len())
            .any(|window| window.iter().zip(run).all(|(word, wanted)| word == wanted))
    }

    /// The first word `read` recognises, with its position.
    pub fn find_map<T>(&self, read: impl Fn(&str) -> Option<T>) -> Option<(usize, T)> {
        self.0
            .iter()
            .enumerate()
            .find_map(|(index, word)| read(word).map(|value| (index, value)))
    }

    /// The words, in order.
    #[cfg(test)]
    pub fn iter(&self) -> impl Iterator<Item = &str> {
        self.0.iter().map(String::as_str)
    }
}

fn is_tens(word: &str) -> bool {
    matches!(word, "twenty" | "thirty")
}

/// "first" to "ninth" as 1 to 9.
fn ordinal_unit(word: &str) -> Option<u32> {
    ORDINALS
        .iter()
        .take(9)
        .position(|ordinal| *ordinal == word)
        .and_then(|index| u32::try_from(index + 1).ok())
}

/// The day of the month an ordinal names: "ninth", "twenty-third", "9th".
pub fn ordinal_day(word: &str) -> Option<u32> {
    let day = if let Some(index) = ORDINALS.iter().position(|ordinal| *ordinal == word) {
        u32::try_from(index + 1).ok()?
    } else if word == "thirtieth" {
        30
    } else if let Some((tens, unit)) = word.split_once('-') {
        let tens = match tens {
            "twenty" => 20,
            "thirty" => 30,
            _ => return None,
        };
        tens + ordinal_unit(unit)?
    } else {
        let digits = ["st", "nd", "rd", "th"]
            .iter()
            .find_map(|suffix| word.strip_suffix(suffix))?;
        digits.parse().ok()?
    };
    (1..=31).contains(&day).then_some(day)
}

/// A small count: digits, "a"/"an", or "one" to "twelve".
pub fn number_word(word: &str) -> Option<u64> {
    if matches!(word, "a" | "an") {
        return Some(1);
    }
    NUMBERS
        .iter()
        .position(|number| *number == word)
        .and_then(|index| u64::try_from(index + 1).ok())
        .or_else(|| word.parse().ok().filter(|count| (1..=365).contains(count)))
}

/// The month (1 to 12) a word names, in full or abbreviated ("oct", "sept").
pub fn month_named(word: &str) -> Option<u32> {
    let word = if word == "sept" { "sep" } else { word };
    MONTHS
        .iter()
        .position(|(full, short)| word == *full || word == *short)
        .and_then(|index| u32::try_from(index + 1).ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words_drop_punctuation_and_join_compound_ordinals() {
        let words = Words::of("By Friday, the twenty-third of Oct. (EOD)");

        assert_eq!(
            words.iter().collect::<Vec<_>>(),
            ["by", "friday", "the", "twenty-third", "of", "oct", "eod"]
        );
        assert_eq!(
            Words::of("thirty first").iter().collect::<Vec<_>>(),
            ["thirty-first"]
        );
    }

    #[test]
    fn ordinals_read_as_words_compounds_and_digits() {
        assert_eq!(ordinal_day("ninth"), Some(9));
        assert_eq!(ordinal_day("twenty-third"), Some(23));
        assert_eq!(ordinal_day("thirtieth"), Some(30));
        assert_eq!(ordinal_day("13th"), Some(13));
        assert_eq!(ordinal_day("32nd"), None);
        assert_eq!(ordinal_day("tenth-ish"), None);
    }

    #[test]
    fn counts_and_months_read_words_and_abbreviations() {
        assert_eq!(number_word("three"), Some(3));
        assert_eq!(number_word("a"), Some(1));
        assert_eq!(number_word("14"), Some(14));
        assert_eq!(month_named("sept"), Some(9));
        assert_eq!(month_named("december"), Some(12));
        assert_eq!(month_named("decision"), None);
    }
}
