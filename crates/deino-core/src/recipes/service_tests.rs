//! [`RecipeService`] over SQLite: the library, the user's recipes, remix,
//! export and import.

use std::sync::Arc;

use chrono::DateTime;
use deino_db::test_db;
use deino_domain::FakeClock;

use super::*;

async fn service() -> RecipeService {
    let repo = Arc::new(SqliteRecipeRepo::new(test_db().await));
    RecipeService::new(repo, Arc::new(FakeClock::new(DateTime::UNIX_EPOCH)))
}

fn draft(name: &str) -> RecipeDraft {
    RecipeDraft {
        name: name.into(),
        description: "Mine".into(),
        prompt: "Tell me the plan.".into(),
    }
}

#[tokio::test]
async fn the_library_is_listed_in_order_before_the_users_recipes() {
    let service = service().await;
    service.create(draft("Plan")).await.unwrap();

    let all = service.list().await.unwrap();

    let library: Vec<&str> = LIBRARY.iter().map(|recipe| recipe.slug).collect();
    let listed: Vec<&str> = all.iter().map(|recipe| recipe.slug.as_str()).collect();
    assert_eq!(listed[..LIBRARY.len()], library[..]);
    assert_eq!(listed.last(), Some(&"plan"));
}

#[tokio::test]
async fn names_become_unique_slugs_that_follow_renames() {
    let service = service().await;

    let first = service.create(draft("Coach me")).await.unwrap();
    let renamed = service.update(&first.id, draft("Pricing")).await.unwrap();
    let again = service.update(&first.id, draft("Pricing")).await.unwrap();

    assert_eq!(first.slug, "coach-me-2");
    assert_eq!(
        (renamed.slug.as_str(), again.slug.as_str()),
        ("pricing", "pricing")
    );
}

#[tokio::test]
async fn built_ins_are_read_only_and_remix_makes_an_editable_copy() {
    let service = service().await;
    let coach = service.list().await.unwrap()[0].clone();

    assert!(matches!(
        service.update(&coach.id, draft("x")).await,
        Err(RecipeError::BuiltIn)
    ));
    assert!(matches!(
        service.delete(&coach.id).await,
        Err(RecipeError::BuiltIn)
    ));
    let copy = service.remix(&coach.id).await.unwrap();

    assert_eq!(
        (copy.name.as_str(), copy.slug.as_str()),
        ("Coach me (remix)", "coach-me-remix")
    );
    assert_eq!(copy.prompt, coach.prompt);
    assert!(!copy.built_in);
    service.delete(&copy.id).await.unwrap();
}

#[tokio::test]
async fn multi_meeting_built_ins_are_listed_but_do_not_run_or_remix() {
    let service = service().await;
    let all = service.list().await.unwrap();
    let recap = all.iter().find(|r| r.slug == "weekly-recap").unwrap();

    assert!(matches!(
        service.runnable(&recap.id).await,
        Err(RecipeError::MultiMeeting)
    ));
    assert!(matches!(
        service.remix(&recap.id).await,
        Err(RecipeError::MultiMeeting)
    ));
    assert!(service.runnable(&all[0].id).await.is_ok());
}

#[tokio::test]
async fn an_export_imports_as_new_recipes_and_a_share_string_as_one() {
    let service = service().await;
    let mine = service.create(draft("Plan")).await.unwrap();
    let coach = service.list().await.unwrap()[0].clone();

    let file = service.export(&[]).await.unwrap();
    let imported = service.import(&file).await.unwrap();
    let link = service.share_link(&coach.id).await.unwrap();
    let shared = service.import(&link).await.unwrap();

    assert_eq!(imported.len(), 1);
    assert_eq!(
        (imported[0].slug.as_str(), imported[0].prompt.as_str()),
        ("plan-2", mine.prompt.as_str())
    );
    assert_eq!(
        (shared[0].name.as_str(), shared[0].built_in),
        ("Coach me", false)
    );
}

#[tokio::test]
async fn imports_of_multi_meeting_or_invalid_recipes_save_nothing() {
    let service = service().await;
    let multi = r#"{"name":"Week","prompt":"Recap","kind":"multi"}"#;
    let file = r#"{"format":"deino.recipes","version":1,"recipes":[
        {"name":"Good","prompt":"Fine"},{"name":"","prompt":"Bad"}]}"#;

    assert!(matches!(
        service.import(multi).await,
        Err(RecipeError::MultiMeeting)
    ));
    assert!(service.import(file).await.is_err());
    assert_eq!(service.list().await.unwrap().len(), LIBRARY.len());
}
