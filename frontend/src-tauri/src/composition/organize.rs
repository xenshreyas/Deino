//! Wires folders (ORG-05) and tags (ORG-06): their services over the SQLite
//! repositories, the `MeetingTagged` event adapter, and the auto-tagger the
//! `auto_tag` pipeline stage calls.

use std::sync::Arc;

use deino_core::folders::{FilingService, FolderPorts, FolderService, SqliteFolderRepo};
use deino_core::meetings::MeetingBulkService;
use deino_core::settings::SettingsService;
use deino_core::tags::{
    AutoTagPorts, AutoTagService, AutoTagger, NoModelAutoTagger, SqliteTagRepo, TagEvents,
    TagPorts, TagService,
};
use deino_db::Database;
use deino_domain::Clock;
use deino_llm::Gateway;
use tauri::{Manager, Runtime};

/// The folder and tag services, built once and managed as Tauri state.
pub(crate) struct Organize {
    folders: Arc<FolderService>,
    filing: Arc<FilingService>,
    tags: Arc<TagService>,
    tag_repo: Arc<SqliteTagRepo>,
}

impl Organize {
    /// Builds the services over `database`; tags announce themselves
    /// through `tag_events`.
    pub(crate) fn build(
        database: &Database,
        clock: &Arc<dyn Clock>,
        tag_events: Arc<dyn TagEvents>,
    ) -> Self {
        let folder_ports = FolderPorts::over(
            Arc::new(SqliteFolderRepo::new(database.clone())),
            Arc::clone(clock),
        );
        let tag_repo = Arc::new(SqliteTagRepo::new(database.clone()));
        let tags = Arc::new(TagService::new(TagPorts {
            tags: tag_repo.clone(),
            applied: tag_repo.clone(),
            events: tag_events,
            clock: Arc::clone(clock),
        }));
        Self {
            folders: Arc::new(FolderService::new(&folder_ports)),
            filing: Arc::new(FilingService::new(&folder_ports)),
            tags,
            tag_repo,
        }
    }

    /// The auto-tagger over the gateway's `tagging` route, switched by
    /// `Settings::organize.auto_tag`; without a gateway, one that skips.
    pub(crate) fn auto_tagger(
        &self,
        gateway: Option<Arc<Gateway>>,
        settings: Arc<SettingsService>,
    ) -> Arc<dyn AutoTagger> {
        let Some(gateway) = gateway else {
            return Arc::new(NoModelAutoTagger);
        };
        Arc::new(AutoTagService::new(AutoTagPorts {
            service: Arc::clone(&self.tags),
            tags: self.tag_repo.clone(),
            applied: self.tag_repo.clone(),
            digests: self.tag_repo.clone(),
            switch: settings,
            gateway,
        }))
    }

    /// The Meetings page's bulk actions over these tag and folder services
    /// (ORG-07), so bulk tagging follows the same rules as tagging one.
    pub(crate) fn bulk(&self) -> MeetingBulkService {
        MeetingBulkService::new(Arc::clone(&self.tags), Arc::clone(&self.filing))
    }

    /// The tag service, for automations' "add tag" step (AUT-01).
    pub(crate) fn tags(&self) -> Arc<TagService> {
        Arc::clone(&self.tags)
    }

    /// The filing service, for automations' "add to folder" step (AUT-01).
    pub(crate) fn filing(&self) -> Arc<FilingService> {
        Arc::clone(&self.filing)
    }

    /// Registers each service as Tauri state, where commands find it.
    pub(crate) fn manage<R: Runtime>(&self, app: &impl Manager<R>) {
        app.manage(Arc::clone(&self.folders));
        app.manage(Arc::clone(&self.filing));
        app.manage(Arc::clone(&self.tags));
    }
}

#[cfg(test)]
mod tests {
    use deino_core::tags::{NewTag, RecordedTagEvents};
    use deino_testkit::{FakeClock, at, test_db};

    use super::*;

    #[tokio::test]
    async fn folders_and_tags_share_the_database_they_are_built_over() {
        let database = test_db().await;
        let clock: Arc<dyn Clock> = Arc::new(FakeClock::new(at("2026-10-09T10:00:00Z")));

        let organize = Organize::build(&database, &clock, Arc::new(RecordedTagEvents::default()));
        let new = NewTag {
            name: "Customer".into(),
            color: None,
            description: String::new(),
        };
        organize.tags.create(&new).await.unwrap();

        assert_eq!(organize.tags.list().await.unwrap().len(), 1);
        assert!(organize.folders.tree().await.unwrap().entries.is_empty());
    }
}
