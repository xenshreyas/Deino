//! Listing, downloading, cancelling and deleting model files.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use tokio_util::sync::CancellationToken;

use super::download::{self, Control, Download, Limits};
use super::{
    ByteSource, Catalog, CatalogFile, CatalogModel, DownloadProgress, ModelError, ModelEvents,
    ModelFileId, ModelFileView, ModelPurpose, ModelState, ModelStore, Quant,
};

/// A download in progress.
#[derive(Clone)]
struct Active {
    cancel: CancellationToken,
    downloaded: Arc<AtomicU64>,
}

/// What a [`ModelService`] is built from.
pub struct ModelServiceParts {
    /// The models it offers.
    pub catalog: Catalog,
    /// Where they live.
    pub store: ModelStore,
    /// Where their bytes come from.
    pub source: Arc<dyn ByteSource>,
    /// Who hears about downloads.
    pub events: Arc<dyn ModelEvents>,
    /// This Mac's memory, which picks the recommended quant.
    pub memory_bytes: u64,
}

/// Manages the files of the catalog model of one purpose: the transcription
/// model by default, or another with [`ModelService::serving`].
pub struct ModelService {
    catalog: Catalog,
    purpose: ModelPurpose,
    store: ModelStore,
    source: Arc<dyn ByteSource>,
    events: Arc<dyn ModelEvents>,
    recommended: Quant,
    limits: Limits,
    active: Mutex<HashMap<ModelFileId, Active>>,
}

impl ModelService {
    /// A service over its ports.
    pub fn new(parts: ModelServiceParts) -> Self {
        Self {
            recommended: Quant::default_for_memory(parts.memory_bytes),
            catalog: parts.catalog,
            purpose: ModelPurpose::Transcription,
            store: parts.store,
            source: parts.source,
            events: parts.events,
            limits: Limits::default(),
            active: Mutex::new(HashMap::new()),
        }
    }

    /// The same service for the catalog's model of `purpose` (its store
    /// should be that model's own folder).
    #[must_use]
    pub fn serving(mut self, purpose: ModelPurpose) -> Self {
        self.purpose = purpose;
        self
    }

    /// The same service with other transfer limits (tests shorten them).
    #[must_use]
    pub fn with_limits(mut self, limits: Limits) -> Self {
        self.limits = limits;
        self
    }

    fn active(&self) -> std::sync::MutexGuard<'_, HashMap<ModelFileId, Active>> {
        self.active.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Every file of the served model, smallest first, with its state here.
    pub async fn list(&self) -> Result<Vec<ModelFileView>, ModelError> {
        let model = self.catalog.model(self.purpose)?;
        let recommended = self.recommended()?;
        let mut views = Vec::with_capacity(model.files.len());
        for file in &model.files {
            views.push(ModelFileView {
                id: file.id.clone(),
                name: model.name.clone(),
                quant: file.quant,
                size_bytes: file.size_bytes,
                state: self.state(file).await?,
                recommended: file.id == recommended,
            });
        }
        Ok(views)
    }

    async fn state(&self, file: &CatalogFile) -> Result<ModelState, ModelError> {
        if let Some(active) = self.active().get(&file.id) {
            let downloaded_bytes = active.downloaded.load(Ordering::Relaxed);
            return Ok(ModelState::Downloading { downloaded_bytes });
        }
        self.store.state(file).await
    }

    /// The file this Mac should use: the served model at the quant its
    /// memory suits, else at its default quant.
    pub fn recommended(&self) -> Result<ModelFileId, ModelError> {
        let model = self.catalog.model(self.purpose)?;
        let file = model
            .file(self.recommended)
            .or_else(|| model.file(model.default_quant))
            .ok_or_else(|| ModelError::InvalidCatalog("no default file".into()))?;
        Ok(file.id.clone())
    }

    /// The served model's file `id`; another model's file is unknown here.
    fn resolve(&self, id: &ModelFileId) -> Result<(&CatalogModel, &CatalogFile), ModelError> {
        self.catalog
            .find(id)
            .filter(|(model, _)| model.purpose == self.purpose)
            .ok_or_else(|| ModelError::UnknownModel(id.clone()))
    }

    /// The path of a downloaded file; `NotDownloaded` until it is ready.
    pub async fn ready_path(&self, id: &ModelFileId) -> Result<PathBuf, ModelError> {
        let (_, file) = self.resolve(id)?;
        match self.state(file).await? {
            ModelState::Ready => Ok(self.store.path(file)),
            _ => Err(ModelError::NotDownloaded(id.clone())),
        }
    }

    /// Downloads a file (resuming any partial) and verifies it. Returns
    /// when it is ready, failed, or was cancelled.
    pub async fn download(&self, id: &ModelFileId) -> Result<(), ModelError> {
        let (model, file) = self.resolve(id)?;
        if self.store.state(file).await? == ModelState::Ready {
            return Ok(());
        }
        let active = self.begin(id)?;
        let result = self.fetch(model, file, &active).await;
        self.active().remove(id);
        self.events.changed(id);
        match &result {
            Ok(()) => log::info!(target: "models", "downloaded and verified {id}"),
            Err(error) => log::warn!(target: "models", "download of {id} stopped: {error}"),
        }
        result
    }

    /// Registers a download, refusing a second one of the same file.
    fn begin(&self, id: &ModelFileId) -> Result<Active, ModelError> {
        let mut active = self.active();
        if active.contains_key(id) {
            return Err(ModelError::AlreadyDownloading(id.clone()));
        }
        let entry = Active {
            cancel: CancellationToken::new(),
            downloaded: Arc::new(AtomicU64::new(0)),
        };
        active.insert(id.clone(), entry.clone());
        drop(active);
        self.events.changed(id);
        Ok(entry)
    }

    async fn fetch(
        &self,
        model: &CatalogModel,
        file: &CatalogFile,
        active: &Active,
    ) -> Result<(), ModelError> {
        self.store.ensure_root().await?;
        let url = model.url(file);
        let dest = self.store.path(file);
        let job = Download {
            url: &url,
            dest: &dest,
            size: file.size_bytes,
            sha256: file.sha256,
        };
        let events = Arc::clone(&self.events);
        let mut progress = |bytes: u64| {
            active.downloaded.store(bytes, Ordering::Relaxed);
            events.progress(&DownloadProgress {
                id: file.id.clone(),
                downloaded_bytes: bytes,
                total_bytes: file.size_bytes,
            });
        };
        let mut control = Control {
            limits: self.limits,
            cancel: &active.cancel,
            progress: &mut progress,
        };
        download::fetch(self.source.as_ref(), &job, &mut control).await
    }

    /// Cancels a running download, keeping what arrived; false if none ran.
    pub fn cancel(&self, id: &ModelFileId) -> bool {
        self.active()
            .get(id)
            .map(|active| active.cancel.cancel())
            .is_some()
    }

    /// Deletes a file (cancelling its download first).
    pub async fn delete(&self, id: &ModelFileId) -> Result<(), ModelError> {
        let (_, file) = self.resolve(id)?;
        self.cancel(id);
        self.store.delete(file).await?;
        self.events.changed(id);
        Ok(())
    }
}
