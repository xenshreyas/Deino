//! What a good answer must satisfy (the anarlog `Expectation` enum, MIT).

/// One check on an answer.
#[derive(Clone, Debug)]
pub enum Expectation {
    /// The trimmed answer is exactly one line.
    SingleLine,
    /// The answer has at most this many words.
    MaxWords(usize),
    /// The answer does not contain this text.
    NotContains(&'static str),
    /// The answer mentions at least one of these words (case-insensitive).
    MentionsAny(Vec<String>),
}

impl Expectation {
    fn check(&self, answer: &str) -> Result<(), String> {
        let answer = answer.trim();
        match self {
            Self::SingleLine if answer.lines().count() == 1 => Ok(()),
            Self::SingleLine => Err(format!("expected one line, got {}", answer.lines().count())),
            Self::MaxWords(max) => {
                let words = answer.split_whitespace().count();
                (words <= *max)
                    .then_some(())
                    .ok_or(format!("expected at most {max} words, got {words}"))
            }
            Self::NotContains(needle) if answer.contains(needle) => {
                Err(format!("should not contain {needle:?}"))
            }
            Self::NotContains(_) => Ok(()),
            Self::MentionsAny(words) => {
                let lower = answer.to_lowercase();
                words
                    .iter()
                    .any(|word| lower.contains(word.as_str()))
                    .then_some(())
                    .ok_or(format!(
                        "expected a mention of one of {words:?}, got {answer:?}"
                    ))
            }
        }
    }
}

/// Checks every expectation, reporting the first that fails.
pub fn check(expectations: &[Expectation], answer: &str) -> Result<(), String> {
    expectations
        .iter()
        .try_for_each(|expectation| expectation.check(answer))
}
