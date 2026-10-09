//! Internal or external: is anyone outside the user's organisation invited?
//!
//! The user's own addresses come from the calendars themselves: every
//! account marks the user's entry in an invitation (`EventKit`'s current
//! user, Google's `self`). Their work domains, and subdomains of them, are
//! internal; personal mail providers never are, since sharing `gmail.com`
//! says nothing about working together.
//!
//! NOT-05 adds the same rule over the profile's addresses
//! (`profile::InternalDomains`); once it is on `main`, this helper takes the
//! profile's addresses too and the two merge (PARITY_PLAN §7.2).

use std::collections::BTreeSet;

use deino_domain::Email;

use crate::companies::is_public_email_domain;

/// The user's own addresses and the work domains they make internal.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OwnAddresses {
    own: BTreeSet<Email>,
    domains: BTreeSet<String>,
}

impl OwnAddresses {
    /// The organisation of someone with these addresses.
    pub fn from_emails(emails: impl IntoIterator<Item = Email>) -> Self {
        let own: BTreeSet<Email> = emails.into_iter().collect();
        let domains = own
            .iter()
            .map(Email::domain)
            .filter(|domain| !is_public_email_domain(domain))
            .map(str::to_owned)
            .collect();
        Self { own, domains }
    }

    /// True when `email` is one of the user's own addresses.
    pub fn is_own(&self, email: &Email) -> bool {
        self.own.contains(email)
    }

    /// True when `email` is the user's, or sits at one of their work
    /// domains or a subdomain of one (`eng.deino.dev` for `deino.dev`).
    pub fn is_internal(&self, email: &Email) -> bool {
        self.is_own(email) || within_any(email.domain(), &self.domains)
    }

    /// True when anyone among `attendees` is outside the organisation.
    /// Unknown (so `false`) while none of the user's addresses is known.
    pub fn any_external<'a>(&self, attendees: impl IntoIterator<Item = &'a Email>) -> bool {
        !self.own.is_empty() && attendees.into_iter().any(|email| !self.is_internal(email))
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

    fn email(address: &str) -> Email {
        Email::parse(address).unwrap()
    }

    fn deino() -> OwnAddresses {
        OwnAddresses::from_emails([email("maya@deino.dev"), email("maya.rao@gmail.com")])
    }

    #[test]
    fn colleagues_are_internal_and_one_outsider_makes_the_event_external() {
        let colleagues = [email("maya@deino.dev"), email("noor@deino.dev")];
        let with_prospect = [email("noor@deino.dev"), email("marcus@brightkite.com")];

        assert!(!deino().any_external(&colleagues));
        assert!(deino().any_external(&with_prospect));
    }

    #[test]
    fn subdomains_are_internal_but_lookalike_domains_are_not() {
        assert!(deino().is_internal(&email("ops@eng.deino.dev")));
        assert!(!deino().is_internal(&email("eve@notdeino.dev")));
    }

    #[test]
    fn a_personal_provider_is_never_the_organisation() {
        assert!(deino().is_own(&email("maya.rao@gmail.com")));
        assert!(!deino().is_internal(&email("sam@gmail.com")));
        assert!(deino().any_external(&[email("sam@gmail.com")]));
    }

    #[test]
    fn nothing_is_external_until_an_own_address_is_known() {
        let strangers = [email("marcus@brightkite.com")];
        assert!(!OwnAddresses::default().any_external(&strangers));
    }
}
