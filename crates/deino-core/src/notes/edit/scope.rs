//! Which text blocks an edit may change: the user's selection, or all.

use std::collections::BTreeSet;

/// The text blocks an edit may change.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Scope {
    /// The selected positions; `None` for the whole notes.
    selected: Option<BTreeSet<usize>>,
}

impl Scope {
    /// The whole notes.
    pub fn whole() -> Self {
        Self::default()
    }

    /// The `selection` among `count` text blocks; positions past the end
    /// are dropped, and an empty selection is the whole notes.
    pub fn of(selection: &[u32], count: usize) -> Self {
        let selected: BTreeSet<usize> = selection
            .iter()
            .filter_map(|&position| usize::try_from(position).ok())
            .filter(|&position| position < count)
            .collect();
        Self {
            selected: (!selected.is_empty()).then_some(selected),
        }
    }

    /// Whether block `position` may change.
    pub fn contains(&self, position: usize) -> bool {
        self.selected
            .as_ref()
            .is_none_or(|selected| selected.contains(&position))
    }

    /// The selection as the prompt names it (`b2 to b4`, or `b2, b5`);
    /// `None` for the whole notes.
    pub fn describe(&self) -> Option<String> {
        let selected = self.selected.as_ref()?;
        let first = *selected.first()?;
        let last = *selected.last()?;
        let contiguous = last - first + 1 == selected.len();
        Some(match (first == last, contiguous) {
            (true, _) => block_id(first),
            (false, true) => format!("{} to {}", block_id(first), block_id(last)),
            (false, false) => selected
                .iter()
                .map(|&position| block_id(position))
                .collect::<Vec<_>>()
                .join(", "),
        })
    }
}

/// The prompt's name for text block `position`: `b1` for the first.
pub fn block_id(position: usize) -> String {
    format!("b{}", position + 1)
}

/// The position `id` (`b3`) names, if it is one.
pub fn parse_block_id(id: &str) -> Option<usize> {
    let number: usize = id
        .trim()
        .trim_matches(['[', ']'])
        .strip_prefix(['b', 'B'])?
        .parse()
        .ok()?;
    number.checked_sub(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_selection_is_described_by_its_blocks() {
        assert_eq!(Scope::of(&[1, 2, 3], 9).describe().unwrap(), "b2 to b4");
        assert_eq!(Scope::of(&[1, 4], 9).describe().unwrap(), "b2, b5");
        assert_eq!(Scope::of(&[0], 9).describe().unwrap(), "b1");
        assert_eq!(Scope::of(&[], 9).describe(), None);
    }

    #[test]
    fn positions_past_the_end_are_dropped() {
        let scope = Scope::of(&[12], 3);

        assert_eq!(scope, Scope::whole());
        assert!(scope.contains(2));
    }

    #[test]
    fn block_ids_round_trip() {
        assert_eq!(parse_block_id(&block_id(4)), Some(4));
        assert_eq!(parse_block_id("b0"), None);
        assert_eq!(parse_block_id("s3"), None);
    }
}
