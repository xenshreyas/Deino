//! [`DbKey`]: the 256-bit key an encrypted database is opened with (PRV-02,
//! ADR 0038).
//!
//! The key is random, lives in the Keychain as 64 hex digits and is handed to
//! SQLCipher as a raw key (`x'…'`), so no passphrase derivation runs on open.
//! Every copy this crate makes is wiped on drop, and the type has no `Debug`,
//! `Display` or `Clone`, so it cannot reach a log by accident.

use zeroize::Zeroizing;

/// Bytes in a database key: AES-256.
pub const DB_KEY_BYTES: usize = 32;

/// Hex digits in a key's text form.
const HEX_CHARS: usize = DB_KEY_BYTES * 2;

/// Why a key could not be made or read.
#[derive(Debug, thiserror::Error)]
pub enum KeyError {
    /// The OS random source failed (its error carries only an OS code).
    #[error("could not generate a database key")]
    Random,
    /// A stored key is not 64 hex digits.
    #[error("the saved database key is not valid")]
    Malformed,
}

/// A database encryption key. Zeroized on drop; never printed.
pub struct DbKey(Zeroizing<[u8; DB_KEY_BYTES]>);

impl DbKey {
    /// A new key from the OS random source.
    pub fn generate() -> Result<Self, KeyError> {
        let mut bytes = Zeroizing::new([0_u8; DB_KEY_BYTES]);
        getrandom::fill(bytes.as_mut_slice()).map_err(|_| KeyError::Random)?;
        Ok(Self(bytes))
    }

    /// Parses the 64-hex-digit text form (either case) that
    /// [`DbKey::to_hex`] writes.
    pub fn from_hex(text: &str) -> Result<Self, KeyError> {
        let digits = text.trim().as_bytes();
        if digits.len() != HEX_CHARS {
            return Err(KeyError::Malformed);
        }
        let mut bytes = Zeroizing::new([0_u8; DB_KEY_BYTES]);
        for (byte, pair) in bytes.iter_mut().zip(digits.chunks_exact(2)) {
            let [high, low] = pair else {
                return Err(KeyError::Malformed);
            };
            *byte = (nibble(*high)? << 4) | nibble(*low)?;
        }
        Ok(Self(bytes))
    }

    /// The key as 64 lowercase hex digits, for the Keychain.
    pub fn to_hex(&self) -> Zeroizing<String> {
        let mut text = Zeroizing::new(String::with_capacity(HEX_CHARS));
        for byte in self.0.iter() {
            text.push(hex_digit(byte >> 4));
            text.push(hex_digit(byte & 0x0f));
        }
        text
    }

    /// The SQLCipher raw-key literal, `"x'<hex>'"`, for `PRAGMA key` and
    /// `ATTACH … KEY`.
    pub(crate) fn sql_literal(&self) -> Zeroizing<String> {
        let hex = self.to_hex();
        let mut literal = Zeroizing::new(String::with_capacity(HEX_CHARS + 5));
        literal.push_str("\"x'");
        literal.push_str(&hex);
        literal.push_str("'\"");
        literal
    }

    /// The raw-key form bound as a parameter, `x'<hex>'` (no outer quotes).
    pub(crate) fn bound_value(&self) -> Zeroizing<String> {
        let hex = self.to_hex();
        let mut value = Zeroizing::new(String::with_capacity(HEX_CHARS + 3));
        value.push_str("x'");
        value.push_str(&hex);
        value.push('\'');
        value
    }
}

impl PartialEq for DbKey {
    /// Compares in constant time, so a timing side channel cannot leak a key.
    fn eq(&self, other: &Self) -> bool {
        self.0
            .iter()
            .zip(other.0.iter())
            .fold(0_u8, |difference, (a, b)| difference | (a ^ b))
            == 0
    }
}

impl Eq for DbKey {}

/// The lowercase hex digit for a nibble (`0..16`).
fn hex_digit(nibble: u8) -> char {
    // A nibble is always below 16, so the fallback never runs.
    char::from_digit(u32::from(nibble), 16).unwrap_or('0')
}

/// The value of one hex digit.
fn nibble(digit: u8) -> Result<u8, KeyError> {
    match digit {
        b'0'..=b'9' => Ok(digit - b'0'),
        b'a'..=b'f' => Ok(digit - b'a' + 10),
        b'A'..=b'F' => Ok(digit - b'A' + 10),
        _ => Err(KeyError::Malformed),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_key_round_trips_through_hex() {
        let key = DbKey::generate().unwrap();
        let text = key.to_hex();
        assert_eq!(text.len(), HEX_CHARS);
        assert!(
            text.chars()
                .all(|digit| matches!(digit, '0'..='9' | 'a'..='f'))
        );
        assert!(DbKey::from_hex(&text).unwrap() == key);
        assert!(DbKey::from_hex(&text.to_uppercase()).unwrap() == key);
    }

    #[test]
    fn two_generated_keys_differ() {
        assert!(DbKey::generate().unwrap() != DbKey::generate().unwrap());
    }

    #[test]
    fn malformed_text_is_rejected() {
        let long = "a".repeat(HEX_CHARS + 2);
        let bad = "g".repeat(HEX_CHARS);
        for text in ["", "abcd", long.as_str(), bad.as_str()] {
            assert!(matches!(DbKey::from_hex(text), Err(KeyError::Malformed)));
        }
    }

    #[test]
    fn the_sql_forms_wrap_the_hex() {
        let key = DbKey::from_hex(&"0f".repeat(DB_KEY_BYTES)).unwrap();
        let hex = "0f".repeat(DB_KEY_BYTES);
        assert_eq!(*key.sql_literal(), format!("\"x'{hex}'\""));
        assert_eq!(*key.bound_value(), format!("x'{hex}'"));
    }
}
