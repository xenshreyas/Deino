//! [`Opened`]: the database opened read-only, and what reads it.

use std::path::Path;
use std::sync::Arc;

use deino_core::export::{ExportAssembler, ExportPorts};
use deino_core::query::{Library, ReadContext};
use deino_core::redaction::TextRedactor;
use deino_core::search::semantic::Embedder;
use deino_db::{Database, MIGRATOR};
use deino_domain::{SystemClock, SystemZone};
use deino_secrets::KeychainSecretStore;

use crate::CliError;
use crate::db_key::key_for;
use crate::db_path::APP_IDENTIFIER;
use crate::embedder::HelperEmbedder;
use crate::redaction::redactor_for;

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
    /// overrides it). An encrypted database opens with the app's key from
    /// the Keychain (PRV-02). Secrets are redacted when the app's setting
    /// says so.
    pub async fn read_only(path: &Path) -> Result<Self, CliError> {
        let keychain = KeychainSecretStore::new(APP_IDENTIFIER);
        let key = key_for(path, &keychain).await?;
        let database = Database::open_read_only_keyed(path, key.as_ref(), &MIGRATOR).await?;
        let embedder = HelperEmbedder::locate(path)
            .await
            .map(|embedder| Arc::new(embedder) as Arc<dyn Embedder>);
        Ok(Self::over(&database, embedder, redactor_for(path)))
    }

    /// The readers over an open `database`, searching with `embedder` and
    /// hiding secrets with `redactor`.
    pub fn over(
        database: &Database,
        embedder: Option<Arc<dyn Embedder>>,
        redactor: Arc<dyn TextRedactor>,
    ) -> Self {
        let zone = Arc::new(SystemZone);
        let context = ReadContext {
            zone: zone.clone(),
            clock: Arc::new(SystemClock),
            embedder,
            redactor: redactor.clone(),
        };
        let ports = ExportPorts::sqlite(database, zone).with_redactor(redactor);
        Self {
            library: Library::sqlite(database, &context),
            exports: ExportAssembler::new(ports),
        }
    }
}
