//! IDs of asking about meetings: recipes (AI-04) and the "Ask about this
//! meeting" thread (AI-02-lite, D13). Declared with the parent's
//! `uuid_id!`, re-exported by `id`.

use super::{Clock, Deserialize, FromStr, Hyphenated, IdError, Serialize, Uuid, fmt, new_v7};

uuid_id!(
    /// Identifies a recipe (AI-04): a saved prompt run on a meeting.
    RecipeId
);

uuid_id!(
    /// Identifies one message of a meeting's "Ask about this meeting"
    /// thread (AI-02-lite, D13).
    AskMessageId
);
