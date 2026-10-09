//! Versioned seeding: built-ins are added, upgraded and removed by catalogue
//! version, and the user's templates (copies of built-ins included) survive.

use std::sync::Arc;

use deino_db::test_db;
use deino_domain::{FakeClock, SectionFormat, TemplateId};
use deino_testkit::at;

use super::*;

const V1_SECTIONS: &[BuiltinSection] = &[BuiltinSection {
    title: "Overview",
    format: SectionFormat::Bullets,
    instructions: "v1 instructions",
}];
const V2_SECTIONS: &[BuiltinSection] = &[
    BuiltinSection {
        title: "Overview",
        format: SectionFormat::Bullets,
        instructions: "v2 instructions",
    },
    BuiltinSection {
        title: "Risks",
        format: SectionFormat::Bullets,
        instructions: "new in v2",
    },
];

const fn one_on_one(version: u32, sections: &'static [BuiltinSection]) -> BuiltinTemplate {
    BuiltinTemplate {
        id: "one_on_one",
        version,
        name: "1:1",
        description: "A one-on-one.",
        meeting_context: "Two people.",
        sections,
        rules: &[],
    }
}

const RETIRED: BuiltinTemplate = BuiltinTemplate {
    id: "retired",
    ..one_on_one(1, V1_SECTIONS)
};

struct Harness {
    sync: BuiltinSync,
    library: TemplateService,
}

async fn harness() -> Harness {
    let repo = Arc::new(SqliteTemplateRepo::new(test_db().await));
    let clock = Arc::new(FakeClock::new(at("2026-10-09T09:00:00Z")));
    Harness {
        sync: BuiltinSync::new(repo.clone(), clock.clone()),
        library: TemplateService::new(repo, clock),
    }
}

#[tokio::test]
async fn upgrading_a_built_in_keeps_the_users_copy_of_it() {
    let Harness { sync, library } = harness().await;
    let first = sync
        .run(&[one_on_one(1, V1_SECTIONS), RETIRED])
        .await
        .unwrap();
    let copy = library
        .duplicate(&TemplateId::new("one_on_one"))
        .await
        .unwrap();

    let second = sync.run(&[one_on_one(2, V2_SECTIONS)]).await.unwrap();

    assert_eq!(
        first,
        SyncReport {
            added: 2,
            upgraded: 0,
            removed: 0
        }
    );
    assert_eq!(
        second,
        SyncReport {
            added: 0,
            upgraded: 1,
            removed: 1
        }
    );
    let upgraded = library.get(&TemplateId::new("one_on_one")).await.unwrap();
    assert_eq!(upgraded.content.sections.len(), 2);
    assert_eq!(upgraded.content.sections[0].instructions, "v2 instructions");
    let kept = library.get(&copy.id).await.unwrap();
    assert_eq!(kept, copy, "the copy is the user's and never upgraded");
    assert_eq!(kept.content.sections[0].instructions, "v1 instructions");
}

#[tokio::test]
async fn an_up_to_date_or_newer_built_in_is_left_alone() {
    let Harness { sync, library } = harness().await;
    sync.run(&[one_on_one(3, V2_SECTIONS)]).await.unwrap();

    let report = sync.run(&[one_on_one(2, V1_SECTIONS)]).await.unwrap();

    assert_eq!(report, SyncReport::default());
    let stored = library.get(&TemplateId::new("one_on_one")).await.unwrap();
    assert_eq!(stored.content.sections.len(), 2);
}

#[tokio::test]
async fn the_real_catalogue_seeds_once_and_then_does_nothing() {
    let Harness { sync, library } = harness().await;

    let first = sync.run(&CATALOG).await.unwrap();
    let second = sync.run(&CATALOG).await.unwrap();

    assert_eq!(first.added, 30);
    assert_eq!(second, SyncReport::default());
    assert_eq!(library.list().await.unwrap().len(), 30);
}

#[tokio::test]
async fn placeholders_from_the_migration_are_filled_in() {
    let database = test_db().await;
    sqlx::query(
        "INSERT INTO templates (id, name, builtin_version, created_at, updated_at)
         VALUES ('standup', 'standup', 0, '2026-10-09T08:00:00Z', '2026-10-09T08:00:00Z')",
    )
    .execute(database.writer())
    .await
    .unwrap();
    let repo = Arc::new(SqliteTemplateRepo::new(database));
    let clock = Arc::new(FakeClock::new(at("2026-10-09T09:00:00Z")));

    BuiltinSync::new(repo.clone(), clock)
        .run(&CATALOG)
        .await
        .unwrap();

    let standup = repo
        .get(&TemplateId::new("standup"))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(standup.content.name, "Stand-up");
    assert!(!standup.content.sections.is_empty());
}
