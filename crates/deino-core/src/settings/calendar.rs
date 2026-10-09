//! The calendar section (ORG-01): whether Deino reads the user's calendars,
//! and which of them it shows.
//!
//! Turning sync on is what first asks macOS for Calendars access (PARITY_PLAN
//! §3C: a permission is requested only when the feature needing it is turned
//! on), so the switch is off by default. Hiding a calendar only filters what
//! Deino shows; its events stay synced, so showing it again is instant.

use std::collections::BTreeSet;

use deino_domain::CalendarId;
use serde::{Deserialize, Serialize};

use super::SettingsError;

/// The most calendars that can be hidden; far more than anyone subscribes to.
pub const MAX_HIDDEN_CALENDARS: usize = 500;

/// The calendar section of `Settings`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct CalendarSettings {
    /// Read macOS Calendar (every account the Mac knows) through `EventKit`.
    pub sync_enabled: bool,
    /// Calendars whose events Deino does not show (Settings → Calendars).
    pub hidden_calendars: BTreeSet<CalendarId>,
}

impl CalendarSettings {
    /// True when the user has not hidden `calendar`.
    pub fn is_visible(&self, calendar: &CalendarId) -> bool {
        !self.hidden_calendars.contains(calendar)
    }

    /// Shows or hides `calendar`.
    pub fn set_visible(&mut self, calendar: CalendarId, visible: bool) {
        if visible {
            self.hidden_calendars.remove(&calendar);
        } else {
            self.hidden_calendars.insert(calendar);
        }
    }

    /// Checks that the hidden list stays bounded.
    pub(super) fn validate(&self) -> Result<(), SettingsError> {
        if self.hidden_calendars.len() > MAX_HIDDEN_CALENDARS {
            return Err(SettingsError::invalid(
                "calendar.hiddenCalendars",
                "lists too many calendars",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use deino_domain::FakeClock;

    use super::*;

    fn calendar() -> CalendarId {
        let clock = FakeClock::new(chrono::Utc.with_ymd_and_hms(2026, 10, 9, 9, 0, 0).unwrap());
        CalendarId::generate(&clock)
    }

    #[test]
    fn sync_is_off_and_every_calendar_visible_by_default() {
        let settings = CalendarSettings::default();
        assert!(!settings.sync_enabled);
        assert!(settings.is_visible(&calendar()));
    }

    #[test]
    fn hiding_and_showing_a_calendar_round_trips() {
        let mut settings = CalendarSettings::default();
        let work = calendar();

        settings.set_visible(work, false);
        assert!(!settings.is_visible(&work));
        let json = serde_json::to_value(&settings).unwrap();
        assert_eq!(json["hiddenCalendars"][0], work.to_string());

        settings.set_visible(work, true);
        assert!(settings.is_visible(&work));
    }

    #[test]
    fn an_unbounded_hidden_list_is_rejected() {
        let clock = FakeClock::new(chrono::Utc.with_ymd_and_hms(2026, 10, 9, 9, 0, 0).unwrap());
        let settings = CalendarSettings {
            sync_enabled: true,
            hidden_calendars: (0..=MAX_HIDDEN_CALENDARS)
                .map(|_| CalendarId::generate(&clock))
                .collect(),
        };
        assert!(settings.validate().is_err());
    }
}
