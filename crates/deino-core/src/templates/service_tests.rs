//! `TemplateService` over the fake repository.

use std::sync::Arc;

use deino_domain::{ErrorClass, ErrorCode, FakeClock, SectionFormat, TemplateId};
use deino_testkit::at;

use super::*;

struct Harness {
    service: TemplateService,
    repo: Arc<FakeTemplateRepo>,
}

async fn harness() -> Harness {
    let repo = Arc::new(FakeTemplateRepo::default());
    let clock = Arc::new(FakeClock::new(at("2026-10-09T09:00:00Z")));
    BuiltinSync::new(repo.clone(), clock.clone())
        .run(&CATALOG)
        .await
        .unwrap();
    Harness {
        service: TemplateService::new(repo.clone(), clock),
        repo,
    }
}

fn content(name: &str) -> TemplateContent {
    TemplateContent {
        name: name.into(),
        description: String::new(),
        meeting_context: String::new(),
        sections: vec![TemplateSection {
            title: " Notes ".into(),
            instructions: String::new(),
            format: SectionFormat::Paragraph,
        }],
        auto_rules: Vec::new(),
    }
}

fn general() -> TemplateId {
    TemplateId::new(GENERAL_TEMPLATE_ID)
}

#[tokio::test]
async fn created_templates_are_validated_and_belong_to_the_user() {
    let Harness { service, .. } = harness().await;

    let created = service.create(content(" Board prep ")).await.unwrap();
    let rejected = service.create(content(" ")).await.unwrap_err();

    assert!(!created.is_builtin);
    assert_eq!(created.content.name, "Board prep");
    assert_eq!(created.content.sections[0].title, "Notes");
    assert_eq!(rejected.code(), ErrorCode::Invalid);
    assert_eq!(service.list().await.unwrap().len(), 31);
}

#[tokio::test]
async fn built_ins_are_read_only_and_duplicate_into_editable_copies() {
    let Harness { service, .. } = harness().await;

    let edit = service
        .update(&general(), content("Mine"))
        .await
        .unwrap_err();
    let delete = service.delete(&general()).await.unwrap_err();
    let copy = service.duplicate(&general()).await.unwrap();
    let edited = service
        .update(&copy.id, content("My general"))
        .await
        .unwrap();

    assert_eq!(edit.code(), ErrorCode::Conflict);
    assert_eq!(delete.code(), ErrorCode::Conflict);
    assert_eq!(copy.content.name, "General (copy)");
    assert_eq!(
        copy.content.sections,
        service.get(&general()).await.unwrap().content.sections
    );
    assert_eq!(edited.content.name, "My general");
    service.delete(&copy.id).await.unwrap();
    assert_eq!(
        service.get(&copy.id).await.unwrap_err().code(),
        ErrorCode::NotFound
    );
}

#[tokio::test]
async fn a_long_name_still_fits_after_copying() {
    let Harness { service, .. } = harness().await;
    let long = service
        .create(content(&"x".repeat(MAX_NAME_CHARS)))
        .await
        .unwrap();

    let copy = service.duplicate(&long.id).await.unwrap();

    assert_eq!(copy.content.name.chars().count(), MAX_NAME_CHARS);
    assert!(copy.content.name.ends_with(" (copy)"));
}

#[tokio::test]
async fn exports_import_as_new_templates() {
    let Harness { service, repo } = harness().await;

    let exported = service
        .export(&TemplateId::new("one_on_one"))
        .await
        .unwrap();
    let imported = service.import(&exported).await.unwrap();

    assert_ne!(imported.id, TemplateId::new("one_on_one"));
    assert!(!imported.is_builtin);
    assert_eq!(imported.content.name, "1:1");
    assert_eq!(repo.list().await.unwrap().len(), 31);
}

#[tokio::test]
async fn legacy_files_are_imported_once_and_archived() {
    let Harness { service, .. } = harness().await;
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("mine.json"),
        r#"{ "name": "My standup", "description": "d",
             "sections": [{ "title": "Yesterday", "instruction": "What got done", "format": "list" }] }"#,
    )
    .unwrap();
    let folder = LegacyTemplateFolder::new(dir.path());

    let first = service.import_legacy(&folder).await.unwrap();
    let second = service.import_legacy(&folder).await.unwrap();

    assert_eq!((first, second), (1, 0));
    let names: Vec<String> = service
        .list()
        .await
        .unwrap()
        .into_iter()
        .map(|t| t.content.name)
        .collect();
    assert!(names.contains(&"My standup".to_owned()));
}

#[tokio::test]
async fn the_summary_template_comes_from_the_stored_template() {
    let Harness { service, .. } = harness().await;

    let summary = service
        .summary_template(&TemplateId::new("standup"))
        .await
        .unwrap();
    let missing = service
        .summary_template(&TemplateId::new("daily_standup"))
        .await;

    assert!(summary.template_markdown.contains("## Blockers"));
    assert!(
        summary
            .section_instructions
            .contains("**For the 'Updates' section:**")
    );
    assert!(matches!(missing, Err(TemplateError::NotFound(_))));
}
