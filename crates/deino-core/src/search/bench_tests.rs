//! ORG-08's acceptance: search over 2,000 meetings answers in under 100 ms.
//!
//! Seeds 2,000 synthetic meetings (50 transcript segments and a note each,
//! 100,000 segments in all) through the real triggers, then times whole
//! searches through `SearchService` (parse, filters, FTS5, snippets). It
//! takes about a minute in a debug build, so it is ignored by default:
//!
//! ```sh
//! cargo test -p deino-core --release --lib search::bench_tests -- --ignored --nocapture
//! ```

#![allow(
    clippy::disallowed_methods,
    reason = "a benchmark measures wall-clock time"
)]

use std::sync::Arc;
use std::time::{Duration, Instant};

use deino_db::Database;
use deino_testkit::test_db;

use super::semantic::NoSemanticSearch;
use super::{SearchFilters, SearchRequest, SearchService, SqliteSearchIndex};

const MEETINGS: usize = 2_000;
const SEGMENTS_PER_MEETING: usize = 50;
/// The acceptance bound for one search.
const BUDGET: Duration = Duration::from_millis(100);
/// Words the synthetic transcript is made of; a few are rare on purpose.
const VOCABULARY: [&str; 24] = [
    "roadmap",
    "customer",
    "calendar",
    "sync",
    "release",
    "budget",
    "hiring",
    "webhook",
    "dispatch",
    "technician",
    "invoice",
    "renewal",
    "beta",
    "pricing",
    "onboarding",
    "support",
    "metrics",
    "churn",
    "pipeline",
    "security",
    "audit",
    "offline",
    "mobile",
    "quarter",
];

/// Text of `words` words picked deterministically from `seed`.
fn sentence(seed: usize, words: usize) -> String {
    (0..words)
        .map(|index| VOCABULARY[(seed * 7 + index * 13 + seed / 3) % VOCABULARY.len()])
        .collect::<Vec<_>>()
        .join(" ")
}

async fn seed(database: &Database) {
    let mut transaction = database.writer().begin().await.unwrap();
    for meeting in 0..MEETINGS {
        let id = format!("meeting-{meeting:05}");
        sqlx::query(
            "INSERT INTO meetings (id, title, created_at, updated_at) VALUES (?1, ?2, ?3, ?3)",
        )
        .bind(&id)
        .bind(format!("{} review {meeting}", sentence(meeting, 2)))
        .bind(format!("2026-01-01T00:00:{:02}Z", meeting % 60))
        .execute(&mut *transaction)
        .await
        .unwrap();
        for segment in 0..SEGMENTS_PER_MEETING {
            let start = i64::try_from(segment * 4_000).unwrap();
            sqlx::query(
                "INSERT INTO transcript_segments (id, meeting_id, channel, start_ms, end_ms, text)
                 VALUES (?1, ?2, 'mixed', ?3, ?4, ?5)",
            )
            .bind(uuid::Uuid::new_v4().to_string())
            .bind(&id)
            .bind(start)
            .bind(start + 3_500)
            .bind(sentence(meeting * 31 + segment, 18))
            .execute(&mut *transaction)
            .await
            .unwrap();
        }
    }
    transaction.commit().await.unwrap();
}

/// The slowest of three runs of `text`.
async fn slowest(service: &SearchService, text: &str) -> Duration {
    let request = SearchRequest {
        text: text.to_owned(),
        filters: SearchFilters::default(),
        limit: 8,
    };
    let mut slowest = Duration::ZERO;
    for _ in 0..3 {
        let started = Instant::now();
        let results = service.search(&request).await.unwrap();
        slowest = slowest.max(started.elapsed());
        assert!(
            !results.meetings.is_empty() || text.contains("zzz"),
            "{text}"
        );
    }
    slowest
}

#[tokio::test]
#[ignore = "seeds 100,000 segments; run by hand (see the module docs)"]
async fn search_over_two_thousand_meetings_answers_within_100_ms() {
    let database = test_db().await;
    let seeding = Instant::now();
    seed(&database).await;
    println!("seeded {MEETINGS} meetings in {:?}", seeding.elapsed());
    let index = Arc::new(SqliteSearchIndex::new(database));
    let service = SearchService::new(index.clone(), index, Arc::new(NoSemanticSearch));

    for text in [
        "roadmap",
        "calendar sync",
        "\"webhook audit\"",
        "dispat",
        "budget hiring churn",
        "zzz",
    ] {
        let took = slowest(&service, text).await;
        println!("{text:>24}: {took:?}");
        assert!(took < BUDGET, "{text} took {took:?}");
    }
}
