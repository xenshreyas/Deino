//! The tables whose changes the UI can subscribe to.
//!
//! [`Table`] is generated: `build.rs` replays every migration's `CREATE
//! TABLE`, `DROP TABLE` and `RENAME TO` and hands the tables that remain to
//! the `tables!` macro below, so a card that creates a table registers it
//! with its migration alone. A new table's doc is the `--` comment block
//! directly above its `CREATE TABLE`.

use serde::{Deserialize, Serialize};

/// Declares [`Table`] with [`Table::ALL`] and its by-name index from the list
/// `build.rs` generates: the variants in order with their SQL names, then the
/// same pairs sorted by name for [`Table::from_name`].
macro_rules! tables {
    (
        count: $count:literal;
        tables: [$($(#[doc = $doc:literal])* $variant:ident = $name:literal,)*]
        by_name: [$($sorted_name:literal => $sorted_variant:ident,)*]
    ) => {
        /// A table in Deino's database, as named in SQL.
        ///
        /// The UI tags each query with the tables it reads, and a `DbChanged` event
        /// invalidates the queries whose tables changed. The variants are generated
        /// from the migrations' `CREATE TABLE` statements (`deino-db/build.rs`).
        #[derive(
            Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
            specta::Type,
        )]
        #[serde(rename_all = "snake_case")]
        pub enum Table {
            $($(#[doc = $doc])* $variant,)*
        }

        impl Table {
            /// Every table, for consumers that must assume everything changed.
            pub const ALL: [Self; $count] = [$(Self::$variant,)*];

            /// Every table by its SQL name, sorted by name for [`Table::from_name`].
            const BY_NAME: [(&'static str, Self); $count] =
                [$(($sorted_name, Self::$sorted_variant),)*];
        }
    };
}

include!(concat!(env!("OUT_DIR"), "/tables.rs"));

impl Table {
    /// The table with this SQL name, if the UI tracks it.
    ///
    /// Runs inside SQLite's update hook for every changed row, so it neither
    /// allocates nor logs.
    pub fn from_name(name: &str) -> Option<Self> {
        let index = Self::BY_NAME
            .binary_search_by(|(table, _)| (*table).cmp(name))
            .ok()?;
        Self::BY_NAME.get(index).map(|(_, table)| *table)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_name_matches_the_serialized_name_of_every_table() {
        for table in Table::ALL {
            let json = serde_json::to_value(table).unwrap();
            let name = json.as_str().unwrap();
            assert_eq!(Table::from_name(name), Some(table), "{name}");
        }
    }

    #[test]
    fn internal_tables_and_views_are_not_tracked() {
        assert_eq!(Table::from_name("_sqlx_migrations"), None);
        // A view never reaches the update hook; writes through it report the
        // table underneath (`transcript_segments`).
        assert_eq!(Table::from_name("transcripts"), None);
        // FTS5 keeps its index in shadow tables; `search_documents` reports it.
        assert_eq!(Table::from_name("search_fts_data"), None);
        assert_eq!(Table::from_name("search_fts"), None);
        // Tables a later migration dropped or renamed away.
        assert_eq!(Table::from_name("licensing"), None);
        assert_eq!(Table::from_name("egress_log_next"), None);
    }
}
