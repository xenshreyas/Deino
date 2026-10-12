//! Signing: [Standard Webhooks](https://www.standardwebhooks.com/) headers
//! and Circleback's `x-signature`, from one [`SigningSecret`].
//!
//! - `webhook-id`, `webhook-timestamp` (Unix seconds) and
//!   `webhook-signature: v1,<base64 HMAC-SHA256>` over
//!   `<id>.<timestamp>.<body>`, keyed with the base64-decoded part of the
//!   `whsec_` secret (the spec's own construction).
//! - `x-signature: <hex HMAC-SHA256>` over the raw body, keyed with the
//!   secret exactly as shown (`whsec_…` as UTF-8), which a no-code tool can
//!   check without decoding anything.

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use hmac::{Hmac, Mac};
use sha2::Sha256;

use super::WebhookError;

/// The prefix every secret is shown with.
const PREFIX: &str = "whsec_";
/// Random bytes in a new secret (the spec allows 24 to 64).
const SECRET_BYTES: usize = 32;

type HmacSha256 = Hmac<Sha256>;

/// An endpoint's signing secret: `whsec_` and base64 of its key bytes.
pub struct SigningSecret {
    shown: String,
    key: Vec<u8>,
}

impl SigningSecret {
    /// A new random secret.
    pub fn generate() -> Result<Self, WebhookError> {
        let mut key = vec![0_u8; SECRET_BYTES];
        getrandom::fill(&mut key).map_err(|_| WebhookError::Random)?;
        let shown = format!("{PREFIX}{}", STANDARD.encode(&key));
        Ok(Self { shown, key })
    }

    /// Reads a secret as shown (`whsec_<base64>`).
    pub fn parse(shown: &str) -> Result<Self, WebhookError> {
        let encoded = shown.strip_prefix(PREFIX).ok_or(WebhookError::BadSecret)?;
        let key = STANDARD
            .decode(encoded)
            .map_err(|_| WebhookError::BadSecret)?;
        if key.is_empty() {
            return Err(WebhookError::BadSecret);
        }
        Ok(Self {
            shown: shown.to_owned(),
            key,
        })
    }

    /// The secret as the user sees it, once, and as the Keychain keeps it.
    pub fn expose(&self) -> &str {
        &self.shown
    }

    /// The signature headers for one attempt.
    pub fn sign(&self, message: &SignedMessage<'_>) -> Result<SignatureHeaders, WebhookError> {
        let id = message.id;
        let timestamp = message.timestamp.to_string();
        let mut standard = mac(&self.key)?;
        standard.update(format!("{id}.{timestamp}.").as_bytes());
        standard.update(message.body);
        let mut plain = mac(self.shown.as_bytes())?;
        plain.update(message.body);
        Ok(SignatureHeaders {
            id: id.to_owned(),
            timestamp,
            signature: format!("v1,{}", STANDARD.encode(standard.finalize().into_bytes())),
            x_signature: hex(&plain.finalize().into_bytes()),
        })
    }
}

impl std::fmt::Debug for SigningSecret {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("SigningSecret(…)")
    }
}

/// What one attempt signs.
#[derive(Clone, Copy, Debug)]
pub struct SignedMessage<'a> {
    /// The `webhook-id`: the same on every retry of a delivery.
    pub id: &'a str,
    /// The attempt's time, in Unix seconds.
    pub timestamp: i64,
    /// The exact body bytes sent.
    pub body: &'a [u8],
}

/// The headers that prove a request came from this Deino.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SignatureHeaders {
    /// `webhook-id`.
    pub id: String,
    /// `webhook-timestamp`.
    pub timestamp: String,
    /// `webhook-signature`: `v1,<base64>`.
    pub signature: String,
    /// `x-signature`: hex HMAC-SHA256 of the body.
    pub x_signature: String,
}

impl SignatureHeaders {
    /// The headers as `(name, value)` pairs.
    pub fn pairs(&self) -> [(&'static str, &str); 4] {
        [
            ("webhook-id", &self.id),
            ("webhook-timestamp", &self.timestamp),
            ("webhook-signature", &self.signature),
            ("x-signature", &self.x_signature),
        ]
    }
}

/// An HMAC-SHA256 keyed with `key`. HMAC takes keys of any length, so
/// this never fails in practice; the error is the type's, not ours.
fn mac(key: &[u8]) -> Result<HmacSha256, WebhookError> {
    HmacSha256::new_from_slice(key).map_err(|_| WebhookError::BadSecret)
}

/// Lower-case hex of `bytes`.
fn hex(bytes: &[u8]) -> String {
    let digit = |nibble: u8| char::from_digit(u32::from(nibble), 16).unwrap_or('0');
    bytes
        .iter()
        .flat_map(|byte| [digit(byte >> 4), digit(byte & 0x0f)])
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The Standard Webhooks specification's example (and its reference
    /// libraries' test vector).
    const SPEC_SECRET: &str = "whsec_MfKQ9r8GKYqrTwjUPD8ILPZIo2LaLaSw";
    const SPEC_ID: &str = "msg_p5jXN8AQM9LWM0D4loKWxJek";
    const SPEC_TIMESTAMP: i64 = 1_614_265_330;
    const SPEC_BODY: &str = r#"{"test": 2432232314}"#;
    const SPEC_SIGNATURE: &str = "v1,g0hM9SsE+OTPJTGt/tmIKtSyZlE3uFJELVlNIOLJ1OE=";

    fn spec_headers() -> SignatureHeaders {
        SigningSecret::parse(SPEC_SECRET)
            .unwrap()
            .sign(&SignedMessage {
                id: SPEC_ID,
                timestamp: SPEC_TIMESTAMP,
                body: SPEC_BODY.as_bytes(),
            })
            .unwrap()
    }

    #[test]
    fn the_standard_webhooks_vector_signs_as_the_spec_says() {
        let headers = spec_headers();

        assert_eq!(headers.signature, SPEC_SIGNATURE);
        assert_eq!(headers.id, SPEC_ID);
        assert_eq!(headers.timestamp, "1614265330");
    }

    #[test]
    fn any_change_to_the_id_time_or_body_changes_the_signature() {
        let secret = SigningSecret::parse(SPEC_SECRET).unwrap();
        let sign = |id, timestamp, body: &str| {
            secret
                .sign(&SignedMessage {
                    id,
                    timestamp,
                    body: body.as_bytes(),
                })
                .unwrap()
                .signature
        };

        assert_ne!(sign("msg_other", SPEC_TIMESTAMP, SPEC_BODY), SPEC_SIGNATURE);
        assert_ne!(sign(SPEC_ID, SPEC_TIMESTAMP + 1, SPEC_BODY), SPEC_SIGNATURE);
        assert_ne!(
            sign(SPEC_ID, SPEC_TIMESTAMP, r#"{"test": 2432232315}"#),
            SPEC_SIGNATURE
        );
    }

    #[test]
    fn x_signature_is_the_hex_hmac_of_the_body_keyed_with_the_shown_secret() {
        // Checked against an independent computation (Python's `hmac`).
        let headers = spec_headers();

        assert_eq!(
            headers.x_signature,
            "80ec8a89ce3cd22133a1066caecb4d04fea7467657c8514d717ec42c38a5c94c"
        );
        assert_eq!(headers.x_signature.len(), 64);
    }

    #[test]
    fn hmac_sha256_matches_rfc_4231() {
        let mut mac = mac(b"Jefe").unwrap();
        mac.update(b"what do ya want for nothing?");

        assert_eq!(
            hex(&mac.finalize().into_bytes()),
            "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
        );
    }

    #[test]
    fn generated_secrets_parse_back_and_never_print() {
        let secret = SigningSecret::generate().unwrap();
        let again = SigningSecret::parse(secret.expose()).unwrap();

        assert!(secret.expose().starts_with("whsec_"));
        assert_eq!(again.key, secret.key);
        assert_eq!(again.key.len(), SECRET_BYTES);
        assert_eq!(format!("{secret:?}"), "SigningSecret(…)");
        assert!(SigningSecret::parse("MfKQ9r8GKYqrTwjUPD8ILPZIo2LaLaSw").is_err());
        assert!(SigningSecret::parse("whsec_!!!").is_err());
    }
}
