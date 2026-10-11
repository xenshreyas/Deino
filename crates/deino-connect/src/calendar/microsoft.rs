//! Outlook calendars through Microsoft Graph (v1.0) as a calendar source:
//! the signed-in user's calendars and each one's calendar view (recurring
//! events expanded) in the window, times in UTC.

mod map;
mod wire;

use async_trait::async_trait;
use deino_core::calendar::{SourceAccount, SourceEvent, SourceSnapshot, SyncWindow};
use deino_domain::{CalendarProvider, ServiceId};
use deino_net::{NetClient, NetError};
use deino_secrets::SecretName;
use serde::de::DeserializeOwned;
use url::Url;

use super::api::{CalendarApi, get_json};
use deino_integrations::OAuthSpec;
use wire::{Calendar, Me, Page};

/// The API's root.
const API: &str = "https://graph.microsoft.com/v1.0";

/// Times in UTC and bodies as plain text, so no zone table or HTML is needed.
const PREFER: &str = "outlook.timezone=\"UTC\", outlook.body-content-type=\"text\"";

/// Events asked for per page.
const PAGE_SIZE: &str = "250";

/// The Microsoft Graph calendar API.
#[derive(Clone, Copy, Debug, Default)]
pub struct MicrosoftCalendar;

#[async_trait]
impl CalendarApi for MicrosoftCalendar {
    fn provider(&self) -> CalendarProvider {
        CalendarProvider::Microsoft
    }

    fn oauth(&self) -> OAuthSpec {
        super::oauth::microsoft()
    }

    fn service(&self) -> ServiceId {
        ServiceId::MicrosoftGraph
    }

    fn refresh_token_secret(&self) -> SecretName {
        SecretName::MicrosoftCalendarRefreshToken
    }

    async fn snapshot(
        &self,
        http: &dyn NetClient,
        access_token: &str,
        window: SyncWindow,
    ) -> Result<SourceSnapshot, NetError> {
        let me: Me = get_json(http, access_token, &format!("{API}/me"), None).await?;
        let name = me
            .mail
            .or(me.user_principal_name)
            .unwrap_or_else(|| "Microsoft".to_owned());
        let entries: Vec<Calendar> =
            all_pages(http, access_token, format!("{API}/me/calendars")).await?;
        let mut calendars = Vec::with_capacity(entries.len());
        for entry in &entries {
            let mut calendar = map::calendar(entry);
            calendar.events = events(http, access_token, (entry, &name), window).await?;
            calendars.push(calendar);
        }
        Ok(SourceSnapshot {
            accounts: vec![SourceAccount { name, calendars }],
        })
    }
}

/// One calendar's view of `window`, for the user `me`.
async fn events(
    http: &dyn NetClient,
    token: &str,
    (calendar, me): (&Calendar, &str),
    window: SyncWindow,
) -> Result<Vec<SourceEvent>, NetError> {
    let mut url = Url::parse(API).map_err(|_| NetError::InvalidUrl)?;
    url.path_segments_mut()
        .map_err(|()| NetError::InvalidUrl)?
        .extend(["me", "calendars", &calendar.id, "calendarView"]);
    url.query_pairs_mut()
        .append_pair("startDateTime", &window.start.to_rfc3339())
        .append_pair("endDateTime", &window.end.to_rfc3339())
        .append_pair("$top", PAGE_SIZE);
    let events: Vec<wire::Event> = all_pages(http, token, url.into()).await?;
    Ok(events
        .into_iter()
        .filter_map(|event| map::event(event, me))
        .collect())
}

/// Every item of a collection, following `@odata.nextLink`.
async fn all_pages<T: DeserializeOwned>(
    http: &dyn NetClient,
    token: &str,
    first: String,
) -> Result<Vec<T>, NetError> {
    let mut items = Vec::new();
    let mut next = Some(first);
    while let Some(url) = next {
        let page: Page<T> = get_json(http, token, &url, Some(PREFER)).await?;
        items.extend(page.value);
        next = page.next_link;
    }
    Ok(items)
}

#[cfg(test)]
mod microsoft_tests;
