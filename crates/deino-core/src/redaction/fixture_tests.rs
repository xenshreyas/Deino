//! The NOT-09 fixture: a transcript with every kind of secret, typed and
//! spoken, and the near-misses that must survive; the LLM check over the
//! mock gateway; and property tests of the redactor.

use std::sync::Arc;

use deino_llm::{MockProvider, ProviderId, Route, Target, mock_gateway};
use proptest::prelude::*;

use super::detectors::luhn;
use super::*;

/// One line per secret: the line, the secret in it, and its kind.
pub(crate) const SECRETS: [(&str, &str, SecretKind); 11] = [
    (
        "Sure, the wifi password is hunter2, then join the guest network.",
        "hunter2",
        SecretKind::Password,
    ),
    (
        "Staging uses sk-ant-api03-Xq7Lm2Pz9Rt4Vb8Nc1Kd6Hs3Jf5Gw0Ya for now.",
        "sk-ant-api03-Xq7Lm2Pz9Rt4Vb8Nc1Kd6Hs3Jf5Gw0Ya",
        SecretKind::ApiKey,
    ),
    (
        "The old AWS key AKIAIOSFODNN7EXAMPLE was rotated.",
        "AKIAIOSFODNN7EXAMPLE",
        SecretKind::ApiKey,
    ),
    (
        "Paste this token q8Zr2LmX9vTn4KbW7pYs3HcJ into the config.",
        "q8Zr2LmX9vTn4KbW7pYs3HcJ",
        SecretKind::ApiKey,
    ),
    (
        "My card is 4242 4242 4242 4242, expiry next June.",
        "4242 4242 4242 4242",
        SecretKind::CardNumber,
    ),
    (
        "It's four two four two, four two four two, four two four two, four two four two.",
        "four two four two, four two four two, four two four two, four two four two",
        SecretKind::CardNumber,
    ),
    (
        "Her social is 123-45-6789 on the form.",
        "123-45-6789",
        SecretKind::Ssn,
    ),
    (
        "My social security number is one two three four five six seven eight nine.",
        "one two three four five six seven eight nine",
        SecretKind::Ssn,
    ),
    (
        "Okay, the code is 482913.",
        "482913",
        SecretKind::OneTimeCode,
    ),
    (
        "Your PIN is four two nine one, don't share it.",
        "four two nine one",
        SecretKind::OneTimeCode,
    ),
    (
        "The verification code is double four oh one seven two.",
        "double four oh one seven two",
        SecretKind::OneTimeCode,
    ),
];

/// Lines that look like secrets but are not; redaction leaves them alone.
pub(crate) const NEAR_MISSES: [&str; 11] = [
    "Order number 4111 1111 1111 1112 shipped on Tuesday.",
    "Order 1234 5678 9012 3456 is late again.",
    "We launched in 2024 and grew again in 2025.",
    "Our code freeze starts in 2026.",
    "Call me at 415 555 2671 or at (212) 555-0199.",
    "Dial 1-800-555-0199 for support.",
    "The zip code is 94107.",
    "Room 4821 is on floor three.",
    "The password reset email went out to everyone.",
    "Meeting 0199f000-0000-7000-8000-000000000002 was moved.",
    "We sold 1,250,000 units in four quarters.",
];

#[tokio::test]
async fn every_fixture_secret_is_redacted() {
    let redactor = Redactor::regex_only();
    for (line, secret, kind) in SECRETS {
        let kinds: Vec<SecretKind> = redactor
            .candidates(line)
            .iter()
            .map(|found| found.kind)
            .collect();
        let out = redactor.redact(vec![line.to_owned()]).await;

        assert!(kinds.contains(&kind), "{line}: found {kinds:?}");
        assert!(!out[0].contains(secret), "{line} → {}", out[0]);
        assert!(out[0].contains(REDACTED), "{line} → {}", out[0]);
    }
}

#[tokio::test]
async fn near_misses_are_left_alone() {
    let lines: Vec<String> = NEAR_MISSES.iter().map(|line| (*line).to_owned()).collect();

    let out = Redactor::regex_only().redact(lines.clone()).await;

    assert_eq!(out, lines);
}

#[tokio::test]
async fn the_marker_replaces_only_the_secret() {
    let out = Redactor::regex_only()
        .redact(vec!["Okay, the code is 482913.".to_owned()])
        .await;

    assert_eq!(out, ["Okay, the code is [redacted]."]);
}

#[tokio::test]
async fn the_local_check_may_reject_likely_spans_only() {
    let mock = Arc::new(MockProvider::new().reply(
        Route::Tagging,
        r#"{"verdicts":[{"number":1,"secret":false}]}"#,
    ));
    let gateway = Arc::new(mock_gateway(mock.clone()).gateway);
    let redactor = Redactor::with_check(Arc::new(LocalModelCheck::new(gateway)));
    let lines = vec![
        "The password is on the sticky note.".to_owned(),
        "My card is 4242 4242 4242 4242.".to_owned(),
    ];

    let out = redactor.redact(lines).await;

    assert_eq!(out[0], "The password is on the sticky note.");
    assert_eq!(out[1], "My card is [redacted].");
    let asked = format!("{:?}", mock.requests());
    assert!(asked.contains("«on the sticky note»"), "{asked}");
    assert!(!asked.contains("4242"), "certain spans are never sent");
}

#[tokio::test]
async fn a_hosted_route_skips_the_check_and_redacts_everything() {
    let mock = Arc::new(MockProvider::new());
    let gateway = Arc::new(mock_gateway(mock.clone()).gateway);
    let cloud = Target {
        provider: ProviderId::Anthropic,
        model: "claude-haiku".to_owned(),
        effort: None,
    };
    gateway
        .router()
        .set(Route::Tagging, Some(cloud))
        .await
        .unwrap();
    let redactor = Redactor::with_check(Arc::new(LocalModelCheck::new(gateway)));

    let out = redactor
        .redact(vec!["The password is on the sticky note.".to_owned()])
        .await;

    assert_eq!(out, ["The password is [redacted]."]);
    assert!(mock.requests().is_empty(), "nothing left the Mac");
}

#[tokio::test]
async fn a_failing_check_redacts_everything() {
    let gateway = Arc::new(mock_gateway(Arc::new(MockProvider::new())).gateway);
    let redactor = Redactor::with_check(Arc::new(LocalModelCheck::new(gateway)));

    let out = redactor
        .redact(vec!["The code is 482913.".to_owned()])
        .await;

    assert_eq!(out, ["The code is [redacted]."]);
}

#[tokio::test]
async fn the_setting_switches_redaction() {
    struct Fixed(bool);
    #[async_trait::async_trait]
    impl RedactionSetting for Fixed {
        async fn enabled(&self) -> bool {
            self.0
        }
    }
    let line = vec!["The code is 482913.".to_owned()];
    let redactor: Arc<dyn TextRedactor> = Arc::new(Redactor::regex_only());

    let off = SettingRedactor::new(Arc::new(Fixed(false)), redactor.clone());
    let on = SettingRedactor::new(Arc::new(Fixed(true)), redactor);

    assert_eq!(off.redact(line.clone()).await, line);
    assert_eq!(on.redact(line).await, ["The code is [redacted]."]);
}

/// A 16-digit Luhn-valid Visa number: 4, then `body` (14 digits), then the check digit.
fn visa(body: &[u8]) -> String {
    let partial: String = std::iter::once('4')
        .chain(body.iter().map(|digit| char::from(b'0' + digit % 10)))
        .collect();
    (0..10)
        .map(|check| format!("{partial}{check}"))
        .find(|number| luhn(number))
        .unwrap_or_default()
}

/// `digits` in groups of four, split by `separator`.
fn grouped(digits: &str, separator: &str) -> String {
    let chars: Vec<char> = digits.chars().collect();
    chars
        .chunks(4)
        .map(|chunk| chunk.iter().collect::<String>())
        .collect::<Vec<_>>()
        .join(separator)
}

proptest! {
    #[test]
    fn every_valid_card_is_redacted(
        body in proptest::collection::vec(0u8..10, 14),
        separator in prop::sample::select(vec![" ", "-", ""]),
    ) {
        let number = grouped(&visa(&body), separator);
        let line = format!("Charge it to {number} please.");
        let found = Redactor::regex_only().candidates(&line);
        prop_assert!(found.iter().any(|span| line.get(span.range.clone()) == Some(number.as_str())));
    }

    #[test]
    fn one_wrong_digit_fails_luhn(body in proptest::collection::vec(0u8..10, 14), at in 0usize..16, delta in 1u8..10) {
        let number = visa(&body);
        let mut digits = number.into_bytes();
        if let Some(digit) = digits.get_mut(at) {
            *digit = b'0' + (*digit - b'0' + delta) % 10;
        }
        prop_assert!(!luhn(&String::from_utf8(digits).unwrap_or_default()));
    }

    #[test]
    fn plain_words_are_never_redacted(words in proptest::collection::vec("[a-z]{1,9}", 0..30)) {
        let line = words.join(" ");
        let cues = ["password", "passphrase", "passwd", "pwd", "pw", "passcode", "code", "codes", "pin", "otp", "mfa", "social", "ssn", "security"];
        prop_assume!(!words.iter().any(|word| cues.contains(&word.as_str())));
        prop_assert!(Redactor::regex_only().candidates(&line).is_empty());
    }

    #[test]
    fn spans_are_inside_the_text_on_char_boundaries(line in "\\PC{0,120}") {
        for span in Redactor::regex_only().candidates(&line) {
            prop_assert!(line.get(span.range.clone()).is_some());
        }
    }

    #[test]
    fn redacting_twice_changes_nothing_more(index in 0usize..SECRETS.len()) {
        let (line, _, _) = SECRETS[index];
        let redactor = Redactor::regex_only();
        let runtime = tokio::runtime::Builder::new_current_thread().build().unwrap();
        let once = runtime.block_on(redactor.redact(vec![line.to_owned()]));
        let twice = runtime.block_on(redactor.redact(once.clone()));
        prop_assert_eq!(once, twice);
    }
}
