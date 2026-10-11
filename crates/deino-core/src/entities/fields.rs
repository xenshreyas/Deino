//! Checking what the user types into the person and company forms.

use deino_domain::Email;

use super::{EntityError, MAX_NAME_CHARS};
use crate::companies::CompanyDomain;

/// A name: trimmed, non-empty and at most [`MAX_NAME_CHARS`] characters.
pub(super) fn name(input: &str) -> Result<String, EntityError> {
    let name = input.trim();
    if name.is_empty() {
        return Err(EntityError::EmptyName);
    }
    if name.chars().count() > MAX_NAME_CHARS {
        return Err(EntityError::NameTooLong);
    }
    Ok(name.to_owned())
}

/// An optional email address; blank means none.
pub(super) fn email(input: Option<&str>) -> Result<Option<Email>, EntityError> {
    match blank_to_none(input) {
        None => Ok(None),
        Some(text) => Email::parse(text)
            .map(Some)
            .map_err(|_| EntityError::InvalidEmail),
    }
}

/// An optional title or other free text; blank means none.
pub(super) fn text(input: Option<&str>) -> Option<String> {
    blank_to_none(input).map(str::to_owned)
}

/// An optional web domain; blank means none. Accepts what people paste: a
/// URL (`https://www.acme.com/about`), an address at the domain
/// (`sales@acme.com`) or the domain itself. A leading `www.` is dropped.
pub(super) fn domain(input: Option<&str>) -> Result<Option<CompanyDomain>, EntityError> {
    let Some(text) = blank_to_none(input) else {
        return Ok(None);
    };
    let text = text.rsplit_once('@').map_or(text, |(_, host)| host);
    let text = text.split_once("://").map_or(text, |(_, rest)| rest);
    let host = text.split(['/', '?', '#', ':']).next().unwrap_or(text);
    let host = host.strip_prefix("www.").unwrap_or(host);
    CompanyDomain::parse(host)
        .map(Some)
        .ok_or(EntityError::InvalidDomain)
}

fn blank_to_none(input: Option<&str>) -> Option<&str> {
    input.map(str::trim).filter(|text| !text.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_trimmed_and_bounded() {
        assert_eq!(name("  Priya Nair ").unwrap(), "Priya Nair");
        assert!(matches!(name("   "), Err(EntityError::EmptyName)));
        let long = "a".repeat(MAX_NAME_CHARS + 1);
        assert!(matches!(name(&long), Err(EntityError::NameTooLong)));
    }

    #[test]
    fn blank_addresses_are_none_and_bad_ones_are_refused() {
        assert_eq!(email(Some("  ")).unwrap(), None);
        assert_eq!(
            email(Some(" Priya@Deino.dev ")).unwrap().unwrap().as_str(),
            "priya@deino.dev"
        );
        assert!(matches!(
            email(Some("priya")),
            Err(EntityError::InvalidEmail)
        ));
    }

    #[test]
    fn domains_are_read_from_urls_addresses_and_bare_names() {
        for input in [
            "acme.com",
            "https://www.Acme.com/about?x=1",
            "sales@acme.com",
            "acme.com:443",
        ] {
            assert_eq!(
                domain(Some(input)).unwrap().unwrap().as_str(),
                "acme.com",
                "{input}"
            );
        }
        assert_eq!(domain(None).unwrap(), None);
        assert!(matches!(
            domain(Some("acme")),
            Err(EntityError::InvalidDomain)
        ));
    }
}
