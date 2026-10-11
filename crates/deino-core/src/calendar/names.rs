//! Display names for people and companies known only by an address.

/// Two-part public suffixes, so `acme.co.uk` names "Acme", not "Co".
const TWO_PART_SUFFIXES: &[&str] = &[
    "co.in", "co.jp", "co.kr", "co.nz", "co.uk", "co.za", "com.au", "com.br", "com.mx", "com.sg",
];

/// A name for someone known only by `email`: its local part in words
/// (`maya.rao@…` → "Maya Rao").
pub(super) fn person_name(email: &str) -> String {
    let local = email.split_once('@').map_or(email, |(local, _)| local);
    let local = local.split_once('+').map_or(local, |(base, _)| base);
    title_case(local)
}

/// A name for the company at `domain` (`brightkite-logistics.com` →
/// "Brightkite Logistics", `eng.acme.co.uk` → "Acme").
pub(super) fn company_name(domain: &str) -> String {
    let registrable = TWO_PART_SUFFIXES
        .iter()
        .find_map(|suffix| domain.strip_suffix(suffix)?.strip_suffix('.'))
        .or_else(|| domain.rsplit_once('.').map(|(rest, _)| rest))
        .unwrap_or(domain);
    let label = registrable.rsplit('.').next().unwrap_or(registrable);
    title_case(label)
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
}
