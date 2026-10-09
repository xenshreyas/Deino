//! [`PaletteColor`]: a slot of the UI's shared speaker and tag palette.

use serde::{Deserialize, Serialize};

/// A colour from the `--speaker-1` … `--speaker-6` palette
/// (`docs/references/ui.md`), used by speakers and tags. Stored as the slot
/// number, which the schema checks is between 1 and 6, so the database never
/// holds a hard-coded colour and both themes stay in charge of the shade.
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    specta::Type,
    sqlx::Type,
)]
#[serde(try_from = "u8", into = "u8")]
#[sqlx(transparent)]
pub struct PaletteColor(u8);

/// A palette slot outside `1..=6`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("palette colours are numbered 1 to {max}, got {0}", max = PaletteColor::SLOTS)]
pub struct InvalidPaletteColor(pub u8);

impl PaletteColor {
    /// How many colours the palette has.
    pub const SLOTS: u8 = 6;

    /// The palette slot `slot` (1-based).
    pub fn new(slot: u8) -> Result<Self, InvalidPaletteColor> {
        if (1..=Self::SLOTS).contains(&slot) {
            Ok(Self(slot))
        } else {
            Err(InvalidPaletteColor(slot))
        }
    }

    /// The colour for the `index`-th item of a list (0-based), cycling
    /// through the palette, so the n-th speaker of a meeting always gets the
    /// same colour.
    pub fn cycle(index: usize) -> Self {
        let offset = index % usize::from(Self::SLOTS);
        // `offset` is below SLOTS, so it fits in a u8.
        Self(u8::try_from(offset).unwrap_or_default() + 1)
    }

    /// The 1-based slot number.
    pub fn slot(self) -> u8 {
        self.0
    }
}

impl TryFrom<u8> for PaletteColor {
    type Error = InvalidPaletteColor;

    fn try_from(slot: u8) -> Result<Self, Self::Error> {
        Self::new(slot)
    }
}

impl From<PaletteColor> for u8 {
    fn from(color: PaletteColor) -> Self {
        color.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_slots_one_to_six_exist() {
        assert_eq!(PaletteColor::new(0), Err(InvalidPaletteColor(0)));
        assert_eq!(PaletteColor::new(1).unwrap().slot(), 1);
        assert_eq!(PaletteColor::new(6).unwrap().slot(), 6);
        assert_eq!(PaletteColor::new(7), Err(InvalidPaletteColor(7)));
    }

    #[test]
    fn cycling_wraps_after_the_last_slot() {
        let slots: Vec<u8> = (0..8).map(|i| PaletteColor::cycle(i).slot()).collect();
        assert_eq!(slots, [1, 2, 3, 4, 5, 6, 1, 2]);
    }

    #[test]
    fn serde_rejects_slots_outside_the_palette() {
        assert_eq!(serde_json::to_string(&PaletteColor::cycle(2)).unwrap(), "3");
        assert!(serde_json::from_str::<PaletteColor>("9").is_err());
    }
}
