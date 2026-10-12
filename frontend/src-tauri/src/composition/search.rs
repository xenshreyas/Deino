//! Wires global search (ORG-08) and the semantic index (AI-01): the keyword
//! index, the embedding model's files and embedder, the in-memory vectors,
//! the indexer behind the `index` stage, the backfill, and the services the
//! `search` plugin's commands use.
//!
//! The embedding model lives in `<models>/embedding/` and downloads through
//! the same verified, approved model-download client as Parakeet. Without
//! the LLM gateway or the model, search is keyword-only.

use std::sync::Arc;

use deino_core::models::{Catalog, ModelPurpose, ModelService, ModelServiceParts, ModelStore};
use deino_core::search::semantic::{
    AdminParts, Backfill, Embedder, GatewayEmbedder, IndexerParts, NoEmbedder, SemanticAdmin,
    SemanticIndex, SemanticIndexer, SharedVectors, SqliteEmbeddingStore,
};
use deino_core::search::{
    MeetingIndexer, ReindexQueue, SearchIndexing, SearchService, SqliteSearchIndex,
};
use deino_core::settings::SettingsService;
use deino_db::Database;
use deino_domain::Clock;
use deino_llm::Gateway;
use tauri::{AppHandle, Manager};

use crate::features::search::{TauriEmbeddingModelEvents, TauriIndexEvents};
use crate::paths::AppPaths;
use crate::stt::NetByteSource;

/// The LLM gateway, which embeds with the local embedding model when it is
/// installed, and the search services over it, with the transcript edits'
/// `ReindexQueue`; returns the gateway (`None` when AI is unavailable) and
/// the `index` stage's indexer.
pub(super) fn attach_gateway_and_search(
    app: &AppHandle,
    database: &Database,
    settings: &Arc<SettingsService>,
    clock: &Arc<dyn Clock>,
) -> (Option<Arc<Gateway>>, Arc<dyn SearchIndexing>) {
    let models = embedding_models(app);
    let gateway =
        super::llm::attach_llm(app, database, settings, (Arc::clone(clock), models.clone()));
    let search = attach_search(app, database, (gateway.clone(), models), clock);
    // Transcript edits (TRN-02) re-index their meetings through this queue.
    app.manage(Arc::new(ReindexQueue::start(Arc::clone(&search))));
    (gateway, search)
}

/// The embedding model's folder under the models dir.
const MODELS_FOLDER: &str = "embedding";

/// The embedding model's file service, or `None` (logged) when the app's
/// folders or the network client are not available.
fn embedding_models(app: &AppHandle) -> Option<Arc<ModelService>> {
    let built = (|| -> Result<ModelService, String> {
        let paths = AppPaths::resolve(app).map_err(|error| error.to_string())?;
        let source = NetByteSource::new(super::network::model_downloads(app)?);
        let catalog = Catalog::bundled().map_err(|error| error.to_string())?;
        let service = ModelService::new(ModelServiceParts {
            catalog,
            store: ModelStore::new(paths.models().join(MODELS_FOLDER)),
            source: Arc::new(source),
            events: Arc::new(TauriEmbeddingModelEvents::new(app.clone())),
            memory_bytes: 0,
        });
        Ok(service.serving(ModelPurpose::Embedding))
    })();
    built
        .inspect_err(|error| log::error!(target: "search", "no embedding model: {error}"))
        .ok()
        .map(Arc::new)
}

/// Builds search over `database`, manages `Arc<SearchService>`,
/// `Arc<SemanticAdmin>` (when the model's files are wired) and
/// `Arc<Backfill>`, and returns the `index` stage's indexer.
fn attach_search(
    app: &AppHandle,
    database: &Database,
    (gateway, models): (Option<Arc<Gateway>>, Option<Arc<ModelService>>),
    clock: &Arc<dyn Clock>,
) -> Arc<dyn SearchIndexing> {
    let keyword = Arc::new(SqliteSearchIndex::new(database));
    let store = Arc::new(SqliteEmbeddingStore::new(database));
    let embedder: Arc<dyn Embedder> = match (gateway, &models) {
        (Some(gateway), Some(models)) => {
            Arc::new(GatewayEmbedder::new(gateway, Arc::clone(models)))
        }
        _ => Arc::new(NoEmbedder),
    };
    let vectors = Arc::new(SharedVectors::new(store.clone(), embedder.model()));
    let indexer = Arc::new(SemanticIndexer::new(IndexerParts {
        sources: store.clone(),
        store: store.clone(),
        embedder: Arc::clone(&embedder),
        vectors: Arc::clone(&vectors),
        clock: Arc::clone(clock),
    }));
    let events = Arc::new(TauriIndexEvents::new(app.clone()));
    let backfill = Arc::new(Backfill::new(Arc::clone(&indexer), store.clone(), events));
    let semantic = Arc::new(SemanticIndex::new(
        store.clone(),
        Arc::clone(&embedder),
        vectors,
    ));
    if let Some(models) = models {
        app.manage(Arc::new(SemanticAdmin::new(AdminParts {
            models,
            store,
            backfill: Arc::clone(&backfill),
            model: embedder.model().to_owned(),
        })));
    }
    app.manage(Arc::new(SearchService::new(
        keyword.clone(),
        keyword.clone(),
        semantic,
    )));
    app.manage(backfill);
    Arc::new(MeetingIndexer::new(keyword, indexer))
}
