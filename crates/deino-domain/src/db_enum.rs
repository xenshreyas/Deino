//! `db_enum!`: declares an enum stored as a TEXT column and sent over IPC.
//!
//! One declaration gives the enum its serde, sqlx and specta impls (all
//! `snake_case`), an `ALL` list and `as_str()`. The names must match the
//! column's `CHECK (... IN (...))` list in the migration; every enum has a
//! test that its serde, sqlx and `as_str` names agree, and the repositories
//! in `deino-core` round-trip every value through the real schema.

/// Declares a closed set of values stored as TEXT (see the module docs).
macro_rules! db_enum {
    (
        $(#[$meta:meta])*
        $name:ident {
            $( $(#[$variant_meta:meta])* $variant:ident = $text:literal, )+
        }
    ) => {
        $(#[$meta])*
        #[derive(
            Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord,
            ::serde::Serialize, ::serde::Deserialize, ::specta::Type, ::sqlx::Type,
        )]
        #[serde(rename_all = "snake_case")]
        #[sqlx(rename_all = "snake_case")]
        pub enum $name {
            $( $(#[$variant_meta])* $variant, )+
        }

        impl $name {
            /// Every value, in declaration order.
            pub const ALL: &'static [Self] = &[$(Self::$variant),+];

            /// The value as stored in the database and sent over IPC.
            pub const fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $text,)+
                }
            }
        }

        impl ::std::fmt::Display for $name {
            fn fmt(&self, formatter: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                formatter.write_str(self.as_str())
            }
        }
    };
}

/// Asserts that serde writes every value of a `db_enum!` as its `as_str()`.
#[cfg(test)]
pub(crate) fn assert_names_agree<T: serde::Serialize + Copy>(
    all: &[T],
    as_str: fn(T) -> &'static str,
) {
    for value in all {
        let json = serde_json::to_value(value).unwrap();
        assert_eq!(json.as_str(), Some(as_str(*value)));
    }
}
