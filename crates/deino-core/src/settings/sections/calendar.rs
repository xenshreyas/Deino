//! The calendar section (ORG-01): whether Deino reads the user's calendars,
//! and which of them it shows.
//!
//! Turning sync on is what first asks macOS for Calendars access (PARITY_PLAN
//! §3C: a permission is requested only when the feature needing it is turned
//! on), so the switch is off by default. Hiding a calendar only filters what
//! Deino shows; its events stay synced, so showing it again is instant.
//!
//! The direct connectors (Google Calendar, Microsoft Graph) take the client
//! ID of the user's own app registration (PARITY_PLAN §9). It is not a
//! secret; Google's client secret and every refresh token live in the
//! Keychain.

use std::collections::BTreeSet;

use deino_domain::CalendarId;
use serde::{Deserialize, Serialize};

use crate::settings::SettingsError;

/// The most calendars that can be hidden; far more than anyone subscribes to.
pub const MAX_HIDDEN_CALENDARS: usize = 500;

/// The longest OAuth client ID accepted (Google's are about 75 characters).
pub const MAX_CLIENT_ID_CHARS: usize = 256;

/// The calendar section of `Settings`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct CalendarSettings {
    /// Read macOS Calendar (every account the Mac knows) through `EventKit`.
    pub sync_enabled: bool,
    /// Calendars whose events Deino does not show (Settings → Calendars).
    pub hidden_calendars: BTreeSet<CalendarId>,
    /// The Google Calendar connector's app registration.
    pub google: ConnectorSettings,
    /// The Microsoft Graph connector's app registration.
    pub microsoft: ConnectorSettings,
    /// Which events record on their own (ORG-02).
    pub auto_record: AutoRecordSettings,
    /// The "starting in 1 minute" window before meetings (ORG-02).
    pub reminders: ReminderSettings,
}

/// Which calendar events record on their own (ORG-02, Circleback's
/// "joining preferences"). An event records when it passes every switch;
/// the user's per-event and per-series choices win over all of them
/// ([`crate::calendar::rules`]). Off by default: nothing records until the
/// user picks an audience.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase", default)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "independent switches the user toggles one by one in Settings → Calendar"
)]
pub struct AutoRecordSettings {
    /// Record meetings with only people from the user's own domains.
    pub internal: bool,
    /// Record meetings with someone from outside them.
    pub external: bool,
    /// Also record meetings someone else organised.
    pub organized_by_others: bool,
    /// Also record meetings the user has not accepted (tentative or no answer).
    pub not_accepted: bool,
}

impl Default for AutoRecordSettings {
    fn default() -> Self {
        Self {
            internal: false,
            external: false,
            organized_by_others: true,
            not_accepted: false,
        }
    }
}

/// The reminder window shown a minute before a meeting, with Join & record.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct ReminderSettings {
    /// Show the window at all.
    pub enabled: bool,
    /// Calendars whose events never show it.
    pub quiet_calendars: BTreeSet<CalendarId>,
}

impl Default for ReminderSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            quiet_calendars: BTreeSet::new(),
        }
    }
}

impl ReminderSettings {
    /// True when events of `calendar` (`None`: no known calendar) may remind.
    pub fn reminds_for(&self, calendar: Option<&CalendarId>) -> bool {
        self.enabled && calendar.is_none_or(|id| !self.quiet_calendars.contains(id))
    }
}

/// A direct calendar connector's app registration.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct ConnectorSettings {
    /// The OAuth client ID of the user's own app registration; empty until
    /// they set one up (docs/setup/google-calendar.md, microsoft-graph.md).
    pub client_id: String,
}

impl ConnectorSettings {
    /// The client ID, if one is set.
    pub fn client_id(&self) -> Option<&str> {
        Some(self.client_id.trim()).filter(|id| !id.is_empty())
    }
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
        if self.hidden_calendars.len() > MAX_HIDDEN_CALENDARS
            || self.reminders.quiet_calendars.len() > MAX_HIDDEN_CALENDARS
        {
            return Err(SettingsError::invalid(
                "calendar.hiddenCalendars or calendar.reminders.quietCalendars",
                "lists too many calendars",
            ));
        }
        for (setting, connector) in [
            ("calendar.google.clientId", &self.google),
            ("calendar.microsoft.clientId", &self.microsoft),
        ] {
            let id = &connector.client_id;
            if id.chars().count() > MAX_CLIENT_ID_CHARS || id.contains(char::is_whitespace) {
                return Err(SettingsError::invalid(setting, "is not a client ID"));
            }
        }
        Ok(())
    }
}

crate::settings::register_section! {
    /// Calendar sync and which calendars are shown (ORG-01).
    calendar: CalendarSettings,
    order: 150,
    validate: CalendarSettings::validate,
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
    fn reminders_are_on_except_for_quiet_calendars() {
        let mut reminders = ReminderSettings::default();
        let work = calendar();
        assert!(reminders.reminds_for(Some(&work)) && reminders.reminds_for(None));

        reminders.quiet_calendars.insert(work);
        assert!(!reminders.reminds_for(Some(&work)));
        reminders.enabled = false;
        assert!(!reminders.reminds_for(None));
    }

    #[test]
    fn an_unbounded_hidden_list_is_rejected() {
        let clock = FakeClock::new(chrono::Utc.with_ymd_and_hms(2026, 10, 9, 9, 0, 0).unwrap());
        let settings = CalendarSettings {
            sync_enabled: true,
            hidden_calendars: (0..=MAX_HIDDEN_CALENDARS)
                .map(|_| CalendarId::generate(&clock))
                .collect(),
            ..CalendarSettings::default()
        };
        assert!(settings.validate().is_err());
    }

    #[test]
    fn a_client_id_with_spaces_is_rejected_and_a_blank_one_is_unset() {
        let mut settings = CalendarSettings::default();
        assert_eq!(settings.google.client_id(), None);

        settings.google.client_id = "1234-abc.apps.googleusercontent.com".into();
        assert!(settings.validate().is_ok());
        settings.microsoft.client_id = "not an id".into();
        assert!(settings.validate().is_err());
    }
}
