//! The engine and the service over the real schema with the fixture
//! meetings seeded, and scripted steps: what `engine_tests` and
//! `service_tests` build on.

use std::sync::Arc;

use deino_db::{Database, test_db};
use deino_domain::{AutomationId, ConditionGroups, FakeClock, MeetingId, TagId};
use deino_testkit::{at, seed_fixtures};

use super::{
    Action, ActionKind, Automation, AutomationEngine, AutomationPorts, AutomationRepo,
    AutomationService, EngineDeps, FactsSource, RecordedAutomationScheduler, Script, ScriptedStep,
    SqliteAutomationRepo, StepRegistry,
};
use crate::insights::{FixedDomains, SqliteInsightRepo};
use crate::meetings::SqliteMeetingRepo;

/// A seeded fixture meeting (ready, external, with a transcript).
pub(super) const SALES: &str = "meeting-fixture-sales-discovery";
/// Another seeded fixture meeting.
pub(super) const STANDUP: &str = "meeting-fixture-standup";
/// The tag the harness creates.
pub(super) const SALES_TAG: &str = "01920000-0000-7000-8000-0000000000aa";

/// The seeded database, the repositories and the scripted steps.
pub(super) struct Harness {
    pub database: Database,
    pub repo: Arc<SqliteAutomationRepo>,
    pub clock: Arc<FakeClock>,
    pub notify: Arc<ScriptedStep>,
    pub tag: Arc<ScriptedStep>,
    pub scheduler: Arc<RecordedAutomationScheduler>,
}

impl Harness {
    /// Notification steps answer with `notify`, tag steps with `tag`.
    pub async fn new(notify: Vec<Script>, tag: Vec<Script>) -> Self {
        let database = test_db().await;
        seed_fixtures(&database).await.unwrap();
        sqlx::query(
            "INSERT INTO tags (id, name, color, created_at, updated_at) VALUES (?1, 'Sales', 1, ?2, ?2)",
        )
        .bind(SALES_TAG)
        .bind(at("2026-10-12T08:00:00Z"))
        .execute(database.writer())
        .await
        .unwrap();
        Self {
            repo: Arc::new(SqliteAutomationRepo::new(database.clone())),
            clock: Arc::new(FakeClock::new(at("2026-10-12T09:00:00Z"))),
            notify: Arc::new(ScriptedStep::new(ActionKind::Notify, notify)),
            tag: Arc::new(ScriptedStep::new(ActionKind::AddTag, tag)),
            scheduler: Arc::new(RecordedAutomationScheduler::default()),
            database,
        }
    }

    fn facts(&self) -> FactsSource {
        FactsSource::new(
            Arc::new(SqliteInsightRepo::new(self.database.clone())),
            Arc::new(FixedDomains::default()),
        )
    }

    fn steps(&self) -> StepRegistry {
        StepRegistry::new()
            .with(self.notify.clone())
            .with(self.tag.clone())
    }

    /// The engine over the harness.
    pub fn engine(&self) -> AutomationEngine {
        AutomationEngine::new(EngineDeps {
            automations: self.repo.clone(),
            runs: self.repo.clone(),
            facts: self.facts(),
            steps: self.steps(),
            clock: self.clock.clone(),
        })
    }

    /// The service over the harness.
    pub fn service(&self) -> AutomationService {
        AutomationService::new(AutomationPorts {
            automations: self.repo.clone(),
            runs: self.repo.clone(),
            facts: self.facts(),
            meetings: Arc::new(SqliteMeetingRepo::new(self.database.clone())),
            steps: self.steps(),
            scheduler: self.scheduler.clone(),
            clock: self.clock.clone(),
        })
    }

    /// Stores an enabled, automatic automation.
    pub async fn add(
        &self,
        name: &str,
        conditions: ConditionGroups,
        steps: Vec<Action>,
    ) -> Automation {
        let automation = Automation {
            id: AutomationId::generate(self.clock.as_ref()),
            name: name.into(),
            enabled: true,
            auto_run: true,
            stop_on_failure: false,
            conditions,
            steps,
            created_at: at("2026-10-12T08:30:00Z"),
            updated_at: at("2026-10-12T08:30:00Z"),
        };
        self.repo.create(&automation).await.unwrap();
        automation
    }

    /// Replaces a stored automation.
    pub async fn save(&self, automation: &Automation) {
        assert!(self.repo.update(automation).await.unwrap());
    }

    /// Puts the harness's tag on `meeting`.
    pub async fn tag_meeting(&self, meeting: &str) {
        sqlx::query(
            "INSERT INTO meeting_tags (meeting_id, tag_id, applied_by, applied_at) VALUES (?1, ?2, 'user', ?3)",
        )
        .bind(meeting)
        .bind(SALES_TAG)
        .bind(at("2026-10-12T09:00:00Z"))
        .execute(self.database.writer())
        .await
        .unwrap();
    }

    /// Sets a meeting's status column directly.
    pub async fn set_status(&self, meeting: &str, status: &str) {
        sqlx::query("UPDATE meetings SET status = ?1 WHERE id = ?2")
            .bind(status)
            .bind(meeting)
            .execute(self.database.writer())
            .await
            .unwrap();
    }
}

/// The harness's tag.
pub(super) fn sales_tag() -> TagId {
    SALES_TAG.parse().unwrap()
}

/// A meeting ID.
pub(super) fn meeting(id: &str) -> MeetingId {
    MeetingId::new(id)
}

/// A notification step.
pub(super) fn notify() -> Action {
    Action::Notify {
        message: String::new(),
    }
}

/// A tag step.
pub(super) fn add_tag() -> Action {
    Action::AddTag {
        tag_id: sales_tag(),
    }
}
