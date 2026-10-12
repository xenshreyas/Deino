//! A participant's label as a meeting app writes it, cleaned into a
//! [`DisplayName`] and whether it is the user's own tile.
//!
//! Apps decorate names with roles and markers in parentheses: Zoom's
//! "Alex Chen (Host, me)", Teams' "Alex Chen (Organizer)" and
//! "Sam (You)", Meet's "Bea (Presentation)". Every trailing parenthesised
//! group is dropped; a group that says "me" or "you", or a label that is
//! just "You", marks the user's own tile, which never names a remote voice.

use deino_domain::DisplayName;

/// The words that mark the user's own tile, inside parentheses or alone.
const SELF_MARKERS: &[&str] = &["me", "you"];

/// A cleaned label.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Label {
    /// The name without decorations.
    pub name: DisplayName,
    /// The label is the user's own tile.
    pub is_self: bool,
}

impl Label {
    /// Cleans `raw` (see the module docs); `None` when nothing is left.
    pub fn parse(raw: &str) -> Option<Self> {
        let mut rest = raw.trim();
        let mut is_self = false;
        while let Some((head, group)) = trailing_group(rest) {
            is_self |= marks_self(group);
            rest = head.trim_end();
        }
        is_self |= marks_self(rest);
        let name = DisplayName::parse(rest).ok()?;
        Some(Self { name, is_self })
    }
}

/// `("Alex Chen", "Host, me")` for `"Alex Chen (Host, me)"`.
fn trailing_group(text: &str) -> Option<(&str, &str)> {
    let inner = text.strip_suffix(')')?;
    let open = inner.rfind('(')?;
    let head = inner.get(..open)?;
    let group = inner.get(open + 1..)?;
    Some((head, group))
}

/// Whether a group or label is "me" or "you" (among commas).
fn marks_self(text: &str) -> bool {
    text.split(',').map(str::trim).any(|word| {
        SELF_MARKERS
            .iter()
            .any(|marker| word.eq_ignore_ascii_case(marker))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clean(raw: &str) -> Option<(String, bool)> {
        Label::parse(raw).map(|label| (label.name.to_string(), label.is_self))
    }

    #[test]
    fn role_groups_are_dropped() {
        assert_eq!(clean("Alex Chen (Host)"), Some(("Alex Chen".into(), false)));
        assert_eq!(
            clean(" Bea  Ruiz (Co-host) (she/her) "),
            Some(("Bea Ruiz".into(), false))
        );
        assert_eq!(clean("Cleo"), Some(("Cleo".into(), false)));
    }

    #[test]
    fn me_and_you_mark_the_users_own_tile() {
        assert_eq!(clean("Shreyas (Host, me)"), Some(("Shreyas".into(), true)));
        assert_eq!(clean("Shreyas (You)"), Some(("Shreyas".into(), true)));
        assert_eq!(clean("You"), Some(("You".into(), true)));
        assert_eq!(clean("Yousef"), Some(("Yousef".into(), false)));
    }

    #[test]
    fn empty_labels_are_not_names() {
        assert_eq!(clean("(Host)"), None);
        assert_eq!(clean("   "), None);
    }
}
