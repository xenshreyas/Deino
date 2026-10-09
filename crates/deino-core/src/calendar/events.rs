//! The calendar event port: moments the UI reacts to.
//!
//! Stored rows refresh the UI through `DbChanged` already. This port covers
//! what is not a row: the sync switch, macOS access, the last sync's time and
//! error, and which calendars are hidden (a setting). The app emits each as
//! the typed `CalendarChanged` event.

use super::CalendarStatus;

/// Receives calendar moments.
pub trait CalendarEvents: Send + Sync {
    /// Calendar sync changed what the UI shows: `status` is the new state.
    fn changed(&self, status: &CalendarStatus);
}
