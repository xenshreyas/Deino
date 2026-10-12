//! Runs one gallery insight on one fixture meeting through the real engine
//! (`deino_core::insights::GenerateService`) over a seeded database, and
//! builds the ground-truth answer the smoke trials feed `MockProvider`.

use std::sync::Arc;

use async_trait::async_trait;
use deino_core::enhance::MaterialSources;
use deino_core::insights::{
    Attempt, GatewayInsightModel, GenerateDeps, GenerateService, InsightAnswer, InsightDef,
    InsightGenerator, InsightRepo, InsightResultRepo, OwnDomains, SqliteInsightRepo, template,
};
use deino_core::profile::InternalDomains;
use deino_db::test_db;
use deino_domain::{Clock, Email, FixedZone, InsightId, SystemClock};
use deino_llm::Gateway;
use deino_testkit::fixture::{FixtureMeeting, USER_EMAIL, seed_fixtures};
use libtest_mimic::Failed;
use serde_json::json;

use crate::scoring::overlap;

/// The fixtures' zone, Pacific daylight time.
const FIXTURE_HOURS: i32 = -7;

/// The answers the engine stores for the gallery template `key` on `meeting`.
pub fn run_template(
    meeting: &FixtureMeeting,
    key: &str,
    gateway: Gateway,
) -> Result<Vec<InsightAnswer>, Failed> {
    let runtime = tokio::runtime::Runtime::new().map_err(|error| error.to_string())?;
    runtime.block_on(async {
        let database = test_db().await;
        seed_fixtures(&database)
            .await
            .map_err(|error| error.to_string())?;
        let repo = Arc::new(SqliteInsightRepo::new(database.clone()));
        let def = define(&repo, key).await?;
        let now = SystemClock.now();
        let meetings = std::slice::from_ref(&meeting.id);
        repo.request(&def.id, meetings, now)
            .await
            .map_err(|error| error.to_string())?;
        generator(&database, &repo, gateway)?
            .generate(&meeting.id, Attempt::Last)
            .await
            .map_err(|error| Failed::from(format!("the insight failed: {error}")))?;
        let runs = repo
            .for_meeting(&meeting.id)
            .await
            .map_err(|error| error.to_string())?;
        Ok(runs.into_iter().flat_map(|run| run.answers).collect())
    })
}

/// Stores the gallery template `key` as an insight.
async fn define(repo: &SqliteInsightRepo, key: &str) -> Result<InsightDef, Failed> {
    let input = template(key).ok_or("no such template")?.input;
    let now = SystemClock.now();
    let def = InsightDef {
        id: InsightId::generate(&SystemClock),
        name: input.name,
        description: input.description,
        fields: input.fields,
        cardinality: input.cardinality,
        apply_when: input.apply_when,
        revision: 1,
        created_at: now,
        updated_at: now,
    };
    repo.create(&def).await.map_err(|error| error.to_string())?;
    Ok(def)
}

fn generator(
    database: &deino_db::Database,
    repo: &Arc<SqliteInsightRepo>,
    gateway: Gateway,
) -> Result<GenerateService, Failed> {
    let zone = FixedZone::hours(FIXTURE_HOURS).ok_or("an invalid fixture offset")?;
    Ok(GenerateService::new(GenerateDeps {
        defs: repo.clone(),
        results: repo.clone(),
        records: repo.clone(),
        domains: Arc::new(UserDomains),
        material: MaterialSources::sqlite(database),
        model: Arc::new(GatewayInsightModel::new(Arc::new(gateway))),
        clock: Arc::new(SystemClock),
        zone: Arc::new(zone),
    }))
}

/// The fixture user's organisation.
struct UserDomains;

#[async_trait]
impl OwnDomains for UserDomains {
    async fn internal_domains(&self) -> InternalDomains {
        InternalDomains::from_emails(&Email::parse(USER_EMAIL).into_iter().collect::<Vec<_>>())
    }
}

/// The ground truth's decisions as the model would answer them, each citing
/// the transcript line whose words fit it best.
pub fn decisions_answer(meeting: &FixtureMeeting) -> String {
    let answers: Vec<serde_json::Value> = meeting
        .ground_truth
        .decisions
        .iter()
        .map(|decision| json!({ "value": decision, "evidence": format!("s{}", best_line(meeting, decision)) }))
        .collect();
    json!({ "answers": answers }).to_string()
}

/// The 1-based reference of the segment that shares most words with `text`.
fn best_line(meeting: &FixtureMeeting, text: &str) -> usize {
    meeting
        .segments
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| overlap(&a.text, text).total_cmp(&overlap(&b.text, text)))
        .map_or(1, |(index, _)| index + 1)
}
