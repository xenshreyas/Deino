//! How people and tags are written: plain names, or Obsidian wiki links and
//! `#tags` (EXP-02's Obsidian mode).

use super::blocks::escape;
use crate::LinkStyle;

/// Characters a wiki link target cannot hold: Obsidian reads them as
/// heading, block, alias or link syntax.
const WIKI_RESERVED: &[char] = &['[', ']', '|', '#', '^', '\\', '/', ':'];

/// The note name a person's wiki link points at: their name without the
/// characters Obsidian reserves. The people pages use the same name.
pub fn page_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|char| {
            if WIKI_RESERVED.contains(&char) {
                ' '
            } else {
                char
            }
        })
        .collect();
    cleaned.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// A person in body text: their escaped name, or `[[Name]]`.
pub(crate) fn person(name: &str, style: LinkStyle) -> String {
    match style {
        LinkStyle::Plain => escape(name),
        LinkStyle::Obsidian => format!("[[{}]]", page_name(name)),
    }
}

/// A person in the frontmatter: their name, or a quoted wiki link, which
/// Obsidian shows as a link property.
pub(crate) fn person_property(name: &str, style: LinkStyle) -> String {
    match style {
        LinkStyle::Plain => name.to_owned(),
        LinkStyle::Obsidian => format!("[[{}]]", page_name(name)),
    }
}

/// A tag as Obsidian accepts it: letters, digits, `_`, `-` and `/`, with
/// spaces as `-`, never only digits. Empty when nothing usable is left.
pub fn tag_slug(name: &str) -> String {
    let slug: String = name
        .trim()
        .chars()
        .filter_map(|char| match char {
            ' ' => Some('-'),
            '_' | '-' | '/' => Some(char),
            char if char.is_alphanumeric() => Some(char),
            _ => None,
        })
        .collect();
    let slug = slug.trim_matches('-');
    if !slug.is_empty() && slug.chars().all(|char| char.is_ascii_digit()) {
        format!("_{slug}")
    } else {
        slug.to_owned()
    }
}

/// The meeting's tags as Obsidian tags, without the ones nothing is left of.
pub(crate) fn tag_slugs(tags: &[String]) -> Vec<String> {
    tags.iter()
        .map(|tag| tag_slug(tag))
        .filter(|slug| !slug.is_empty())
        .collect()
}

/// The tag line under the title in Obsidian mode: `#planning #q4`.
pub(crate) fn tag_line(tags: &[String]) -> String {
    tag_slugs(tags)
        .iter()
        .map(|slug| format!("#{slug}"))
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn obsidian_links_people_by_their_page_name() {
        assert_eq!(person("Maya Chen", LinkStyle::Obsidian), "[[Maya Chen]]");
        assert_eq!(person("A|B [ops]", LinkStyle::Obsidian), "[[A B ops]]");
        assert_eq!(person("Maya_Chen", LinkStyle::Plain), "Maya\\_Chen");
    }

    #[test]
    fn tags_become_valid_obsidian_tags() {
        assert_eq!(tag_slug("Q4 planning"), "Q4-planning");
        assert_eq!(tag_slug("eng/infra"), "eng/infra");
        assert_eq!(tag_slug("2026"), "_2026");
        assert_eq!(tag_slug("café ☕"), "café");
        assert_eq!(
            tag_line(&["Sales".into(), "☕".into(), "Q4 plan".into()]),
            "#Sales #Q4-plan"
        );
    }
}
