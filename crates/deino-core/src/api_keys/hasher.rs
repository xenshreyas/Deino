//! The key's secret: where it comes from ([`TokenSource`]) and how it is
//! kept ([`SecretHasher`]): argon2id with the crate's defaults (19 MiB, two
//! passes), a fresh salt per key, stored as a PHC string.

use argon2::Argon2;
use argon2::password_hash::rand_core::{OsRng, RngCore};
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};

/// How many random bytes a secret has (256 bits).
pub const SECRET_BYTES: usize = 32;

/// Makes new secrets.
pub trait TokenSource: Send + Sync {
    /// A new secret, as lowercase hex.
    fn secret(&self) -> Result<String, String>;
}

/// Hashes secrets and checks them against a hash.
pub trait SecretHasher: Send + Sync {
    /// The hash to store for `secret`.
    fn hash(&self, secret: &str) -> Result<String, String>;
    /// True when `secret` is the one `hash` was made from.
    fn verify(&self, secret: &str, hash: &str) -> bool;
}

/// Secrets from the operating system's random source.
#[derive(Clone, Copy, Debug, Default)]
pub struct OsTokens;

impl TokenSource for OsTokens {
    fn secret(&self) -> Result<String, String> {
        let mut bytes = [0_u8; SECRET_BYTES];
        OsRng
            .try_fill_bytes(&mut bytes)
            .map_err(|error| error.to_string())?;
        let mut hex = String::with_capacity(SECRET_BYTES * 2);
        for byte in bytes {
            for nibble in [byte >> 4, byte & 0x0f] {
                hex.push(char::from_digit(u32::from(nibble), 16).unwrap_or('0'));
            }
        }
        Ok(hex)
    }
}

/// argon2id with the crate's default parameters.
#[derive(Clone, Copy, Debug, Default)]
pub struct Argon2Hasher;

impl SecretHasher for Argon2Hasher {
    fn hash(&self, secret: &str) -> Result<String, String> {
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default()
            .hash_password(secret.as_bytes(), &salt)
            .map(|hash| hash.to_string())
            .map_err(|error| error.to_string())
    }

    fn verify(&self, secret: &str, hash: &str) -> bool {
        PasswordHash::new(hash).is_ok_and(|parsed| {
            Argon2::default()
                .verify_password(secret.as_bytes(), &parsed)
                .is_ok()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_secret_verifies_against_its_own_hash_only() {
        let secret = OsTokens.secret().unwrap();
        assert_eq!(secret.len(), SECRET_BYTES * 2);
        let hash = Argon2Hasher.hash(&secret).unwrap();

        assert!(hash.starts_with("$argon2id$"));
        assert!(!hash.contains(&secret));
        assert!(Argon2Hasher.verify(&secret, &hash));
        assert!(!Argon2Hasher.verify("not-the-secret", &hash));
        assert!(!Argon2Hasher.verify(&secret, "not a hash"));
    }

    #[test]
    fn two_secrets_differ() {
        assert_ne!(OsTokens.secret().unwrap(), OsTokens.secret().unwrap());
    }
}
