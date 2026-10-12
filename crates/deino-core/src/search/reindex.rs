//! [`ReindexQueue`]: re-indexes meetings whose transcript the user edited
//! (TRN-02), one at a time, on one tracked background task.
//!
//! One worker, in order, matters: two edits indexed side by side could let
//! an older read of the transcript land last and bring back words the user
//! just deleted. Meetings queued while the worker is busy are coalesced, so
//! a burst of edits to one meeting indexes it once more, not once per edit.
//! Keyword search needs none of this (its triggers already follow every
//! write); the queue keeps the semantic index's windows current, which the
//! TRN-02 migration's trigger dropped at the moment of the edit.

use std::sync::Arc;

use deino_domain::MeetingId;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

use super::SearchIndexing;
use crate::transcript::TranscriptIndexer;

/// The queue of meetings to re-index, and its worker.
pub struct ReindexQueue {
    sender: UnboundedSender<MeetingId>,
    tracker: TaskTracker,
    cancel: CancellationToken,
}

impl ReindexQueue {
    /// Starts the worker over `indexing`. Call it inside the async runtime.
    pub fn start(indexing: Arc<dyn SearchIndexing>) -> Self {
        let (sender, receiver) = unbounded_channel();
        let tracker = TaskTracker::new();
        let cancel = CancellationToken::new();
        let stop = cancel.clone();
        tracker.spawn(async move {
            tokio::select! {
                () = stop.cancelled() => {}
                () = work(indexing, receiver) => {}
            }
        });
        tracker.close();
        Self {
            sender,
            tracker,
            cancel,
        }
    }

    /// Stops the worker (a meeting mid-index is indexed again by the next
    /// backfill) and waits for it. Call it before the app exits.
    pub async fn shutdown(&self) {
        self.cancel.cancel();
        self.tracker.wait().await;
    }
}

impl TranscriptIndexer for ReindexQueue {
    fn changed(&self, meetings: &[MeetingId]) {
        for meeting in meetings {
            if self.sender.send(meeting.clone()).is_err() {
                log::warn!(target: "search", "the re-index queue has stopped; meeting {meeting} waits for the backfill");
            }
        }
    }
}

/// Indexes queued meetings until the queue closes.
async fn work(indexing: Arc<dyn SearchIndexing>, mut receiver: UnboundedReceiver<MeetingId>) {
    while let Some(first) = receiver.recv().await {
        for meeting in drain(first, &mut receiver) {
            if let Err(error) = indexing.index_meeting(&meeting).await {
                log::warn!(target: "search", "could not re-index meeting {meeting} after a transcript edit: {error}");
            }
        }
    }
}

/// `first` and everything queued behind it, each meeting once, in order.
fn drain(first: MeetingId, receiver: &mut UnboundedReceiver<MeetingId>) -> Vec<MeetingId> {
    let mut batch = vec![first];
    while let Ok(next) = receiver.try_recv() {
        if !batch.contains(&next) {
            batch.push(next);
        }
    }
    batch
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use async_trait::async_trait;
    use tokio::sync::mpsc::{UnboundedSender, unbounded_channel};

    use super::*;
    use crate::search::{MeetingIndexOutcome, SearchError};

    /// How long a test waits for the worker.
    const WAIT: Duration = Duration::from_secs(5);

    /// Reports each meeting it indexes.
    struct Reporting(UnboundedSender<MeetingId>);

    #[async_trait]
    impl SearchIndexing for Reporting {
        async fn index_meeting(
            &self,
            meeting: &MeetingId,
        ) -> Result<MeetingIndexOutcome, SearchError> {
            self.0.send(meeting.clone()).unwrap();
            Ok(MeetingIndexOutcome::KeywordOnly)
        }
    }

    #[test]
    fn a_burst_of_edits_indexes_each_meeting_once_in_order() {
        let (sender, mut receiver) = unbounded_channel();
        for id in ["a", "b", "a", "b"] {
            sender.send(MeetingId::new(id)).unwrap();
        }
        let first = MeetingId::new("b");

        let batch = drain(first, &mut receiver);

        assert_eq!(batch, [MeetingId::new("b"), MeetingId::new("a")]);
    }

    #[tokio::test]
    async fn changed_meetings_are_indexed_in_the_background() {
        let (sender, mut indexed) = unbounded_channel();
        let queue = ReindexQueue::start(Arc::new(Reporting(sender)));

        queue.changed(&[MeetingId::new("meeting-a")]);

        let got = tokio::time::timeout(WAIT, indexed.recv()).await.unwrap();
        assert_eq!(got, Some(MeetingId::new("meeting-a")));
        queue.shutdown().await;
    }
}
