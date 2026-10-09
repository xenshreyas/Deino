//! The conference-link parser: which video call an event is held on.
//!
//! Calendar providers put the join link in different places: Google and
//! Microsoft in structured conference data, Zoom's add-in in the location,
//! Outlook in the notes (often behind a Safe Links redirect), people in the
//! URL field. [`find_conference`] reads those fields in order of trust and
//! returns the first link a known service ([`MeetingService`]) owns, so a
//! slide-deck link in the notes never wins over the Teams link below it.
//!
//! Pure and synchronous; the table test in `conference_tests.rs` covers the
//! invitation formats seen in the wild.

mod links;
mod services;

use deino_domain::ConferencePlatform;

pub use services::MeetingService;

/// A recognised video-call link.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Conference {
    /// The join link, with any redirect wrapper removed.
    pub url: String,
    /// The service it belongs to.
    pub service: MeetingService,
}

impl Conference {
    /// The service as `calendar_events.platform` stores it.
    pub fn platform(&self) -> ConferencePlatform {
        self.service.platform()
    }
}

/// The places an event can carry its join link, most trusted first.
#[derive(Clone, Copy, Debug, Default)]
pub struct ConferenceFields<'a> {
    /// The provider's structured conference data.
    pub conference: Option<&'a str>,
    /// The event's URL field.
    pub url: Option<&'a str>,
    /// The location field.
    pub location: Option<&'a str>,
    /// The notes or description.
    pub notes: Option<&'a str>,
}

/// The first join link of a known service in `fields`, searched in order of
/// trust; within one field, the first such link.
pub fn find_conference(fields: ConferenceFields<'_>) -> Option<Conference> {
    [fields.conference, fields.url, fields.location, fields.notes]
        .into_iter()
        .flatten()
        .flat_map(links::links_in)
        .find_map(|url| {
            MeetingService::of(&url).map(|service| Conference {
                url: url.into(),
                service,
            })
        })
}

#[cfg(test)]
mod conference_tests;
