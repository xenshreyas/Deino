//! The device authorization grant (RFC 8628), for providers that offer it
//! and a sign-in that cannot use a loopback redirect: Deino shows a short
//! code, the user enters it on the provider's page, and Deino polls.
//!
//! No provider in `docs/research/integrations-oauth.md` needs it yet
//! (Microsoft supports it; Google limits it to devices without a browser),
//! so the Integrations page does not offer it; a provider card wires it in
//! when it is the better path.

use std::time::Duration;

use chrono::{DateTime, TimeDelta, Utc};
use deino_domain::Clock;
use deino_net::Request;
use deino_secrets::SecretValue;
use serde::Deserialize;
use tokio_util::sync::CancellationToken;
use url::form_urlencoded;

use super::{IssuedTokens, OAuthError, TokenEndpoint};
use crate::Sleeper;

/// The grant type of the polling requests.
const DEVICE_GRANT: &str = "urn:ietf:params:oauth:grant-type:device_code";

/// The poll interval when the provider names none (RFC 8628 §3.2).
const DEFAULT_INTERVAL: Duration = Duration::from_secs(5);

/// What `slow_down` adds to the interval (RFC 8628 §3.5).
const SLOW_DOWN_STEP: Duration = Duration::from_secs(5);

/// A started device authorization: what to show, and what to poll with.
#[derive(Debug)]
pub struct DeviceAuthorization {
    /// The code the user types.
    pub user_code: String,
    /// Where the user types it.
    pub verification_uri: String,
    /// The same page with the code filled in, if the provider offers one.
    pub verification_uri_complete: Option<String>,
    device_code: SecretValue,
    interval: Duration,
    expires_at: DateTime<Utc>,
}

#[derive(Deserialize)]
struct DeviceWire {
    device_code: String,
    user_code: String,
    verification_uri: String,
    verification_uri_complete: Option<String>,
    expires_in: u64,
    interval: Option<u64>,
}

/// Starts a device authorization at `url` for `scopes`.
pub async fn start(
    endpoint: &TokenEndpoint<'_>,
    url: &str,
    scopes: &str,
    clock: &dyn Clock,
) -> Result<DeviceAuthorization, OAuthError> {
    let body = form_urlencoded::Serializer::new(String::new())
        .append_pair("client_id", &endpoint.app.client_id)
        .append_pair("scope", scopes)
        .finish();
    let request = Request::post(url)
        .map_err(|_| OAuthError::BadEndpoint)?
        .header("content-type", "application/x-www-form-urlencoded")
        .header("accept", "application/json")
        .body(body);
    let wire: DeviceWire = endpoint
        .client
        .send(request)
        .await?
        .error_for_status()?
        .json()
        .await?;
    let lifetime = TimeDelta::seconds(i64::try_from(wire.expires_in).unwrap_or(i64::MAX));
    Ok(DeviceAuthorization {
        device_code: SecretValue::parse(&wire.device_code)
            .map_err(|_| OAuthError::BadTokenResponse)?,
        user_code: wire.user_code,
        verification_uri: wire.verification_uri,
        verification_uri_complete: wire.verification_uri_complete,
        interval: wire.interval.map_or(DEFAULT_INTERVAL, Duration::from_secs),
        expires_at: clock.now() + lifetime,
    })
}

/// The clock and the way to wait between polls.
pub struct Pacing<'a> {
    /// The current time.
    pub clock: &'a dyn Clock,
    /// Waits for the next poll.
    pub sleeper: &'a dyn Sleeper,
}

/// Polls until the user approves (the tokens), declines
/// ([`OAuthError::AccessDenied`]), the code expires ([`OAuthError::Expired`])
/// or `cancel` fires (`Ok(None)`).
pub async fn poll(
    endpoint: &TokenEndpoint<'_>,
    device: &DeviceAuthorization,
    pacing: &Pacing<'_>,
    cancel: &CancellationToken,
) -> Result<Option<IssuedTokens>, OAuthError> {
    let mut interval = device.interval;
    let form = [
        ("grant_type", DEVICE_GRANT),
        ("device_code", device.device_code.expose()),
    ];
    loop {
        let next = pacing.clock.now() + TimeDelta::from_std(interval).unwrap_or(TimeDelta::MAX);
        if next >= device.expires_at {
            return Err(OAuthError::Expired);
        }
        tokio::select! {
            () = pacing.sleeper.sleep_until(next) => {}
            () = cancel.cancelled() => return Ok(None),
        }
        match endpoint.post_for_tokens(&form).await {
            Ok(tokens) => return Ok(Some(tokens)),
            Err(OAuthError::Provider(code)) if code == "authorization_pending" => {}
            Err(OAuthError::Provider(code)) if code == "slow_down" => interval += SLOW_DOWN_STEP,
            Err(OAuthError::Provider(code)) if code == "access_denied" => {
                return Err(OAuthError::AccessDenied);
            }
            Err(OAuthError::Provider(code)) if code == "expired_token" => {
                return Err(OAuthError::Expired);
            }
            Err(error) => return Err(error),
        }
    }
}

#[cfg(test)]
mod device_tests;
