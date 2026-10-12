//! API keys: tokens on a known provider prefix (certain), and long tokens
//! that mix lower case, upper case and digits with high entropy (likely).

use std::collections::HashMap;
use std::sync::LazyLock;

use regex::Regex;

use super::super::detector::{Detector, Scan};
use super::super::model::{Confidence, Redaction, SecretKind};

/// Provider key formats: Anthropic, OpenAI, GitHub, GitLab, Slack, AWS,
/// Google, Stripe, npm and Hugging Face.
static PROVIDER_KEYS: LazyLock<Option<Regex>> = LazyLock::new(|| {
    Regex::new(concat!(
        r"\b(?:sk-ant-[A-Za-z0-9_\-]{20,}",
        r"|sk-(?:proj-|svcacct-)?[A-Za-z0-9_\-]{20,}",
        r"|gh[pousr]_[A-Za-z0-9]{30,}",
        r"|github_pat_[A-Za-z0-9_]{30,}",
        r"|glpat-[A-Za-z0-9_\-]{20,}",
        r"|xox[abposr]-[A-Za-z0-9\-]{10,}",
        r"|AKIA[0-9A-Z]{16}",
        r"|AIza[0-9A-Za-z_\-]{35}",
        r"|[rsp]k_(?:live|test)_[A-Za-z0-9]{16,}",
        r"|npm_[A-Za-z0-9]{36}",
        r"|hf_[A-Za-z0-9]{30,})",
    ))
    .ok()
});

/// Tokens long enough to be a key.
static LONG_TOKENS: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"[A-Za-z0-9_\-+/=]{24,}").ok());

/// The least Shannon entropy, in bits per character, of a random key.
const MIN_ENTROPY: f64 = 3.5;

/// Finds API keys.
pub(crate) struct ApiKeyDetector;

impl Detector for ApiKeyDetector {
    fn kind(&self) -> SecretKind {
        SecretKind::ApiKey
    }

    fn detect(&self, scan: &Scan<'_>) -> Vec<Redaction> {
        let mut found = matches(&PROVIDER_KEYS, scan.text, Confidence::Certain);
        let random = matches(&LONG_TOKENS, scan.text, Confidence::Likely)
            .into_iter()
            .filter(|token| scan.text.get(token.range.clone()).is_some_and(looks_random))
            .filter(|token| !found.iter().any(|known| known.overlaps(token)));
        found.extend(random.collect::<Vec<_>>());
        found
    }
}

fn matches(pattern: &LazyLock<Option<Regex>>, text: &str, sure: Confidence) -> Vec<Redaction> {
    let Some(pattern) = pattern.as_ref() else {
        return Vec::new();
    };
    pattern
        .find_iter(text)
        .map(|found| Redaction::new(SecretKind::ApiKey, found.range(), sure))
        .collect()
}

/// True for a token with lower case, upper case and digits whose
/// characters are spread like random ones.
fn looks_random(token: &str) -> bool {
    let has = |test: fn(&char) -> bool| token.chars().any(|c| test(&c));
    has(char::is_ascii_lowercase)
        && has(char::is_ascii_uppercase)
        && has(char::is_ascii_digit)
        && entropy(token) >= MIN_ENTROPY
}

/// Shannon entropy of `token`'s characters, in bits per character.
#[expect(
    clippy::cast_precision_loss,
    reason = "token lengths are far below f64's exact integer range"
)]
fn entropy(token: &str) -> f64 {
    let mut counts: HashMap<char, usize> = HashMap::new();
    for character in token.chars() {
        *counts.entry(character).or_default() += 1;
    }
    let length = token.chars().count() as f64;
    counts
        .values()
        .map(|&count| {
            let share = count as f64 / length;
            -share * share.log2()
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn random_tokens_are_told_from_words() {
        let cases = [
            ("q8Zr2LmX9vTn4KbW7pYs3HcJ", true),
            ("aaaaaaaaaaaaaaaaaaaaaaaa1A", false),
            ("quarterly_planning_review_2026", false),
            ("0199f000-0000-7000-8000-000000000002", false),
        ];
        for (token, random) in cases {
            assert_eq!(looks_random(token), random, "{token}");
        }
    }
}
