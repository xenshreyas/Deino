//! [`BuiltinSync`]: writes the built-in catalogue into the database.
//!
//! Versioned seeding: a built-in is written when it is missing or its
//! catalogue `version` is newer than the stored one, and removed when the
//! catalogue dropped it. The user's own templates (duplicates of built-ins
//! included) have no catalogue version, so a sync never touches them. Runs at
//! every start; with nothing to do it only reads.

use std::collections::HashMap;
use std::sync::Arc;

use deino_domain::{Clock, TemplateId};
use serde::Serialize;

use super::{BuiltinStore, BuiltinTemplate, Template, TemplateError};

/// What a sync changed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct SyncReport {
    /// Built-ins written for the first time.
    pub added: usize,
    /// Built-ins rewritten from a newer catalogue version.
    pub upgraded: usize,
    /// Built-ins the catalogue no longer has, deleted.
    pub removed: usize,
}

/// Keeps the stored built-ins in step with a catalogue.
pub struct BuiltinSync {
    store: Arc<dyn BuiltinStore>,
    clock: Arc<dyn Clock>,
}

impl BuiltinSync {
    /// Creates the sync over its store.
    pub fn new(store: Arc<dyn BuiltinStore>, clock: Arc<dyn Clock>) -> Self {
        Self { store, clock }
    }

    /// Brings the stored built-ins up to `catalog` (see the module docs).
    pub async fn run(&self, catalog: &[BuiltinTemplate]) -> Result<SyncReport, TemplateError> {
        let stored: HashMap<TemplateId, u32> =
            self.store.builtin_versions().await?.into_iter().collect();
        let mut report = SyncReport::default();
        for builtin in catalog {
            match stored.get(&builtin.template_id()) {
                Some(version) if *version >= builtin.version => {}
                Some(_) => {
                    self.save(builtin).await?;
                    report.upgraded += 1;
                }
                None => {
                    self.save(builtin).await?;
                    report.added += 1;
                }
            }
        }
        for id in stored.keys() {
            if !catalog.iter().any(|builtin| builtin.id == id.as_str()) {
                report.removed += usize::from(self.store.remove_builtin(id).await?);
            }
        }
        log::info!(
            target: "templates",
            "built-in templates: {} added, {} upgraded, {} removed",
            report.added, report.upgraded, report.removed
        );
        Ok(report)
    }

    async fn save(&self, builtin: &BuiltinTemplate) -> Result<(), TemplateError> {
        let now = self.clock.now();
        let template = Template {
            id: builtin.template_id(),
            is_builtin: true,
            content: builtin.content(),
            created_at: now,
            updated_at: now,
        };
        Ok(self.store.save_builtin(&template, builtin.version).await?)
    }
}
