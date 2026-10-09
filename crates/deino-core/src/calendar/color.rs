//! [`CalendarColor`]: the colour a calendar provider gives a calendar.

use std::fmt;

use serde::{Deserialize, Serialize};

/// A calendar's colour as `#RRGGBB` in upper case, which the schema checks.
///
/// It is the user's own data (the dot macOS Calendar shows), not a design
/// token, so the UI paints it as given in both themes.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type, sqlx::Type)]
#[serde(try_from = "String", into = "String")]
#[sqlx(transparent)]
pub struct CalendarColor(String);

/// A string that is not a `#RRGGBB` colour.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("{0:?} is not a #RRGGBB colour")]
pub struct InvalidCalendarColor(pub String);

impl CalendarColor {
    /// The colour of 8-bit sRGB channels.
    pub fn from_rgb(red: u8, green: u8, blue: u8) -> Self {
        Self(format!("#{red:02X}{green:02X}{blue:02X}"))
    }

    /// The colour of sRGB channels between 0 and 1, as Apple's colour APIs
    /// report them; values outside the range are clamped.
    pub fn from_unit_rgb(red: f64, green: f64, blue: f64) -> Self {
        Self::from_rgb(unit_to_byte(red), unit_to_byte(green), unit_to_byte(blue))
    }

    /// Parses `#rrggbb` in either case.
    pub fn parse(input: &str) -> Result<Self, InvalidCalendarColor> {
        Self::try_from(input.trim().to_uppercase())
    }

    /// The colour as stored.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// `value` (0–1) as a channel byte, rounded to the nearest.
fn unit_to_byte(value: f64) -> u8 {
    let scaled = (value.clamp(0.0, 1.0) * f64::from(u8::MAX)).round();
    // `scaled` is within 0..=255 after the clamp, so the cast is exact.
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "clamped to 0..=255 above"
    )]
    let byte = scaled as u8;
    byte
}

impl TryFrom<String> for CalendarColor {
    type Error = InvalidCalendarColor;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let hex = value.strip_prefix('#').unwrap_or_default();
        let valid = hex.len() == 6
            && hex
                .chars()
                .all(|c| c.is_ascii_digit() || matches!(c, 'A'..='F'));
        if valid {
            Ok(Self(value))
        } else {
            Err(InvalidCalendarColor(value))
        }
    }
}

impl From<CalendarColor> for String {
    fn from(color: CalendarColor) -> Self {
        color.0
    }
}

impl fmt::Display for CalendarColor {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channels_become_upper_case_hex() {
        assert_eq!(CalendarColor::from_rgb(52, 170, 220).as_str(), "#34AADC");
        assert_eq!(
            CalendarColor::from_unit_rgb(1.0, 0.5, -0.2).as_str(),
            "#FF8000"
        );
    }

    #[test]
    fn parsing_accepts_either_case_and_rejects_anything_else() {
        assert_eq!(CalendarColor::parse("#34aadc").unwrap().as_str(), "#34AADC");
        for invalid in ["", "#34AAD", "34AADC", "#34AADG", "#34AADC0"] {
            assert!(CalendarColor::parse(invalid).is_err(), "{invalid:?}");
        }
        assert!(serde_json::from_str::<CalendarColor>(r##""#34aadc""##).is_err());
    }
}
