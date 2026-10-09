//! Internal or external: who a meeting is with, judged by email domain.
//!
//! The user's own addresses (Settings → Profile) define their organisation:
//! every work domain they list is internal, and so are its subdomains.
//! Personal mail providers never are, since sharing `gmail.com` with someone
//! says nothing about working together (Granola draws the same line for its
//! organisation sharing, `docs/research/granola.md` §3.4).
//!
//! Pure and synchronous, so template auto rules (NOT-04), follow-up emails
//! (NOT-07), briefs (NOT-08) and calendar sync (ORG-01) can all share it.

use std::collections::BTreeSet;

use deino_domain::{Audience, Email};

use crate::settings::ProfileSettings;

/// Mail providers whose domain does not identify an organisation.
pub const PERSONAL_EMAIL_DOMAINS: &[&str] = &[
    "aol.com",
    "fastmail.com",
    "gmail.com",
    "gmx.com",
    "googlemail.com",
    "hey.com",
    "hotmail.com",
    "icloud.com",
    "live.com",
    "mac.com",
    "mail.com",
    "me.com",
    "msn.com",
    "outlook.com",
    "pm.me",
    "proton.me",
    "protonmail.com",
    "yahoo.com",
    "yandex.com",
    "ymail.com",
    "zoho.com",
];

/// The user's own addresses and the work domains they make internal.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct InternalDomains {
    own: BTreeSet<Email>,
    domains: BTreeSet<String>,
}

impl InternalDomains {
    /// The organisation described by `profile`'s email addresses.
    pub fn of(profile: &ProfileSettings) -> Self {
        Self::from_emails(&profile.emails)
    }

    /// The organisation of someone with these addresses.
    pub fn from_emails(emails: &[Email]) -> Self {
        let domains = emails
            .iter()
            .map(Email::domain)
            .filter(|domain| !PERSONAL_EMAIL_DOMAINS.contains(domain))
            .map(str::to_owned)
            .collect();
        Self {
            own: emails.iter().cloned().collect(),
            domains,
        }
    }

    /// True when `email` is one of the user's own addresses.
    pub fn is_own(&self, email: &Email) -> bool {
        self.own.contains(email)
    }

    /// True when `email` is the user's, or sits at one of their work domains
    /// or a subdomain of one (`eng.deino.dev` for `deino.dev`).
    pub fn is_internal(&self, email: &Email) -> bool {
        self.is_own(email) || within_any(email.domain(), &self.domains)
    }

    /// Who a meeting with `attendees` is with.
    ///
    /// `None` when that cannot be told: the user has listed no address, or
    /// the meeting has no attendee list at all. A meeting with only the user
    /// in it is internal.
    pub fn classify<'a>(&self, attendees: impl IntoIterator<Item = &'a Email>) -> Option<Audience> {
        if self.own.is_empty() {
            return None;
        }
        let mut attendees = attendees.into_iter().peekable();
        attendees.peek()?;
        let mut others = attendees.filter(|email| !self.is_own(email));
        if others.any(|email| !self.is_internal(email)) {
            Some(Audience::External)
        } else {
            Some(Audience::Internal)
        }
    }
}

/// True when `domain` is one of `domains` or a subdomain of one.
fn within_any(domain: &str, domains: &BTreeSet<String>) -> bool {
    domains.iter().any(|candidate| {
        domain == candidate
            || domain
                .strip_suffix(candidate.as_str())
                .is_some_and(|prefix| prefix.ends_with('.'))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn emails(addresses: &[&str]) -> Vec<Email> {
        addresses
            .iter()
            .map(|address| Email::parse(address).unwrap())
            .collect()
    }

    fn deino() -> InternalDomains {
        InternalDomains::from_emails(&emails(&["shreyas@deino.dev", "shreyas.rao@gmail.com"]))
    }

    #[test]
    fn colleagues_only_is_internal_and_one_outsider_makes_it_external() {
        let colleagues = emails(&["shreyas@deino.dev", "maya@deino.dev"]);
        let with_prospect = emails(&["maya@deino.dev", "marcus@brightkite-logistics.com"]);

        assert_eq!(deino().classify(&colleagues), Some(Audience::Internal));
        assert_eq!(deino().classify(&with_prospect), Some(Audience::External));
    }

    #[test]
    fn subdomains_are_internal_but_lookalike_domains_are_not() {
        let internal = deino();

        assert!(internal.is_internal(&emails(&["ops@eng.deino.dev"])[0]));
        assert!(!internal.is_internal(&emails(&["eve@notdeino.dev"])[0]));
    }

    #[test]
    fn personal_providers_never_define_the_organisation() {
        let friend = emails(&["sam@gmail.com"]);

        assert!(deino().is_own(&emails(&["shreyas.rao@gmail.com"])[0]));
        assert_eq!(deino().classify(&friend), Some(Audience::External));
    }

    #[test]
    fn a_solo_meeting_is_internal() {
        assert_eq!(
            deino().classify(&emails(&["shreyas@deino.dev"])),
            Some(Audience::Internal)
        );
    }

    #[test]
    fn unknown_without_attendees_or_without_a_profile_email() {
        let nobody: Vec<Email> = Vec::new();
        let someone = emails(&["maya@deino.dev"]);

        assert_eq!(deino().classify(&nobody), None);
        assert_eq!(InternalDomains::default().classify(&someone), None);
    }
}
