//! The direct calendar connectors (ORG-01): Google Calendar and Microsoft
//! Graph, for accounts the user would rather not add to macOS Calendar, or
//! whose conference links and RSVPs the API reports more fully.
//!
//! [`ConnectorSource`] is a `deino_core::calendar::CalendarSource` over a
//! [`CalendarApi`] ([`GoogleCalendar`], [`MicrosoftCalendar`]): OAuth sign-in
//! with the user's own client ID ([`Credentials`], [`SettingsCredentials`]),
//! the refresh token in the Keychain, and each provider's JSON translated
//! into the core's plain source values. Every rule (join links, hiding,
//! external, series) is the core's, the same as for macOS Calendar.

mod api;
mod connector;
mod credentials;
mod google;
mod microsoft;

pub use api::CalendarApi;
pub use connector::{ConnectorParts, ConnectorSource};
pub use credentials::{Credentials, SettingsCredentials};
pub use google::GoogleCalendar;
pub use microsoft::MicrosoftCalendar;
