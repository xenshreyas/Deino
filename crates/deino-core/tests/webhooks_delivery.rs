//! End to end (DEV-01): a webhook endpoint on a local test server receives
//! a fixture meeting through the real HTTP client and the real `webhooks`
//! scoped client (ADR 0019). The receiver checks the Standard Webhooks
//! signature and `x-signature` itself; offline mode sends nothing; and every
//! attempt, sent or blocked, lands in the egress log.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "an integration-test crate: helpers fail the test by panicking"
)]

use std::fmt::Write as _;
use std::sync::{Arc, Mutex, PoisonError};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use chrono::{FixedOffset, TimeZone, Utc};
use deino_core::action_items::SqliteActionItemRepo;
use deino_core::automations::{AssembledMeetings, PayloadFields};
use deino_core::export::{ExportAssembler, ExportPorts, FixedZone};
use deino_core::meetings::SqliteMeetingRepo;
use deino_core::redaction::NoRedaction;
use deino_core::webhooks::{
    DeliveryStatus, RepoSubjectItems, SqliteDeliveryRepo, SqliteWebhookRepo, WebhookEvent,
    WebhookInput, WebhookPorts, WebhookSecrets, WebhookService, WebhookSignal,
};
use deino_db::{Database, test_db};
use deino_domain::{FakeClock, MeetingId, ServiceId};
use deino_net::{
    EgressLog, EgressOutcome, EgressReader, FakeServicePolicy, Net, NetParts, ReqwestNetClient,
};
use deino_secrets::InMemoryCredentialStore;
use deino_testkit::fixture::seed_fixtures;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::task::JoinSet;

const STANDUP: &str = "meeting-fixture-standup";

/// One request the receiver got: its head and its body.
#[derive(Clone, Debug)]
struct Received {
    head: String,
    body: Vec<u8>,
}

impl Received {
    /// A header's value; names are matched without regard to case.
    fn header(&self, name: &str) -> String {
        self.head
            .lines()
            .filter_map(|line| line.split_once(": "))
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.to_owned())
            .unwrap()
    }
}

/// A receiver on 127.0.0.1 that answers every request with `status`.
struct Receiver {
    url: String,
    received: Arc<Mutex<Vec<Received>>>,
    _tasks: JoinSet<()>,
}

impl Receiver {
    async fn start(status: u16) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/hooks/deino", listener.local_addr().unwrap());
        let received = Arc::new(Mutex::new(Vec::new()));
        let mut tasks = JoinSet::new();
        let store = received.clone();
        tasks.spawn(async move {
            loop {
                let (mut socket, _) = listener.accept().await.unwrap();
                let request = read_request(&mut socket).await;
                store
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .push(request);
                let reply = format!(
                    "HTTP/1.1 {status} OK\r\ncontent-length: 0\r\nconnection: close\r\n\r\n"
                );
                socket.write_all(reply.as_bytes()).await.unwrap();
            }
        });
        Self {
            url,
            received,
            _tasks: tasks,
        }
    }

    fn received(&self) -> Vec<Received> {
        self.received
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

async fn read_request(socket: &mut tokio::net::TcpStream) -> Received {
    let mut head = Vec::new();
    let mut byte = [0_u8; 1];
    while !head.ends_with(b"\r\n\r\n") {
        assert_eq!(
            socket.read(&mut byte).await.unwrap(),
            1,
            "the request ended early"
        );
        head.push(byte[0]);
    }
    let head = String::from_utf8_lossy(&head).into_owned();
    let mut request = Received {
        head,
        body: Vec::new(),
    };
    let length: usize = request.header("content-length").trim().parse().unwrap();
    request.body = vec![0_u8; length];
    socket.read_exact(&mut request.body).await.unwrap();
    request
}

struct World {
    service: WebhookService,
    policy: Arc<FakeServicePolicy>,
    egress: Arc<EgressLog>,
}

fn world(database: &Database) -> World {
    let clock = Arc::new(FakeClock::new(
        Utc.with_ymd_and_hms(2026, 10, 12, 9, 0, 0).unwrap(),
    ));
    let policy = Arc::new(FakeServicePolicy::default());
    // What the app's policy does while an endpoint names the host.
    policy.add_custom_host(ServiceId::Webhooks, "127.0.0.1");
    let egress = Arc::new(EgressLog::default());
    let net = Net::new(NetParts {
        client: Arc::new(ReqwestNetClient::new().unwrap()),
        policy: policy.clone(),
        egress: egress.clone(),
        clock: clock.clone(),
    });
    let zone = FixedZone(FixedOffset::west_opt(7 * 3_600).unwrap());
    let assembler = ExportAssembler::new(ExportPorts::sqlite(database, Arc::new(zone)));
    let service = WebhookService::new(WebhookPorts {
        webhooks: Arc::new(SqliteWebhookRepo::new(database.clone())),
        deliveries: Arc::new(SqliteDeliveryRepo::new(database.clone())),
        secrets: WebhookSecrets::new(Arc::new(InMemoryCredentialStore::default())),
        meetings: Arc::new(AssembledMeetings::new(
            Arc::new(assembler),
            Arc::new(SqliteMeetingRepo::new(database.clone())),
        )),
        items: Arc::new(RepoSubjectItems::new(
            Arc::new(SqliteActionItemRepo::new(database.clone())),
            Arc::new(NoRedaction),
        )),
        client: Arc::new(net.scoped(ServiceId::Webhooks)),
        switch: policy.clone(),
        clock,
    });
    World {
        service,
        policy,
        egress,
    }
}

fn endpoint(url: &str) -> WebhookInput {
    WebhookInput {
        url: url.to_owned(),
        label: "Local".into(),
        events: vec![WebhookEvent::MeetingReady],
        fields: PayloadFields {
            transcript: true,
            ..PayloadFields::default()
        },
        enabled: true,
    }
}

fn hmac(key: &[u8], parts: &[&[u8]]) -> Vec<u8> {
    let mut mac = Hmac::<Sha256>::new_from_slice(key).unwrap();
    for part in parts {
        mac.update(part);
    }
    mac.finalize().into_bytes().to_vec()
}

/// Checks both signatures the way a receiver would, from the secret as shown.
fn assert_signed(request: &Received, secret: &str) {
    // The receiver verifies, as the Standard Webhooks spec tells it to.
    let key = STANDARD
        .decode(secret.trim_start_matches("whsec_"))
        .unwrap();
    let signed = format!(
        "{}.{}.",
        request.header("webhook-id"),
        request.header("webhook-timestamp")
    );
    let expected = STANDARD.encode(hmac(&key, &[signed.as_bytes(), &request.body]));
    assert_eq!(
        request.header("webhook-signature"),
        format!("v1,{expected}")
    );
    let hex: String =
        hmac(secret.as_bytes(), &[&request.body])
            .iter()
            .fold(String::new(), |mut hex, byte| {
                write!(hex, "{byte:02x}").unwrap();
                hex
            });
    assert_eq!(request.header("x-signature"), hex);
}

#[tokio::test]
async fn a_ready_meeting_reaches_a_local_endpoint_signed_and_logged() {
    let database = test_db().await;
    seed_fixtures(&database).await.unwrap();
    let receiver = Receiver::start(200).await;
    let world = world(&database);
    let created = world.service.create(endpoint(&receiver.url)).await.unwrap();
    let ready = WebhookSignal::meeting(WebhookEvent::MeetingReady, MeetingId::new(STANDUP));

    world.service.accept(&ready).await.unwrap();
    world.service.dispatcher().run_due().await.unwrap();

    let got = receiver.received();
    assert_eq!(got.len(), 1);
    let request = &got[0];
    assert!(request.head.starts_with("POST /hooks/deino "));
    assert_signed(request, &created.secret);
    let json: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
    assert_eq!(json["type"], "meeting.ready");
    assert_eq!(json["data"]["name"], "Eng standup");
    assert!(json["data"]["transcript"].as_array().unwrap().len() > 3);
    let log = world.service.deliveries(&created.webhook.id).await.unwrap();
    assert_eq!(
        (log[0].status, log[0].last_status),
        (DeliveryStatus::Succeeded, Some(200))
    );
    let entries = world.egress.recent(10).await.unwrap();
    assert_eq!(entries[0].service, ServiceId::Webhooks);
    assert_eq!(entries[0].outcome, EgressOutcome::Sent);
    assert_eq!(
        entries[0].host,
        receiver
            .url
            .trim_start_matches("http://")
            .trim_end_matches("/hooks/deino")
    );
}

#[tokio::test]
async fn offline_mode_sends_nothing_and_logs_the_block() {
    let database = test_db().await;
    seed_fixtures(&database).await.unwrap();
    let receiver = Receiver::start(200).await;
    let world = world(&database);
    let created = world.service.create(endpoint(&receiver.url)).await.unwrap();
    world.policy.set_offline(true);

    let delivery = world.service.send_test(&created.webhook.id).await.unwrap();

    assert!(receiver.received().is_empty());
    assert_eq!(delivery.status, DeliveryStatus::Failed);
    assert!(delivery.last_error.unwrap().contains("offline mode"));
    let entries = world.egress.recent(10).await.unwrap();
    assert_eq!(entries[0].outcome, EgressOutcome::Blocked);
    let webhook = world.service.list().await.unwrap().remove(0);
    assert_eq!(
        webhook.failure_streak, 0,
        "offline is not the endpoint's fault"
    );
}

#[tokio::test]
async fn a_host_no_endpoint_names_is_refused() {
    let database = test_db().await;
    seed_fixtures(&database).await.unwrap();
    let world = world(&database);
    let created = world
        .service
        .create(endpoint("https://hooks.zapier.com/x"))
        .await
        .unwrap();

    let delivery = world.service.send_test(&created.webhook.id).await.unwrap();

    assert_eq!(delivery.status, DeliveryStatus::Failed);
    assert!(
        delivery
            .last_error
            .unwrap()
            .contains("not a host of webhooks")
    );
}
