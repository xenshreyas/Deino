//! [`TagStep`] and [`FolderStep`]: tag the meeting, file the meeting.

use std::sync::Arc;

use async_trait::async_trait;
use deino_domain::{FolderId, MeetingId, TagId, TagSource};

use crate::automations::{Action, ActionKind, Step, StepContext, StepError, StepOutput};
use crate::folders::{FilingService, FolderError};
use crate::tags::{TagError, TagService};

/// Puts tags on meetings on an automation's behalf.
#[async_trait]
pub trait TagApplier: Send + Sync {
    /// Applies `tag` to `meeting` as "automation"; true when it was new.
    async fn apply_tag(&self, meeting: &MeetingId, tag: &TagId) -> Result<bool, TagError>;
}

#[async_trait]
impl TagApplier for TagService {
    async fn apply_tag(&self, meeting: &MeetingId, tag: &TagId) -> Result<bool, TagError> {
        self.apply(meeting, tag, TagSource::Automation).await
    }
}

/// Files meetings into folders on an automation's behalf.
#[async_trait]
pub trait MeetingFiler: Send + Sync {
    /// Files `meeting` in `folder`; filing it again changes nothing.
    async fn file(&self, meeting: &MeetingId, folder: &FolderId) -> Result<(), FolderError>;
}

#[async_trait]
impl MeetingFiler for FilingService {
    async fn file(&self, meeting: &MeetingId, folder: &FolderId) -> Result<(), FolderError> {
        self.add_meeting(folder, meeting).await.map(|_| ())
    }
}

/// Runs [`Action::AddTag`].
pub struct TagStep {
    tags: Arc<dyn TagApplier>,
}

impl TagStep {
    /// The step over `tags`.
    pub fn new(tags: Arc<dyn TagApplier>) -> Self {
        Self { tags }
    }
}

#[async_trait]
impl Step for TagStep {
    fn kind(&self) -> ActionKind {
        ActionKind::AddTag
    }

    async fn run(&self, action: &Action, ctx: StepContext<'_>) -> Result<StepOutput, StepError> {
        let Action::AddTag { tag_id } = action else {
            return Err(StepError::Unavailable);
        };
        match self.tags.apply_tag(ctx.meeting, tag_id).await {
            Ok(added) => Ok(StepOutput::Tagged {
                tag_id: *tag_id,
                added,
            }),
            Err(TagError::NotFound(_)) => Err(StepError::MissingTarget("tag")),
            Err(error) => Err(error.into()),
        }
    }
}

/// Runs [`Action::AddToFolder`].
pub struct FolderStep {
    filer: Arc<dyn MeetingFiler>,
}

impl FolderStep {
    /// The step over `filer`.
    pub fn new(filer: Arc<dyn MeetingFiler>) -> Self {
        Self { filer }
    }
}

#[async_trait]
impl Step for FolderStep {
    fn kind(&self) -> ActionKind {
        ActionKind::AddToFolder
    }

    async fn run(&self, action: &Action, ctx: StepContext<'_>) -> Result<StepOutput, StepError> {
        let Action::AddToFolder { folder_id } = action else {
            return Err(StepError::Unavailable);
        };
        match self.filer.file(ctx.meeting, folder_id).await {
            Ok(()) => Ok(StepOutput::Filed {
                folder_id: *folder_id,
            }),
            Err(FolderError::NotFound(_)) => Err(StepError::MissingTarget("folder")),
            Err(error) => Err(error.into()),
        }
    }
}
