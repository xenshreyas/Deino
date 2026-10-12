//! [`InstalledSnapshot`]: which transcription models are on this Mac, read
//! once from each engine's folder.
//!
//! It is the app's [`InstalledModels`]: the startup self-heal
//! (`composition::stt`) and the lanes' missing-model error read it. Reading
//! never loads a model or touches the network.

use std::path::Path;

use deino_core::models::{ModelFileId, ModelService, ModelState};
use deino_core::settings::{DEFAULT_PARAKEET_MODEL, InstalledModels, TranscriptionEngine};

use crate::parakeet_engine::{ModelStatus as TdtStatus, ParakeetEngine};
use crate::whisper_engine::{ModelStatus as WhisperStatus, WhisperEngine};

/// The installed models of every engine.
#[derive(Debug, Default)]
pub struct InstalledSnapshot {
    /// Ready Parakeet Unified files, the recommended quant first.
    unified: Vec<ModelFileId>,
    /// The quant this Mac suits, which an empty Unified model means.
    recommended: Option<ModelFileId>,
    /// Ready Parakeet TDT model folders.
    tdt: Vec<String>,
    /// Ready Whisper models.
    whisper: Vec<String>,
}

impl InstalledSnapshot {
    /// Reads Parakeet Unified's store through `models` and the Advanced
    /// engines' folders under `models_dir`. A folder that cannot be read
    /// counts as empty (and is logged).
    pub async fn read(models: &ModelService, models_dir: &Path) -> Self {
        let (unified, recommended) = unified_files(models).await;
        Self {
            unified,
            recommended,
            tdt: tdt_models(models_dir).await,
            whisper: whisper_models(models_dir).await,
        }
    }
}

impl InstalledModels for InstalledSnapshot {
    fn is_installed(&self, engine: TranscriptionEngine, model: &str) -> bool {
        let model = model.trim();
        match engine {
            TranscriptionEngine::ParakeetUnified => {
                let id = if model.is_empty() {
                    self.recommended.clone()
                } else {
                    Some(ModelFileId::from_setting(model))
                };
                id.is_some_and(|id| self.unified.contains(&id))
            }
            TranscriptionEngine::Parakeet => {
                let name = if model.is_empty() {
                    DEFAULT_PARAKEET_MODEL
                } else {
                    model
                };
                self.tdt.iter().any(|ready| ready == name)
            }
            TranscriptionEngine::Whisper if model.is_empty() => !self.whisper.is_empty(),
            TranscriptionEngine::Whisper => self.whisper.iter().any(|ready| ready == model),
        }
    }

    fn installed_unified(&self) -> Option<String> {
        self.unified.first().map(|id| id.as_str().to_owned())
    }
}

/// Ready Unified files (recommended first) and the recommended file.
async fn unified_files(models: &ModelService) -> (Vec<ModelFileId>, Option<ModelFileId>) {
    let views = models.list().await.unwrap_or_else(|error| {
        log::warn!(target: "stt", "could not list the Parakeet Unified models: {error}");
        Vec::new()
    });
    let recommended = views
        .iter()
        .find(|view| view.recommended)
        .map(|view| view.id.clone());
    let mut ready: Vec<_> = views
        .into_iter()
        .filter(|view| view.state == ModelState::Ready)
        .collect();
    ready.sort_by_key(|view| !view.recommended);
    (ready.into_iter().map(|view| view.id).collect(), recommended)
}

/// Parakeet TDT folders that hold every file the engine needs.
async fn tdt_models(models_dir: &Path) -> Vec<String> {
    let found = match ParakeetEngine::new_with_models_dir(Some(models_dir.to_path_buf())) {
        Ok(engine) => engine.discover_models().await,
        Err(error) => Err(error),
    };
    match found {
        Ok(models) => models
            .into_iter()
            .filter(|model| matches!(model.status, TdtStatus::Available))
            .map(|model| model.name)
            .collect(),
        Err(error) => {
            log::warn!(target: "stt", "could not read the Parakeet TDT models: {error}");
            Vec::new()
        }
    }
}

/// Whisper models whose file is complete.
async fn whisper_models(models_dir: &Path) -> Vec<String> {
    let found = match WhisperEngine::new_with_models_dir(Some(models_dir.to_path_buf())) {
        Ok(engine) => engine.discover_models().await,
        Err(error) => Err(error),
    };
    match found {
        Ok(models) => models
            .into_iter()
            .filter(|model| matches!(model.status, WhisperStatus::Available))
            .map(|model| model.name)
            .collect(),
        Err(error) => {
            log::warn!(target: "stt", "could not read the Whisper models: {error}");
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const Q8: &str = "parakeet-unified-en-0.6b-Q8_0";

    fn snapshot() -> InstalledSnapshot {
        InstalledSnapshot {
            unified: vec![ModelFileId::new(Q8)],
            recommended: Some(ModelFileId::new(Q8)),
            ..InstalledSnapshot::default()
        }
    }

    #[test]
    fn a_unified_model_resolves_by_its_id_or_its_gguf_file_name() {
        let installed = snapshot();

        assert!(installed.is_installed(TranscriptionEngine::ParakeetUnified, Q8));
        assert!(installed.is_installed(
            TranscriptionEngine::ParakeetUnified,
            "parakeet-unified-en-0.6b-Q8_0.gguf"
        ));
        assert!(installed.is_installed(TranscriptionEngine::ParakeetUnified, ""));
    }

    #[test]
    fn an_unresolvable_or_missing_model_is_not_installed() {
        let installed = snapshot();

        assert!(!installed.is_installed(TranscriptionEngine::ParakeetUnified, "nope.gguf"));
        assert!(!installed.is_installed(TranscriptionEngine::Parakeet, ""));
        assert!(!installed.is_installed(TranscriptionEngine::Whisper, ""));
        assert_eq!(installed.installed_unified().as_deref(), Some(Q8));
    }
}
