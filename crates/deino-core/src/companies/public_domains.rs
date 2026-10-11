//! Personal mail providers: email domains that say nothing about where
//! someone works.
//!
//! Companies are upserted from attendee email domains (ORG-01, ORG-04), and
//! the user's own work domains tell internal meetings from external ones.
//! Neither may treat `gmail.com` as a company: two people sharing it have
//! nothing in common but their mail provider.

/// Mail providers whose domain never identifies a company. Sorted, lower
/// case, so [`is_public_email_domain`] can binary-search it.
pub const PUBLIC_EMAIL_DOMAINS: &[&str] = &[
    "126.com",
    "163.com",
    "aol.com",
    "att.net",
    "bellsouth.net",
    "btinternet.com",
    "comcast.net",
    "cox.net",
    "duck.com",
    "earthlink.net",
    "email.com",
    "fastmail.com",
    "fastmail.fm",
    "free.fr",
    "gmail.com",
    "gmx.com",
    "gmx.de",
    "gmx.net",
    "googlemail.com",
    "hey.com",
    "hotmail.co.uk",
    "hotmail.com",
    "hotmail.fr",
    "hushmail.com",
    "icloud.com",
    "inbox.com",
    "juno.com",
    "laposte.net",
    "live.co.uk",
    "live.com",
    "mac.com",
    "mail.com",
    "mail.ru",
    "me.com",
    "msn.com",
    "naver.com",
    "orange.fr",
    "outlook.com",
    "pm.me",
    "proton.me",
    "protonmail.ch",
    "protonmail.com",
    "qq.com",
    "rediffmail.com",
    "rocketmail.com",
    "sbcglobal.net",
    "seznam.cz",
    "sky.com",
    "t-online.de",
    "tutanota.com",
    "verizon.net",
    "web.de",
    "yahoo.ca",
    "yahoo.co.in",
    "yahoo.co.jp",
    "yahoo.co.uk",
    "yahoo.com",
    "yahoo.fr",
    "yandex.com",
    "yandex.ru",
    "ymail.com",
    "zoho.com",
];

/// True when `domain` (lower case) belongs to a personal mail provider.
pub fn is_public_email_domain(domain: &str) -> bool {
    PUBLIC_EMAIL_DOMAINS.binary_search(&domain).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_list_is_sorted_lower_case_and_unique_for_binary_search() {
        let mut sorted = PUBLIC_EMAIL_DOMAINS.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted, PUBLIC_EMAIL_DOMAINS);
        assert!(
            PUBLIC_EMAIL_DOMAINS
                .iter()
                .all(|domain| *domain == domain.to_lowercase())
        );
    }

    #[test]
    fn personal_providers_are_public_and_work_domains_are_not() {
        assert!(is_public_email_domain("gmail.com"));
        assert!(is_public_email_domain("outlook.com"));
        assert!(!is_public_email_domain("deino.dev"));
        assert!(!is_public_email_domain("mail.acme.com"));
    }
}
