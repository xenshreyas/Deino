//! [`ElevenLabsTranscriber`]: the `CloudTranscriber` the stage sends each
//! finished track to. The key is read from the Keychain per run; the file's
//! size is checked before it is read; the upload stops when the stage is
//! cancelled.

use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use deino_core::final_transcription::{CloudError, CloudRequest, CloudSpan, CloudTranscriber};
use deino_core::settings::CloudTranscriptionModel;
use deino_net::NetClient;
use deino_secrets::{SecretName, SecretStore};

use crate::{ScribeApi, ScribeOptions, utterances};

/// The largest file ElevenLabs accepts: "files up to 3 GB" (the speech to
/// text overview; the API reference allows 5 GB, so the smaller holds).
pub const MAX_UPLOAD_BYTES: u64 = 3 * 1024 * 1024 * 1024;

/// ElevenLabs Scribe as a [`CloudTranscriber`].
pub struct ElevenLabsTranscriber {
    client: Arc<dyn NetClient>,
    secrets: Arc<dyn SecretStore>,
    model: CloudTranscriptionModel,
}

impl ElevenLabsTranscriber {
    /// Scribe `model` over `client`, with the key in `secrets`.
    pub fn new(
        client: Arc<dyn NetClient>,
        secrets: Arc<dyn SecretStore>,
        model: CloudTranscriptionModel,
    ) -> Self {
        Self {
            client,
            secrets,
            model,
        }
    }

    async fn api(&self) -> Result<ScribeApi, CloudError> {
        let key = self.secrets.get(SecretName::ElevenLabsApiKey).await;
        let key = key.ok().flatten().ok_or(CloudError::MissingKey)?;
        Ok(ScribeApi::new(
            Arc::clone(&self.client),
            key.expose().to_owned(),
        ))
    }

    async fn send(&self, request: &CloudRequest) -> Result<Vec<CloudSpan>, CloudError> {
        let api = self.api().await?;
        let audio = read(&request.audio).await?;
        if audio.is_empty() {
            return Ok(Vec::new());
        }
        let file = file_name(&request.audio);
        let options = ScribeOptions {
            model: self.model.id(),
            diarize: request.diarize,
        };
        let transcript = api.transcribe((&file, &audio), options).await?;
        let lines = utterances(&transcript.words, request.diarize);
        log::info!(
            target: "cloud_transcription",
            "ElevenLabs transcribed {} bytes into {} lines",
            audio.len(),
            lines.len()
        );
        Ok(lines)
    }
}

#[async_trait]
impl CloudTranscriber for ElevenLabsTranscriber {
    async fn transcribe(&self, request: CloudRequest) -> Result<Vec<CloudSpan>, CloudError> {
        tokio::select! {
            biased;
            () = request.cancel.cancelled() => Err(CloudError::Cancelled),
            lines = self.send(&request) => lines,
        }
    }
}

/// The track's bytes, refused before reading when ElevenLabs would refuse
/// them.
async fn read(path: &Path) -> Result<Vec<u8>, CloudError> {
    let size = tokio::fs::metadata(path)
        .await
        .map_err(CloudError::Audio)?
        .len();
    if size > MAX_UPLOAD_BYTES {
        return Err(CloudError::TooLarge);
    }
    tokio::fs::read(path).await.map_err(CloudError::Audio)
}

/// The name the upload carries: the track's own (`mic.m4a`), which says
/// nothing about the meeting.
fn file_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("audio.m4a")
        .replace('"', "")
}

#[cfg(test)]
#[path = "transcriber_tests.rs"]
mod tests;
