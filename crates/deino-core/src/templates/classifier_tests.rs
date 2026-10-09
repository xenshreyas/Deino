//! The gateway classifier over `MockProvider` (route `tagging`).

use std::sync::Arc;

use chrono::DateTime;
use deino_domain::{Email, MeetingId, TemplateId};
use deino_llm::{MockProvider, Route, mock_gateway};

use super::classifier::pick_prompt;
use super::*;

fn candidates() -> Vec<Template> {
    CATALOG
        .iter()
        .filter(|builtin| ["general", "sales_discovery", "podcast"].contains(&builtin.id))
        .map(|builtin| Template {
            id: builtin.template_id(),
            is_builtin: true,
            content: builtin.content(),
            created_at: DateTime::UNIX_EPOCH,
            updated_at: DateTime::UNIX_EPOCH,
        })
        .collect()
}

fn meeting() -> MeetingFacts {
    MeetingFacts {
        id: MeetingId::new("meeting-1"),
        title: "Brightkite × Deino".into(),
        template_id: None,
        started_at: None,
        description: "First conversation about routing.".into(),
        attendee_emails: vec![Email::parse("marcus@brightkite-logistics.com").unwrap()],
        attendee_count: 3,
    }
}

async fn classify(answer: &str) -> Option<Classification> {
    let mock = Arc::new(MockProvider::new().reply(Route::Tagging, answer));
    let gateway = Arc::new(mock_gateway(mock).gateway);
    GatewayTemplateClassifier::new(gateway)
        .classify(&meeting(), &candidates())
        .await
        .unwrap()
}

#[tokio::test]
async fn a_listed_id_becomes_the_classification() {
    let classification =
        classify(r#"{ "template_id": "sales_discovery", "reason": "A prospect's first call." }"#)
            .await;

    assert_eq!(
        classification,
        Some(Classification {
            template_id: TemplateId::new("sales_discovery"),
            explanation: "A prospect's first call.".into(),
        })
    );
}

#[tokio::test]
async fn an_invented_id_is_no_classification() {
    assert_eq!(
        classify(r#"{ "template_id": "sonnet", "reason": "?" }"#).await,
        None
    );
}

#[tokio::test]
async fn the_request_goes_to_the_tagging_route_with_the_pick_prompt() {
    let mock = Arc::new(MockProvider::new().reply(
        Route::Tagging,
        r#"{ "template_id": "general", "reason": "x" }"#,
    ));
    let gateway = Arc::new(mock_gateway(mock.clone()).gateway);

    GatewayTemplateClassifier::new(gateway)
        .classify(&meeting(), &candidates())
        .await
        .unwrap();

    let request = &mock.requests()[0];
    assert_eq!(request.route, Route::Tagging);
    assert!(
        request
            .system
            .as_deref()
            .unwrap_or_default()
            .contains("choose the note template")
    );
}

#[test]
fn the_pick_prompt_lists_the_meeting_and_every_candidate() {
    insta::assert_snapshot!(pick_prompt(&meeting(), &candidates()));
}
