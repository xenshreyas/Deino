use std::sync::Mutex;

use super::super::test_harness::Harness;
use super::super::{Coverage, EmbeddingStore};
use super::*;

/// Records every progress event.
#[derive(Default)]
struct Recorded(Mutex<Vec<BackfillProgress>>);

impl IndexEvents for Recorded {
    fn backfill(&self, progress: &BackfillProgress) {
        self.0.lock().unwrap().push(*progress);
    }
}

fn backfill(harness: &Harness, events: Arc<Recorded>) -> Backfill {
    Backfill::new(harness.indexer.clone(), harness.store.clone(), events)
}

#[tokio::test]
async fn the_backfill_embeds_every_finished_meeting_and_reports_progress() {
    let harness = Harness::new().await;
    harness.meeting("meeting-a", "A", &["first meeting"]).await;
    harness.meeting("meeting-b", "B", &["second meeting"]).await;
    let events = Arc::new(Recorded::default());
    let backfill = backfill(&harness, events.clone());

    assert!(backfill.start());
    assert!(!backfill.start(), "one run at a time");
    backfill.until_idle().await;

    let progress = backfill.progress();
    assert_eq!(
        progress,
        BackfillProgress {
            running: false,
            done: 2,
            total: 2,
            failed: 0
        }
    );
    let recorded = events.0.lock().unwrap().clone();
    assert_eq!(recorded.first().map(|p| p.total), Some(2));
    assert_eq!(recorded.last(), Some(&progress));
    let coverage = harness.store.coverage("fake-embedder").await.unwrap();
    assert_eq!(
        coverage,
        Coverage {
            indexed: 2,
            total: 2
        }
    );
    assert!(backfill.start(), "a finished run can start again");
    backfill.until_idle().await;
}

#[tokio::test]
async fn the_backfill_stops_when_the_model_is_missing() {
    let harness = Harness::new().await;
    harness.meeting("meeting-a", "A", &["first meeting"]).await;
    harness.embedder.set_ready(false);
    let backfill = backfill(&harness, Arc::new(Recorded::default()));

    backfill.start();
    backfill.until_idle().await;

    assert_eq!(
        backfill.progress(),
        BackfillProgress {
            running: false,
            done: 0,
            total: 1,
            failed: 0
        }
    );
}

#[tokio::test]
async fn no_run_starts_after_shutdown() {
    let harness = Harness::new().await;
    let backfill = backfill(&harness, Arc::new(Recorded::default()));

    backfill.shutdown().await;

    assert!(!backfill.start());
}
