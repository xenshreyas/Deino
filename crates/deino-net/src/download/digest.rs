//! [`Sha256Digest`]: a pinned file hash.

use std::fmt;
use std::str::FromStr;

/// A SHA-256 digest, parsed from 64 hex digits.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Sha256Digest([u8; 32]);

/// The text is not 64 hex digits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("a SHA-256 digest is 64 hex digits")]
pub struct InvalidDigest;

impl Sha256Digest {
    /// A digest from its raw bytes.
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// The raw bytes.
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl FromStr for Sha256Digest {
    type Err = InvalidDigest;

    fn from_str(hex: &str) -> Result<Self, Self::Err> {
        let hex = hex.as_bytes();
        if hex.len() != 64 {
            return Err(InvalidDigest);
        }
        let mut bytes = [0u8; 32];
        for (byte, pair) in bytes.iter_mut().zip(hex.chunks_exact(2)) {
            let &[high, low] = pair else {
                return Err(InvalidDigest);
            };
            *byte = (nibble(high)? << 4) | nibble(low)?;
        }
        Ok(Self(bytes))
    }
}

/// One hex digit's value.
fn nibble(digit: u8) -> Result<u8, InvalidDigest> {
    match digit {
        b'0'..=b'9' => Ok(digit - b'0'),
        b'a'..=b'f' => Ok(digit - b'a' + 10),
        b'A'..=b'F' => Ok(digit - b'A' + 10),
        _ => Err(InvalidDigest),
    }
}

impl fmt::Display for Sha256Digest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0
            .iter()
            .try_for_each(|byte| write!(formatter, "{byte:02x}"))
    }
}

impl fmt::Debug for Sha256Digest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "Sha256Digest({self})")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EMPTY: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

    #[test]
    fn digests_round_trip_through_hex_in_any_case() {
        let digest: Sha256Digest = EMPTY.to_uppercase().parse().unwrap();
        assert_eq!(digest.to_string(), EMPTY);
    }

    #[test]
    fn anything_but_64_hex_digits_is_refused() {
        assert_eq!("abc".parse::<Sha256Digest>(), Err(InvalidDigest));
        let not_hex = format!("{}zz", &EMPTY[2..]);
        assert_eq!(not_hex.parse::<Sha256Digest>(), Err(InvalidDigest));
    }
}
