//! The Microsoft Graph v1.0 JSON this connector reads: only the fields it
//! uses, every one optional.

use serde::Deserialize;

/// `/me`: whose calendars these are.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Me {
    pub(super) mail: Option<String>,
    pub(super) user_principal_name: Option<String>,
}

/// A page of a collection, with the next page's absolute URL.
#[derive(Debug, Deserialize)]
pub(super) struct Page<T> {
    #[serde(default = "Vec::new")]
    pub(super) value: Vec<T>,
    #[serde(rename = "@odata.nextLink")]
    pub(super) next_link: Option<String>,
}

/// One calendar.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Calendar {
    pub(super) id: String,
    pub(super) name: Option<String>,
    pub(super) hex_color: Option<String>,
}

/// One event of a calendar view (occurrences already expanded).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Event {
    pub(super) id: String,
    #[serde(rename = "iCalUId")]
    pub(super) ical_uid: Option<String>,
    pub(super) subject: Option<String>,
    pub(super) body: Option<Body>,
    pub(super) location: Option<Location>,
    pub(super) online_meeting: Option<OnlineMeeting>,
    pub(super) online_meeting_url: Option<String>,
    #[serde(default)]
    pub(super) is_all_day: bool,
    #[serde(default)]
    pub(super) is_cancelled: bool,
    #[serde(default)]
    pub(super) is_organizer: bool,
    pub(super) show_as: Option<String>,
    pub(super) series_master_id: Option<String>,
    pub(super) start: Option<DateTimeZone>,
    pub(super) end: Option<DateTimeZone>,
    #[serde(default)]
    pub(super) attendees: Vec<Attendee>,
    pub(super) organizer: Option<Recipient>,
    pub(super) response_status: Option<ResponseStatus>,
}

/// The body, asked for as plain text.
#[derive(Debug, Deserialize)]
pub(super) struct Body {
    pub(super) content: Option<String>,
}

/// Where it takes place.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Location {
    pub(super) display_name: Option<String>,
}

/// A Teams (or other online) meeting's details.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct OnlineMeeting {
    pub(super) join_url: Option<String>,
}

/// A wall-clock time and its zone (UTC, as the request prefers).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct DateTimeZone {
    pub(super) date_time: String,
}

/// An invitee: `required`, `optional` or `resource`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Attendee {
    #[serde(rename = "type")]
    pub(super) kind: Option<String>,
    pub(super) status: Option<ResponseStatus>,
    pub(super) email_address: Option<EmailAddress>,
}

/// The organiser.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Recipient {
    pub(super) email_address: Option<EmailAddress>,
}

/// A name and an address.
#[derive(Debug, Deserialize)]
pub(super) struct EmailAddress {
    pub(super) name: Option<String>,
    pub(super) address: Option<String>,
}

/// An answer: `none`, `organizer`, `tentativelyAccepted`, `accepted`,
/// `declined` or `notResponded`.
#[derive(Debug, Deserialize)]
pub(super) struct ResponseStatus {
    pub(super) response: Option<String>,
}
