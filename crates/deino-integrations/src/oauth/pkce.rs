//! PKCE (RFC 7636) and the random values of a sign-in.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

use super::OAuthError;

/// Random bytes in a verifier: 32 bytes are 43 base64url characters, the
/// shortest verifier RFC 7636 §4.1 allows, with 256 bits of entropy.
const VERIFIER_BYTES: usize = 32;

/// A PKCE pair: the verifier stays in Deino, the challenge goes to the
/// browser. One per sign-in, never reused.
pub struct Pkce {
    verifier: Zeroizing<String>,
    challenge: String,
}

impl Pkce {
    /// A fresh pair from the OS's randomness.
    pub fn generate() -> Result<Self, OAuthError> {
        Ok(Self::from_verifier(&random_token(VERIFIER_BYTES)?))
    }

    /// The pair for a known verifier (RFC 7636 appendix B's test vector).
    pub fn from_verifier(verifier: &str) -> Self {
        let digest = Sha256::digest(verifier.as_bytes());
        Self {
            verifier: Zeroizing::new(verifier.to_owned()),
            challenge: URL_SAFE_NO_PAD.encode(digest),
        }
    }

    /// The verifier, sent once with the code exchange.
    pub fn verifier(&self) -> &str {
        &self.verifier
    }

    /// The S256 challenge, sent in the authorization URL.
    pub fn challenge(&self) -> &str {
        &self.challenge
    }
}

/// `bytes` random bytes from the OS, base64url-encoded without padding:
/// verifiers and the `state` that ties a callback to its sign-in.
pub fn random_token(bytes: usize) -> Result<String, OAuthError> {
    let mut buffer = Zeroizing::new(vec![0_u8; bytes]);
    getrandom::fill(&mut buffer).map_err(|_| OAuthError::Random)?;
    Ok(URL_SAFE_NO_PAD.encode(&*buffer))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_challenge_matches_rfc_7636_appendix_b() {
        let pkce = Pkce::from_verifier("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk");
        assert_eq!(
            pkce.challenge(),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }

    #[test]
    fn generated_verifiers_are_long_enough_and_never_repeat() {
        let first = Pkce::generate().unwrap();
        let second = Pkce::generate().unwrap();

        assert_eq!(first.verifier().len(), 43);
        assert_ne!(first.verifier(), second.verifier());
        let unreserved = |c: char| c.is_ascii_alphanumeric() || "-._~".contains(c);
        assert!(first.verifier().chars().all(unreserved));
    }
}
