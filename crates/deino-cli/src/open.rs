//! [`Opened`]: the database opened read-only, and what reads it.

use std::path::Path;
use std::sync::Arc;

use deino_core::export::{ExportAssembler, ExportPorts};
use deino_core::query::{Library, ReadContext};
use deino_core::search::semantic::Embedder;
use deino_db::{Database, MIGRATOR};
use deino_domain::{SystemClock, SystemZone};

use crate::CliError;
use crate::embedder::HelperEmbedder;

/// The library and the export assembler over one read-only database.
pub struct Opened {
    /// Every read the commands and tools offer.
    pub library: Library,
    /// Meetings assembled for exports.
    pub exports: ExportAssembler,
}

impl Opened {
    /// Opens the database at `path` read-only, after checking its schema.
    /// Search adds meaning to keywords when the app's embedding model and
    /// llama-helper are installed. Times are in the Mac's zone (`TZ`
    /// overrides it).
    pub async fn read_only(path: &Path) -> Result<Self, CliError> {
        let database = Database::open_read_only(path, &MIGRATOR).await?;
        let embedder = HelperEmbedder::locate(path)
            .await
            .map(|embedder| Arc::new(embedder) as Arc<dyn Embedder>);
        Ok(Self::over(&database, embedder))
    }

    /// The readers over an open `database`, searching with `embedder`.
    pub fn over(database: &Database, embedder: Option<Arc<dyn Embedder>>) -> Self {
        let zone = Arc::new(SystemZone);
        let context = ReadContext {
            zone: zone.clone(),
            clock: Arc::new(SystemClock),
            embedder,
        };
        Self {
            library: Library::sqlite(database, &context),
            exports: ExportAssembler::new(ExportPorts::sqlite(database, zone)),
        }
    }
}
