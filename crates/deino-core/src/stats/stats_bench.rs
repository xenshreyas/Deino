//! How long the Stats page takes to compute on read (PARITY_PLAN §4.0):
//! 2,000 meetings over the last year, 4 attendees each from 300 people,
//! and 20,000 LLM calls. Run on demand:
//!
//! `cargo test -p deino-core --release stats_bench -- --ignored --nocapture`

#![allow(
    clippy::disallowed_methods,
    reason = "a wall-clock benchmark measures elapsed time; no product code reads the clock here"
)]

use std::time::{Duration, Instant};

use super::testing::World;
use super::*;

const MEETINGS: usize = 2_000;
const PEOPLE: usize = 300;
const PER_MEETING: usize = 4;
const CALLS: usize = 20_000;
/// The budget for the whole page (overview and heatmap), in a debug build.
const BUDGET: Duration = Duration::from_millis(250);

/// Everything, as set-based SQL over a recursive counter.
fn seed_sql() -> String {
    let counter = |count: usize| {
        format!(
            "WITH RECURSIVE n(i) AS (SELECT 0 UNION ALL SELECT i + 1 FROM n WHERE i < {})",
            count - 1
        )
    };
    format!(
        "INSERT INTO people (id, name, email, notes_md, created_at, updated_at)
         {people} SELECT printf('00000000-0000-7000-9000-%012d', i), 'Person ' || i,
             'p' || i || '@c' || (i % 40) || '.com', '', '2025-01-01T00:00:00Z', '2025-01-01T00:00:00Z'
         FROM n;
         INSERT INTO meetings (id, title, created_at, updated_at, started_at, duration_s)
         {meetings} SELECT printf('meeting-%06d', i), 'Meeting ' || i,
             strftime('%Y-%m-%dT%H:%M:%SZ', '2026-10-11T08:00:00Z', '-' || (i % 365) || ' days', '+' || (i % 9) || ' hours'),
             '2026-10-11T00:00:00Z',
             strftime('%Y-%m-%dT%H:%M:%SZ', '2026-10-11T08:00:00Z', '-' || (i % 365) || ' days', '+' || (i % 9) || ' hours'),
             1800 + i % 1800
         FROM n;
         INSERT OR IGNORE INTO meeting_attendees (meeting_id, person_id, role)
         {attendance} SELECT printf('meeting-%06d', i / {PER_MEETING}),
             printf('00000000-0000-7000-9000-%012d', (i * 7919) % {PEOPLE}), 'invitee'
         FROM n;
         INSERT INTO llm_calls (id, started_at, route, provider, model, input_tokens, output_tokens,
             latency_ms, outcome)
         {calls} SELECT printf('00000000-0000-7000-a000-%012d', i),
             strftime('%Y-%m-%dT%H:%M:%S+00:00', '2026-10-11T08:00:00Z', '-' || (i % 365) || ' days'),
             'notes.enhance', 'anthropic',
             CASE i % 2 WHEN 0 THEN 'claude-sonnet-5-5' ELSE 'claude-haiku-5-5' END,
             8000, 900, 1200, 'ok'
         FROM n;",
        people = counter(PEOPLE),
        meetings = counter(MEETINGS),
        attendance = counter(MEETINGS * PER_MEETING),
        calls = counter(CALLS),
    )
}

#[tokio::test]
#[ignore = "a benchmark; run on demand with --release --nocapture"]
async fn stats_bench() {
    let world = World::new(&["me@c0.com"]).await;
    world.run(&seed_sql()).await;
    let stats = world.stats();

    let mut slowest = Duration::ZERO;
    for _ in 0..5 {
        let started = Instant::now();
        let overview = stats.overview(StatsRange::AllTime).await.unwrap();
        let heatmap = stats.heatmap().await.unwrap();
        let elapsed = started.elapsed();
        slowest = slowest.max(elapsed);
        assert_eq!(overview.meetings, 2_000);
        assert_eq!(overview.llm.models.len(), 2);
        assert!(!heatmap.days.is_empty());
    }

    let repo = SqliteStatsRepo::new(world.database.clone());
    let started = Instant::now();
    let meetings = repo.meetings(None).await.unwrap().len();
    let meetings_time = started.elapsed();
    let started = Instant::now();
    let attendance = repo.attendance(None).await.unwrap().len();
    let attendance_time = started.elapsed();
    let started = Instant::now();
    repo.llm_usage(None).await.unwrap();
    let usage_time = started.elapsed();
    println!(
        "{meetings} meetings in {meetings_time:?}, {attendance} attendance rows in \
         {attendance_time:?}, usage in {usage_time:?}"
    );
    println!("stats over {MEETINGS} meetings and {CALLS} LLM calls: slowest of 5 runs {slowest:?}");
    assert!(slowest < BUDGET, "{slowest:?} is over {BUDGET:?}");
}
