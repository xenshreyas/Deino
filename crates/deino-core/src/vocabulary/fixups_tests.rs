use deino_domain::VocabKind;
use deino_testkit::at;

use super::*;
use crate::segments::SegmentWriter;
use crate::vocabulary::UndoScope;
use crate::vocabulary::test_world::World;

#[tokio::test]
async fn shreyas_misheard_as_sreyas_is_corrected_after_the_meeting() {
    let world = World::new().await;
    world.add(VocabKind::Term, "Shreyas", &["Sreyas"]).await;
    world
        .transcript(&["Thanks Sreyas.", "Sreyas's plan works"])
        .await;

    let outcome = world.fixups.run(&World::meeting()).await.unwrap();

    assert_eq!(
        outcome,
        FixupOutcome {
            segments: 2,
            corrected: 2,
            empty_vocabulary: false
        }
    );
    assert_eq!(
        world.texts().await,
        ["Thanks Shreyas.", "Shreyas's plan works"]
    );
    let words: Vec<String> = world.segments().await[0]
        .words
        .iter()
        .map(|w| w.text.clone())
        .collect();
    assert_eq!(words, ["Thanks", "Shreyas."]);
}

#[tokio::test]
async fn a_misheard_attendee_needs_no_entry_at_all() {
    let world = World::new().await;
    world.person("Shreyas Patel").await;
    world.transcript(&["Sreyas Patel will send it"]).await;

    world.fixups.run(&World::meeting()).await.unwrap();

    assert_eq!(world.texts().await, ["Shreyas Patel will send it"]);
}

#[tokio::test]
async fn running_again_changes_nothing_and_undone_corrections_stay_undone() {
    let world = World::new().await;
    world.add(VocabKind::Term, "Shreyas", &["Sreyas"]).await;
    world.transcript(&["Sreyas", "Sreyas again"]).await;
    let meeting = World::meeting();
    world.fixups.run(&meeting).await.unwrap();
    let first = world.segments().await[0].id;
    world
        .vocabulary
        .undo(UndoScope::Segment(&first))
        .await
        .unwrap();

    let again = world.fixups.run(&meeting).await.unwrap();

    assert_eq!(again.corrected, 0);
    assert_eq!(world.texts().await, ["Sreyas", "Shreyas again"]);
    assert_eq!(
        world.vocabulary.corrections(&meeting).await.unwrap().len(),
        1
    );
}

#[tokio::test]
async fn segments_the_user_edited_are_never_corrected() {
    let world = World::new().await;
    world.add(VocabKind::Term, "Shreyas", &["Sreyas"]).await;
    let rows = world.transcript(&["Sreyas"]).await;
    world
        .segments
        .edit_text(&rows[0].id, "Sreyas", at("2026-10-11T09:30:00Z"))
        .await
        .unwrap();

    let outcome = world.fixups.run(&World::meeting()).await.unwrap();

    assert_eq!(outcome.corrected, 0);
    assert_eq!(world.texts().await, ["Sreyas"]);
}

#[tokio::test]
async fn an_empty_vocabulary_reads_nothing() {
    let world = World::new().await;
    world.transcript(&["Sreyas"]).await;

    let outcome = world.fixups.run(&World::meeting()).await.unwrap();

    assert!(outcome.empty_vocabulary);
    assert_eq!(outcome.segments, 0);
}
