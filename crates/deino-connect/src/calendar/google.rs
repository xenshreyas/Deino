//! Google Calendar (API v3) as a calendar source: every calendar on the
//! account's calendar list, and each one's events in the window with
//! recurring events expanded into instances.

mod map;
mod wire;

use async_trait::async_trait;
use deino_core::calendar::{SourceAccount, SourceCalendar, SourceSnapshot, SyncWindow};
use deino_domain::{CalendarProvider, ServiceId};
use deino_net::{NetClient, NetError};
use deino_secrets::SecretName;
use url::Url;

use super::api::{CalendarApi, get_json};
use deino_integrations::OAuthSpec;
use wire::{CalendarEntry, CalendarListPage, EventsPage};

/// The API's root.
const API: &str = "https://www.googleapis.com/calendar/v3";

/// Events asked for per page (Google's maximum).
const PAGE_SIZE: &str = "2500";

/// The Google Calendar API.
#[derive(Clone, Copy, Debug, Default)]
pub struct GoogleCalendar;

#[async_trait]
impl CalendarApi for GoogleCalendar {
    fn provider(&self) -> CalendarProvider {
        CalendarProvider::Google
    }

    fn oauth(&self) -> OAuthSpec {
        super::oauth::google()
    }

    fn service(&self) -> ServiceId {
        ServiceId::GoogleCalendar
    }

    fn refresh_token_secret(&self) -> SecretName {
        SecretName::GoogleCalendarRefreshToken
    }

    async fn snapshot(
        &self,
        http: &dyn NetClient,
        access_token: &str,
        window: SyncWindow,
    ) -> Result<SourceSnapshot, NetError> {
        let entries = calendar_list(http, access_token).await?;
        // The primary calendar's ID is the account's address.
        let name = entries
            .iter()
            .find(|entry| entry.primary)
            .map_or_else(|| "Google".to_owned(), |entry| entry.id.clone());
        let mut calendars = Vec::with_capacity(entries.len());
        for entry in &entries {
            let mut calendar: SourceCalendar = map::calendar(entry);
            calendar.events = events(http, access_token, &entry.id, window).await?;
            calendars.push(calendar);
        }
        Ok(SourceSnapshot {
            accounts: vec![SourceAccount { name, calendars }],
        })
    }
}

/// Every calendar on the account's list, all pages.
async fn calendar_list(http: &dyn NetClient, token: &str) -> Result<Vec<CalendarEntry>, NetError> {
    let mut entries = Vec::new();
    let mut page_token: Option<String> = None;
    loop {
        let mut url = endpoint(&["users", "me", "calendarList"])?;
        if let Some(page) = &page_token {
            url.query_pairs_mut().append_pair("pageToken", page);
        }
        let page: CalendarListPage = get_json(http, token, url.as_str(), None).await?;
        entries.extend(page.items.into_iter().filter(|entry| !entry.deleted));
        page_token = page.next_page_token;
        if page_token.is_none() {
            return Ok(entries);
        }
    }
}

/// One calendar's events overlapping `window`, all pages.
async fn events(
    http: &dyn NetClient,
    token: &str,
    calendar: &str,
    window: SyncWindow,
) -> Result<Vec<deino_core::calendar::SourceEvent>, NetError> {
    let mut events = Vec::new();
    let mut page_token: Option<String> = None;
    loop {
        let mut url = endpoint(&["calendars", calendar, "events"])?;
        url.query_pairs_mut()
            .append_pair("singleEvents", "true")
            .append_pair("timeMin", &window.start.to_rfc3339())
            .append_pair("timeMax", &window.end.to_rfc3339())
            .append_pair("maxResults", PAGE_SIZE);
        if let Some(page) = &page_token {
            url.query_pairs_mut().append_pair("pageToken", page);
        }
        let page: EventsPage = get_json(http, token, url.as_str(), None).await?;
        events.extend(page.items.into_iter().filter_map(map::event));
        page_token = page.next_page_token;
        if page_token.is_none() {
            return Ok(events);
        }
    }
}

/// The API URL with `segments` appended, each percent-encoded (calendar IDs
/// hold `@` and `#`).
fn endpoint(segments: &[&str]) -> Result<Url, NetError> {
    let mut url = Url::parse(API).map_err(|_| NetError::InvalidUrl)?;
    url.path_segments_mut()
        .map_err(|()| NetError::InvalidUrl)?
        .extend(segments);
    Ok(url)
}

#[cfg(test)]
mod google_tests;
