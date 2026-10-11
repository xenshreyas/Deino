//! [`InsightService`]: what the `insights` plugin calls. Definitions here;
//! the views ([`super::views`]), runs ([`super::runs`]) and the CSV
//! ([`super::csv`]) extend it in their own files.

use std::sync::Arc;

use async_trait::async_trait;
use deino_domain::{AppError, Clock, InsightId, LocalZone, MeetingId};
use serde::Serialize;

use super::{
    InsightDef, InsightError, InsightInput, InsightRepo, InsightResultRepo, InsightTemplate,
    MeetingRecords, OwnDomains, RunProgress, SpeakerNames, gallery,
};
use crate::export::{SaveDialog, Vault};

/// Asks the post-meeting pipeline to run a meeting's `insights` stage.
#[async_trait]
pub trait RunScheduler: Send + Sync {
    /// Schedules the stage for `meeting`; the error is user-safe.
    async fn schedule(&self, meeting: &MeetingId) -> Result<(), AppError>;
}

/// What the CSV export needs from the OS.
#[derive(Clone)]
pub struct InsightFiles {
    /// Where the user saves the file.
    pub save_dialog: Arc<dyn SaveDialog>,
    /// Writes it.
    pub vault: Arc<dyn Vault>,
    /// Meeting dates in the user's zone.
    pub zone: Arc<dyn LocalZone>,
}

/// The ports the service works through.
#[derive(Clone)]
pub struct InsightPorts {
    /// The definitions.
    pub defs: Arc<dyn InsightRepo>,
    /// The runs.
    pub results: Arc<dyn InsightResultRepo>,
    /// Meetings' facts and the past meetings to run on.
    pub records: Arc<dyn MeetingRecords>,
    /// Who counts as external.
    pub domains: Arc<dyn OwnDomains>,
    /// Speakers' current names.
    pub names: Arc<dyn SpeakerNames>,
    /// The pipeline.
    pub scheduler: Arc<dyn RunScheduler>,
    /// The CSV export's dialogs and files.
    pub files: InsightFiles,
    /// Time.
    pub clock: Arc<dyn Clock>,
}

/// An insight with where its runs stand, for the Insights page's list.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct InsightSummary {
    /// The definition.
    pub def: InsightDef,
    /// Its runs by state.
    pub progress: RunProgress,
}

/// Defines insights, shows their answers and runs them.
pub struct InsightService {
    pub(super) ports: InsightPorts,
}

impl InsightService {
    /// The service over its ports.
    pub fn new(ports: InsightPorts) -> Self {
        Self { ports }
    }

    /// The template gallery.
    pub fn templates(&self) -> Vec<InsightTemplate> {
        gallery()
    }

    /// Every insight, oldest first, with its progress.
    pub async fn list(&self) -> Result<Vec<InsightSummary>, InsightError> {
        let mut summaries = Vec::new();
        for def in self.ports.defs.list().await? {
            let progress = self.ports.results.progress(&def.id).await?;
            summaries.push(InsightSummary { def, progress });
        }
        Ok(summaries)
    }

    /// The insight with `id`.
    pub async fn get(&self, id: &InsightId) -> Result<InsightDef, InsightError> {
        self.ports
            .defs
            .get(id)
            .await?
            .ok_or(InsightError::NotFound(*id))
    }

    /// Defines a new insight.
    pub async fn create(&self, input: InsightInput) -> Result<InsightDef, InsightError> {
        let input = input.validated()?;
        self.ensure_unique(&input.name, None).await?;
        let now = self.ports.clock.now();
        let def = InsightDef {
            id: InsightId::generate(self.ports.clock.as_ref()),
            name: input.name,
            description: input.description,
            fields: input.fields,
            cardinality: input.cardinality,
            apply_when: input.apply_when,
            revision: 1,
            created_at: now,
            updated_at: now,
        };
        self.ports.defs.create(&def).await.map_err(duplicate_name)?;
        log::info!(target: "deino_core::insights", "created insight {}", def.id);
        Ok(def)
    }

    /// Replaces an insight's definition. An edit of what it asks bumps its
    /// revision, so earlier answers show as answering an older question.
    pub async fn update(
        &self,
        id: &InsightId,
        input: InsightInput,
    ) -> Result<InsightDef, InsightError> {
        let input = input.validated()?;
        let current = self.get(id).await?;
        self.ensure_unique(&input.name, Some(id)).await?;
        let revision = current.revision + u32::from(current.asks_differently(&input));
        let def = InsightDef {
            name: input.name,
            description: input.description,
            fields: input.fields,
            cardinality: input.cardinality,
            apply_when: input.apply_when,
            revision,
            updated_at: self.ports.clock.now(),
            ..current
        };
        if !self.ports.defs.update(&def).await.map_err(duplicate_name)? {
            return Err(InsightError::NotFound(*id));
        }
        Ok(def)
    }

    /// Refuses `name` when another insight than `except` has it, ignoring
    /// case (the column's `UNIQUE COLLATE NOCASE` backs it up).
    async fn ensure_unique(
        &self,
        name: &str,
        except: Option<&InsightId>,
    ) -> Result<(), InsightError> {
        let taken =
            self.ports.defs.list().await?.iter().any(|def| {
                Some(&def.id) != except && def.name.to_lowercase() == name.to_lowercase()
            });
        if taken {
            Err(InsightError::DuplicateName)
        } else {
            Ok(())
        }
    }

    /// Deletes an insight and every answer it gave.
    pub async fn delete(&self, id: &InsightId) -> Result<(), InsightError> {
        if self.ports.defs.delete(id).await? {
            Ok(())
        } else {
            Err(InsightError::NotFound(*id))
        }
    }
}

/// A unique violation on `insight_defs.name` as [`InsightError::DuplicateName`].
fn duplicate_name(error: deino_db::DbError) -> InsightError {
    if error.is_unique_violation() {
        InsightError::DuplicateName
    } else {
        InsightError::Storage(error)
    }
}

#[cfg(test)]
#[path = "service_tests.rs"]
mod tests;
