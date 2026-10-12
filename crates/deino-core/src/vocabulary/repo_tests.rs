//! The vocabulary repository against the real schema.

use deino_db::test_db;
use deino_domain::FakeClock;
use deino_testkit::at;

use super::*;
use crate::testing::query_plan;

fn entry(kind: VocabKind, text: &str, sounds_like: &[&str]) -> VocabTerm {
    let created_at = at("2026-10-11T09:00:00Z");
    VocabTerm {
        id: VocabTermId::generate(&FakeClock::new(created_at)),
        kind,
        text: text.into(),
        sounds_like: sounds_like.iter().map(|&s| s.to_owned()).collect(),
        created_at,
        updated_at: created_at,
    }
}

async fn repo() -> SqliteVocabRepo {
    SqliteVocabRepo::new(test_db().await)
}

#[tokio::test]
async fn entries_list_by_kind_and_spelling_with_variants_in_order() {
    let repo = repo().await;
    let shreyas = entry(
        VocabKind::Term,
        "Shreyas",
        &["Sreyas", "Shrey as", "Ashreyas"],
    );
    let gonna = entry(VocabKind::Replacement, "going to", &["gonna"]);
    let deino = entry(VocabKind::Term, "deino", &[]);
    for term in [&shreyas, &gonna, &deino] {
        repo.create(term).await.unwrap();
    }

    let listed = repo.list().await.unwrap();

    assert_eq!(listed, [gonna, deino, shreyas]);
}

#[tokio::test]
async fn a_spelling_is_unique_per_kind_ignoring_case() {
    let repo = repo().await;
    repo.create(&entry(VocabKind::Term, "Deino", &[]))
        .await
        .unwrap();

    let again = repo
        .create(&entry(VocabKind::Term, "DEINO", &["dino"]))
        .await;
    let as_rule = repo
        .create(&entry(VocabKind::Replacement, "deino", &["dino"]))
        .await;

    assert!(again.unwrap_err().is_unique_violation());
    as_rule.unwrap();
    assert_eq!(
        repo.list().await.unwrap().len(),
        2,
        "the failed insert left no variants"
    );
}

#[tokio::test]
async fn an_update_replaces_the_variants_and_a_delete_takes_them_along() {
    let repo = repo().await;
    let mut term = entry(VocabKind::Term, "Shreyas", &["Sreyas", "Shreya"]);
    repo.create(&term).await.unwrap();

    term.sounds_like = vec!["Shrey as".into()];
    term.updated_at = at("2026-10-11T10:00:00Z");
    assert!(repo.update(&term).await.unwrap());
    assert_eq!(repo.list().await.unwrap(), [term.clone()]);

    assert!(repo.delete(&term.id).await.unwrap());
    assert!(!repo.delete(&term.id).await.unwrap());
    assert!(!repo.update(&term).await.unwrap());
    assert!(repo.list().await.unwrap().is_empty());
}

#[tokio::test]
async fn the_list_is_read_through_the_spelling_index() {
    let database = test_db().await;

    let plan = query_plan(
        &database,
        "SELECT id FROM vocab_terms ORDER BY kind, text COLLATE NOCASE",
    )
    .await;

    assert!(
        plan.iter()
            .any(|step| step.contains("idx_vocab_terms_kind_text")),
        "{plan:?}"
    );
    assert!(
        !plan.iter().any(|step| step.contains("TEMP B-TREE")),
        "{plan:?}"
    );
}
