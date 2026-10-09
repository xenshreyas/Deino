//! Auto-selection on the fixture meetings (FND-04), with the fixtures' user
//! (`shreyas@deino.dev`) as the profile: the card's acceptance tests.

use deino_db::test_db;
use deino_domain::{Email, TemplateId};
use deino_testkit::fixture::{Fixtures, MeetingKind, USER_EMAIL, seed_fixtures};

use super::meeting_service_tests::{service, settings};
use super::*;

async fn chosen_for(kind: MeetingKind) -> TemplateChoice {
    let database = test_db().await;
    seed_fixtures(&database).await.unwrap();
    let fixture = Fixtures::load().unwrap().meeting(kind).unwrap().clone();
    let profile = settings(|s| s.profile.emails = vec![Email::parse(USER_EMAIL).unwrap()]);
    let service = service(&database, profile.await, None).await;
    service.choice(&fixture.id).await.unwrap()
}

#[tokio::test]
async fn the_external_sales_call_gets_sales_discovery() {
    let choice = chosen_for(MeetingKind::SalesDiscovery).await;

    assert_eq!(choice.template.id, TemplateId::new("sales_discovery"));
    assert_eq!(choice.template.content.name, "Sales discovery");
    assert_eq!(
        choice.reason,
        ChoiceReason::Rule {
            rule: "external discovery call".into()
        }
    );
}

#[tokio::test]
async fn the_one_on_one_gets_the_one_on_one_template() {
    let choice = chosen_for(MeetingKind::OneOnOne).await;

    assert_eq!(choice.template.content.name, "1:1");
    assert!(matches!(choice.reason, ChoiceReason::Rule { .. }));
}

#[tokio::test]
async fn the_other_fixtures_get_fitting_templates() {
    let expected = [
        (MeetingKind::Standup, "standup"),
        (MeetingKind::Interview, "candidate_interview"),
        (MeetingKind::BoardUpdate, "board_meeting"),
        (MeetingKind::Brainstorm, "brainstorm"),
        (MeetingKind::Planning, GENERAL_TEMPLATE_ID),
    ];
    for (kind, template) in expected {
        let choice = chosen_for(kind).await;
        assert_eq!(choice.template.id, TemplateId::new(template), "{kind:?}");
    }
}
