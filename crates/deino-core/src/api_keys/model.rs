//! API keys as the service and the UI see them, and the token format.

use chrono::{DateTime, Utc};
use deino_domain::ApiKeyId;
use serde::{Deserialize, Serialize};

/// Every token starts with this, so a leaked one is easy to recognise.
pub const TOKEN_PREFIX: &str = "deino_";
/// The longest key name.
pub const MAX_KEY_NAME_CHARS: usize = 60;

/// What a key may do.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type, sqlx::Type,
)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum ApiKeyScope {
    /// Reads only: every `GET`.
    Read,
    /// Reads and writes: every endpoint.
    ReadWrite,
}

impl ApiKeyScope {
    /// True when the key may change the library.
    pub fn can_write(self) -> bool {
        self == Self::ReadWrite
    }
}

/// A key as Settings → Developer lists it. The secret is never here.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ApiKey {
    /// The key's ID, also the public half of its token.
    pub id: ApiKeyId,
    /// What the user called it ("Raycast", "Zapier script").
    pub name: String,
    /// What it may do.
    pub scope: ApiKeyScope,
    /// When it was made.
    pub created_at: DateTime<Utc>,
    /// When a request last used it, to the minute.
    pub last_used_at: Option<DateTime<Utc>>,
    /// When it was revoked; a revoked key is refused.
    pub revoked_at: Option<DateTime<Utc>>,
}

/// A key as stored: the listing plus the hash of its secret.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredApiKey {
    /// The key.
    pub key: ApiKey,
    /// The argon2id PHC string of its secret.
    pub secret_hash: String,
}

/// What the user asks for when making a key.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct NewApiKey {
    /// Its name; trimmed, 1 to [`MAX_KEY_NAME_CHARS`] characters.
    pub name: String,
    /// What it may do.
    pub scope: ApiKeyScope,
}

/// A key just made, with its token: the only time the token is shown.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CreatedApiKey {
    /// The new key.
    pub key: ApiKey,
    /// `deino_<id>_<secret>`, for the `Authorization: Bearer` header.
    pub token: String,
}

/// Who is calling: the key a request authenticated with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ApiCaller {
    /// The key's ID.
    pub key_id: ApiKeyId,
    /// What it may do.
    pub scope: ApiKeyScope,
}

/// A token split into the key it names and its secret.
#[derive(Debug, PartialEq, Eq)]
pub struct KeyToken {
    /// The key's ID.
    pub id: ApiKeyId,
    /// The secret, as hex.
    pub secret: String,
}

impl KeyToken {
    /// The token's text: `deino_<id, 32 hex>_<secret>`.
    pub fn render(&self) -> String {
        format!("{TOKEN_PREFIX}{}_{}", self.id.as_uuid().simple(), self.secret)
    }

    /// Reads a token; `None` when it does not have the token's shape.
    pub fn parse(token: &str) -> Option<Self> {
        let rest = token.strip_prefix(TOKEN_PREFIX)?;
        let (id, secret) = rest.split_once('_')?;
        if id.len() != 32 || secret.is_empty() {
            return None;
        }
        Some(Self {
            id: id.parse().ok()?,
            secret: secret.to_owned(),
        })
    }
}

#[cfg(test)]
mod tests {
    use deino_domain::FakeClock;
    use deino_testkit::at;

    use super::*;

    #[test]
    fn a_token_round_trips_and_odd_text_is_not_a_token() {
        let id = ApiKeyId::generate(&FakeClock::new(at("2026-10-12T09:00:00Z")));
        let token = KeyToken {
            id,
            secret: "ab12".into(),
        };
        let text = token.render();
        assert!(text.starts_with("deino_"));
        assert_eq!(KeyToken::parse(&text), Some(token));
        for odd in ["", "deino_", "Bearer x", "deino_abc_def", "cb_123_456"] {
            assert_eq!(KeyToken::parse(odd), None, "{odd}");
        }
    }

    #[test]
    fn scopes_serialize_in_snake_case_and_only_read_write_writes() {
        let json = serde_json::to_value(ApiKeyScope::ReadWrite).unwrap();
        assert_eq!(json, serde_json::json!("read_write"));
        assert!(ApiKeyScope::ReadWrite.can_write());
        assert!(!ApiKeyScope::Read.can_write());
    }
}
