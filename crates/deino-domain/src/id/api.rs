//! IDs of the local REST API (DEV-03), declared with the parent's `uuid_id!`.

use super::{Clock, Deserialize, FromStr, Hyphenated, IdError, Serialize, Uuid, fmt, new_v7};

uuid_id!(
    /// Identifies a key of the local REST API; also the public half of the
    /// key's token.
    ApiKeyId
);
