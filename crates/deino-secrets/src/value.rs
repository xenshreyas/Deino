//! A secret's value, validated once and impossible to leak by accident.

use std::fmt;

use zeroize::Zeroizing;

use crate::SecretError;

/// The longest secret Deino accepts, in characters. Real API keys are under
/// 200; the cap only stops a pasted document from landing in the keychain.
pub const MAX_SECRET_CHARS: usize = 4096;

/// A secret's value: trimmed, non-empty and at most [`MAX_SECRET_CHARS`].
///
/// It has no `Display`, `Serialize` or `Clone`, its `Debug` is redacted and
/// its memory is wiped on drop, so it cannot reach a log, an IPC response or
/// a stray copy without a deliberate [`SecretValue::expose`].
pub struct SecretValue(Zeroizing<String>);

impl SecretValue {
    /// Validates user input as a secret.
    pub fn parse(input: &str) -> Result<Self, SecretError> {
        let value = input.trim();
        if value.is_empty() {
            return Err(SecretError::Empty);
        }
        if value.chars().count() > MAX_SECRET_CHARS {
            return Err(SecretError::TooLong);
        }
        Ok(Self(Zeroizing::new(value.to_owned())))
    }

    /// The secret itself, for the one call that sends it to its service.
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for SecretValue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SecretValue(<redacted>)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_are_trimmed() {
        assert_eq!(SecretValue::parse("  sk-123\n").unwrap().expose(), "sk-123");
    }

    #[test]
    fn blank_and_overlong_values_are_rejected() {
        assert!(matches!(SecretValue::parse(" "), Err(SecretError::Empty)));
        let overlong = "k".repeat(MAX_SECRET_CHARS + 1);
        assert!(matches!(
            SecretValue::parse(&overlong),
            Err(SecretError::TooLong)
        ));
    }

    #[test]
    fn debug_output_never_contains_the_value() {
        let value = SecretValue::parse("sk-very-secret").unwrap();
        assert_eq!(format!("{value:?}"), "SecretValue(<redacted>)");
    }
}
