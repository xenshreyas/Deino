//! [`HelperEmbedder`]: the local embedding model (`nomic-embed-text-v1.5`,
//! downloaded by the app) through the llama-helper sidecar, so `search`
//! finds meaning as well as words, as the app's own search does.
//!
//! It exists only when both files are there: the model in the app's
//! `models/embedding/` folder beside the database, and `llama-helper` beside
//! this binary (`Deino.app/Contents/MacOS/`; `DEINO_LLAMA_HELPER` overrides
//! it). The helper starts on the first search, loads the model once
//! ([`ready`](Embedder::ready) waits for that, outside the search's 300 ms
//! query deadline), and is stopped when the embedder drops; a helper whose
//! parent dies exits at the end of its stdin. Without it, search is
//! keyword-only and says so.

mod worker;

use std::path::{Path, PathBuf};
use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::JoinHandle;
use std::time::Duration;

use async_trait::async_trait;
use deino_core::models::{Catalog, ModelPurpose, ModelState, ModelStore};
use deino_core::search::SearchError;
use deino_core::search::semantic::{
    DOCUMENT_PREFIX, EMBED_BATCH, EmbedRole, Embedder, QUERY_PREFIX,
};
use deino_llm::{EmbedReply, HelperRequest, LOCAL_EMBEDDING_MODEL};
use tokio::sync::{OnceCell, oneshot};

use self::worker::{Job, Slot};

/// Overrides where `llama-helper` is.
pub const HELPER_ENV: &str = "DEINO_LLAMA_HELPER";
/// The sidecar's file name beside this binary.
const HELPER_NAME: &str = "llama-helper";
/// The embedding model's folder under the app's `models/` (the app's
/// `composition/search.rs` uses the same).
const MODELS_FOLDER: &str = "embedding";
/// How long starting the helper and loading the model may take.
const WARM_UP: Duration = Duration::from_secs(20);
/// What the warm-up embeds to load the model.
const WARM_UP_TEXT: &str = "warm-up";

/// Embeds through one llama-helper process.
pub struct HelperEmbedder {
    helper: PathBuf,
    model: PathBuf,
    jobs: Mutex<Option<Sender<Job>>>,
    slot: Slot,
    thread: Mutex<Option<JoinHandle<()>>>,
    warm: OnceCell<bool>,
}

impl HelperEmbedder {
    /// The embedder for the database at `database`, when the model is
    /// downloaded beside it and the helper is installed.
    pub async fn locate(database: &Path) -> Option<Self> {
        let model = model_file(database).await?;
        let helper = helper_file()?;
        Some(Self {
            helper,
            model,
            jobs: Mutex::new(None),
            slot: Slot::default(),
            thread: Mutex::new(None),
            warm: OnceCell::new(),
        })
    }

    /// Sends one request line and waits for its reply line.
    async fn request(&self, line: String) -> Result<String, SearchError> {
        let (reply, answer) = oneshot::channel();
        let sent = self.sender().map(|jobs| jobs.send(Job { line, reply }));
        if !matches!(sent, Some(Ok(()))) {
            return Err(SearchError::ModelMissing);
        }
        match answer.await {
            Ok(Ok(line)) => Ok(line),
            Ok(Err(error)) => Err(SearchError::Embedding(error)),
            Err(_) => Err(SearchError::ModelMissing),
        }
    }

    /// The worker's queue, starting the worker thread on first use.
    fn sender(&self) -> Option<Sender<Job>> {
        let mut jobs = self.jobs.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(sender) = jobs.as_ref() {
            return Some(sender.clone());
        }
        let (sender, receiver) = channel();
        let (helper, slot) = (self.helper.clone(), Arc::clone(&self.slot));
        let thread = std::thread::Builder::new()
            .name("deino-embed".to_owned())
            .spawn(move || worker::serve(&helper, &receiver, &slot))
            .ok()?;
        *self.thread.lock().unwrap_or_else(PoisonError::into_inner) = Some(thread);
        *jobs = Some(sender.clone());
        Some(sender)
    }

    /// One batch of texts, already prefixed.
    async fn batch(&self, inputs: Vec<String>) -> Result<Vec<Vec<f32>>, SearchError> {
        let expected = inputs.len();
        let request = HelperRequest::Embed {
            model_path: self.model.to_string_lossy().into_owned(),
            inputs,
        };
        let line = serde_json::to_string(&request)
            .map_err(|error| SearchError::Embedding(error.to_string()))?;
        let reply = self.request(line).await?;
        match serde_json::from_str(&reply) {
            Ok(EmbedReply::Embeddings { vectors }) if vectors.len() == expected => Ok(vectors),
            Ok(EmbedReply::Error { message }) => Err(SearchError::Embedding(message)),
            _ => Err(SearchError::Embedding("unexpected reply".to_owned())),
        }
    }
}

#[async_trait]
impl Embedder for HelperEmbedder {
    fn model(&self) -> &str {
        LOCAL_EMBEDDING_MODEL
    }

    /// Starts the helper and loads the model the first time; false when
    /// that fails or takes longer than [`WARM_UP`].
    async fn ready(&self) -> bool {
        *self
            .warm
            .get_or_init(|| async {
                let text = [WARM_UP_TEXT.to_owned()];
                let probe = self.embed(&text, EmbedRole::Query);
                matches!(tokio::time::timeout(WARM_UP, probe).await, Ok(Ok(_)))
            })
            .await
    }

    async fn embed(&self, texts: &[String], role: EmbedRole) -> Result<Vec<Vec<f32>>, SearchError> {
        let prefix = match role {
            EmbedRole::Document => DOCUMENT_PREFIX,
            EmbedRole::Query => QUERY_PREFIX,
        };
        let mut vectors = Vec::with_capacity(texts.len());
        for chunk in texts.chunks(EMBED_BATCH) {
            let inputs = chunk.iter().map(|text| format!("{prefix}{text}")).collect();
            vectors.extend(self.batch(inputs).await?);
        }
        Ok(vectors)
    }
}

impl Drop for HelperEmbedder {
    /// Hangs up on the worker, stops the helper and waits for the thread,
    /// so no helper outlives this process.
    fn drop(&mut self) {
        self.jobs
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take();
        worker::stop(&self.slot);
        let thread = self
            .thread
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take();
        if let Some(thread) = thread {
            let _finished = thread.join();
        }
    }
}

/// The downloaded embedding model beside `database`, if it is complete.
async fn model_file(database: &Path) -> Option<PathBuf> {
    let catalog = Catalog::bundled().ok()?;
    let model = catalog.model(ModelPurpose::Embedding).ok()?;
    let file = model.file(model.default_quant)?;
    let store = ModelStore::new(database.parent()?.join("models").join(MODELS_FOLDER));
    (store.state(file).await.ok()? == ModelState::Ready).then(|| store.path(file))
}

/// `llama-helper`: `DEINO_LLAMA_HELPER`, else beside this binary (through
/// the `deino` symlink to the app bundle).
fn helper_file() -> Option<PathBuf> {
    let path = match std::env::var_os(HELPER_ENV) {
        Some(path) => PathBuf::from(path),
        None => std::env::current_exe()
            .ok()?
            .canonicalize()
            .ok()?
            .parent()?
            .join(HELPER_NAME),
    };
    path.is_file().then_some(path)
}
