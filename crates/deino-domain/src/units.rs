//! Units of time as newtypes, so milliseconds and seconds never mix.
//!
//! Both are `u32` because SQLite stores them as INTEGER and sqlx encodes
//! `u32` losslessly; that covers 49 days in milliseconds, far beyond any
//! meeting.

use std::fmt;

use serde::{Deserialize, Serialize};

/// Declares a `u32` unit newtype with its serde, sqlx and specta impls.
macro_rules! unit {
    ($(#[$meta:meta])* $name:ident, $suffix:literal) => {
        $(#[$meta])*
        #[derive(
            Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord,
            Serialize, Deserialize, specta::Type, sqlx::Type,
        )]
        #[serde(transparent)]
        #[sqlx(transparent)]
        pub struct $name(pub u32);

        impl $name {
            /// The raw count.
            pub const fn get(self) -> u32 {
                self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(formatter, "{}{}", self.0, $suffix)
            }
        }
    };
}

unit!(
    /// A count of milliseconds, for example a segment's offset in a recording.
    Millis,
    "ms"
);

unit!(
    /// A count of whole seconds, for example a meeting's recorded length.
    Seconds,
    "s"
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn units_serialize_as_bare_numbers_and_display_their_unit() {
        assert_eq!(serde_json::to_string(&Millis(1_250)).unwrap(), "1250");
        assert_eq!(Seconds(61).to_string(), "61s");
        assert!(Millis(1) < Millis(2));
    }
}
