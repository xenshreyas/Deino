//! Proof Key for Code Exchange (RFC 7636, S256) and random states.
//!
//! A desktop app cannot keep a client secret, so the code it receives on
//! the loopback redirect is useless without the verifier only this process
//! knows.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use sha2::{Digest, Sha256};

use super::OAuthError;

/// Random bytes behind a verifier: 32 bytes is 256 bits, which encodes to
/// 43 characters, the minimum RFC 7636 allows.
const VERIFIER_BYTES: usize = 32;

/// Random bytes behind a `state`.
const STATE_BYTES: usize = 16;

/// A PKCE pair: the secret verifier and its public S256 challenge.
#[derive(Clone, PartialEq, Eq)]
pub struct Pkce {
    verifier: String,
    challenge: String,
}

impl Pkce {
    /// A fresh pair from the operating system's random source.
    pub fn generate() -> Result<Self, OAuthError> {
        Ok(Self::from_verifier(random_token(VERIFIER_BYTES)?))
    }

    /// The pair for a known verifier (tests use RFC 7636's example).
    pub fn from_verifier(verifier: String) -> Self {
        let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
        Self {
            verifier,
            challenge,
        }
    }

    /// The verifier, sent only with the code to the token endpoint.
    pub fn verifier(&self) -> &str {
        &self.verifier
    }

    /// The challenge, sent with the sign-in page.
    pub fn challenge(&self) -> &str {
        &self.challenge
    }
}

impl std::fmt::Debug for Pkce {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("Pkce(<redacted>)")
    }
}

/// A random `state` that ties the redirect to this sign-in.
pub fn random_state() -> Result<String, OAuthError> {
    random_token(STATE_BYTES)
}

/// `bytes` random bytes as base64url without padding.
fn random_token(bytes: usize) -> Result<String, OAuthError> {
    let mut buffer = vec![0_u8; bytes];
    getrandom::fill(&mut buffer).map_err(|_| OAuthError::Random)?;
    Ok(URL_SAFE_NO_PAD.encode(buffer))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_challenge_matches_rfc_7636_appendix_b() {
        let pkce = Pkce::from_verifier("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk".into());
        assert_eq!(
            pkce.challenge(),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }

    #[test]
    fn generated_verifiers_are_long_unique_and_url_safe() {
        let (a, b) = (Pkce::generate().unwrap(), Pkce::generate().unwrap());

        assert_eq!(a.verifier().len(), 43);
        assert_ne!(a.verifier(), b.verifier());
        let url_safe = |c: char| c.is_ascii_alphanumeric() || c == '-' || c == '_';
        assert!(a.verifier().chars().all(url_safe));
        assert_eq!(format!("{a:?}"), "Pkce(<redacted>)");
    }
}
