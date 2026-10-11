//! How long the People and Companies lists take to compute their aggregates
//! on read (ADR 0030): 2,000 people at 200 companies, 5,000 meetings and
//! 20,000 attendance rows. Run on demand:
//!
//! `cargo test -p deino-core --release list_bench -- --ignored --nocapture`

#![allow(
    clippy::disallowed_methods,
    reason = "a wall-clock benchmark measures elapsed time; no product code reads the clock here"
)]

use std::time::Instant;

use super::*;
use crate::entities::testing::World;

const PEOPLE: usize = 2_000;
const MEETINGS: usize = 5_000;
const PER_MEETING: usize = 4;

/// One table filled with set-based SQL: `values` over `i` in `0..count`.
struct Fill {
    table: &'static str,
    columns: &'static str,
    count: usize,
    values: &'static str,
}

const FILLS: [Fill; 3] = [
    Fill {
        table: "companies",
        columns: "id, name, domain, notes_md, created_at, updated_at",
        count: 200,
        values: "printf('00000000-0000-7000-8000-%012d', i), 'Company ' || i, 'c' || i || '.com', '',
            '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z'",
    },
    Fill {
        table: "people",
        columns: "id, name, email, company_id, notes_md, created_at, updated_at",
        count: PEOPLE,
        values: "printf('00000000-0000-7000-9000-%012d', i), 'Person ' || i,
            'p' || i || '@c' || (i % 200) || '.com', printf('00000000-0000-7000-8000-%012d', i % 200),
            '', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z'",
    },
    Fill {
        table: "meetings",
        columns: "id, title, created_at, updated_at",
        count: MEETINGS,
        values: "printf('meeting-%06d', i), 'Meeting ' || i,
            datetime('2026-01-01', '+' || (i % 280) || ' days'),
            datetime('2026-01-01', '+' || (i % 280) || ' days')",
    },
];

/// Runs one fill as a single statement over a recursive counter.
async fn fill(world: &World, fill: &Fill) {
    let sql = format!(
        "INSERT INTO {} ({})
         WITH RECURSIVE n(i) AS (SELECT 0 UNION ALL SELECT i + 1 FROM n WHERE i < {})
         SELECT {} FROM n",
        fill.table,
        fill.columns,
        fill.count - 1,
        fill.values
    );
    world.run(&sql, &[]).await;
}

/// Every table, then four people per meeting.
async fn seed(world: &World) {
    for table in &FILLS {
        fill(world, table).await;
    }
    let attendance = format!(
        "INSERT OR IGNORE INTO meeting_attendees (meeting_id, person_id, role)
         WITH RECURSIVE n(i) AS (SELECT 0 UNION ALL SELECT i + 1 FROM n WHERE i < {})
         SELECT printf('meeting-%06d', i / {PER_MEETING}),
                printf('00000000-0000-7000-9000-%012d', (i * 7919) % {PEOPLE}), 'invitee'
         FROM n",
        MEETINGS * PER_MEETING - 1
    );
    world.run(&attendance, &[]).await;
}

#[tokio::test]
#[ignore = "a benchmark; run on demand with --release --nocapture"]
async fn list_bench() {
    let world = World::new().await;
    seed(&world).await;
    let repo = SqliteEntityRepo::new(world.database.clone());

    let started = Instant::now();
    let people = repo.person_rows(None).await.unwrap();
    let people_ms = started.elapsed().as_millis();
    let started = Instant::now();
    let companies = repo.company_rows().await.unwrap();
    let companies_ms = started.elapsed().as_millis();

    println!(
        "{} people in {people_ms} ms, {} companies in {companies_ms} ms",
        people.len(),
        companies.len()
    );
    assert_eq!(people.len(), PEOPLE);
}
