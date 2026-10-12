//! Recipe names to `/slugs`: lowercase ASCII letters and digits joined by
//! single hyphens, at most [`MAX_SLUG_CHARS`], unique among recipes.

/// The longest slug (the `recipes.slug` CHECK).
pub const MAX_SLUG_CHARS: usize = 48;
/// The slug of a name with no letters or digits.
const FALLBACK: &str = "recipe";

/// `"Objections & responses"` → `"objections-responses"`.
pub fn slugify(name: &str) -> String {
    let mut slug = String::new();
    for word in name
        .to_ascii_lowercase()
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
    {
        if !slug.is_empty() {
            slug.push('-');
        }
        slug.push_str(word);
    }
    let slug = cut(&slug, MAX_SLUG_CHARS);
    if slug.is_empty() {
        FALLBACK.to_owned()
    } else {
        slug
    }
}

/// Whether `slug` is a well-formed slug.
pub fn is_valid_slug(slug: &str) -> bool {
    (1..=MAX_SLUG_CHARS).contains(&slug.len())
        && slug
            .split('-')
            .all(|word| !word.is_empty() && word.bytes().all(|b| b.is_ascii_alphanumeric()))
        && !slug.bytes().any(|b| b.is_ascii_uppercase())
}

/// `base`, or `base-2`, `base-3`, … : the first that `taken` does not hold.
pub fn unique_slug(base: &str, taken: &[String]) -> String {
    let free = |candidate: &str| !taken.iter().any(|slug| slug == candidate);
    if free(base) {
        return base.to_owned();
    }
    (2_u32..=9_999)
        .map(|n| {
            let suffix = format!("-{n}");
            format!("{}{suffix}", cut(base, MAX_SLUG_CHARS - suffix.len()))
        })
        .find(|candidate| free(candidate))
        .unwrap_or_else(|| base.to_owned())
}

/// `slug` cut to `limit` bytes, without a trailing hyphen.
fn cut(slug: &str, limit: usize) -> String {
    slug.get(..limit.min(slug.len()))
        .unwrap_or(slug)
        .trim_end_matches('-')
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_become_hyphenated_lowercase_words() {
        assert_eq!(slugify("Objections & responses"), "objections-responses");
        assert_eq!(slugify("  What did I commit to? "), "what-did-i-commit-to");
        assert_eq!(
            slugify("Mochary-style 1:1 review"),
            "mochary-style-1-1-review"
        );
        assert_eq!(slugify("✨"), "recipe");
    }

    #[test]
    fn long_names_are_cut_on_a_word_boundary_character() {
        let slug = slugify(&"word ".repeat(30));

        assert!(slug.len() <= MAX_SLUG_CHARS);
        assert!(is_valid_slug(&slug));
    }

    #[test]
    fn taken_slugs_get_the_next_number() {
        let taken = vec!["coach-me".to_owned(), "coach-me-2".to_owned()];

        assert_eq!(unique_slug("coach-me", &taken), "coach-me-3");
        assert_eq!(unique_slug("prep-me", &taken), "prep-me");
    }

    #[test]
    fn malformed_slugs_are_invalid() {
        for slug in ["", "-a", "a-", "a--b", "A", "a b", &"a".repeat(49)] {
            assert!(!is_valid_slug(slug), "{slug:?}");
        }
        assert!(is_valid_slug("write-follow-up-email"));
    }
}
