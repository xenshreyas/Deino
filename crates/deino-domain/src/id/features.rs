//! IDs of the later feature cards (TRN-05 vocabulary onward). New feature
//! IDs go in a file of their own under `id/` (like `api.rs`, `chat.rs`,
//! `webhook.rs`), so `id.rs` stays under the size cap.

use super::{Clock, Deserialize, FromStr, Hyphenated, IdError, Serialize, Uuid, fmt, new_v7};

uuid_id!(
    /// Identifies a custom vocabulary term or replacement rule (TRN-05).
    VocabTermId
);

uuid_id!(
    /// Identifies an insight the user defined (AI-06): what to extract from
    /// meetings, and when.
    InsightId
);

uuid_id!(
    /// Identifies a folder Deino watches for recordings to import (CAP-11).
    WatchedFolderId
);

uuid_id!(
    /// Identifies a file a watched folder imported or failed to (CAP-11).
    WatchedFileId
);

uuid_id!(
    /// Identifies an automation (AUT-01): conditions and the steps to run.
    AutomationId
);

uuid_id!(
    /// Identifies one automation's run on one meeting (AUT-01).
    AutomationRunId
);

uuid_id!(
    /// Identifies a clip: a named range of a meeting's recording (TRN-04).
    ClipId
);

uuid_id!(
    /// Identifies a comment thread anchored to a transcript or notes span
    /// (TRN-06).
    CommentThreadId
);

uuid_id!(
    /// Identifies one comment of a thread (TRN-06).
    CommentId
);
