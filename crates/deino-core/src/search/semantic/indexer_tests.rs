use super::super::EmbeddingStore;
use super::super::test_harness::Harness;
use super::*;

#[tokio::test]
async fn a_meeting_is_chunked_embedded_and_stored_with_its_notes() {
    let harness = Harness::new().await;
    let meeting = harness
        .meeting(
            "meeting-a",
            "Sync",
            &["The beta ships in November", "Renew Graph subscriptions"],
        )
        .await;
    harness.notes(&meeting, "# Risks\n- webhook expiry").await;

    let indexed = harness.indexer.index_meeting(&meeting).await.unwrap();

    assert_eq!(
        indexed,
        IndexedMeeting {
            chunks: 2,
            embedded: 2
        }
    );
    let stored = harness.store.existing(&meeting).await.unwrap();
    assert_eq!(stored.len(), 2);
    assert!(stored.iter().all(|row| row.model == "fake-embedder"));
}

#[tokio::test]
async fn reindexing_embeds_only_what_changed() {
    let harness = Harness::new().await;
    let meeting = harness
        .meeting("meeting-a", "Sync", &["The beta ships in November"])
        .await;
    harness
        .notes(&meeting, "# One\nfirst\n\n# Two\nsecond")
        .await;
    harness.indexer.index_meeting(&meeting).await.unwrap();

    let unchanged = harness.indexer.index_meeting(&meeting).await.unwrap();
    harness
        .notes(&meeting, "# One\nfirst\n\n# Two\nchanged")
        .await;
    let edited = harness.indexer.index_meeting(&meeting).await.unwrap();

    assert_eq!(
        unchanged,
        IndexedMeeting {
            chunks: 3,
            embedded: 0
        }
    );
    assert_eq!(
        edited,
        IndexedMeeting {
            chunks: 3,
            embedded: 1
        }
    );
    assert_eq!(harness.embedder.embedded(), 4);
}

#[tokio::test]
async fn without_the_model_nothing_is_touched() {
    let harness = Harness::new().await;
    let meeting = harness.meeting("meeting-a", "Sync", &["hello there"]).await;
    harness.embedder.set_ready(false);

    let error = harness.indexer.index_meeting(&meeting).await.unwrap_err();

    assert!(matches!(error, SearchError::ModelMissing));
    assert!(harness.store.existing(&meeting).await.unwrap().is_empty());
}
