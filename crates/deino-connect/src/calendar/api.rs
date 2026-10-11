//! [`CalendarApi`]: what one provider's calendar API adds to a connector,
//! and the request helpers both providers share.

use async_trait::async_trait;
use chrono::{DateTime, Local, NaiveDate, Utc};
use deino_core::calendar::{SourceSnapshot, SyncWindow};
use deino_domain::{CalendarProvider, ServiceId};
use deino_net::{NetClient, NetError, Request};
use deino_secrets::SecretName;
use serde::de::DeserializeOwned;

use deino_integrations::OAuthSpec;

/// A provider's calendar API, read with an access token.
#[async_trait]
pub trait CalendarApi: Send + Sync {
    /// The provider, as `calendar_accounts.provider` stores it.
    fn provider(&self) -> CalendarProvider;

    /// Its OAuth authorization server and rules.
    fn oauth(&self) -> OAuthSpec;

    /// The `deino-net` service its requests belong to.
    fn service(&self) -> ServiceId;

    /// Where the connected account's refresh token is kept.
    fn refresh_token_secret(&self) -> SecretName;

    /// Every calendar of the signed-in account, with its events in `window`.
    async fn snapshot(
        &self,
        http: &dyn NetClient,
        access_token: &str,
        window: SyncWindow,
    ) -> Result<SourceSnapshot, NetError>;
}

/// `GET`s `url` with `access_token` (and `prefer`, if any) and decodes the
/// JSON of a 2xx answer.
pub(crate) async fn get_json<T: DeserializeOwned>(
    http: &dyn NetClient,
    access_token: &str,
    url: &str,
    prefer: Option<&str>,
) -> Result<T, NetError> {
    let mut request = Request::get(url)?
        .header("authorization", format!("Bearer {access_token}"))
        .header("accept", "application/json");
    if let Some(prefer) = prefer {
        request = request.header("prefer", prefer);
    }
    http.send(request).await?.error_for_status()?.json().await
}

/// The instant an all-day event's `date` starts: midnight in the user's
/// time zone, the same reading macOS Calendar gives all-day events.
pub(crate) fn local_midnight(date: NaiveDate) -> Option<DateTime<Utc>> {
    date.and_hms_opt(0, 0, 0)?
        .and_local_timezone(Local)
        .earliest()
        .map(|instant| instant.with_timezone(&Utc))
}
