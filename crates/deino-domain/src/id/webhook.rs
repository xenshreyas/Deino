//! The webhooks' IDs (DEV-01), kept apart from `id.rs` to hold it under
//! the file-size cap; declared with the same macro.

use super::{Clock, Deserialize, FromStr, Hyphenated, IdError, Serialize, Uuid, fmt, new_v7};

uuid_id!(
    /// Identifies an outbound webhook endpoint the user added (DEV-01).
    WebhookId
);

uuid_id!(
    /// Identifies one message to a webhook endpoint, across its retries
    /// (DEV-01); its `webhook-id` header is derived from it.
    WebhookDeliveryId
);
