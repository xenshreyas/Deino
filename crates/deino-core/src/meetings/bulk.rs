//! [`MeetingBulkService`]: the Meetings page's bulk actions that reach
//! other features (Circleback §14 "Bulk actions"): tag, untag and file the
//! selected meetings.
//!
//! Each meeting goes through the owning feature's service, so tagging
//! still announces the tag (folder rules collect tagged meetings) and
//! filing still offers the series, exactly as on the meeting page. Trash
//! and delete are batch writes of their own ([`TrashService`](super::TrashService)).

use std::sync::Arc;

use deino_domain::{ErrorClass, ErrorCode, FolderId, MeetingId, TagId, TagSource};
use serde::Serialize;

use crate::folders::{FilingService, FolderError};
use crate::tags::{TagError, TagService};

/// A bulk action that stopped part-way. The meetings before the failing one
/// keep the change; running the action again is safe (each step is
/// idempotent).
#[derive(Debug, thiserror::Error)]
pub enum BulkError {
    /// Tagging failed.
    #[error(transparent)]
    Tag(#[from] TagError),
    /// Filing failed.
    #[error(transparent)]
    Folder(#[from] FolderError),
}

impl ErrorClass for BulkError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::Tag(error) => error.code(),
            Self::Folder(error) => error.code(),
        }
    }
}

/// What a bulk action changed.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct BulkOutcome {
    /// How many meetings changed; the others already were that way.
    pub changed: u32,
}

/// Tags and files many meetings at once.
pub struct MeetingBulkService {
    tags: Arc<TagService>,
    filing: Arc<FilingService>,
}

impl MeetingBulkService {
    /// The service over the tag and folder services.
    pub fn new(tags: Arc<TagService>, filing: Arc<FilingService>) -> Self {
        Self { tags, filing }
    }

    /// Puts `tag` on every meeting in `ids`, as the user.
    pub async fn tag(&self, ids: &[MeetingId], tag: &TagId) -> Result<BulkOutcome, BulkError> {
        let mut changed = 0;
        for id in ids {
            if self.tags.apply(id, tag, TagSource::User).await? {
                changed += 1;
            }
        }
        Ok(BulkOutcome { changed })
    }

    /// Takes `tag` off every meeting in `ids`.
    pub async fn untag(&self, ids: &[MeetingId], tag: &TagId) -> Result<BulkOutcome, BulkError> {
        for id in ids {
            self.tags.remove(id, tag).await?;
        }
        Ok(BulkOutcome {
            changed: u32::try_from(ids.len()).unwrap_or(u32::MAX),
        })
    }

    /// Files every meeting in `ids` into `folder`.
    pub async fn file(
        &self,
        ids: &[MeetingId],
        folder: &FolderId,
    ) -> Result<BulkOutcome, BulkError> {
        for id in ids {
            self.filing.add_meeting(folder, id).await?;
        }
        Ok(BulkOutcome {
            changed: u32::try_from(ids.len()).unwrap_or(u32::MAX),
        })
    }
}
