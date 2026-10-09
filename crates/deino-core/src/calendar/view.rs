//! What the UI is told about calendar sync: [`CalendarStatus`] and
//! [`CalendarView`].

use chrono::{DateTime, Utc};
use deino_domain::{CalendarAccountId, CalendarId, CalendarProvider};
use serde::Serialize;

use super::{CalendarAccess, CalendarColor};

/// The state of calendar sync, for Settings → Calendars.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CalendarStatus {
    /// The user turned macOS Calendar sync on.
    pub sync_enabled: bool,
    /// Whether macOS lets Deino read the calendars.
    pub access: CalendarAccess,
    /// When macOS Calendar's last sync finished, this session.
    pub last_synced_at: Option<DateTime<Utc>>,
    /// Why the last sync failed, in plain English; `None` after a success.
    pub last_error: Option<String>,
    /// The direct connectors (Google, Microsoft).
    pub connectors: Vec<ConnectorStatus>,
}

/// A direct connector's state, for Settings → Calendar.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ConnectorStatus {
    /// Which provider.
    pub provider: CalendarProvider,
    /// The user entered the client ID of their app registration.
    pub configured: bool,
    /// An account is signed in.
    pub connected: bool,
    /// When its last sync finished, this session.
    pub last_synced_at: Option<DateTime<Utc>>,
    /// Why its last sync failed; `None` after a success.
    pub last_error: Option<String>,
}

/// One calendar as Settings → Calendars lists it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CalendarView {
    /// The calendar.
    pub id: CalendarId,
    /// Its name.
    pub title: String,
    /// Its colour, `#RRGGBB`.
    pub color: Option<CalendarColor>,
    /// The account it belongs to.
    pub account_id: CalendarAccountId,
    /// The account's name ("iCloud", "ada@acme.com").
    pub account_name: String,
    /// Where the account's events come from.
    pub provider: CalendarProvider,
    /// False when the user hid it.
    pub visible: bool,
}
