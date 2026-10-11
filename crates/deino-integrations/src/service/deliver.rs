//! Delivering: once per idempotency key, paced, retried and logged.
//!
//! Retries here cover a call that is already running (a rate limit with a
//! short `Retry-After`, a 5xx, a dropped connection), at most [`MAX_TRIES`]
//! tries. A delivery that still fails is left `failed` with a retryable or
//! permanent error; sending the same key again (the FND-08 pipeline
//! re-running an automation stage, or the user pressing Retry) resumes it,
//! and a key that already succeeded is never sent twice.

use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, TimeDelta, Utc};
use deino_domain::{AppError, AuthMethod, DeliveryId, DeliveryStatus, ErrorClass};

use super::IntegrationService;
use super::reads::{ensure_capability, with_capability};
use crate::connection::capped;
use crate::{
    Action, Connection, Delivery, DeliveryFailure, DeliveryRequest, ExternalRef, Integration,
    IntegrationError, NewDelivery, ProviderApi, ProviderError,
};

/// Tries per delivery call.
pub const MAX_TRIES: u32 = 3;

/// The wait before the second try; it doubles for each try after.
const FIRST_BACKOFF: Duration = Duration::from_secs(2);

/// The longest wait inside one call. A provider that asks for more is
/// left for a later attempt.
const MAX_WAIT: Duration = Duration::from_mins(1);

/// A `sending` row this old belongs to an attempt that died with the app.
const STALE_SENDING: TimeDelta = TimeDelta::minutes(10);

/// What to do after a failed try.
enum Step {
    /// Refresh the token, then try again at once.
    Refresh,
    /// Try again at this time.
    Wait(DateTime<Utc>),
    /// Give up.
    Stop,
}

impl IntegrationService {
    /// Sends `request` once per idempotency key and returns its log row. A
    /// key that already succeeded returns the first result without sending.
    pub async fn deliver(&self, request: DeliveryRequest) -> Result<Delivery, IntegrationError> {
        let connection = self.connection(request.connection).await?;
        let integration = Arc::clone(self.ports.registry.get(connection.provider)?);
        ensure_capability(integration.as_ref(), request.action.capability())?;
        let delivery = self
            .ports
            .deliveries
            .begin(&self.new_delivery(&connection, &request))
            .await?;
        if delivery.status == DeliveryStatus::Succeeded {
            return Ok(delivery);
        }
        let now = self.ports.clock.now();
        if !self
            .ports
            .deliveries
            .claim(delivery.id, now - STALE_SENDING, now)
            .await?
        {
            return Err(IntegrationError::InFlight);
        }
        let outcome = match self.ensure_online(connection.provider).await {
            Ok(()) => self.send(&connection, integration.as_ref(), &request).await,
            Err(offline) => Err(offline),
        };
        self.settle(&connection, delivery.id, outcome).await?;
        let key = request.idempotency_key.as_str();
        self.ports
            .deliveries
            .by_key(key)
            .await?
            .ok_or(IntegrationError::InFlight)
    }

    /// Up to [`MAX_TRIES`] tries, paced by the rate limiter.
    async fn send(
        &self,
        connection: &Connection,
        integration: &dyn Integration,
        request: &DeliveryRequest,
    ) -> Result<ExternalRef, IntegrationError> {
        // A pasted token cannot be refreshed: a 401 is final at once.
        let refreshable = connection.auth_method == AuthMethod::OAuth;
        let (mut force_refresh, mut refreshed) = (false, !refreshable);
        for attempt in 1..=MAX_TRIES {
            let limit = integration.descriptor().rate_limit;
            self.limiter.acquire(connection.id, limit).await;
            let api = self.session(connection, integration, force_refresh).await?;
            let api = api.with_idempotency_key(request.idempotency_key.clone());
            let error = match perform(integration, &api, &request.action).await {
                Ok(created) => return Ok(created),
                Err(error) => self.classified(integration, error),
            };
            force_refresh = false;
            match self.next_step(&error, attempt, refreshed) {
                Step::Refresh => (force_refresh, refreshed) = (true, true),
                Step::Wait(until) => self.ports.sleeper.sleep_until(until).await,
                Step::Stop => return Err(IntegrationError::from_provider(integration.id(), error)),
            }
        }
        Err(IntegrationError::Unavailable(integration.id()))
    }

    /// What follows the failed try number `attempt`.
    fn next_step(&self, error: &ProviderError, attempt: u32, refreshed: bool) -> Step {
        if matches!(error, ProviderError::Unauthorized) && !refreshed {
            return Step::Refresh;
        }
        if attempt >= MAX_TRIES || !error.is_retryable() {
            return Step::Stop;
        }
        let backoff = FIRST_BACKOFF
            .saturating_mul(1 << (attempt - 1).min(8))
            .min(MAX_WAIT);
        let wait = match error {
            ProviderError::RateLimited {
                retry_after: Some(after),
            } => *after,
            _ => backoff,
        };
        match TimeDelta::from_std(wait) {
            Ok(wait) if wait.to_std().is_ok_and(|wait| wait <= MAX_WAIT) => {
                Step::Wait(self.ports.clock.now() + wait)
            }
            _ => Step::Stop,
        }
    }

    /// Records the outcome on the delivery and the connection.
    async fn settle(
        &self,
        connection: &Connection,
        id: DeliveryId,
        outcome: Result<ExternalRef, IntegrationError>,
    ) -> Result<(), IntegrationError> {
        let now = self.ports.clock.now();
        match outcome {
            Ok(created) => {
                self.ports.deliveries.succeed(id, &created, now).await?;
                self.ports
                    .connections
                    .record_success(connection.id, now)
                    .await?;
                log::info!(target: "integrations", "delivery {id} succeeded");
                Ok(())
            }
            Err(error) => {
                let failure = failure_of(&error);
                self.ports.deliveries.fail(id, &failure, now).await?;
                self.note_failure(connection, &error).await;
                log::info!(target: "integrations", "delivery {id} failed: {}", failure.message);
                Err(error)
            }
        }
    }

    fn new_delivery(&self, connection: &Connection, request: &DeliveryRequest) -> NewDelivery {
        NewDelivery {
            id: DeliveryId::generate(self.ports.clock.as_ref()),
            connection_id: connection.id,
            provider: connection.provider,
            capability: request.action.capability(),
            meeting_id: request.meeting.clone(),
            idempotency_key: request.idempotency_key.as_str().to_owned(),
            at: self.ports.clock.now(),
        }
    }
}

/// Runs `action` through the matching capability.
async fn perform(
    integration: &dyn Integration,
    api: &ProviderApi,
    action: &Action,
) -> Result<ExternalRef, ProviderError> {
    match action {
        Action::PostMessage(draft) => {
            with_capability(integration.messages(), |c| c.post_message(api, draft)).await
        }
        Action::CreatePage(draft) => {
            with_capability(integration.pages(), |c| c.create_page(api, draft)).await
        }
        Action::CreateTask(draft) => {
            with_capability(integration.tasks(), |c| c.create_task(api, draft)).await
        }
        Action::LogActivity(draft) => {
            with_capability(integration.activities(), |c| c.log_activity(api, draft)).await
        }
    }
}

/// The user-safe failure recorded on a delivery.
fn failure_of(error: &IntegrationError) -> DeliveryFailure {
    let app = AppError::new(error.code(), error.to_string());
    DeliveryFailure {
        code: app.code,
        message: capped(&app.message),
    }
}
