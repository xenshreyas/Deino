//! [`MeetingFilter`]: what the Meetings page asks for, as the user chose it
//! (Circleback §14 "Filter by participants, companies, tags and date").
//! Saved views and the settings store one; [`ListQuery::resolve`] turns it
//! into the list query's parameters.

use chrono::{DateTime, Days, Utc};
use deino_domain::{Audience, CompanyId, FolderId, LocalZone, MeetingId, PersonId, TagId};
use serde::{Deserialize, Serialize};

pub use crate::action_items::DateRange;
use crate::profile::InternalDomains;

/// One Meetings filter. Every condition narrows the list and an absent or
/// empty one does not filter. Within a list any value matches (meetings
/// with Maya *or* Lee); the conditions combine (with Maya, *and* tagged).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct MeetingFilter {
    /// Only meetings held on these local days.
    pub date_range: Option<DateRange>,
    /// Only meetings one of these people attended or spoke in.
    pub people: Vec<PersonId>,
    /// Only meetings someone from one of these companies attended.
    pub companies: Vec<CompanyId>,
    /// Only meetings with one of these tags.
    pub tags: Vec<TagId>,
    /// Only meetings in one of these folders or a folder inside one.
    pub folders: Vec<FolderId>,
    /// Only meetings with at least one action item.
    pub has_action_items: bool,
    /// Only meetings with colleagues only, or with someone from outside.
    pub audience: Option<Audience>,
}

/// Where a page of the list starts: just after this row.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct MeetingCursor {
    /// The row's `created_at` exactly as stored, so the keyset comparison
    /// matches the index order.
    pub at: String,
    /// The row's ID, which breaks ties.
    pub id: MeetingId,
}

/// Who counts as a colleague, for the audience condition.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AudienceQuery {
    /// Internal or external.
    pub audience: Option<Audience>,
    /// The user's own addresses.
    pub own: Vec<String>,
    /// Their work domains; subdomains count too.
    pub domains: Vec<String>,
}

/// A [`MeetingFilter`] resolved against the time zone and the profile.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ListQuery {
    /// The first instant of the first day, if bounded.
    pub held_from: Option<DateTime<Utc>>,
    /// The first instant after the last day, if bounded.
    pub held_before: Option<DateTime<Utc>>,
    /// See [`MeetingFilter::people`].
    pub people: Vec<PersonId>,
    /// See [`MeetingFilter::companies`].
    pub companies: Vec<CompanyId>,
    /// See [`MeetingFilter::tags`].
    pub tags: Vec<TagId>,
    /// See [`MeetingFilter::folders`].
    pub folders: Vec<FolderId>,
    /// See [`MeetingFilter::has_action_items`].
    pub has_action_items: bool,
    /// See [`MeetingFilter::audience`].
    pub audience: AudienceQuery,
}

impl ListQuery {
    /// Resolves `filter`: local days become UTC instants in `zone`, and the
    /// audience is judged against the user's `domains`.
    pub fn resolve(
        filter: &MeetingFilter,
        zone: &dyn LocalZone,
        domains: &InternalDomains,
    ) -> Self {
        let range = filter.date_range.unwrap_or_default();
        Self {
            held_from: range.from.map(|day| zone.start_of(day)),
            held_before: range.to.map(|day| zone.start_of(day + Days::new(1))),
            people: filter.people.clone(),
            companies: filter.companies.clone(),
            tags: filter.tags.clone(),
            folders: filter.folders.clone(),
            has_action_items: filter.has_action_items,
            audience: AudienceQuery {
                audience: filter.audience,
                own: domains
                    .own()
                    .map(|email| email.as_str().to_owned())
                    .collect(),
                domains: domains.domains().map(str::to_owned).collect(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use deino_domain::{Email, FixedZone};
    use serde_json::json;

    use super::*;

    fn day(text: &str) -> NaiveDate {
        text.parse().unwrap()
    }

    #[test]
    fn a_saved_filter_missing_fields_reads_with_their_defaults() {
        let filter: MeetingFilter =
            serde_json::from_value(json!({ "hasActionItems": true })).unwrap();

        assert!(filter.has_action_items);
        assert_eq!(
            filter,
            MeetingFilter {
                has_action_items: true,
                ..MeetingFilter::default()
            }
        );
    }

    #[test]
    fn local_days_become_a_half_open_utc_window() {
        let filter = MeetingFilter {
            date_range: Some(DateRange {
                from: Some(day("2026-10-05")),
                to: Some(day("2026-10-06")),
            }),
            ..MeetingFilter::default()
        };
        let zone = FixedZone::hours(2).unwrap();

        let query = ListQuery::resolve(&filter, &zone, &InternalDomains::default());

        assert_eq!(
            query.held_from,
            Some("2026-10-04T22:00:00Z".parse().unwrap())
        );
        assert_eq!(
            query.held_before,
            Some("2026-10-06T22:00:00Z".parse().unwrap())
        );
    }

    #[test]
    fn the_audience_carries_the_users_addresses_and_work_domains() {
        let filter = MeetingFilter {
            audience: Some(Audience::External),
            ..MeetingFilter::default()
        };
        let emails = [
            Email::parse("me@deino.dev").unwrap(),
            Email::parse("me@gmail.com").unwrap(),
        ];

        let query = ListQuery::resolve(
            &filter,
            &FixedZone::utc(),
            &InternalDomains::from_emails(&emails),
        );

        assert_eq!(query.audience.audience, Some(Audience::External));
        assert_eq!(query.audience.own, ["me@deino.dev", "me@gmail.com"]);
        assert_eq!(query.audience.domains, ["deino.dev"]);
    }
}
