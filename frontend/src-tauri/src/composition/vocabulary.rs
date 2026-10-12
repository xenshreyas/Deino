//! Wiring for the custom vocabulary (TRN-05): `VocabularyService` behind the
//! `vocabulary` plugin and the recorder's live pass, and `FixupService`
//! behind the `vocab_fixups` stage, over the SQLite repositories.

use std::sync::Arc;

use deino_core::calendar::SqliteCalendarRepo;
use deino_core::companies::SqliteCompanyRepo;
use deino_core::meetings::SqliteMeetingRepo;
use deino_core::people::SqlitePeopleRepo;
use deino_core::segments::SqliteSegmentRepo;
use deino_core::vocabulary::{
    FixupPorts, FixupService, NameSources, SqliteCorrectionRepo, SqliteVocabRepo, VocabularyPorts,
    VocabularyService,
};
use deino_db::Database;
use deino_domain::Clock;
use tauri::{Manager, Runtime};

use super::attach::Phase;

super::attach::attach!(
    "vocabulary",
    20,
    Phase::Local(|cx| {
        Vocabulary::build(cx.database, cx.clock).manage(cx.app);
    })
);

/// The vocabulary services.
pub(crate) struct Vocabulary {
    service: Arc<VocabularyService>,
    fixups: Arc<FixupService>,
}

impl Vocabulary {
    /// Builds both services over `database`.
    pub(crate) fn build(database: &Database, clock: &Arc<dyn Clock>) -> Self {
        let corrections = Arc::new(SqliteCorrectionRepo::new(database.clone()));
        let service = Arc::new(VocabularyService::new(VocabularyPorts {
            terms: Arc::new(SqliteVocabRepo::new(database.clone())),
            corrections: corrections.clone(),
            names: names(database),
            clock: Arc::clone(clock),
        }));
        let fixups = Arc::new(FixupService::new(FixupPorts {
            vocabulary: Arc::clone(&service),
            segments: Arc::new(SqliteSegmentRepo::new(database.clone())),
            corrections,
            clock: Arc::clone(clock),
        }));
        Self { service, fixups }
    }

    /// Registers both as Tauri state: the plugin and the recorder find the
    /// service there, the pipeline the fixups.
    pub(crate) fn manage<R: Runtime>(self, app: &impl Manager<R>) {
        app.manage(self.service);
        app.manage(self.fixups);
    }
}

/// People, companies and invitees over SQLite.
fn names(database: &Database) -> NameSources {
    NameSources {
        people: Arc::new(SqlitePeopleRepo::new(database.clone())),
        companies: Arc::new(SqliteCompanyRepo::new(database.clone())),
        meetings: Arc::new(SqliteMeetingRepo::new(database.clone())),
        calendar: Arc::new(SqliteCalendarRepo::new(database.clone())),
    }
}
