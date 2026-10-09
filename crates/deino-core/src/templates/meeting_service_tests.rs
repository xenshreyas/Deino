//! `MeetingTemplateService` over SQLite: choosing, changing and previewing a
//! meeting's template.

use std::sync::Arc;

use deino_db::{Database, test_db};
use deino_domain::{ErrorClass, ErrorCode, FakeClock, MeetingId, TemplateId};
use deino_testkit::{at, seed};

use super::*;
use crate::meetings::SqliteMeetingRepo;
use crate::settings::{FakeSettingsStore, RecordedSettingsEvents, Settings, SettingsService};

pub(super) async fn settings(change: impl FnOnce(&mut Settings)) -> Arc<SettingsService> {
    let mut saved = Settings::default();
    change(&mut saved);
    let store = Arc::new(FakeSettingsStore::with(Some(saved)));
    let service = SettingsService::load(store, Arc::new(RecordedSettingsEvents::default()));
    Arc::new(service.await.unwrap())
}

pub(super) async fn service(
    database: &Database,
    settings: Arc<SettingsService>,
    classifier: Option<Classification>,
) -> MeetingTemplateService {
    let repo = Arc::new(SqliteTemplateRepo::new(database.clone()));
    let clock = Arc::new(FakeClock::new(at("2026-10-09T09:00:00Z")));
    BuiltinSync::new(repo.clone(), clock.clone())
        .run(&CATALOG)
        .await
        .unwrap();
    MeetingTemplateService::new(MeetingTemplatePorts {
        templates: repo.clone(),
        facts: repo,
        meetings: Arc::new(SqliteMeetingRepo::new(database.clone())),
        settings,
        classifier: Arc::new(FixedClassifier(classifier)),
        clock,
    })
}

async fn database_with(title: &str) -> Database {
    let database = test_db().await;
    seed::meeting(&database, "meeting-1", title, at("2026-10-08T09:00:00Z")).await;
    database
}

fn meeting() -> MeetingId {
    MeetingId::new("meeting-1")
}

fn podcast_pick() -> Classification {
    Classification {
        template_id: TemplateId::new("podcast"),
        explanation: "sounds like a show".into(),
    }
}

#[tokio::test]
async fn nothing_applies_so_the_general_template_is_the_fallback() {
    let database = database_with("Catch-up").await;
    let service = service(&database, settings(|_| {}).await, None).await;

    let choice = service.choice(&meeting()).await.unwrap();

    assert_eq!(choice.template.id, TemplateId::new(GENERAL_TEMPLATE_ID));
    assert_eq!(choice.reason, ChoiceReason::Fallback);
}

#[tokio::test]
async fn a_rule_beats_the_default_and_the_default_beats_the_fallback() {
    let database = database_with("Retro: sprint 42").await;
    let default =
        settings(|s| s.templates.default_template = Some(TemplateId::new("circleback_style")));
    let service = service(&database, default.await, None).await;

    let by_rule = service.choice(&meeting()).await.unwrap();
    seed::meeting(
        &database,
        "meeting-2",
        "Catch-up",
        at("2026-10-08T10:00:00Z"),
    )
    .await;
    let by_default = service.choice(&MeetingId::new("meeting-2")).await.unwrap();

    assert_eq!(by_rule.template.id, TemplateId::new("retrospective"));
    assert_eq!(
        by_rule.reason,
        ChoiceReason::Rule {
            rule: "retrospective".into()
        }
    );
    assert_eq!(by_default.template.id, TemplateId::new("circleback_style"));
    assert_eq!(by_default.reason, ChoiceReason::Default);
}

#[tokio::test]
async fn the_users_choice_wins_until_it_is_cleared() {
    let database = database_with("Retro: sprint 42").await;
    let service = service(&database, settings(|_| {}).await, None).await;
    let podcast = TemplateId::new("podcast");

    let chosen = service.set(&meeting(), Some(&podcast)).await.unwrap();
    let cleared = service.set(&meeting(), None).await.unwrap();

    assert_eq!(
        (chosen.template.id, chosen.reason),
        (podcast, ChoiceReason::Chosen)
    );
    assert_eq!(cleared.template.id, TemplateId::new("retrospective"));
}

#[tokio::test]
async fn unknown_templates_and_meetings_are_not_found() {
    let database = database_with("Sync").await;
    let service = service(&database, settings(|_| {}).await, None).await;

    let template = service
        .set(&meeting(), Some(&TemplateId::new("nope")))
        .await
        .unwrap_err();
    let unknown_meeting = service
        .set(&MeetingId::new("nope"), None)
        .await
        .unwrap_err();
    let choice = service.choice(&MeetingId::new("nope")).await.unwrap_err();

    assert!(matches!(template, TemplateError::NotFound(_)));
    assert!(matches!(unknown_meeting, TemplateError::MeetingNotFound(_)));
    assert_eq!(choice.code(), ErrorCode::NotFound);
}

#[tokio::test]
async fn only_the_pipeline_asks_the_classifier_and_only_when_the_default_is_auto() {
    let database = database_with("Catch-up").await;
    let auto = service(&database, settings(|_| {}).await, Some(podcast_pick())).await;
    let with_default =
        settings(|s| s.templates.default_template = Some(TemplateId::new("one_on_one")));
    let defaulted = service(&database, with_default.await, Some(podcast_pick())).await;

    let shown = auto.choice(&meeting()).await.unwrap();
    let classified = auto.auto_choice(&meeting()).await.unwrap();
    let not_asked = defaulted.auto_choice(&meeting()).await.unwrap();

    assert_eq!(shown.reason, ChoiceReason::Fallback);
    assert_eq!(classified.template.id, TemplateId::new("podcast"));
    assert_eq!(
        classified.reason,
        ChoiceReason::Classified {
            explanation: "sounds like a show".into()
        }
    );
    assert_eq!(not_asked.reason, ChoiceReason::Default);
}

#[tokio::test]
async fn a_classifier_answer_outside_the_templates_is_ignored() {
    let database = database_with("Catch-up").await;
    let made_up = Some(Classification {
        template_id: TemplateId::new("haiku"),
        explanation: "?".into(),
    });
    let service = service(&database, settings(|_| {}).await, made_up).await;

    let choice = service.auto_choice(&meeting()).await.unwrap();

    assert_eq!(choice.reason, ChoiceReason::Fallback);
}

#[tokio::test]
async fn previews_render_an_unsaved_template_with_the_profile() {
    let database = database_with("Board prep").await;
    let with_role = settings(|s| s.profile.role = "CTO".into());
    let service = service(&database, with_role.await, None).await;
    let draft = CATALOG[0].content();

    let preview = service.preview(draft, &meeting()).await.unwrap();

    assert_eq!(preview.blocks[0].heading, "About you");
    assert!(preview.blocks[0].body.contains("CTO"));
    assert!(preview.blocks[1].body.starts_with("Title: Board prep"));
}
