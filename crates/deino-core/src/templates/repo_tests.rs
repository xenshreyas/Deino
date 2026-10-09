//! The template ports' contract, run against SQLite and the fakes, plus what
//! only the real schema can show.

use deino_db::test_db;
use deino_domain::{FakeClock, MeetingId, SectionFormat, TemplateId};
use deino_testkit::{at, seed};

use super::*;
use crate::testing::query_plan;

const AT: &str = "2026-10-09T09:00:00Z";

fn content(name: &str, sections: &[&str]) -> TemplateContent {
    TemplateContent {
        name: name.into(),
        description: "d".into(),
        meeting_context: String::new(),
        sections: sections
            .iter()
            .map(|title| TemplateSection {
                title: (*title).into(),
                instructions: format!("about {title}"),
                format: SectionFormat::Bullets,
            })
            .collect(),
        auto_rules: vec![AutoRule {
            label: "demo".into(),
            title_pattern: Some("demo".into()),
            ..AutoRule::default()
        }],
    }
}

fn user_template(name: &str, sections: &[&str]) -> Template {
    Template {
        id: TemplateId::generate(&FakeClock::new(at(AT))),
        is_builtin: false,
        content: content(name, sections),
        created_at: at(AT),
        updated_at: at(AT),
    }
}

fn builtin(id: &str, name: &str) -> Template {
    Template {
        id: TemplateId::new(id),
        is_builtin: true,
        ..user_template(name, &["Overview"])
    }
}

async fn sqlite() -> SqliteTemplateRepo {
    SqliteTemplateRepo::new(test_db().await)
}

/// Saved templates read back whole, by name, with sections in order.
async fn templates_round_trip_in_name_order<R: TemplateRepo + BuiltinStore>(repo: &R) {
    let mine = user_template("zebra notes", &["Second", "First"]);
    repo.insert(&mine).await.unwrap();
    repo.save_builtin(&builtin("general", "General"), 1)
        .await
        .unwrap();

    let listed = repo.list().await.unwrap();

    let names: Vec<&str> = listed.iter().map(|t| t.content.name.as_str()).collect();
    assert_eq!(names, ["General", "zebra notes"]);
    assert_eq!(repo.get(&mine.id).await.unwrap(), Some(mine));
    assert!(listed[0].is_builtin);
}

/// Edits replace sections; deletes report what they removed; built-ins are
/// out of reach of both.
async fn only_user_templates_change_through_the_repo<R: TemplateRepo + BuiltinStore>(repo: &R) {
    let mut mine = user_template("Mine", &["A", "B", "C"]);
    repo.insert(&mine).await.unwrap();
    let general = builtin("general", "General");
    repo.save_builtin(&general, 1).await.unwrap();

    mine.content = content("Mine v2", &["C", "A"]);
    let mut renamed_builtin = general.clone();
    renamed_builtin.content.name = "Hacked".into();

    assert!(repo.update(&mine).await.unwrap());
    assert_eq!(repo.get(&mine.id).await.unwrap(), Some(mine.clone()));
    assert!(!repo.update(&renamed_builtin).await.unwrap());
    assert!(!repo.delete(&general.id).await.unwrap());
    assert!(repo.delete(&mine.id).await.unwrap());
    assert!(!repo.delete(&mine.id).await.unwrap());
}

/// Built-ins upsert by ID with their version; user templates are never
/// overwritten or removed through the store.
async fn built_ins_upsert_by_version<R: TemplateRepo + BuiltinStore>(repo: &R) {
    let mine = user_template("Mine", &["A"]);
    repo.insert(&mine).await.unwrap();
    repo.save_builtin(&builtin("general", "General"), 1)
        .await
        .unwrap();
    repo.save_builtin(&builtin("general", "General v2"), 2)
        .await
        .unwrap();
    let mut impostor = builtin(mine.id.as_str(), "Not mine");
    impostor.id = mine.id.clone();
    repo.save_builtin(&impostor, 9).await.unwrap();

    assert_eq!(
        repo.builtin_versions().await.unwrap(),
        [(TemplateId::new("general"), 2)]
    );
    let general = repo
        .get(&TemplateId::new("general"))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(general.content.name, "General v2");
    assert_eq!(repo.get(&mine.id).await.unwrap(), Some(mine.clone()));
    assert!(!repo.remove_builtin(&mine.id).await.unwrap());
    assert!(
        repo.remove_builtin(&TemplateId::new("general"))
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn sqlite_templates_round_trip_in_name_order() {
    templates_round_trip_in_name_order(&sqlite().await).await;
}

#[tokio::test]
async fn fake_templates_round_trip_in_name_order() {
    templates_round_trip_in_name_order(&FakeTemplateRepo::default()).await;
}

#[tokio::test]
async fn sqlite_only_user_templates_change_through_the_repo() {
    only_user_templates_change_through_the_repo(&sqlite().await).await;
}

#[tokio::test]
async fn fake_only_user_templates_change_through_the_repo() {
    only_user_templates_change_through_the_repo(&FakeTemplateRepo::default()).await;
}

#[tokio::test]
async fn sqlite_built_ins_upsert_by_version() {
    built_ins_upsert_by_version(&sqlite().await).await;
}

#[tokio::test]
async fn fake_built_ins_upsert_by_version() {
    built_ins_upsert_by_version(&FakeTemplateRepo::default()).await;
}

#[tokio::test]
async fn deleting_a_template_returns_its_meetings_to_automatic_selection() {
    let repo = sqlite().await;
    let mine = user_template("Mine", &["A"]);
    repo.insert(&mine).await.unwrap();
    seed::meeting(&repo.database, "meeting-1", "Demo", at(AT)).await;
    sqlx::query("UPDATE meetings SET template_id = ?1 WHERE id = 'meeting-1'")
        .bind(&mine.id)
        .execute(repo.database.writer())
        .await
        .unwrap();

    repo.delete(&mine.id).await.unwrap();

    let facts = repo
        .facts(&MeetingId::new("meeting-1"))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(facts.template_id, None);
    assert_eq!(facts.title, "Demo");
    assert_eq!(facts.attendee_count, 0);
    assert_eq!(repo.facts(&MeetingId::new("nope")).await.unwrap(), None);
}

#[tokio::test]
async fn lists_are_read_through_their_indexes() {
    let repo = sqlite().await;

    let list = query_plan(&repo.database, include_str!("queries/list.sql")).await;
    let sections = query_plan(&repo.database, include_str!("queries/sections.sql")).await;
    let attendees = query_plan(&repo.database, include_str!("queries/attendee_emails.sql")).await;

    assert_eq!(list, ["SCAN templates USING INDEX idx_templates_name"]);
    assert_eq!(
        sections,
        ["SCAN template_sections USING INDEX sqlite_autoindex_template_sections_1"]
    );
    assert!(
        attendees[0].starts_with("SEARCH meeting_attendees USING"),
        "{attendees:?}"
    );
}
