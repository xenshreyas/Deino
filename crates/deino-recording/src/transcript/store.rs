//! `SegmentStore`: the router's blocking view of the segment writer.
//!
//! The router runs on its own thread; [`WriterStore`] runs the async
//! `SegmentWriter::save_live` to completion on the app's Tokio runtime
//! through a `Handle`, so every write still goes through the database's one
//! writer (and its change notifier).

use std::sync::Arc;

use deino_core::segments::{NewSegment, SegmentWriter};
use deino_db::DbError;
use deino_domain::SegmentId;
use tokio::runtime::Handle;

/// Why a batch was not saved.
#[derive(Debug, thiserror::Error)]
#[error("could not save the live transcript")]
pub struct StoreError(#[from] pub DbError);

/// Saves batches of live segments.
pub trait SegmentStore: Send + Sync {
    /// Inserts or completes `segments` and deletes `removed`, in one transaction.
    fn save(&self, segments: &[NewSegment], removed: &[SegmentId]) -> Result<(), StoreError>;
}

/// A [`SegmentStore`] over a `SegmentWriter`, blocking on a Tokio runtime.
pub struct WriterStore {
    writer: Arc<dyn SegmentWriter>,
    runtime: Handle,
}

impl WriterStore {
    /// A store writing through `writer` on `runtime`.
    pub fn new(writer: Arc<dyn SegmentWriter>, runtime: Handle) -> Self {
        Self { writer, runtime }
    }
}

impl SegmentStore for WriterStore {
    fn save(&self, segments: &[NewSegment], removed: &[SegmentId]) -> Result<(), StoreError> {
        Ok(self
            .runtime
            .block_on(self.writer.save_live(segments, removed))?)
    }
}
