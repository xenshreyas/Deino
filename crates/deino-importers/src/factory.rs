//! [`ImportSources`]: each source's API key in the Keychain (ADR 0031, never
//! in settings), its network service, and the adapter for an import. Saving
//! a key switches the provider's service on, as connecting an integration
//! does; forgetting it switches it off again. Offline mode still blocks it.

use std::path::PathBuf;
use std::sync::Arc;

use deino_core::importers::{Importer, ImporterError};
use deino_domain::{ImportSource, ServiceId};
use deino_net::{Net, ServiceSwitch};
use deino_secrets::{SecretName, SecretStatus, SecretStore, SecretValue};

use crate::{CirclebackImporter, FathomImporter, FirefliesImporter, FolderImporter, GranolaImporter};

/// The network service a source's API is reached through; `None` for a folder.
pub const fn service(source: ImportSource) -> Option<ServiceId> {
    match source {
        ImportSource::Granola => Some(ServiceId::Granola),
        ImportSource::Circleback => Some(ServiceId::Circleback),
        ImportSource::Fireflies => Some(ServiceId::Fireflies),
        ImportSource::Fathom => Some(ServiceId::Fathom),
        ImportSource::Folder => None,
    }
}

/// The Keychain item of a source's API key; `None` for a folder.
pub const fn secret(source: ImportSource) -> Option<SecretName> {
    match source {
        ImportSource::Granola => Some(SecretName::GranolaApiKey),
        ImportSource::Circleback => Some(SecretName::CirclebackApiKey),
        ImportSource::Fireflies => Some(SecretName::FirefliesApiKey),
        ImportSource::Fathom => Some(SecretName::FathomApiKey),
        ImportSource::Folder => None,
    }
}

/// The sources' keys and adapters.
pub struct ImportSources {
    net: Net,
    secrets: Arc<dyn SecretStore>,
    switch: Arc<dyn ServiceSwitch>,
}

impl ImportSources {
    /// Sources over the app's network, Keychain and network switches.
    pub fn new(net: Net, secrets: Arc<dyn SecretStore>, switch: Arc<dyn ServiceSwitch>) -> Self {
        Self {
            net,
            secrets,
            switch,
        }
    }

    /// Whether `source` has a key saved; a folder needs none.
    pub async fn key_status(&self, source: ImportSource) -> Result<SecretStatus, ImporterError> {
        match secret(source) {
            Some(name) => Ok(self.secrets.status(name).await?),
            None => Ok(SecretStatus::Stored),
        }
    }

    /// Saves `key` for `source` and switches its service on.
    pub async fn save_key(&self, source: ImportSource, key: &str) -> Result<(), ImporterError> {
        let (Some(name), Some(service)) = (secret(source), service(source)) else {
            return Ok(());
        };
        self.secrets.set(name, SecretValue::parse(key)?).await?;
        self.switch_service(service, true).await
    }

    /// Forgets `source`'s key and switches its service off.
    pub async fn forget_key(&self, source: ImportSource) -> Result<(), ImporterError> {
        let (Some(name), Some(service)) = (secret(source), service(source)) else {
            return Ok(());
        };
        self.secrets.delete(name).await?;
        self.switch_service(service, false).await
    }

    /// The adapter for `source`: a provider with its saved key, or the
    /// folder at `folder`.
    pub async fn importer(
        &self,
        source: ImportSource,
        folder: Option<PathBuf>,
    ) -> Result<Arc<dyn Importer>, ImporterError> {
        match (source, folder) {
            (ImportSource::Folder, Some(path)) => self.folder(path).await,
            (ImportSource::Folder, None) => Err(ImporterError::Folder(std::io::Error::other(
                "no folder was chosen",
            ))),
            (source, _) => self.provider(source).await,
        }
    }

    /// The adapter for `source` with its saved key; a folder source needs
    /// [`ImportSources::folder`].
    pub async fn provider(&self, source: ImportSource) -> Result<Arc<dyn Importer>, ImporterError> {
        let (Some(name), Some(service)) = (secret(source), service(source)) else {
            return Err(ImporterError::Unreadable(source, "a folder needs a path"));
        };
        let key = self
            .secrets
            .get(name)
            .await?
            .ok_or(ImporterError::MissingKey(source))?;
        let client = Arc::new(self.net.scoped(service));
        let key = key.expose();
        Ok(match source {
            ImportSource::Granola => Arc::new(GranolaImporter::new(client, key)),
            ImportSource::Circleback => Arc::new(CirclebackImporter::new(client, key)),
            ImportSource::Fireflies => Arc::new(FirefliesImporter::new(client, key)),
            ImportSource::Fathom | ImportSource::Folder => Arc::new(FathomImporter::new(client, key)),
        })
    }

    /// The adapter for the folder at `path`, which must be a folder.
    pub async fn folder(&self, path: PathBuf) -> Result<Arc<dyn Importer>, ImporterError> {
        let meta = tokio::fs::metadata(&path)
            .await
            .map_err(ImporterError::Folder)?;
        if !meta.is_dir() {
            return Err(ImporterError::Folder(std::io::Error::other("not a folder")));
        }
        Ok(Arc::new(FolderImporter::new(path)))
    }

    async fn switch_service(&self, service: ServiceId, on: bool) -> Result<(), ImporterError> {
        Ok(self.switch.set_enabled(service, on).await?)
    }
}
