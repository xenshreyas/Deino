//! Raw rows for the ORG-07 repository tests: people, companies, tags,
//! folders, attendance, speakers, transcript lines and action items, each
//! with a readable short name turned into a valid 36-character ID.

use deino_db::Database;

/// A valid UUID-shaped ID ending in `n`.
pub(crate) fn uuid(n: u32) -> String {
    format!("00000000-0000-7000-8000-{n:012}")
}

const STAMP: &str = "2026-10-01T09:00:00Z";

async fn run(database: &Database, sql: &str, binds: &[Option<&str>]) {
    let mut query = sqlx::query(sql);
    for bind in binds {
        query = query.bind(*bind);
    }
    query.execute(database.writer()).await.unwrap();
}

/// A company `n` at `domain`.
pub(crate) async fn company(database: &Database, n: u32, domain: &str) -> String {
    let id = uuid(n);
    let sql = "INSERT INTO companies (id, name, domain, created_at, updated_at) VALUES (?1, ?2, ?2, ?3, ?3)";
    run(database, sql, &[Some(&id), Some(domain), Some(STAMP)]).await;
    id
}

/// A person `n`, with an address and a company if given.
pub(crate) async fn person(
    database: &Database,
    n: u32,
    email: Option<&str>,
    company: Option<&str>,
) -> String {
    let id = uuid(n);
    let name = format!("Person {n}");
    let sql = "INSERT INTO people (id, name, email, company_id, created_at, updated_at)
               VALUES (?1, ?2, ?3, ?4, ?5, ?5)";
    run(
        database,
        sql,
        &[Some(&id), Some(&name), email, company, Some(STAMP)],
    )
    .await;
    id
}

/// Marks person `id` as the user.
pub(crate) async fn make_me(database: &Database, id: &str) {
    run(
        database,
        "UPDATE people SET is_me = 1 WHERE id = ?1",
        &[Some(id)],
    )
    .await;
}

/// `person` attended `meeting` as `role`.
pub(crate) async fn attend(database: &Database, meeting: &str, person: &str, role: &str) {
    let sql = "INSERT INTO meeting_attendees (meeting_id, person_id, role) VALUES (?1, ?2, ?3)";
    run(database, sql, &[Some(meeting), Some(person), Some(role)]).await;
}

/// A tag `n` named `name`.
pub(crate) async fn tag(database: &Database, n: u32, name: &str) -> String {
    let id = uuid(n);
    let sql =
        "INSERT INTO tags (id, name, color, created_at, updated_at) VALUES (?1, ?2, 1, ?3, ?3)";
    run(database, sql, &[Some(&id), Some(name), Some(STAMP)]).await;
    id
}

/// `tag` is on `meeting`.
pub(crate) async fn tag_meeting(database: &Database, meeting: &str, tag: &str) {
    let sql = "INSERT INTO meeting_tags (meeting_id, tag_id, applied_by, applied_at) VALUES (?1, ?2, 'user', ?3)";
    run(database, sql, &[Some(meeting), Some(tag), Some(STAMP)]).await;
}

/// A folder `n`, inside `parent` if given.
pub(crate) async fn folder(database: &Database, n: u32, parent: Option<&str>) -> String {
    let id = uuid(n);
    let name = format!("Folder {n}");
    let sql = "INSERT INTO folders (id, parent_id, name, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?4)";
    run(
        database,
        sql,
        &[Some(&id), parent, Some(&name), Some(STAMP)],
    )
    .await;
    id
}

/// `meeting` is in `folder`.
pub(crate) async fn file(database: &Database, meeting: &str, folder: &str) {
    let sql = "INSERT INTO meeting_folders (meeting_id, folder_id, added_at) VALUES (?1, ?2, ?3)";
    run(database, sql, &[Some(meeting), Some(folder), Some(STAMP)]).await;
}

/// An action item `n` from `meeting`.
pub(crate) async fn action_item(database: &Database, n: u32, meeting: &str) {
    let sql = "INSERT INTO action_items (id, meeting_id, title, created_by, created_at, updated_at)
               VALUES (?1, ?2, 'Follow up', 'ai', ?3, ?3)";
    run(database, sql, &[Some(&uuid(n)), Some(meeting), Some(STAMP)]).await;
}

/// A speaker `n` of `meeting`, labelled `label`, linked to `person`.
pub(crate) async fn speaker(
    database: &Database,
    n: u32,
    meeting: &str,
    (label, person): (&str, Option<&str>),
) -> String {
    let id = uuid(n);
    let sql =
        "INSERT INTO speakers (id, meeting_id, label, person_id, color) VALUES (?1, ?2, ?3, ?4, 1)";
    run(
        database,
        sql,
        &[Some(&id), Some(meeting), Some(label), person],
    )
    .await;
    id
}

/// A transcript line `n` of `meeting` from `start` to `end` ms, said by `speaker`.
pub(crate) async fn segment(
    database: &Database,
    n: u32,
    meeting: &str,
    (start, end, speaker): (u32, u32, Option<&str>),
) {
    let words = format!(r#"[{{"text":"hi","start":{start},"end":{end}}}]"#);
    let (start, end) = (start.to_string(), end.to_string());
    let sql = "INSERT INTO transcript_segments (id, meeting_id, channel, speaker_id, start_ms, end_ms, text, words_json)
               VALUES (?1, ?2, 'mic', ?3, CAST(?4 AS INTEGER), CAST(?5 AS INTEGER), 'hi', ?6)";
    run(
        database,
        sql,
        &[
            Some(&uuid(n)),
            Some(meeting),
            speaker,
            Some(&start),
            Some(&end),
            Some(&words),
        ],
    )
    .await;
}

/// Sets a column of `meeting` to `value` (tests only: the name is a literal).
pub(crate) async fn set(database: &Database, meeting: &str, (column, value): (&str, Option<&str>)) {
    let sql = format!("UPDATE meetings SET {column} = ?1 WHERE id = ?2");
    run(database, &sql, &[value, Some(meeting)]).await;
}
