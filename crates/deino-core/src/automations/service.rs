//! [`AutomationService`]: the Automations page's definitions (list, get,
//! create, update, delete, duplicate) and the step picker's catalogue.
//! Runs and views are in `triggers.rs` and `views.rs`.

use std::sync::Arc;

use async_trait::async_trait;
use deino_db::DbError;
use deino_domain::{AppError, AutomationId, Clock, MeetingId};

use super::{
    Automation, AutomationError, AutomationInput, AutomationRepo, FactsSource, RunRepo, StepInfo,
    StepRegistry, catalog,
};
use crate::meetings::MeetingLifecycleRepo;

/// What a duplicate's name ends with ("Weekly recap (copy)").
const COPY_SUFFIX: &str = " (copy)";
/// The most numbered copies tried before a name repeats (and is refused).
const MAX_COPIES: u32 = 1_000;

/// Runs a meeting's `automations` stage again in the post-meeting pipeline.
#[async_trait]
pub trait AutomationScheduler: Send + Sync {
    /// Schedules the stage for `meeting`.
    async fn schedule(&self, meeting: &MeetingId) -> Result<(), AppError>;
}

/// The ports the service works through.
#[derive(Clone)]
pub struct AutomationPorts {
    /// The automations.
    pub automations: Arc<dyn AutomationRepo>,
    /// Their runs.
    pub runs: Arc<dyn RunRepo>,
    /// What conditions read, and the past meetings.
    pub facts: FactsSource,
    /// Meetings' status: upcoming meetings arm tag-triggered runs.
    pub meetings: Arc<dyn MeetingLifecycleRepo>,
    /// The steps the engine runs, for the catalogue.
    pub steps: StepRegistry,
    /// Runs the stage.
    pub scheduler: Arc<dyn AutomationScheduler>,
    /// IDs and times.
    pub clock: Arc<dyn Clock>,
}

/// Automations: definitions, runs and what the UI shows of them.
pub struct AutomationService {
    pub(super) ports: AutomationPorts,
}

impl AutomationService {
    /// The service over its ports.
    pub fn new(ports: AutomationPorts) -> Self {
        Self { ports }
    }

    /// The step picker: every kind of step, ready or WIP.
    pub fn catalog(&self) -> Vec<StepInfo> {
        catalog(&self.ports.steps)
    }

    /// Every automation, oldest first.
    pub async fn list(&self) -> Result<Vec<Automation>, AutomationError> {
        Ok(self.ports.automations.list().await?)
    }

    /// One automation; `NotFound` when it does not exist.
    pub async fn get(&self, id: &AutomationId) -> Result<Automation, AutomationError> {
        self.ports
            .automations
            .get(id)
            .await?
            .ok_or(AutomationError::NotFound(*id))
    }

    /// Stores a new automation.
    pub async fn create(&self, input: AutomationInput) -> Result<Automation, AutomationError> {
        let input = input.normalized()?;
        let now = self.ports.clock.now();
        let id = AutomationId::generate(self.ports.clock.as_ref());
        let automation = input.into_automation(id, now);
        self.ports
            .automations
            .create(&automation)
            .await
            .map_err(name_taken)?;
        log::info!(target: "automations", "created automation {id}");
        Ok(automation)
    }

    /// Replaces an automation's settings, conditions and steps.
    pub async fn update(
        &self,
        id: &AutomationId,
        input: AutomationInput,
    ) -> Result<Automation, AutomationError> {
        let current = self.get(id).await?;
        let input = input.normalized()?;
        let automation = Automation {
            updated_at: self.ports.clock.now(),
            ..input.into_automation(*id, current.created_at)
        };
        if !self
            .ports
            .automations
            .update(&automation)
            .await
            .map_err(name_taken)?
        {
            return Err(AutomationError::NotFound(*id));
        }
        Ok(automation)
    }

    /// Deletes an automation and its runs.
    pub async fn delete(&self, id: &AutomationId) -> Result<(), AutomationError> {
        if self.ports.automations.delete(id).await? {
            Ok(())
        } else {
            Err(AutomationError::NotFound(*id))
        }
    }

    /// A copy of an automation, named "… (copy)" (then "(copy 2)", …),
    /// switched off so it never runs twice by surprise.
    pub async fn duplicate(&self, id: &AutomationId) -> Result<Automation, AutomationError> {
        let original = self.get(id).await?;
        let names: Vec<String> = self
            .list()
            .await?
            .into_iter()
            .map(|automation| automation.name.to_lowercase())
            .collect();
        let input = AutomationInput {
            name: copy_name(&original.name, &names),
            enabled: false,
            auto_run: original.auto_run,
            stop_on_failure: original.stop_on_failure,
            conditions: original.conditions,
            steps: original.steps,
        };
        self.create(input).await
    }
}

/// The first free "name (copy)" / "name (copy N)", within the name limit.
fn copy_name(name: &str, taken: &[String]) -> String {
    let stem: String = name.chars().take(super::MAX_NAME_CHARS - 12).collect();
    (1_u32..=MAX_COPIES)
        .map(|n| match n {
            1 => format!("{stem}{COPY_SUFFIX}"),
            n => format!("{stem} (copy {n})"),
        })
        .find(|candidate| !taken.contains(&candidate.to_lowercase()))
        .unwrap_or_else(|| format!("{stem}{COPY_SUFFIX}"))
}

/// A unique violation on `automations.name` as a duplicate name.
fn name_taken(error: DbError) -> AutomationError {
    if error.is_unique_violation() {
        AutomationError::DuplicateName
    } else {
        AutomationError::Storage(error)
    }
}

#[cfg(test)]
mod tests {
    use super::copy_name;

    #[test]
    fn copies_take_the_first_free_name() {
        assert_eq!(copy_name("Recap", &[]), "Recap (copy)");
        let taken = ["recap (copy)".to_owned(), "recap (copy 2)".to_owned()];
        assert_eq!(copy_name("Recap", &taken), "Recap (copy 3)");
        assert!(copy_name(&"x".repeat(80), &[]).chars().count() <= 80);
    }
}
