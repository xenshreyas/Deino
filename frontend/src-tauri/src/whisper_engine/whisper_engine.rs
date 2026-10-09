// Commit name to recover the serial whisper engine processing for smaller meetings [Slower processing but dooes not fail] - "before parallel processing implementation"

use std::path::{PathBuf};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{watch, Mutex, RwLock};
use tokio_util::sync::CancellationToken;
use deino_stt::{BackendChoice, BatchTranscript, EngineSupervisor, LoadSpec, SupervisorConfig, WorkerLauncher};
use serde::{Serialize, Deserialize};
use anyhow::{Result, anyhow};
use crate::config::TRANSCRIPTION_LANGUAGE;
use deino_net::{NetClient, Request};
use tokio::fs;
use tokio::io::AsyncWriteExt;
use crate::config::WHISPER_MODEL_CATALOG;
use super::acceleration::WhisperCompiledBackend;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModelStatus {
    Available,
    Missing,
    Downloading { progress: u8 },
    Error(String),
    Corrupted { file_size: u64, expected_min_size: u64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub name: String,
    pub path: PathBuf,
    pub size_mb: u32,
    pub accuracy: String,
    pub speed: String,
    pub status: ModelStatus,
    pub description: String,
}

struct ActiveDownload {
    cancellation: CancellationToken,
    completion: watch::Sender<bool>,
}

#[cfg(test)]
struct DownloadStateTestHook {
    cancellation_first_lock_acquired: tokio::sync::Notify,
    continue_cancellation: tokio::sync::Notify,
    discovery_active_lock_acquired: tokio::sync::Notify,
    continue_discovery: tokio::sync::Notify,
}

#[derive(Debug, thiserror::Error)]
#[error("Download cancelled by user")]
pub(crate) struct DownloadCancelled;

pub(crate) fn is_download_cancelled(error: &anyhow::Error) -> bool {
    error.downcast_ref::<DownloadCancelled>().is_some()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CancelDownloadOutcome {
    Cancelled,
    Pending,
}

const CANCEL_DOWNLOAD_CLEANUP_TIMEOUT: Duration = Duration::from_secs(5);

pub struct WhisperEngine {
    models_dir: PathBuf,
    /// The worker process running the loaded model (CAP-01: whisper.cpp `.bin`
    /// files run through transcribe-cpp, since whisper-rs cannot link beside it).
    current_engine: Arc<RwLock<Option<Arc<EngineSupervisor>>>>,
    current_model: Arc<RwLock<Option<String>>>,
    available_models: Arc<RwLock<HashMap<String, ModelInfo>>>,
    // State tracking for smart logging
    last_transcription_was_short: Arc<RwLock<bool>>,
    short_audio_warning_logged: Arc<RwLock<bool>>,
    // Performance optimization: reduce logging frequency
    transcription_count: Arc<RwLock<u64>>,
    // A model remains active until its owning worker has completed cleanup.
    active_downloads: Arc<Mutex<HashMap<String, Arc<ActiveDownload>>>>,
    #[cfg(test)]
    download_state_test_hook: std::sync::Mutex<Option<Arc<DownloadStateTestHook>>>,
}

impl WhisperEngine {
    /// Detect available GPU acceleration capabilities
    fn detect_gpu_acceleration() -> bool {
        match WhisperCompiledBackend::current() {
            WhisperCompiledBackend::Metal => {
                log::info!("macOS detected - attempting to enable Metal GPU acceleration");
                true
            }
            WhisperCompiledBackend::Cuda => {
                log::info!("CUDA feature enabled - attempting GPU acceleration");
                true
            }
            WhisperCompiledBackend::Vulkan => {
                log::info!("Vulkan feature enabled - attempting GPU acceleration");
                true
            }
            WhisperCompiledBackend::HipBlas => {
                log::info!("HIP BLAS feature enabled - attempting GPU acceleration");
                true
            }
            WhisperCompiledBackend::Cpu => {
                log::info!("No GPU acceleration features detected - using CPU processing");
                false
            }
        }
    }

    /// Create a WhisperEngine using the models directory configured at startup.
    pub fn new() -> Result<Self> {
        Self::new_with_models_dir(super::commands::configured_models_dir())
    }

    /// Create a new WhisperEngine with optional custom models directory
    /// If models_dir is None, uses default location (app data dir for production, local for dev)
    pub fn new_with_models_dir(models_dir: Option<PathBuf>) -> Result<Self> {
        // PERFORMANCE: Suppress verbose whisper.cpp and Metal logs
        // These C library logs bypass Rust logging and clutter output
        // Set environment variables to reduce C library verbosity
        std::env::set_var("GGML_METAL_LOG_LEVEL", "1"); // 0=off, 1=error, 2=warn, 3=info
        std::env::set_var("WHISPER_LOG_LEVEL", "1");    // Reduce whisper.cpp verbosity

        let models_dir = if let Some(dir) = models_dir {
            // Use provided directory (for production with app_data_dir)
            dir
        } else {
            // Fallback: determine based on debug/release mode
            let current_dir = std::env::current_dir()
                .map_err(|e| anyhow!("Failed to get current directory: {}", e))?;

            // Development: use a local `models/` directory.
            // Production: the caller must configure the directory.
            if cfg!(debug_assertions) {
                if current_dir.join("models").exists() {
                    current_dir.join("models")
                } else if current_dir.join("../models").exists() {
                    current_dir.join("../models")
                } else {
                    // Create models directory in current directory for development
                    current_dir.join("models")
                }
            } else {
                // Production: the models dir lives in the app-identifier data dir and
                // is resolved by Tauri at startup (`commands::set_models_directory`).
                return Err(anyhow!("Whisper models directory is not configured"));
            }
        };
        
        log::info!("WhisperEngine using models directory: {}", models_dir.display());
        log::info!("Debug mode: {}", cfg!(debug_assertions));

        // Log acceleration capabilities
        let gpu_support = Self::detect_gpu_acceleration();
        log::info!("Hardware acceleration support: {}", if gpu_support { "enabled" } else { "disabled" });

        #[cfg(feature = "metal")]
        log::info!("Apple Metal GPU support: enabled");

        #[cfg(feature = "openblas")]
        log::info!("OpenBLAS CPU optimization: enabled");

        #[cfg(feature = "coreml")]
        log::info!("Apple CoreML support: enabled");

        #[cfg(feature = "cuda")]
        log::info!("NVIDIA CUDA support: enabled");

        #[cfg(feature = "vulkan")]
        log::info!("Vulkan GPU support: enabled");

        #[cfg(feature = "openmp")]
        log::info!("OpenMP parallel processing: enabled");
        
        let engine = Self {
            models_dir,
            current_engine: Arc::new(RwLock::new(None)),
            current_model: Arc::new(RwLock::new(None)),
            available_models: Arc::new(RwLock::new(HashMap::new())),
            // Initialize state tracking
            last_transcription_was_short: Arc::new(RwLock::new(false)),
            short_audio_warning_logged: Arc::new(RwLock::new(false)),
            // Performance optimization: reduce logging frequency
            transcription_count: Arc::new(RwLock::new(0)),
            active_downloads: Arc::new(Mutex::new(HashMap::new())),
            #[cfg(test)]
            download_state_test_hook: std::sync::Mutex::new(None),
        };
        
        Ok(engine)
    }

    #[cfg(test)]
    fn test_hook(&self) -> Option<Arc<DownloadStateTestHook>> {
        self.download_state_test_hook
            .lock()
            .expect("download state test hook mutex should not be poisoned")
            .clone()
    }
    
    pub async fn discover_models(&self) -> Result<Vec<ModelInfo>> {
        let models_dir = &self.models_dir;
        let mut models = Vec::new();
        // Use centralized model catalog from config.rs
        let model_configs = WHISPER_MODEL_CATALOG;

        for &(name, filename, size_mb, accuracy, speed, description) in model_configs {
            let model_path = models_dir.join(filename);
            let status = if model_path.exists() {
                // Check if file size is reasonable (at least 1MB for a valid model)
                match std::fs::metadata(&model_path) {
                    Ok(metadata) => {
                        let file_size_bytes = metadata.len();
                        let file_size_mb = file_size_bytes / (1024 * 1024);
                        let expected_min_size_mb = (size_mb as f64 * 0.9) as u64; // Allow 90% of expected size as minimum for more accurate corruption detection

                        if file_size_mb >= expected_min_size_mb && file_size_mb > 1 {
                            // File size looks good, but let's also check if it's a valid GGML file
                            match self.validate_model_file(&model_path).await {
                                Ok(_) => ModelStatus::Available,
                                Err(_) => {
                                    log::warn!("Model file {} has correct size but appears corrupted (failed validation)",
                                             filename);
                                    ModelStatus::Corrupted {
                                        file_size: file_size_bytes,
                                        expected_min_size: (expected_min_size_mb * 1024 * 1024) as u64
                                    }
                                }
                            }
                        } else if file_size_mb > 0 {
                            log::warn!("Model file {} exists but is corrupted ({} MB, expected ~{} MB)",
                                     filename, file_size_mb, size_mb);
                            ModelStatus::Corrupted {
                                file_size: file_size_bytes,
                                expected_min_size: (expected_min_size_mb * 1024 * 1024) as u64
                            }
                        } else {
                            ModelStatus::Missing
                        }
                    }
                    Err(_) => ModelStatus::Missing
                }
            } else {
                ModelStatus::Missing
            };
            
            let model_info = ModelInfo {
                name: name.to_string(),
                path: model_path,
                size_mb: size_mb as u32,
                accuracy: accuracy.to_string(),
                speed: speed.to_string(),
                status,
                description: description.to_string(),
            };
            
            models.push(model_info);
        }
        
        let active_downloads = self.active_downloads.lock().await;
        #[cfg(test)]
        if let Some(hook) = self.test_hook() {
            hook.discovery_active_lock_acquired.notify_one();
            hook.continue_discovery.notified().await;
        }
        let mut available_models = self.available_models.write().await;
        for model in &mut models {
            if active_downloads.contains_key(&model.name) {
                let progress = available_models
                    .get(&model.name)
                    .and_then(|cached| match cached.status {
                        ModelStatus::Downloading { progress } => Some(progress),
                        _ => None,
                    })
                    .unwrap_or(0);
                model.status = ModelStatus::Downloading { progress };
            }
        }
        available_models.clear();
        for model in &models {
            available_models.insert(model.name.clone(), model.clone());
        }
        
        Ok(models)
    }
    
    pub async fn load_model(&self, model_name: &str) -> Result<()> {
        let models = self.available_models.read().await;
        let model_info = models.get(model_name)
            .ok_or_else(|| anyhow!("Model {} not found", model_name))?;

        match model_info.status {
            ModelStatus::Available => {
                // FIX 5: Check if this model is already loaded
                if let Some(current_model) = self.current_model.read().await.as_ref() {
                    if current_model == model_name {
                        log::info!("Model {} is already loaded, skipping reload", model_name);
                        return Ok(());
                    }

                    // FIX 5: Unload current model before loading new one
                    log::info!("Unloading current model '{}' before loading '{}'", current_model, model_name);
                    self.unload_model().await;
                }

                log::info!("Loading model: {}", model_name);

                let path = model_info.path.clone();
                let engine = tokio::task::spawn_blocking(move || -> Result<Arc<EngineSupervisor>> {
                    let engine = EngineSupervisor::new(WorkerLauncher::current_exe()?, SupervisorConfig::default())?;
                    let info = engine.load(LoadSpec { path, backend: BackendChoice::Auto })?;
                    log::info!("Whisper model loaded in a transcription worker on {} ({})", info.backend, info.arch);
                    Ok(Arc::new(engine))
                })
                .await
                .map_err(|e| anyhow!("Model load task failed: {}", e))?
                .map_err(|e| anyhow!("Failed to load model {}: {}", model_name, e))?;

                *self.current_engine.write().await = Some(engine);
                *self.current_model.write().await = Some(model_name.to_string());
                log::info!("Successfully loaded model: {}", model_name);
                Ok(())
            },
            ModelStatus::Missing => {
                Err(anyhow!("Model {} is not downloaded", model_name))
            },
            ModelStatus::Downloading { .. } => {
                Err(anyhow!("Model {} is currently downloading", model_name))
            },
            ModelStatus::Error(ref err) => {
                Err(anyhow!("Model {} has error: {}", model_name, err))
            },
            ModelStatus::Corrupted { .. } => {
                Err(anyhow!("Model {} is corrupted and cannot be loaded", model_name))
            }
        }
    }

    pub async fn unload_model(&self) -> bool  {
        // Dropping the last handle stops the worker, which frees the model.
        let mut engine_guard = self.current_engine.write().await;
        let unloaded = engine_guard.take().is_some();
        if unloaded {
            log::info!("📉Whisper model unloaded");
        }

        let mut model_name_guard = self.current_model.write().await;
        model_name_guard.take();

        unloaded
    }

    pub async fn get_current_model(&self) -> Option<String> {
        self.current_model.read().await.clone()
    }
    
    pub async fn is_model_loaded(&self) -> bool {
        self.current_engine.read().await.is_some()
    }
    
    // Enhanced function to clean repetitive text patterns and meaningless outputs
    fn clean_repetitive_text(text: &str) -> String {
        if text.is_empty() {
            return String::new();
        }

        // Check for obviously meaningless patterns first
        if Self::is_meaningless_output(text) {
            // Performance optimization: reduce meaningless output logging to debug level
            // Lengths only: transcript text never reaches the log (PARITY_PLAN §3A).
            perf_debug!("Detected meaningless output ({} chars), returning empty", text.chars().count());
            return String::new();
        }

        let words: Vec<&str> = text.split_whitespace().collect();
        if words.len() < 3 {
            return text.to_string();
        }

        // Enhanced repetition detection with sliding window
        let cleaned_words = Self::remove_word_repetitions(&words);

        // Remove phrase repetitions with more sophisticated detection
        let cleaned_words = Self::remove_phrase_repetitions(&cleaned_words);

        // Check for overall repetition ratio
        let final_text = cleaned_words.join(" ");
        if Self::calculate_repetition_ratio(&final_text) > 0.7 {
            // Performance optimization: reduce repetition ratio logging to debug level
            perf_debug!("High repetition ratio detected, filtering out {} chars", final_text.chars().count());
            return String::new();
        }

        final_text
    }

    // Check for obviously meaningless patterns
    fn is_meaningless_output(text: &str) -> bool {
        let text_lower = text.to_lowercase();

        // Check for common meaningless patterns
        let meaningless_patterns = [
            "thank you for watching",
            "thanks for watching",
            "like and subscribe",
            "music playing",
            "applause",
            "laughter",
            "um um um",
            "uh uh uh",
            "ah ah ah",
        ];

        for pattern in &meaningless_patterns {
            if text_lower.contains(pattern) {
                return true;
            }
        }

        // Check if text is mostly the same character or very short repetitive patterns
        let unique_chars: HashSet<char> = text.chars().collect();
        if unique_chars.len() <= 3 && text.len() > 10 {
            return true;
        }

        false
    }

    // Enhanced word repetition removal
    fn remove_word_repetitions<'a>(words: &'a [&'a str]) -> Vec<&'a str> {
        let mut cleaned_words = Vec::new();
        let mut i = 0;

        while i < words.len() {
            let current_word = words[i];
            let mut repeat_count = 1;

            // Count consecutive repetitions of the same word
            while i + repeat_count < words.len() && words[i + repeat_count] == current_word {
                repeat_count += 1;
            }

            // Be more aggressive: if word is repeated 2+ times, only keep one instance
            if repeat_count >= 2 {
                cleaned_words.push(current_word);
                i += repeat_count;
            } else {
                cleaned_words.push(current_word);
                i += 1;
            }
        }

        cleaned_words
    }

    // Enhanced phrase repetition removal with variable length detection
    fn remove_phrase_repetitions<'a>(words: &'a [&'a str]) -> Vec<&'a str> {
        if words.len() < 4 {
            return words.to_vec();
        }

        let mut final_words = Vec::new();
        let mut i = 0;

        while i < words.len() {
            let mut phrase_found = false;

            // Check for 2-word to 5-word phrase repetitions
            for phrase_len in 2..=std::cmp::min(5, (words.len() - i) / 2) {
                if i + phrase_len * 2 <= words.len() {
                    let phrase1 = &words[i..i + phrase_len];
                    let phrase2 = &words[i + phrase_len..i + phrase_len * 2];

                    if phrase1 == phrase2 {
                        // Add the phrase once and skip the repetition
                        final_words.extend_from_slice(phrase1);
                        i += phrase_len * 2;
                        phrase_found = true;
                        break;
                    }
                }
            }

            if !phrase_found {
                final_words.push(words[i]);
                i += 1;
            }
        }

        final_words
    }

    // Calculate repetition ratio in text
    fn calculate_repetition_ratio(text: &str) -> f32 {
        let words: Vec<&str> = text.split_whitespace().collect();
        if words.len() < 4 {
            return 0.0;
        }

        let mut word_counts = HashMap::new();
        for word in &words {
            *word_counts.entry(word.to_lowercase()).or_insert(0) += 1;
        }

        let total_words = words.len() as f32;
        let repeated_words: usize = word_counts.values().map(|&count| if count > 1 { count - 1 } else { 0 }).sum();

        repeated_words as f32 / total_words
    }
    
    /// Runs `audio_data` through the loaded model's worker process. English
    /// is pinned by the worker for multilingual models ([`TRANSCRIPTION_LANGUAGE`]).
    async fn run_engine(&self, audio_data: Vec<f32>) -> Result<BatchTranscript> {
        let engine = self.current_engine.read().await.clone()
            .ok_or_else(|| anyhow!("No model loaded. Please load a model first."))?;
        tokio::task::spawn_blocking(move || engine.transcribe(audio_data))
            .await
            .map_err(|e| anyhow!("Transcription task failed: {}", e))?
            .map_err(|e| anyhow!("Transcription failed: {}", e))
    }

    /// The mean token confidence, or a neutral 0.85 when the model reports none.
    fn confidence_of(transcript: &BatchTranscript) -> f32 {
        let scores: Vec<f32> = transcript.tokens.iter().filter_map(|t| t.confidence).collect();
        if scores.is_empty() {
            0.85
        } else {
            scores.iter().sum::<f32>() / scores.len() as f32
        }
    }

    /// Transcribe audio, returning (text, confidence, is_partial).
    pub async fn transcribe_audio_with_confidence(&self, audio_data: Vec<f32>) -> Result<(String, f32, bool)> {
        let duration_seconds = audio_data.len() as f64 / 16000.0;
        let is_partial = duration_seconds < 15.0; // Consider chunks under 15s as partial
        let transcript = self.run_engine(audio_data).await?;
        let cleaned_result = Self::clean_repetitive_text(transcript.text.trim());
        Ok((cleaned_result, Self::confidence_of(&transcript), is_partial))
    }

    pub async fn transcribe_audio(&self, audio_data: Vec<f32>) -> Result<String> {
        let duration_seconds = audio_data.len() as f64 / 16000.0;
        let transcription_count = {
            let mut count = self.transcription_count.write().await;
            *count += 1;
            *count
        };
        let transcript = self.run_engine(audio_data).await?;
        let cleaned_result = Self::clean_repetitive_text(transcript.text.trim());
        perf_trace!("Transcription #{} of {:.1}s: {} chars", transcription_count, duration_seconds, cleaned_result.chars().count());
        Ok(cleaned_result)
    }

    pub async fn get_models_directory(&self) -> PathBuf {
        self.models_dir.clone()
    }

    /// Validate if a model file is a valid GGML file by checking its header
    async fn validate_model_file(&self, model_path: &PathBuf) -> Result<()> {
        use tokio::io::AsyncReadExt;

        let mut file = fs::File::open(model_path).await
            .map_err(|e| anyhow!("Failed to open model file: {}", e))?;

        // Read the first 8 bytes to check for GGML magic number
        let mut buffer = [0u8; 8];
        file.read_exact(&mut buffer).await
            .map_err(|e| anyhow!("Failed to read model file header: {}", e))?;

        // Check for GGML magic number (various versions and endianness)
        if buffer.starts_with(b"ggml") || buffer.starts_with(b"GGUF") || buffer.starts_with(b"ggmf") ||
           buffer.starts_with(b"lmgg") || buffer.starts_with(b"FUGU") || buffer.starts_with(b"fmgg") {
            Ok(())
        } else {
            Err(anyhow!("Invalid model file: missing GGML/GGUF magic number. Found: {:?}",
                       String::from_utf8_lossy(&buffer[..4])))
        }
    }

    pub async fn delete_model(&self, model_name: &str) -> Result<String> {
        log::info!("Attempting to delete model: {}", model_name);

        // Get model info to find the file path
        let model_info = {
            let models = self.available_models.read().await;
            models.get(model_name).cloned()
        };

        let model_info = model_info.ok_or_else(|| anyhow!("Model '{}' not found", model_name))?;

        // Check if model is corrupted before allowing deletion
        log::info!("Model '{}' has status: {:?}", model_name, model_info.status);
        match &model_info.status {
            ModelStatus::Corrupted { file_size, expected_min_size } => {
                log::info!("Deleting corrupted model '{}' (file size: {} bytes, expected min: {} bytes)",
                          model_name, file_size, expected_min_size);

                // Delete the file
                if model_info.path.exists() {
                    fs::remove_file(&model_info.path).await
                        .map_err(|e| anyhow!("Failed to delete file '{}': {}", model_info.path.display(), e))?;
                    log::info!("Successfully deleted corrupted file: {}", model_info.path.display());
                } else {
                    log::warn!("File '{}' does not exist, nothing to delete", model_info.path.display());
                }

                // Update model status to Missing
                {
                    let mut models = self.available_models.write().await;
                    if let Some(model) = models.get_mut(model_name) {
                        model.status = ModelStatus::Missing;
                    }
                }

                Ok(format!("Successfully deleted corrupted model '{}'", model_name))
            }
            ModelStatus::Available => {
                // Allow deletion of available models for testing/cleanup
                log::info!("Deleting available model '{}' (for cleanup)", model_name);

                if model_info.path.exists() {
                    fs::remove_file(&model_info.path).await
                        .map_err(|e| anyhow!("Failed to delete file '{}': {}", model_info.path.display(), e))?;
                    log::info!("Successfully deleted available model file: {}", model_info.path.display());
                } else {
                    log::warn!("File '{}' does not exist, nothing to delete", model_info.path.display());
                }

                // Update model status to Missing
                {
                    let mut models = self.available_models.write().await;
                    if let Some(model) = models.get_mut(model_name) {
                        model.status = ModelStatus::Missing;
                    }
                }

                Ok(format!("Successfully deleted model '{}'", model_name))
            }
            _ => {
                Err(anyhow!("Can only delete corrupted or available models. Model '{}' has status: {:?}", model_name, model_info.status))
            }
        }
    }
    
    async fn reserve_active_download(&self, model_name: &str) -> Result<Arc<ActiveDownload>> {
        let mut active_downloads = self.active_downloads.lock().await;
        if active_downloads.contains_key(model_name) {
            return Err(anyhow!("Download already in progress for model: {}", model_name));
        }

        let (completion, _) = watch::channel(false);
        let active_download = Arc::new(ActiveDownload {
            cancellation: CancellationToken::new(),
            completion,
        });

        active_downloads.insert(model_name.to_string(), Arc::clone(&active_download));
        Ok(active_download)
    }

    async fn finish_download(
        &self,
        model_name: &str,
        active_download: &Arc<ActiveDownload>,
        file_path: &PathBuf,
        mut result: Result<()>,
    ) -> Result<()> {
        let active_owner_matches = {
            let active_downloads = self.active_downloads.lock().await;
            matches!(
                active_downloads.get(model_name),
                Some(current) if Arc::ptr_eq(current, active_download)
            )
        };
        if !active_owner_matches {
            log::warn!("Download owner for {} was no longer active during finalization", model_name);
            active_download.completion.send_replace(true);
            return result;
        }

        if result.is_ok() && !active_download.cancellation.is_cancelled() {
            result = self.validate_model_file(file_path).await;

            if result.is_ok() {
                let expected_min_size = WHISPER_MODEL_CATALOG
                    .iter()
                    .find(|model| model.0 == model_name)
                    .map(|model| ((model.2 as f64 * 0.9) as u64) * 1024 * 1024);

                result = match expected_min_size {
                    Some(expected_min_size) => match fs::metadata(file_path).await {
                        Ok(metadata) if metadata.len() >= expected_min_size => Ok(()),
                        Ok(metadata) => Err(anyhow!(
                            "Downloaded model file is too small: {} bytes (expected at least {} bytes)",
                            metadata.len(),
                            expected_min_size
                        )),
                        Err(e) => Err(anyhow!(
                            "Failed to read downloaded model file metadata: {}",
                            e
                        )),
                    },
                    None => Err(anyhow!(
                        "Unsupported model for download validation: {}",
                        model_name
                    )),
                };
            }
        }

        if result.is_err() && !active_download.cancellation.is_cancelled() && file_path.exists() {
            if let Err(e) = fs::remove_file(file_path).await {
                log::warn!("Failed to clean up failed download file: {}", e);
            } else {
                log::info!("Cleaned up failed download file: {}", file_path.display());
            }
        }

        let (released_active_owner, cancellation_won) = {
            let mut active_downloads = self.active_downloads.lock().await;
            match active_downloads.get(model_name) {
                Some(current) if Arc::ptr_eq(current, active_download) => {
                    if active_download.cancellation.is_cancelled() {
                        (false, true)
                    } else {
                        active_downloads.remove(model_name);
                        (true, false)
                    }
                }
                _ => {
                    log::warn!("Download owner for {} was no longer active during finalization", model_name);
                    (false, active_download.cancellation.is_cancelled())
                }
            }
        };

        if cancellation_won {
            result = Err(DownloadCancelled.into());
            if file_path.exists() {
                if let Err(e) = fs::remove_file(file_path).await {
                    log::warn!("Failed to clean up cancelled download file: {}", e);
                } else {
                    log::info!("Cleaned up cancelled download file: {}", file_path.display());
                }
            }

            let removed = {
                let mut active_downloads = self.active_downloads.lock().await;
                #[cfg(test)]
                if let Some(hook) = self.test_hook() {
                    hook.cancellation_first_lock_acquired.notify_one();
                    hook.continue_cancellation.notified().await;
                }

                let mut models = self.available_models.write().await;
                match active_downloads.get(model_name) {
                    Some(current) if Arc::ptr_eq(current, active_download) => {
                        if let Some(model_info) = models.get_mut(model_name) {
                            model_info.status = ModelStatus::Missing;
                        }
                        active_downloads.remove(model_name);
                        true
                    }
                    _ => false,
                }
            };
            if !removed {
                log::warn!("Download owner for {} was no longer active during cancellation cleanup", model_name);
            }
        } else if released_active_owner {
            let mut models = self.available_models.write().await;
            if let Some(model_info) = models.get_mut(model_name) {
                if result.is_ok() {
                    model_info.status = ModelStatus::Available;
                    model_info.path = file_path.clone();
                } else {
                    model_info.status = ModelStatus::Missing;
                }
            }
        }

        active_download.completion.send_replace(true);
        result
    }

    /// Downloads a catalogued model through `http`, a `deino-net` client
    /// scoped to the model host (FND-12); the engine itself holds no client.
    pub async fn download_model(
        &self,
        model_name: &str,
        http: Arc<dyn NetClient>,
        progress_callback: Option<Box<dyn Fn(u8) + Send>>,
    ) -> Result<()> {
        log::info!("Starting download for model: {}", model_name);

        let model_url = match model_name {
            "tiny" => "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin",
            "base" => "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin",
            "small" => "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin",
            "medium" => "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium.bin",
            "large-v3-turbo" => "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-turbo.bin",
            "large-v3" => "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3.bin",
            "tiny-q5_1" => "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny-q5_1.bin",
            "base-q5_1" => "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base-q5_1.bin",
            "small-q5_1" => "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small-q5_1.bin",
            "medium-q5_0" => "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium-q5_0.bin",
            "large-v3-turbo-q5_0" => "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-turbo-q5_0.bin",
            "large-v3-q5_0" => "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-q5_0.bin",
            _ => return Err(anyhow!("Unsupported model: {}", model_name)),
        };

        self.download_model_from_url(model_name, model_url, http, progress_callback).await
    }

    async fn download_model_from_url(
        &self,
        model_name: &str,
        model_url: &str,
        http: Arc<dyn NetClient>,
        progress_callback: Option<Box<dyn Fn(u8) + Send>>,
    ) -> Result<()> {
        let active_download = self.reserve_active_download(model_name).await?;
        let file_path = self.models_dir.join(format!("ggml-{}.bin", model_name));

        let result = self
            .download_model_with_owner(
                model_name,
                model_url,
                http.as_ref(),
                &file_path,
                &active_download,
                progress_callback,
            )
            .await;

        self.finish_download(model_name, &active_download, &file_path, result)
            .await
    }

    async fn download_model_with_owner(
        &self,
        model_name: &str,
        model_url: &str,
        http: &dyn NetClient,
        file_path: &PathBuf,
        active_download: &ActiveDownload,
        progress_callback: Option<Box<dyn Fn(u8) + Send>>,
    ) -> Result<()> {
        if active_download.cancellation.is_cancelled() {
            return Err(DownloadCancelled.into());
        }

        if !self.models_dir.exists() {
            fs::create_dir_all(&self.models_dir)
                .await
                .map_err(|e| anyhow!("Failed to create models directory: {}", e))?;
        }

        {
            let mut models = self.available_models.write().await;
            if let Some(model_info) = models.get_mut(model_name) {
                model_info.status = ModelStatus::Downloading { progress: 0 };
            }
        }

        // No end-to-end timeout: a model file can take many minutes.
        let request = Request::get(model_url)
            .map_err(|e| anyhow!("Invalid download URL: {}", e))?
            .timeout(None);
        let response = tokio::select! {
            biased;
            _ = active_download.cancellation.cancelled() => return Err(DownloadCancelled.into()),
            response = http.send(request) => response
                .map_err(|e| anyhow!("Failed to start download: {}", e))?,
        };

        if !response.status.is_success() {
            return Err(anyhow!("Download failed with status: {}", response.status));
        }

        let total_size = response.content_length().unwrap_or(0);
        let mut file = fs::File::create(file_path)
            .await
            .map_err(|e| anyhow!("Failed to create file: {}", e))?;

        use futures_util::StreamExt;
        let mut stream = response.body;
        let mut downloaded = 0u64;
        let mut last_progress_report = 0u8;
        let mut last_report_time = std::time::Instant::now();

        if let Some(ref callback) = progress_callback {
            callback(0);
        }

        loop {
            let chunk_result = tokio::select! {
                biased;
                _ = active_download.cancellation.cancelled() => return Err(DownloadCancelled.into()),
                chunk_result = stream.next() => chunk_result,
            };

            let Some(chunk_result) = chunk_result else {
                break;
            };

            if active_download.cancellation.is_cancelled() {
                return Err(DownloadCancelled.into());
            }

            let chunk = chunk_result.map_err(|e| anyhow!("Failed to read chunk: {}", e))?;
            file.write_all(&chunk)
                .await
                .map_err(|e| anyhow!("Failed to write chunk to file: {}", e))?;

            downloaded += chunk.len() as u64;
            let progress = if total_size > 0 {
                ((downloaded as f64 / total_size as f64) * 100.0) as u8
            } else {
                0
            };

            let time_since_last_report = last_report_time.elapsed().as_secs();
            if progress >= last_progress_report + 1 || progress == 100 || time_since_last_report >= 2 {
                let mut models = self.available_models.write().await;
                if let Some(model_info) = models.get_mut(model_name) {
                    model_info.status = ModelStatus::Downloading { progress };
                }

                if let Some(ref callback) = progress_callback {
                    callback(progress);
                }

                last_progress_report = progress;
                last_report_time = std::time::Instant::now();
            }
        }

        {
            let mut models = self.available_models.write().await;
            if let Some(model_info) = models.get_mut(model_name) {
                model_info.status = ModelStatus::Downloading { progress: 100 };
            }
        }

        if let Some(ref callback) = progress_callback {
            callback(100);
        }

        file.flush()
            .await
            .map_err(|e| anyhow!("Failed to flush file: {}", e))?;

        if active_download.cancellation.is_cancelled() {
            return Err(DownloadCancelled.into());
        }


        Ok(())
    }

    pub async fn cancel_download(&self, model_name: &str) -> Result<CancelDownloadOutcome> {
        self.cancel_download_with_timeout(model_name, CANCEL_DOWNLOAD_CLEANUP_TIMEOUT)
            .await
    }

    async fn cancel_download_with_timeout(
        &self,
        model_name: &str,
        cleanup_timeout: Duration,
    ) -> Result<CancelDownloadOutcome> {
        log::info!("Cancelling download for model: {}", model_name);

        let Some(active_download) = self.active_downloads.lock().await.get(model_name).cloned() else {
            return Ok(CancelDownloadOutcome::Cancelled);
        };

        active_download.cancellation.cancel();

        let mut completion = active_download.completion.subscribe();
        if !*completion.borrow() {
            match tokio::time::timeout(cleanup_timeout, completion.changed()).await {
                Ok(Ok(())) => {}
                Ok(Err(_)) => {
                    return Err(anyhow!("Download worker ended before completing cancellation cleanup"));
                }
                Err(_) => {
                    return Ok(CancelDownloadOutcome::Pending);
                }
            }
        }

        Ok(CancelDownloadOutcome::Cancelled)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The root `deino-net` client: the test servers are on the loopback,
    /// which the app's model-download scope would refuse.
    fn test_http() -> Arc<dyn NetClient> {
        Arc::new(crate::composition::root_client().expect("build the HTTP client"))
    }
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;
    use tokio::sync::oneshot;
    use tokio::time::{timeout, Duration};

    fn tiny_model(models: &[ModelInfo]) -> &ModelInfo {
        models
            .iter()
            .find(|model| model.name == "tiny")
            .expect("tiny should be present in the Whisper model catalog")
    }

    #[tokio::test]
    async fn discover_models_keeps_active_downloads_downloading_before_a_file_exists() {
        let dir = tempfile::tempdir().unwrap();
        let engine = WhisperEngine::new_with_models_dir(Some(dir.path().to_path_buf())).unwrap();

        let _active_download = engine.reserve_active_download("tiny").await.unwrap();
        engine.available_models.write().await.insert(
            "tiny".to_string(),
            ModelInfo {
                name: "tiny".to_string(),
                path: dir.path().join("ggml-tiny.bin"),
                size_mb: 75,
                accuracy: "Basic".to_string(),
                speed: "Fast".to_string(),
                status: ModelStatus::Downloading { progress: 42 },
                description: "test model".to_string(),
            },
        );

        let absent = engine.discover_models().await.unwrap();
        assert!(matches!(
            tiny_model(&absent).status,
            ModelStatus::Downloading { progress: 42 }
        ));

        std::fs::write(dir.path().join("ggml-tiny.bin"), b"").unwrap();
        let zero_byte = engine.discover_models().await.unwrap();
        assert!(matches!(
            tiny_model(&zero_byte).status,
            ModelStatus::Downloading { progress: 42 }
        ));

        std::fs::write(dir.path().join("ggml-tiny.bin"), [0_u8; 32]).unwrap();
        let partial = engine.discover_models().await.unwrap();
        assert!(matches!(
            tiny_model(&partial).status,
            ModelStatus::Downloading { progress: 42 }
        ));
    }

    #[tokio::test]
    async fn discover_models_uses_disk_state_after_a_download_is_no_longer_active() {
        let dir = tempfile::tempdir().unwrap();
        let engine = WhisperEngine::new_with_models_dir(Some(dir.path().to_path_buf())).unwrap();

        let model_path = dir.path().join("ggml-tiny.bin");
        std::fs::write(&model_path, b"").unwrap();
        let missing_models = engine.discover_models().await.unwrap();
        assert!(matches!(tiny_model(&missing_models).status, ModelStatus::Missing));

        std::fs::write(&model_path, vec![0_u8; 2 * 1024 * 1024]).unwrap();
        let corrupted_models = engine.discover_models().await.unwrap();

        assert!(matches!(
            tiny_model(&corrupted_models).status,
            ModelStatus::Corrupted { .. }
        ));
    }

    #[tokio::test]
    async fn finish_download_publishes_available_after_releasing_active_owner() {
        let dir = tempfile::tempdir().unwrap();
        let engine = WhisperEngine::new_with_models_dir(Some(dir.path().to_path_buf())).unwrap();
        let model_path = dir.path().join("ggml-tiny.bin");

        std::fs::write(&model_path, b"ggml\0\0\0\0").unwrap();
        std::fs::OpenOptions::new()
            .write(true)
            .open(&model_path)
            .unwrap()
            .set_len(68 * 1024 * 1024)
            .unwrap();

        engine.discover_models().await.unwrap();
        let active_download = engine.reserve_active_download("tiny").await.unwrap();
        let active_models = engine.discover_models().await.unwrap();
        assert!(matches!(
            tiny_model(&active_models).status,
            ModelStatus::Downloading { progress: 0 }
        ));

        engine
            .finish_download("tiny", &active_download, &model_path, Ok(()))
            .await
            .unwrap();

        assert!(!engine.active_downloads.lock().await.contains_key("tiny"));
        assert!(matches!(
            engine
                .available_models
                .read()
                .await
                .get("tiny")
                .expect("tiny should remain cached")
                .status,
            ModelStatus::Available
        ));

        let rediscovered_models = engine.discover_models().await.unwrap();
        assert!(matches!(
            tiny_model(&rediscovered_models).status,
            ModelStatus::Available
        ));
    }

    #[tokio::test]
    async fn finish_download_rejects_invalid_header_and_cleans_file() {
        let dir = tempfile::tempdir().unwrap();
        let engine = WhisperEngine::new_with_models_dir(Some(dir.path().to_path_buf())).unwrap();
        let model_path = dir.path().join("ggml-tiny.bin");
        std::fs::write(&model_path, b"not-a-model").unwrap();

        engine.discover_models().await.unwrap();
        let active_download = engine.reserve_active_download("tiny").await.unwrap();
        let error = engine
            .finish_download("tiny", &active_download, &model_path, Ok(()))
            .await
            .unwrap_err();

        assert!(error
            .to_string()
            .contains("Invalid model file: missing GGML/GGUF magic number"));
        assert!(!engine.active_downloads.lock().await.contains_key("tiny"));
        assert!(!model_path.exists());
        assert!(matches!(
            engine
                .available_models
                .read()
                .await
                .get("tiny")
                .expect("tiny should remain cached")
                .status,
            ModelStatus::Missing
        ));
    }

    #[tokio::test]
    async fn unsupported_download_does_not_leave_an_active_transfer() {
        let dir = tempfile::tempdir().unwrap();
        let engine = WhisperEngine::new_with_models_dir(Some(dir.path().to_path_buf())).unwrap();

        assert!(engine.download_model("not-a-model", test_http(), None).await.is_err());
        assert!(!engine.active_downloads.lock().await.contains_key("not-a-model"));
    }

    #[tokio::test]
    async fn duplicate_download_reservation_is_rejected_while_the_first_owner_is_active() {
        let dir = tempfile::tempdir().unwrap();
        let engine = WhisperEngine::new_with_models_dir(Some(dir.path().to_path_buf())).unwrap();

        let _first_owner = engine.reserve_active_download("tiny").await.unwrap();
        assert!(engine.reserve_active_download("tiny").await.is_err());
    }

    #[tokio::test]
    async fn cancelled_finalization_holds_active_ownership_before_discovery() {
        let dir = tempfile::tempdir().unwrap();
        let engine = Arc::new(WhisperEngine::new_with_models_dir(Some(dir.path().to_path_buf())).unwrap());
        engine.discover_models().await.unwrap();

        let active_download = engine.reserve_active_download("tiny").await.unwrap();
        active_download.cancellation.cancel();

        let hook = Arc::new(DownloadStateTestHook {
            cancellation_first_lock_acquired: tokio::sync::Notify::new(),
            continue_cancellation: tokio::sync::Notify::new(),
            discovery_active_lock_acquired: tokio::sync::Notify::new(),
            continue_discovery: tokio::sync::Notify::new(),
        });
        *engine.download_state_test_hook.lock().unwrap() = Some(Arc::clone(&hook));

        let finalization_engine = Arc::clone(&engine);
        let finalization_owner = Arc::clone(&active_download);
        let model_path = dir.path().join("ggml-tiny.bin");
        let finalization = tokio::spawn(async move {
            finalization_engine
                .finish_download(
                    "tiny",
                    &finalization_owner,
                    &model_path,
                    Err(DownloadCancelled.into()),
                )
                .await
        });

        timeout(
            Duration::from_secs(1),
            hook.cancellation_first_lock_acquired.notified(),
        )
        .await
        .expect("cancellation finalization should reach its first lock");

        let discovery_engine = Arc::clone(&engine);
        let discovery = tokio::spawn(async move { discovery_engine.discover_models().await });

        assert!(
            timeout(
                Duration::from_millis(100),
                hook.discovery_active_lock_acquired.notified(),
            )
            .await
            .is_err(),
            "discovery must wait for cancellation to release active ownership"
        );

        hook.continue_cancellation.notify_one();
        let finalization_result = timeout(Duration::from_secs(1), finalization)
            .await
            .expect("cancelled finalization should complete")
            .unwrap()
            .unwrap_err();
        assert!(is_download_cancelled(&finalization_result));

        timeout(
            Duration::from_secs(1),
            hook.discovery_active_lock_acquired.notified(),
        )
        .await
        .expect("discovery should acquire active ownership after cancellation finalizes");
        hook.continue_discovery.notify_one();

        let models = timeout(Duration::from_secs(1), discovery)
            .await
            .expect("discovery should complete")
            .unwrap()
            .unwrap();
        assert!(matches!(tiny_model(&models).status, ModelStatus::Missing));
        assert!(!engine.active_downloads.lock().await.contains_key("tiny"));
    }

    #[tokio::test]
    async fn cancelling_an_unresponsive_download_keeps_ownership_reserved() {
        let dir = tempfile::tempdir().unwrap();
        let engine = WhisperEngine::new_with_models_dir(Some(dir.path().to_path_buf())).unwrap();
        let active_download = engine.reserve_active_download("tiny").await.unwrap();

        let outcome = engine
            .cancel_download_with_timeout("tiny", Duration::from_millis(1))
            .await
            .expect("an unresponsive download should report cancellation pending");

        assert_eq!(outcome, CancelDownloadOutcome::Pending);
        assert!(active_download.cancellation.is_cancelled());
        assert!(matches!(
            engine.active_downloads.lock().await.get("tiny"),
            Some(current) if Arc::ptr_eq(current, &active_download)
        ));
        assert!(engine.reserve_active_download("tiny").await.is_err());
    }

    #[tokio::test]
    async fn late_cancellation_preserves_a_completed_model_state() {
        let dir = tempfile::tempdir().unwrap();
        let engine = WhisperEngine::new_with_models_dir(Some(dir.path().to_path_buf())).unwrap();
        let model_path = dir.path().join("ggml-tiny.bin");

        std::fs::write(&model_path, b"ggml\0\0\0\0").unwrap();
        std::fs::OpenOptions::new()
            .write(true)
            .open(&model_path)
            .unwrap()
            .set_len(68 * 1024 * 1024)
            .unwrap();

        engine.discover_models().await.unwrap();
        let active_download = engine.reserve_active_download("tiny").await.unwrap();
        engine
            .finish_download("tiny", &active_download, &model_path, Ok(()))
            .await
            .unwrap();

        assert_eq!(
            engine
                .cancel_download_with_timeout("tiny", Duration::from_millis(1))
                .await
                .unwrap(),
            CancelDownloadOutcome::Cancelled
        );
        assert!(matches!(
            tiny_model(&engine.discover_models().await.unwrap()).status,
            ModelStatus::Available
        ));
    }

    async fn stalled_http_server(
    ) -> (
        String,
        oneshot::Receiver<()>,
        oneshot::Sender<()>,
        tokio::task::JoinHandle<()>,
    ) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let (headers_sent, headers_ready) = oneshot::channel();
        let (release_body, body_released) = oneshot::channel();

        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = [0_u8; 1024];
            let read = socket.read(&mut request).await.unwrap();
            assert!(read > 0, "the client sends a request before the fake server answers");
            socket
                .write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Length: 4\r\nConnection: close\r\n\r\n",
                )
                .await
                .unwrap();
            socket.flush().await.unwrap();
            headers_sent.send(()).unwrap();
            let _ = body_released.await;
            let _ = socket.write_all(b"test").await;
        });

        (format!("http://{address}/model.bin"), headers_ready, release_body, server)
    }

    async fn response_http_server(
        content_length: usize,
        body: &'static [u8],
    ) -> (
        String,
        oneshot::Receiver<Vec<u8>>,
        tokio::task::JoinHandle<()>,
    ) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let (request_sent, request_received) = oneshot::channel();

        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = Vec::new();
            let mut buffer = [0_u8; 1024];
            loop {
                let bytes_read = socket.read(&mut buffer).await.unwrap();
                if bytes_read == 0 {
                    break;
                }

                request.extend_from_slice(&buffer[..bytes_read]);
                if request.ends_with(b"\r\n\r\n") {
                    break;
                }
            }
            request_sent.send(request).unwrap();

            let headers = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {content_length}\r\nConnection: close\r\n\r\n"
            );
            socket.write_all(headers.as_bytes()).await.unwrap();
            socket.write_all(body).await.unwrap();
            socket.flush().await.unwrap();
        });

        (format!("http://{address}/model.bin"), request_received, server)
    }

    #[tokio::test]
    async fn downloaded_model_sends_user_agent_and_rejects_undersized_valid_header() {
        let dir = tempfile::tempdir().unwrap();
        let engine = WhisperEngine::new_with_models_dir(Some(dir.path().to_path_buf())).unwrap();
        engine.discover_models().await.unwrap();
        let (url, request, server) = response_http_server(8, b"ggml\0\0\0\0").await;

        let error = engine.download_model_from_url("tiny", &url, test_http(), None).await.unwrap_err();
        let request = String::from_utf8(request.await.unwrap()).unwrap();
        server.await.unwrap();

        assert!(error.to_string().contains("too small"));
        assert!(request.to_ascii_lowercase().contains(&format!(
            "user-agent: deino/{}",
            env!("CARGO_PKG_VERSION")
        )));
        assert!(!engine.active_downloads.lock().await.contains_key("tiny"));
        assert!(!dir.path().join("ggml-tiny.bin").exists());
        assert!(matches!(
            engine
                .available_models
                .read()
                .await
                .get("tiny")
                .expect("tiny should remain cached")
                .status,
            ModelStatus::Missing
        ));
    }


    #[tokio::test]
    async fn cancelling_a_stalled_download_waits_for_cleanup_before_retry_can_start() {
        let dir = tempfile::tempdir().unwrap();
        let engine = Arc::new(WhisperEngine::new_with_models_dir(Some(dir.path().to_path_buf())).unwrap());
        let (url_a, headers_a, release_a, server_a) = stalled_http_server().await;

        let download_a_engine = Arc::clone(&engine);
        let download_a = tokio::spawn(async move {
            download_a_engine
                .download_model_from_url("tiny", &url_a, test_http(), None)
                .await
        });

        timeout(Duration::from_secs(1), headers_a)
            .await
            .expect("first request should receive response headers")
            .unwrap();

        assert!(engine.active_downloads.lock().await.contains_key("tiny"));
        let status_while_active = engine.discover_models().await.unwrap();
        assert!(matches!(
            tiny_model(&status_while_active).status,
            ModelStatus::Downloading { progress: 0 }
        ));

        timeout(Duration::from_secs(1), engine.cancel_download("tiny"))
            .await
            .expect("cancellation should not wait for a stalled response body")
            .unwrap();

        let download_a_result = timeout(Duration::from_secs(1), download_a)
            .await
            .expect("cancelled worker should finish promptly")
            .unwrap();
        assert!(is_download_cancelled(&download_a_result.unwrap_err()));
        assert!(!engine.active_downloads.lock().await.contains_key("tiny"));
        assert!(!dir.path().join("ggml-tiny.bin").exists());

        let (url_b, headers_b, release_b, server_b) = stalled_http_server().await;
        let download_b_engine = Arc::clone(&engine);
        let download_b = tokio::spawn(async move {
            download_b_engine
                .download_model_from_url("tiny", &url_b, test_http(), None)
                .await
        });
        timeout(Duration::from_secs(1), headers_b)
            .await
            .expect("retry should reserve the released model slot")
            .unwrap();
        assert!(engine.active_downloads.lock().await.contains_key("tiny"));

        timeout(Duration::from_secs(1), engine.cancel_download("tiny"))
            .await
            .expect("retry cancellation should also complete promptly")
            .unwrap();
        assert!(is_download_cancelled(
            &timeout(Duration::from_secs(1), download_b)
                .await
                .expect("retry worker should finish promptly")
                .unwrap()
                .unwrap_err()
        ));

        let _ = release_a.send(());
        let _ = release_b.send(());
        let _ = server_a.await;
        let _ = server_b.await;
    }

    #[tokio::test]
    async fn incomplete_response_cleans_partial_file_and_cache() {
        let dir = tempfile::tempdir().unwrap();
        let engine = WhisperEngine::new_with_models_dir(Some(dir.path().to_path_buf())).unwrap();
        let model_path = dir.path().join("ggml-tiny.bin");
        engine.discover_models().await.unwrap();
        let (url, _request, server) = response_http_server(8, b"ggml").await;

        let error = engine
            .download_model_from_url("tiny", &url, test_http(), None)
            .await
            .unwrap_err();
        server.await.unwrap();

        assert!(error.to_string().contains("Failed to read chunk"));
        assert!(!engine.active_downloads.lock().await.contains_key("tiny"));
        assert!(!model_path.exists());
        assert!(matches!(
            engine
                .available_models
                .read()
                .await
                .get("tiny")
                .expect("tiny should remain cached")
                .status,
            ModelStatus::Missing
        ));

        let models = engine.discover_models().await.unwrap();
        assert!(matches!(tiny_model(&models).status, ModelStatus::Missing));
    }

    #[tokio::test]
    async fn terminal_download_error_releases_the_active_transfer() {
        let dir = tempfile::tempdir().unwrap();
        let engine = WhisperEngine::new_with_models_dir(Some(dir.path().to_path_buf())).unwrap();
        let model_path = dir.path().join("ggml-tiny.bin");
        std::fs::write(&model_path, vec![0_u8; 2 * 1024 * 1024]).unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();

        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = [0_u8; 1024];
            let read = socket.read(&mut request).await.unwrap();
            assert!(read > 0, "the client sends a request before the fake server answers");
            socket
                .write_all(b"HTTP/1.1 500 Internal Server Error\r\nContent-Length: 0\r\n\r\n")
                .await
                .unwrap();
        });

        assert!(engine
            .download_model_from_url("tiny", &format!("http://{address}/model.bin"), test_http(), None)
            .await
            .is_err());
        server.await.unwrap();

        assert!(!engine.active_downloads.lock().await.contains_key("tiny"));
        assert!(!model_path.exists());
        let models = engine.discover_models().await.unwrap();
        assert!(matches!(tiny_model(&models).status, ModelStatus::Missing));
    }
}
