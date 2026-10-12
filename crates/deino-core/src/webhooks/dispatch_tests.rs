//! The dispatcher on a fake clock: signed attempts, the retry schedule, a
//! day's limit, auto-disable, and recovery after a restart.

use chrono::TimeDelta;
use deino_domain::{Clock, MeetingId};
use deino_net::{FakeReply, Status};

use super::testing::{Harness, MEETING, start};
use super::{
    DeliveryRepo, DeliveryStatus, DisabledReason, SigningSecret, SqliteDeliveryRepo, WebhookEvent,
    WebhookSignal,
};
use crate::webhooks::signing::SignedMessage;

fn ready() -> WebhookSignal {
    WebhookSignal::meeting(WebhookEvent::MeetingReady, MeetingId::new(MEETING))
}

#[tokio::test]
async fn an_event_is_posted_once_with_valid_standard_webhooks_and_x_signature_headers() {
    let harness = Harness::new().await;
    let created = harness.add(&[WebhookEvent::MeetingReady]).await;

    assert_eq!(harness.service.accept(&ready()).await.unwrap(), 1);
    harness.service.dispatcher().run_due().await.unwrap();

    let sent = harness.client.requests();
    assert_eq!(sent.len(), 1);
    let request = &sent[0];
    let body = request.body.clone().unwrap();
    let header = |name| request.headers.get(name).unwrap().to_owned();
    let secret = SigningSecret::parse(&created.secret).unwrap();
    let expected = secret
        .sign(&SignedMessage {
            id: &header("webhook-id"),
            timestamp: header("webhook-timestamp").parse().unwrap(),
            body: &body,
        })
        .unwrap();
    assert_eq!(header("webhook-signature"), expected.signature);
    assert_eq!(header("x-signature"), expected.x_signature);
    assert_eq!(header("webhook-timestamp"), start().timestamp().to_string());
    assert!(header("webhook-id").starts_with("msg_"));
    assert_eq!(request.timeout, Some(std::time::Duration::from_secs(15)));
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["type"], "meeting.ready");
    assert_eq!(json["data"]["name"], "Acme sync");
    let log = harness
        .service
        .deliveries(&created.webhook.id)
        .await
        .unwrap();
    assert_eq!(log[0].status, DeliveryStatus::Succeeded);
}

/// `count` requests, all one message: the same `webhook-id` and body.
fn assert_one_message_sent(sent: &[deino_net::Request], count: usize) {
    assert_eq!(sent.len(), count);
    let id = |request: &deino_net::Request| request.headers.get("webhook-id").map(str::to_owned);
    assert!(sent.iter().all(|request| id(request) == id(&sent[0])));
    assert!(sent.iter().all(|request| request.body == sent[0].body));
}

#[tokio::test]
async fn failures_retry_on_the_backoff_schedule_with_the_same_id_and_body_until_delivered() {
    let harness = Harness::new().await;
    let created = harness.add(&[WebhookEvent::MeetingReady]).await;
    for _ in 0..3 {
        harness.client.reply(FakeReply::status(Status::new(503)));
    }
    harness.client.reply(FakeReply::status(Status::new(204)));
    harness.service.accept(&ready()).await.unwrap();

    let mut due_times = vec![];
    for _ in 0..4 {
        harness.service.dispatcher().run_due().await.unwrap();
        let log = harness
            .service
            .deliveries(&created.webhook.id)
            .await
            .unwrap();
        if let Some(next) = log[0].next_attempt_at {
            due_times.push((next - start()).num_seconds());
            // Nothing is sent before it is due.
            harness.service.dispatcher().run_due().await.unwrap();
            harness.advance_to(next);
        }
    }

    assert_eq!(due_times, [30, 90, 210]);
    assert_one_message_sent(&harness.client.requests(), 4);
    let log = harness
        .service
        .deliveries(&created.webhook.id)
        .await
        .unwrap();
    assert_eq!(
        (log[0].status, log[0].attempts),
        (DeliveryStatus::Succeeded, 4)
    );
    let webhook = harness.service.list().await.unwrap().remove(0);
    assert_eq!((webhook.failure_streak, webhook.failing_since), (0, None));
}

#[tokio::test]
async fn a_client_error_is_not_retried() {
    let harness = Harness::new().await;
    let created = harness.add(&[WebhookEvent::MeetingReady]).await;
    harness.client.reply(FakeReply::status(Status::NOT_FOUND));
    harness.service.accept(&ready()).await.unwrap();

    harness.service.dispatcher().run_due().await.unwrap();

    let log = harness
        .service
        .deliveries(&created.webhook.id)
        .await
        .unwrap();
    assert_eq!(log[0].status, DeliveryStatus::Failed);
    assert_eq!(log[0].last_status, Some(404));
    assert_eq!(
        log[0].last_error.as_deref(),
        Some("the endpoint answered 404")
    );
}

/// Runs every due attempt, answering 500, until nothing is pending.
async fn fail_for_a_day(harness: &Harness) -> u32 {
    let mut attempts = 0;
    while let Some(next) = harness.service.ports.deliveries.next_due().await.unwrap() {
        harness.advance_to(next.max(harness.clock.now()));
        harness.client.reply(FakeReply::status(Status::new(500)));
        harness.service.dispatcher().run_due().await.unwrap();
        attempts += 1;
        assert!(attempts < 100, "the retries never stop");
    }
    attempts
}

#[tokio::test]
async fn an_endpoint_failing_for_a_day_is_given_up_and_turned_off_with_a_reason() {
    let harness = Harness::new().await;
    let created = harness.add(&[WebhookEvent::MeetingReady]).await;
    harness.service.accept(&ready()).await.unwrap();

    let attempts = fail_for_a_day(&harness).await;

    assert_eq!(attempts, 19);
    assert!(harness.clock.now() - start() < TimeDelta::hours(24));
    let log = harness
        .service
        .deliveries(&created.webhook.id)
        .await
        .unwrap();
    assert_eq!(log[0].status, DeliveryStatus::Failed);
    // 19 attempts span 22 h: not yet a day of failures.
    let webhook = harness.service.list().await.unwrap().remove(0);
    assert!(webhook.enabled);
    assert_eq!(webhook.failure_streak, 19);

    harness.clock.advance(TimeDelta::hours(3));
    harness.service.accept(&ready()).await.unwrap();
    fail_for_a_day(&harness).await;

    let webhook = harness.service.list().await.unwrap().remove(0);
    assert!(!webhook.enabled);
    assert_eq!(webhook.disabled_reason, Some(DisabledReason::Failing));
    assert_eq!(harness.service.accept(&ready()).await.unwrap(), 0);
}

#[tokio::test]
async fn a_410_turns_the_endpoint_off_at_once_and_turning_it_on_clears_the_notice() {
    let harness = Harness::new().await;
    let created = harness.add(&[WebhookEvent::MeetingReady]).await;
    harness.client.reply(FakeReply::status(Status::new(410)));
    harness.service.accept(&ready()).await.unwrap();

    harness.service.dispatcher().run_due().await.unwrap();

    let webhook = harness.service.list().await.unwrap().remove(0);
    assert_eq!(webhook.disabled_reason, Some(DisabledReason::Gone));
    let input = super::testing::input(&webhook.events, webhook.fields);
    let again = harness
        .service
        .update(&created.webhook.id, input)
        .await
        .unwrap();
    assert!(again.enabled);
    assert_eq!((again.disabled_reason, again.failure_streak), (None, 0));
}

#[tokio::test]
async fn an_attempt_cut_off_by_a_quit_is_picked_up_after_a_restart() {
    let harness = Harness::new().await;
    let created = harness.add(&[WebhookEvent::MeetingReady]).await;
    harness.service.accept(&ready()).await.unwrap();
    let repo = SqliteDeliveryRepo::new(harness.database.clone());
    let id = repo.due(start(), 10).await.unwrap()[0];
    repo.claim(&id, start()).await.unwrap().unwrap();

    // The app quits mid-attempt, and starts again later.
    harness.clock.advance(TimeDelta::hours(1));
    assert_eq!(repo.recover(harness.clock.now()).await.unwrap(), 1);
    harness.service.dispatcher().run_due().await.unwrap();

    assert_eq!(harness.client.requests().len(), 1);
    let log = harness
        .service
        .deliveries(&created.webhook.id)
        .await
        .unwrap();
    assert_eq!(log[0].status, DeliveryStatus::Succeeded);
}
