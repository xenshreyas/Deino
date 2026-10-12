//! Tests of the migration scan that generates [`crate::Table`]
//! (`build/schema.rs`, compiled here as well as into `build.rs`).

#[allow(
    dead_code,
    reason = "build.rs uses the file-reading half; these tests the rest"
)]
#[path = "../build/schema.rs"]
mod schema;

use schema::replay;

fn names(migrations: &[&str]) -> Vec<String> {
    replay(migrations.iter().copied())
        .into_iter()
        .map(|table| table.name)
        .collect()
}

#[test]
fn created_tables_are_listed_in_order_of_appearance() {
    let sql = "CREATE TABLE b (id TEXT);\nCREATE TABLE IF NOT EXISTS a (id TEXT) STRICT;";

    assert_eq!(
        names(&[sql, "CREATE TABLE \"c\" (id TEXT);"]),
        ["b", "a", "c"]
    );
}

#[test]
fn dropped_temporary_and_virtual_tables_are_left_out() {
    let sql = "CREATE TABLE gone (id TEXT);
        CREATE TEMP TABLE scratch (id TEXT);
        CREATE VIRTUAL TABLE search_fts USING fts5 (body);
        CREATE VIEW recent AS SELECT 1;
        CREATE TABLE kept (id TEXT);
        DROP TABLE IF EXISTS gone;
        DROP TABLE scratch;";

    assert_eq!(names(&[sql]), ["kept"]);
}

#[test]
fn a_rebuilt_table_keeps_its_place_and_its_doc() {
    let first = "-- The log.\nCREATE TABLE log (id TEXT);\nCREATE TABLE other (id TEXT);";
    let rebuild = "CREATE TABLE log_next (id TEXT, size INTEGER);
        DROP TABLE log;
        ALTER TABLE log_next RENAME TO log;";

    let tables = replay([first, rebuild]);

    let names: Vec<&str> = tables.iter().map(|t| t.name.as_str()).collect();
    assert_eq!(names, ["log", "other"]);
    assert_eq!(tables[0].doc, ["The log."]);
}

#[test]
fn a_table_is_documented_by_the_comment_block_directly_above_it() {
    let sql = "-- About the migration, not a table.

        -- Pre-meeting briefs, one per
        -- upcoming event.
        CREATE TABLE briefs (
            -- a column comment
            id TEXT -- trailing; with a semicolon
        );
        CREATE TABLE bare (note TEXT DEFAULT '-- not a comment; nor an end');";

    let tables = replay([sql]);

    assert_eq!(
        tables[0].doc,
        ["Pre-meeting briefs, one per", "upcoming event."]
    );
    assert!(tables[1].doc.is_empty());
    assert_eq!(tables.len(), 2);
}

#[test]
fn trigger_bodies_change_no_table() {
    let sql = "CREATE TABLE t (id TEXT);
        CREATE TRIGGER t_ai AFTER INSERT ON t BEGIN
            INSERT INTO t (id) VALUES ('x');
            DELETE FROM t WHERE id = 'y';
        END;";

    assert_eq!(names(&[sql]), ["t"]);
}
