//! The event port: model downloads announce their progress and outcome.

use super::{DownloadProgress, ModelFileId};

/// Receives model download events. The app emits them to the UI.
pub trait ModelEvents: Send + Sync {
    /// A download made progress.
    fn progress(&self, progress: &DownloadProgress);

    /// A file's state changed (downloaded, failed, cancelled or deleted).
    fn changed(&self, id: &ModelFileId);
}
