//! The service: endpoints and their secrets, the network switch, which
//! signals become deliveries, coalescing, test sends and automation keys.

use chrono::TimeDelta;
use deino_domain::{ActionItemId, Clock, MeetingId, ServiceId, WebhookId};
use deino_net::{FakeReply, ServicePolicy, Status};

use super::testing::{Harness, MEETING, input};
use super::{
    COALESCE_DELAY, DeliveryKind, DeliveryStatus, OtherKind, PayloadDelivery, SubjectItem,
    WebhookError, WebhookEvent, WebhookSignal,
};
use crate::automations::PayloadFields;

#[tokio::test]
async fn adding_an_endpoint_shows_its_secret_once_and_switches_the_service_on() {
    let harness = Harness::new().await;
    assert!(!harness.policy.grant(ServiceId::Webhooks).await.enabled);

    let created = harness.add(&[WebhookEvent::NotesGenerated]).await;

    assert!(created.secret.starts_with("whsec_"));
    let listed = serde_json::to_string(&harness.service.list().await.unwrap()).unwrap();
    assert!(!listed.contains(&created.secret));
    assert!(harness.policy.grant(ServiceId::Webhooks).await.enabled);
    assert_eq!(harness.service.hosts().await.unwrap(), ["hooks.zapier.com"]);
    let rotated = harness
        .service
        .rotate_secret(&created.webhook.id)
        .await
        .unwrap();
    assert_ne!(rotated, created.secret);

    harness.service.delete(&created.webhook.id).await.unwrap();
    assert!(!harness.policy.grant(ServiceId::Webhooks).await.enabled);
    assert!(harness.service.hosts().await.unwrap().is_empty());
}

#[tokio::test]
async fn endpoints_need_an_https_url_and_a_short_name() {
    let harness = Harness::new().await;
    let mut plain = input(&[], PayloadFields::default());
    plain.url = "http://example.com/hook".into();
    let mut long = input(&[], PayloadFields::default());
    long.label = "x".repeat(81);

    for bad in [plain, long] {
        let error = harness.service.create(bad).await.unwrap_err();
        assert!(matches!(error, WebhookError::Invalid { .. }), "{error}");
    }
    let mut local = input(&[], PayloadFields::default());
    local.url = "http://localhost:8787/hook".into();
    assert!(harness.service.create(local).await.is_ok());
}

#[tokio::test]
async fn only_enabled_endpoints_subscribed_to_the_event_get_a_delivery() {
    let harness = Harness::new().await;
    let notes = harness.add(&[WebhookEvent::NotesGenerated]).await;
    let ready = harness.add(&[WebhookEvent::MeetingReady]).await;
    let mut off = input(&[WebhookEvent::NotesGenerated], PayloadFields::default());
    off.enabled = false;
    harness.service.create(off).await.unwrap();

    let signal = WebhookSignal::meeting(WebhookEvent::NotesGenerated, MeetingId::new(MEETING));
    assert_eq!(harness.service.accept(&signal).await.unwrap(), 1);

    assert_eq!(
        harness
            .service
            .deliveries(&notes.webhook.id)
            .await
            .unwrap()
            .len(),
        1
    );
    assert!(
        harness
            .service
            .deliveries(&ready.webhook.id)
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn edits_in_a_burst_coalesce_into_one_delivery_sent_after_a_pause() {
    let harness = Harness::new().await;
    let created = harness.add(&[WebhookEvent::NotesEdited]).await;
    let edited = WebhookSignal::meeting(WebhookEvent::NotesEdited, MeetingId::new(MEETING));

    for _ in 0..5 {
        harness.service.accept(&edited).await.unwrap();
        harness.clock.advance(TimeDelta::seconds(2));
    }
    harness.service.dispatcher().run_due().await.unwrap();
    assert!(
        harness.client.requests().is_empty(),
        "sent before the pause"
    );
    harness.clock.advance(COALESCE_DELAY);
    harness.service.dispatcher().run_due().await.unwrap();

    assert_eq!(harness.client.requests().len(), 1);
    let log = harness
        .service
        .deliveries(&created.webhook.id)
        .await
        .unwrap();
    assert_eq!(log.len(), 1);
    // A later edit starts a new delivery.
    harness.service.accept(&edited).await.unwrap();
    assert_eq!(
        harness
            .service
            .deliveries(&created.webhook.id)
            .await
            .unwrap()
            .len(),
        2
    );
}

#[tokio::test]
async fn an_action_item_event_finds_its_meeting_and_carries_the_item() {
    let harness = Harness::new().await;
    harness.add(&[WebhookEvent::ActionItemCreated]).await;
    let id = ActionItemId::generate(harness.clock.as_ref());
    harness.items.add(SubjectItem {
        id,
        title: "Send the quote".into(),
        description: String::new(),
        status: "open",
        due_at: None,
    });

    let created = WebhookSignal::item(WebhookEvent::ActionItemCreated, id);
    let gone = ActionItemId::generate(harness.clock.as_ref());
    let unknown = WebhookSignal::item(WebhookEvent::ActionItemCreated, gone);
    assert_eq!(harness.service.accept(&created).await.unwrap(), 1);
    assert_eq!(harness.service.accept(&unknown).await.unwrap(), 0);
    harness.service.dispatcher().run_due().await.unwrap();

    let body = harness.client.requests()[0].body.clone().unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["type"], "action_item.created");
    assert_eq!(json["data"]["id"], MEETING);
    assert_eq!(json["data"]["actionItem"]["id"], id.to_string());
    assert_eq!(json["data"]["actionItem"]["title"], "Send the quote");
}

#[tokio::test]
async fn send_test_posts_the_latest_meeting_once_even_when_it_fails() {
    let harness = Harness::new().await;
    let created = harness.add(&[]).await;
    harness.client.reply(FakeReply::status(Status::new(500)));

    let delivery = harness
        .service
        .send_test(&created.webhook.id)
        .await
        .unwrap();

    assert_eq!(delivery.kind, DeliveryKind::Other(OtherKind::Test));
    assert_eq!(delivery.status, DeliveryStatus::Failed);
    assert_eq!(delivery.meeting_id, Some(MeetingId::new(MEETING)));
    let body = harness.client.requests()[0].body.clone().unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["type"], "test");
    let unknown = WebhookId::generate(harness.clock.as_ref());
    let missing = harness.service.send_test(&unknown).await;
    assert!(matches!(missing, Err(WebhookError::NotFound(_))));
}

#[tokio::test]
async fn an_automation_payload_is_posted_once_per_key() {
    let harness = Harness::new().await;
    let created = harness.add(&[]).await;
    let meeting = MeetingId::new(MEETING);
    let request = PayloadDelivery {
        webhook: &created.webhook.id,
        meeting: &meeting,
        key: "run:1:abc",
        body: r#"{"format":"deino.automation"}"#,
    };

    let first = harness.service.deliver_payload(request).await.unwrap();
    let again = harness.service.deliver_payload(request).await.unwrap();

    assert_eq!(first.id, again.id);
    assert_eq!(again.status, DeliveryStatus::Succeeded);
    assert_eq!(harness.client.requests().len(), 1);
    let body = harness.client.requests()[0].body.clone().unwrap();
    assert_eq!(&body[..], request.body.as_bytes());
    assert!(harness.clock.now() >= created.webhook.created_at);
}

#[tokio::test]
async fn a_turned_off_endpoint_refuses_automation_payloads() {
    let harness = Harness::new().await;
    let mut off = input(&[], PayloadFields::default());
    off.enabled = false;
    let created = harness.service.create(off).await.unwrap();
    let meeting = MeetingId::new(MEETING);

    let result = harness
        .service
        .deliver_payload(PayloadDelivery {
            webhook: &created.webhook.id,
            meeting: &meeting,
            key: "run:1:abc",
            body: "{}",
        })
        .await;

    assert!(matches!(result, Err(WebhookError::Disabled)));
    assert!(harness.client.requests().is_empty());
}
