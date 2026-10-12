//! The Stats page's numbers over the eight fixture meetings (ORG-09's
//! acceptance), plus rows that must not count: the bundled sample, a
//! trashed meeting and a scheduled one. One legacy meeting with no start
//! time, length or attendees does count.
//!
//! The expected numbers are worked out by hand from `fixtures/meetings/`
//! (UTC, the user `shreyas@deino.dev`, declined invitees left out):
//!
//! | Fixture | Started (UTC) | Seconds | With |
//! |---|---|---|---|
//! | planning | Mon 09-28 17:01 | 4959 | internal |
//! | board-update | Tue 09-29 18:01 | 2217 | external |
//! | sales-discovery | Wed 09-30 15:01 | 1545 | external |
//! | brainstorm | Thu 10-01 18:02 | 1762 | internal |
//! | interview | Fri 10-02 17:01 | 1855 | external |
//! | customer-support | Mon 10-05 16:02 | 993 | external |
//! | one-on-one | Wed 10-07 16:01 | 1138 | internal |
//! | standup | Thu 10-08 15:31 | 434 | internal |

use chrono::NaiveDate;
use deino_domain::PersonId;
use deino_llm::ProviderId;
use deino_testkit::fixture::{USER_EMAIL, fixture_uuid, person_key, seed_fixtures};

use super::testing::World;
use super::*;

/// The fixtures' total recorded seconds.
const FIXTURE_SECONDS: u64 = 4959 + 2217 + 1545 + 1762 + 1855 + 993 + 1138 + 434;

async fn fixture_world() -> World {
    let world = World::new(&[USER_EMAIL]).await;
    seed_fixtures(&world.database).await.unwrap();
    world
        .run(
            "INSERT INTO meetings (id, title, created_at, updated_at, started_at, duration_s) VALUES
                ('meeting-sample', 'Sample', '2026-10-09T10:00:00Z', '2026-10-09T10:00:00Z',
                 '2026-10-09T09:00:00Z', 600),
                ('legacy', 'Legacy call', '2026-08-03T09:15:00Z', '2026-08-03T09:15:00Z', NULL, NULL);
             INSERT INTO meetings (id, title, created_at, updated_at, started_at, duration_s, deleted_at)
             VALUES ('trashed', 'Trashed', '2026-10-06T10:00:00Z', '2026-10-06T10:00:00Z',
                 '2026-10-06T09:00:00Z', 900, '2026-10-07T09:00:00Z');
             INSERT INTO meetings (id, title, created_at, updated_at, started_at, status)
             VALUES ('upcoming', 'Upcoming', '2026-10-10T10:00:00Z', '2026-10-10T10:00:00Z',
                 '2026-10-14T09:00:00Z', 'scheduled');",
        )
        .await;
    seed_llm_calls(&world).await;
    world
}

/// Five calls in the last 30 days and one big call in 2025.
async fn seed_llm_calls(world: &World) {
    world
        .run(
            "INSERT INTO llm_calls (id, started_at, route, provider, model, input_tokens,
                 output_tokens, latency_ms, outcome, error_kind) VALUES
             ('00000000-0000-7000-a000-000000000001', '2026-10-01T10:00:00+00:00', 'notes.enhance',
                 'anthropic', 'claude-sonnet-5-5', 10000, 2000, 900, 'ok', NULL),
             ('00000000-0000-7000-a000-000000000002', '2026-10-02T10:00:00+00:00', 'notes.enhance',
                 'anthropic', 'claude-sonnet-5-5', 20000, 3000, 900, 'ok', NULL),
             ('00000000-0000-7000-a000-000000000003', '2026-10-02T11:00:00+00:00', 'title',
                 'anthropic', 'claude-haiku-5-5', 0, 0, 100, 'error', 'rate_limited'),
             ('00000000-0000-7000-a000-000000000004', '2026-10-03T10:00:00+00:00', 'chat',
                 'openai', 'gpt-5', 1000, 100, 700, 'ok', NULL),
             ('00000000-0000-7000-a000-000000000005', '2026-10-04T10:00:00+00:00', 'tagging',
                 'builtin', 'qwen3.5:4b', 5000, 500, 2000, 'ok', NULL),
             ('00000000-0000-7000-a000-000000000006', '2025-06-01T10:00:00+00:00', 'notes.enhance',
                 'anthropic', 'claude-sonnet-5-5', 1000000, 0, 9000, 'ok', NULL);",
        )
        .await;
}

fn person(email: &str, name: &str, meetings: u32) -> TopPerson {
    TopPerson {
        id: fixture_uuid(&person_key(email))
            .parse::<PersonId>()
            .unwrap(),
        name: name.to_owned(),
        meetings,
    }
}

fn usage(provider: ProviderId, model: &str, counts: [u64; 4], cost: Option<u64>) -> ModelUsage {
    let [calls, failed, input_tokens, output_tokens] = counts;
    ModelUsage {
        provider,
        model: model.to_owned(),
        calls: u32::try_from(calls).unwrap(),
        failed: u32::try_from(failed).unwrap(),
        input_tokens,
        output_tokens,
        estimated_cost_micros: cost,
    }
}

#[tokio::test]
async fn all_time_matches_the_fixtures() {
    let world = fixture_world().await;

    let stats = world.stats().overview(StatsRange::AllTime).await.unwrap();

    assert_eq!(stats.since, None);
    assert_eq!(stats.meetings, 9);
    assert_eq!(stats.recorded_seconds, FIXTURE_SECONDS);
    assert_eq!(stats.recorded_seconds, 14_903);
    assert_eq!(stats.people, 13);
    assert_eq!(
        stats.audience,
        AudienceSplit {
            internal: 4,
            external: 4,
            unknown: 1
        }
    );
    assert_eq!(
        stats.top_people,
        [
            person("priya@deino.dev", "Priya Nair", 4),
            person("tom@deino.dev", "Tom Becker", 4),
            person("maya@deino.dev", "Maya Chen", 3),
            person("aisha@deino.dev", "Aisha Rahman", 2),
            person("diego@deino.dev", "Diego Alvarez", 2),
            person("lena@deino.dev", "Lena Okafor", 2),
            person("noor@deino.dev", "Noor Haddad", 2),
            person("daniel@quarrystone.vc", "Daniel Koh", 1),
        ]
    );
    assert_eq!(stats.weekdays, [3, 1, 2, 2, 1, 0, 0]);
    let mut hours = vec![0; 24];
    for (hour, count) in [(9, 1), (15, 2), (16, 2), (17, 2), (18, 2)] {
        hours[hour] = count;
    }
    assert_eq!(stats.hours, hours);
}

#[tokio::test]
async fn all_time_llm_usage_is_summed_per_model_with_its_estimated_cost() {
    let world = fixture_world().await;

    let llm = world
        .stats()
        .overview(StatsRange::AllTime)
        .await
        .unwrap()
        .llm;

    let sonnet = 1_030_000 * 2 + 5_000 * 10;
    assert_eq!(
        llm.models,
        [
            usage(
                ProviderId::Anthropic,
                "claude-sonnet-5-5",
                [3, 0, 1_030_000, 5_000],
                Some(sonnet)
            ),
            usage(
                ProviderId::Anthropic,
                "claude-haiku-5-5",
                [1, 1, 0, 0],
                Some(0)
            ),
            usage(
                ProviderId::BuiltIn,
                "qwen3.5:4b",
                [1, 0, 5_000, 500],
                Some(0)
            ),
            usage(ProviderId::OpenAi, "gpt-5", [1, 0, 1_000, 100], None),
        ]
    );
    assert_eq!(llm.estimated_cost_micros, 2_110_000);
    assert_eq!(llm.unpriced_calls, 1);
}

#[tokio::test]
async fn the_last_30_days_leave_out_older_meetings_and_calls() {
    let world = fixture_world().await;

    let stats = world
        .stats()
        .overview(StatsRange::Last30Days)
        .await
        .unwrap();

    assert_eq!(stats.since, Some(deino_testkit::at("2026-09-12T00:00:00Z")));
    assert_eq!(stats.meetings, 8);
    assert_eq!(stats.recorded_seconds, FIXTURE_SECONDS);
    assert_eq!(stats.audience.unknown, 0);
    assert_eq!(stats.weekdays, [2, 1, 2, 2, 1, 0, 0]);
    assert_eq!(stats.llm.models[0].calls, 2);
    assert_eq!(stats.llm.estimated_cost_micros, 30_000 * 2 + 5_000 * 10);
}

#[tokio::test]
async fn the_last_90_days_reach_the_legacy_meeting() {
    let world = fixture_world().await;

    let stats = world
        .stats()
        .overview(StatsRange::Last90Days)
        .await
        .unwrap();

    assert_eq!(stats.meetings, 9);
    assert_eq!(stats.llm.models[0].calls, 2);
}

#[tokio::test]
async fn the_heatmap_counts_meetings_per_day_over_the_last_year() {
    let world = fixture_world().await;

    let heatmap = world.stats().heatmap().await.unwrap();

    let day = |month, day| NaiveDate::from_ymd_opt(2026, month, day).unwrap();
    assert_eq!(
        heatmap.first_day,
        NaiveDate::from_ymd_opt(2025, 10, 6).unwrap()
    );
    assert_eq!(heatmap.last_day, day(10, 11));
    let days: Vec<(NaiveDate, u32)> = heatmap
        .days
        .iter()
        .map(|entry| (entry.day, entry.meetings))
        .collect();
    let expected: Vec<(NaiveDate, u32)> = [
        day(8, 3),
        day(9, 28),
        day(9, 29),
        day(9, 30),
        day(10, 1),
        day(10, 2),
        day(10, 5),
        day(10, 7),
        day(10, 8),
    ]
    .into_iter()
    .map(|day| (day, 1))
    .collect();
    assert_eq!(days, expected);
}

#[tokio::test]
async fn without_an_address_in_the_profile_the_audience_is_unknown() {
    let world = World::new(&[]).await;
    seed_fixtures(&world.database).await.unwrap();

    let stats = world.stats().overview(StatsRange::AllTime).await.unwrap();

    assert_eq!(stats.audience.unknown, 8);
    // The user cannot be told apart, so they count among the people met.
    assert_eq!(stats.people, 14);
}

#[tokio::test]
async fn the_users_own_person_never_counts_as_someone_met() {
    let world = World::new(&[]).await;
    world
        .run(
            "INSERT INTO meetings (id, title, created_at, updated_at, started_at)
             VALUES ('solo', 'Solo', '2026-10-01T10:00:00Z', '2026-10-01T10:00:00Z',
                 '2026-10-01T09:00:00Z');
             INSERT INTO people (id, name, is_me, notes_md, created_at, updated_at)
             VALUES ('00000000-0000-7000-b000-000000000001', 'Me', 1, '',
                 '2026-10-01T09:00:00Z', '2026-10-01T09:00:00Z');
             INSERT INTO meeting_attendees (meeting_id, person_id, role)
             VALUES ('solo', '00000000-0000-7000-b000-000000000001', 'organizer');",
        )
        .await;

    let stats = world.stats().overview(StatsRange::AllTime).await.unwrap();

    assert_eq!(stats.meetings, 1);
    assert_eq!(stats.people, 0);
    assert!(stats.top_people.is_empty());
}
