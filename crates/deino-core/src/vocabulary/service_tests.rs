use std::sync::Arc;

use deino_domain::{ErrorClass, ErrorCode, VocabKind};

use super::*;
use crate::enhance::Vocabulary;
use crate::vocabulary::test_world::World;
use crate::vocabulary::{PromptVocabulary, SqliteVocabRepo};

#[tokio::test]
async fn entries_are_validated_created_updated_and_deleted() {
    let world = World::new().await;
    let term = world.add(VocabKind::Term, " Shreyas ", &["Sreyas"]).await;
    assert_eq!(term.text, "Shreyas");

    let input = TermInput {
        kind: VocabKind::Term,
        text: "Shreyas".into(),
        sounds_like: vec!["Shrey as".into()],
    };
    let updated = world.vocabulary.update(&term.id, &input).await.unwrap();
    assert_eq!(updated.created_at, term.created_at);
    assert_eq!(
        world.vocabulary.list().await.unwrap(),
        std::slice::from_ref(&updated)
    );

    world.vocabulary.delete(&term.id).await.unwrap();
    let missing = world.vocabulary.delete(&term.id).await.unwrap_err();
    assert_eq!(missing.code(), ErrorCode::NotFound);
    let gone = world.vocabulary.update(&term.id, &input).await.unwrap_err();
    assert_eq!(gone.code(), ErrorCode::NotFound);
}

#[tokio::test]
async fn a_repeated_spelling_is_a_conflict_and_bad_input_is_invalid() {
    let world = World::new().await;
    world.add(VocabKind::Term, "Deino", &[]).await;

    let input = |text: &str| TermInput {
        kind: VocabKind::Term,
        text: text.into(),
        sounds_like: Vec::new(),
    };
    let repeated = world.vocabulary.create(&input("deino")).await.unwrap_err();
    let blank = world.vocabulary.create(&input("  ")).await.unwrap_err();

    assert_eq!(repeated.code(), ErrorCode::Conflict);
    assert_eq!(blank.code(), ErrorCode::Invalid);
}

#[tokio::test]
async fn the_corrector_knows_terms_people_and_companies() {
    let world = World::new().await;
    world
        .add(VocabKind::Replacement, "going to", &["gonna"])
        .await;
    world.person("Shreyas Patel").await;
    world.company("Fernhollow Labs").await;

    let corrector = world.vocabulary.corrector(None).await.unwrap();

    assert_eq!(
        corrector
            .text("Sreyas is gonna call Fernholow Labs")
            .as_deref(),
        Some("Shreyas is going to call Fernhollow Labs")
    );
}

#[tokio::test]
async fn the_notes_prompt_gets_the_users_terms_only() {
    let world = World::new().await;
    world.add(VocabKind::Term, "PagerDuty", &[]).await;
    world
        .add(VocabKind::Replacement, "going to", &["gonna"])
        .await;
    world.person("Shreyas Patel").await;

    let repo = Arc::new(SqliteVocabRepo::new(world.database.clone()));
    let terms = PromptVocabulary::new(repo)
        .terms(&World::meeting())
        .await
        .unwrap();

    assert_eq!(terms, ["PagerDuty"]);
}
