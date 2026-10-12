//! [`ExportStep`]: writes the meeting as a file (EXP-01's renderers) into
//! a folder the user picked. The file name follows the default export
//! template ("2026-10-08 Eng standup.pdf"), so running again overwrites
//! the same file instead of piling up copies.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use deino_export::naming::NameTemplate;
use deino_export::{ExportFormat, MeetingExport, RenderOptions};

use crate::automations::{
    Action, ActionKind, MeetingSource, Step, StepContext, StepError, StepOutput,
};
use crate::export::{ExportError, Vault};
use crate::settings::DEFAULT_FILE_NAME;

/// Runs [`Action::ExportFile`].
pub struct ExportStep {
    meetings: Arc<dyn MeetingSource>,
    vault: Arc<dyn Vault>,
}

impl ExportStep {
    /// The step over the meeting source and the file system.
    pub fn new(meetings: Arc<dyn MeetingSource>, vault: Arc<dyn Vault>) -> Self {
        Self { meetings, vault }
    }
}

#[async_trait]
impl Step for ExportStep {
    fn kind(&self) -> ActionKind {
        ActionKind::ExportFile
    }

    async fn run(&self, action: &Action, ctx: StepContext<'_>) -> Result<StepOutput, StepError> {
        let Action::ExportFile { format, folder } = action else {
            return Err(StepError::Unavailable);
        };
        let format = *format;
        let export = self.meetings.export(ctx.meeting).await?;
        let path = Path::new(folder).join(file_name(&export, format));
        let rendered = tokio::task::spawn_blocking(move || {
            deino_export::render(format, &export, &RenderOptions::default())
        })
        .await
        .map_err(|_| ExportError::Cancelled)?
        .map_err(ExportError::from)?;
        self.vault
            .write(&path, rendered.bytes)
            .await
            .map_err(ExportError::Write)?;
        Ok(StepOutput::Exported {
            path: path.to_string_lossy().into_owned(),
        })
    }
}

/// "2026-10-08 Eng standup.pdf".
fn file_name(export: &MeetingExport, format: ExportFormat) -> PathBuf {
    let stem = NameTemplate::parse(DEFAULT_FILE_NAME).map_or_else(
        |_| deino_export::naming::sanitize(&export.title),
        |template| template.stem(export),
    );
    PathBuf::from(format!("{stem}.{}", format.extension()))
}
