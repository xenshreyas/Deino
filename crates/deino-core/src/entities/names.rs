//! Display names for people and companies known only by an address, and
//! the personal-domain rule.

/// Two-part public suffixes, so `acme.co.uk` names "Acme", not "Co".
const TWO_PART_SUFFIXES: &[&str] = &[
    "co.in", "co.jp", "co.kr", "co.nz", "co.uk", "co.za", "com.au", "com.br", "com.mx", "com.sg",
];

/// A name for someone known only by `email`: its local part in words
/// (`maya.rao@…` → "Maya Rao").
pub(crate) fn person_name(email: &str) -> String {
    let local = email.split_once('@').map_or(email, |(local, _)| local);
    let local = local.split_once('+').map_or(local, |(base, _)| base);
    title_case(local)
}

/// A name for the company at `domain` (`brightkite-logistics.com` →
/// "Brightkite Logistics", `eng.acme.co.uk` → "Acme").
pub(crate) fn company_name(domain: &str) -> String {
    title_case(registrable_label(domain))
}

/// True when `domain` is someone's own vanity domain rather than an
/// employer's: its registrable label is their full name run together
/// (`jordanellis.dev` for Jordan Ellis). Such a domain names one person, so
/// it never becomes a company. A one-word name never matches: "Acme" at
/// `acme.com` is more likely a shared mailbox than a person's own site.
pub(crate) fn is_personal_domain(domain: &str, name: &str) -> bool {
    let full_name = name.split_whitespace().count() >= 2;
    full_name && letters(registrable_label(domain)) == letters(name)
}

/// The label that names the organisation: `acme` in `eng.acme.co.uk`.
fn registrable_label(domain: &str) -> &str {
    let registrable = TWO_PART_SUFFIXES
        .iter()
        .find_map(|suffix| domain.strip_suffix(suffix)?.strip_suffix('.'))
        .or_else(|| domain.rsplit_once('.').map(|(rest, _)| rest))
        .unwrap_or(domain);
    registrable.rsplit('.').next().unwrap_or(registrable)
}

/// The ASCII letters of `text`, lower case.
fn letters(text: &str) -> String {
    text.chars()
        .filter(char::is_ascii_alphabetic)
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

/// `text` split on `.`, `_` and `-`, each word capitalised.
fn title_case(text: &str) -> String {
    text.split(['.', '_', '-'])
        .filter(|word| !word.is_empty())
        .map(capitalised)
        .collect::<Vec<_>>()
        .join(" ")
}

fn capitalised(word: &str) -> String {
    let mut chars = word.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(chars).collect()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn people_are_named_from_their_local_part() {
        assert_eq!(person_name("maya.rao@deino.dev"), "Maya Rao");
        assert_eq!(person_name("j_doe+calendar@acme.com"), "J Doe");
        assert_eq!(person_name("noor@deino.dev"), "Noor");
    }

    #[test]
    fn companies_are_named_from_their_registrable_label() {
        assert_eq!(
            company_name("brightkite-logistics.com"),
            "Brightkite Logistics"
        );
        assert_eq!(company_name("eng.acme.co.uk"), "Acme");
        assert_eq!(company_name("deino.dev"), "Deino");
    }

    #[test]
    fn a_domain_spelling_its_owners_name_is_personal() {
        assert!(is_personal_domain("jordanellis.dev", "Jordan Ellis"));
        assert!(is_personal_domain(
            "mail.jordan-ellis.co.uk",
            "Jordan Ellis"
        ));
        assert!(!is_personal_domain("deino.dev", "Jordan Ellis"));
        assert!(!is_personal_domain("acme.com", "Acme"), "one word");
        assert!(!is_personal_domain("brightkite.com", "Marcus Webb"));
    }
}
