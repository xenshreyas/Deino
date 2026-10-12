//! "Open in Calendar" (ORG-02): where an event lives in the user's own
//! calendar app.
//!
//! A macOS Calendar event opens in Calendar through its `ical://ekevent/`
//! link; Google and Microsoft events open their web calendar on the event's
//! day (the providers' event pages need IDs Deino does not keep). The link
//! is built in Rust from the stored event, never taken from the webview.

use chrono::{Datelike, NaiveDate};
use deino_domain::{CalendarEventId, CalendarProvider, LocalZone};
use url::Url;

use super::{CalendarError, CalendarEvent, CalendarService};

/// Where "Open in Calendar" goes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CalendarLink {
    /// macOS Calendar, through its `ical:` URL scheme.
    App(Url),
    /// The provider's calendar on the web.
    Web(Url),
}

/// The link for `event`, synced from `provider`, which starts on `day`
/// (the user's local date).
pub fn calendar_link(
    provider: CalendarProvider,
    event: &CalendarEvent,
    day: NaiveDate,
) -> Option<CalendarLink> {
    let (year, month, date) = (day.year(), day.month(), day.day());
    let raw = match provider {
        CalendarProvider::Apple => {
            let id = event_identifier(&event.provider_event_id);
            let mut url = Url::parse("ical://ekevent/").ok()?;
            url.path_segments_mut().ok()?.pop_if_empty().push(id);
            url.query_pairs_mut()
                .append_pair("method", "show")
                .append_pair("options", "more");
            return Some(CalendarLink::App(url));
        }
        CalendarProvider::Google => {
            format!("https://calendar.google.com/calendar/r/day/{year}/{month}/{date}")
        }
        CalendarProvider::Microsoft => {
            format!("https://outlook.office.com/calendar/view/day/{year}/{month}/{date}")
        }
    };
    Url::parse(&raw).ok().map(CalendarLink::Web)
}

/// `EventKit`'s `eventIdentifier` of a stored event: an occurrence's ID
/// ends in `/<its original start>`, which Calendar does not know.
fn event_identifier(provider_event_id: &str) -> &str {
    match provider_event_id.rsplit_once('/') {
        Some((id, start)) if !start.is_empty() && start.bytes().all(|b| b.is_ascii_digit()) => id,
        _ => provider_event_id,
    }
}

impl CalendarService {
    /// The "Open in Calendar" link of event `id`, on its day in `zone`.
    /// `None` when no link can be built for it.
    pub async fn calendar_link(
        &self,
        id: CalendarEventId,
        zone: &dyn LocalZone,
    ) -> Result<Option<CalendarLink>, CalendarError> {
        let event = self.event(id).await?;
        let provider = self
            .calendars()
            .await?
            .into_iter()
            .find(|calendar| Some(&calendar.id) == event.calendar_id.as_ref())
            .map_or(CalendarProvider::Apple, |calendar| calendar.provider);
        Ok(calendar_link(provider, &event, zone.day_of(event.start_at)))
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use deino_domain::CalendarProvider;

    use super::{CalendarLink, calendar_link, event_identifier};
    use crate::calendar::test_events::stored;

    fn day() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, 9).unwrap()
    }

    fn link(provider: CalendarProvider, provider_id: &str) -> CalendarLink {
        let mut event = stored("Standup", 0);
        event.provider_event_id = provider_id.into();
        calendar_link(provider, &event, day()).unwrap()
    }

    #[test]
    fn calendar_events_open_in_calendar() {
        let CalendarLink::App(url) = link(CalendarProvider::Apple, "AB12:CD34/1791536400") else {
            panic!("an app link");
        };
        assert_eq!(
            url.as_str(),
            "ical://ekevent/AB12:CD34?method=show&options=more"
        );
    }

    #[test]
    fn an_identifier_is_escaped_as_one_path_segment() {
        let CalendarLink::App(url) = link(CalendarProvider::Apple, "a b/c?d") else {
            panic!("an app link");
        };
        assert_eq!(url.path(), "/a%20b%2Fc%3Fd");
    }

    #[test]
    fn web_calendars_open_on_the_events_day() {
        assert_eq!(
            link(CalendarProvider::Google, "abc"),
            CalendarLink::Web(
                "https://calendar.google.com/calendar/r/day/2026/10/9"
                    .parse()
                    .unwrap()
            )
        );
        assert_eq!(
            link(CalendarProvider::Microsoft, "abc"),
            CalendarLink::Web(
                "https://outlook.office.com/calendar/view/day/2026/10/9"
                    .parse()
                    .unwrap()
            )
        );
    }

    #[test]
    fn only_a_numeric_suffix_is_an_occurrence() {
        assert_eq!(event_identifier("AB:CD/1791536400"), "AB:CD");
        assert_eq!(event_identifier("AB:CD/x1"), "AB:CD/x1");
        assert_eq!(event_identifier("AB:CD/"), "AB:CD/");
        assert_eq!(event_identifier("AB:CD"), "AB:CD");
    }
}
