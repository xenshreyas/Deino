//! [`SqliteRecipeRepo`] against a migrated database.

use chrono::{DateTime, Duration, Utc};
use deino_db::test_db;
use deino_domain::{FakeClock, RecipeId};

use super::*;

fn at(seconds: i64) -> DateTime<Utc> {
    DateTime::UNIX_EPOCH + Duration::seconds(seconds)
}

fn recipe(slug: &str, built_in: bool, created: i64) -> Recipe {
    Recipe {
        id: RecipeId::generate(&FakeClock::new(at(created))),
        slug: slug.into(),
        name: slug.into(),
        description: String::new(),
        prompt: format!("Do {slug}"),
        kind: RecipeKind::Single,
        built_in,
        created_at: at(created),
        updated_at: at(created),
    }
}

#[tokio::test]
async fn built_ins_list_first_and_the_user_cannot_change_them_through_the_repo() {
    let repo = SqliteRecipeRepo::new(test_db().await);
    repo.insert(&recipe("mine", false, 0)).await.unwrap();
    repo.sync_built_ins(&[recipe("b", true, 5), recipe("a", true, 6)])
        .await
        .unwrap();
    let builtin = repo.list().await.unwrap()[0].clone();

    let slugs: Vec<String> = repo
        .list()
        .await
        .unwrap()
        .into_iter()
        .map(|r| r.slug)
        .collect();
    assert_eq!(slugs, ["b", "a", "mine"]);
    assert!(!repo.delete(&builtin.id).await.unwrap());
    repo.update(&Recipe {
        prompt: "changed".into(),
        ..builtin.clone()
    })
    .await
    .unwrap();
    assert_eq!(repo.find(&builtin.id).await.unwrap(), Some(builtin));
}

#[tokio::test]
async fn a_sync_updates_built_ins_in_place_and_drops_retired_ones() {
    let repo = SqliteRecipeRepo::new(test_db().await);
    repo.sync_built_ins(&[recipe("keep", true, 0), recipe("retire", true, 1)])
        .await
        .unwrap();
    let kept = repo.list().await.unwrap()[0].clone();

    let newer = Recipe {
        prompt: "Better".into(),
        ..recipe("keep", true, 9)
    };
    repo.sync_built_ins(&[newer]).await.unwrap();

    let all = repo.list().await.unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!((all[0].id, all[0].prompt.as_str()), (kept.id, "Better"));
}

#[tokio::test]
async fn a_users_recipe_keeps_its_slug_when_the_library_adds_the_same_one() {
    let repo = SqliteRecipeRepo::new(test_db().await);
    repo.insert(&recipe("coach-me", false, 0)).await.unwrap();

    repo.sync_built_ins(&[recipe("coach-me", true, 1)])
        .await
        .unwrap();

    let all = repo.list().await.unwrap();
    assert_eq!(all.len(), 1);
    assert!(!all[0].built_in);
}
