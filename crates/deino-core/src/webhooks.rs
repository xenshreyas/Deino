//! Outbound webhooks (DEV-01, ADR 2026-10-12-webhook-deliveries): signed
//! JSON posts to endpoints the user adds, for Zapier, Make or their own
//! server.
//!
//! - **Endpoints** ([`Webhook`]): an `https` URL (or `http://localhost`),
//!   the events it receives, the payload fields, and a signing secret shown
//!   once and kept in the Keychain ([`WebhookSecrets`]).
//! - **Events** ([`WebhookEvent`]) come from the existing services' event
//!   ports through [`taps`], never by polling, as [`WebhookSignal`]s.
//! - **Payload** ([`payload`]): Circleback's meeting payload in a Standard
//!   Webhooks envelope, assembled by EXP-01 (redacted under NOT-09).
//! - **Signing** ([`signing`]): Standard Webhooks headers and `x-signature`.
//! - **Delivery** ([`Dispatcher`], [`DispatchLoop`], [`schedule`]): 15 s
//!   per attempt, exponential backoff for 24 h, durable across restarts, a
//!   delivery log, and auto-disable after a day of failures.
//! - **Automations:** [`WebhookStep`] is AUT-01's webhook step.
//!
//! Every request goes through the `webhooks` scoped client (ADR 0019): its
//! hosts are the endpoints' ([`WebhookService::hosts`]), offline mode stops
//! it, and the egress log records it.
//!
//! | File | Role |
//! |---|---|
//! | `model.rs`, `delivery_log.rs` | Events, URLs, endpoints; delivery log entries |
//! | `signing.rs` | [`SigningSecret`] and the signature headers |
//! | `schedule.rs` | Timeout, retry, give-up and auto-disable rules |
//! | `payload.rs` | The event body |
//! | `repo.rs`, `deliveries.rs` | SQLite storage |
//! | `secrets.rs` | Signing secrets in the Keychain |
//! | `ports.rs` | [`WebhookPorts`], [`SubjectItems`] |
//! | `service.rs`, `manual.rs` | [`WebhookService`]: endpoints, signals, test sends |
//! | `dispatch.rs`, `sender.rs`, `run_loop.rs` | Attempts and the background loop |
//! | `signal.rs`, `taps.rs` | Signals and the event-port taps |
//! | `step.rs` | The automations step |

mod deliveries;
mod delivery_log;
mod dispatch;
mod error;
mod manual;
mod model;
pub mod payload;
mod ports;
mod repo;
mod run_loop;
pub mod schedule;
mod secrets;
mod sender;
mod service;
mod signal;
pub mod signing;
mod step;
pub mod taps;

#[cfg(test)]
mod dispatch_tests;
#[cfg(test)]
mod payload_tests;
#[cfg(test)]
mod service_tests;
#[cfg(test)]
mod testing;

pub use deliveries::{
    AttemptRecord, Claimed, DeliveryRepo, KEEP_FINISHED, NewDelivery, SqliteDeliveryRepo,
};
pub use delivery_log::{Delivery, DeliveryStatus};
pub use dispatch::{Dispatcher, message_id};
pub use error::WebhookError;
pub use manual::PayloadDelivery;
pub use model::{
    CreatedWebhook, DeliveryKind, DisabledReason, MAX_LABEL_CHARS, MAX_URL_BYTES, OtherKind,
    Webhook, WebhookEvent, WebhookInput, WebhookUrl,
};
pub use payload::{MeetingData, SubjectItem, WebhookPayload};
pub use ports::{RepoSubjectItems, SubjectItems, WebhookPorts};
pub use repo::{SqliteWebhookRepo, Streak, WebhookRepo};
pub use run_loop::{DispatchLoop, IDLE_WAKE};
pub use secrets::WebhookSecrets;
pub use service::{COALESCE_DELAY, MAX_LOG, WebhookService};
pub use signal::{LateSink, NoWebhooks, RecordedSignals, SignalQueue, WebhookSignal, WebhookSink};
pub use signing::SigningSecret;
pub use step::WebhookStep;
