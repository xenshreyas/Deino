//! [`ScribeApi`]: the two ElevenLabs calls Deino makes, over the
//! `deino-net` client of `ServiceId::ElevenLabs`, so the user's switch,
//! offline mode and the egress log apply (ADR 0019).
//!
//! - `POST /v1/speech-to-text`: `multipart/form-data` with `model_id`,
//!   `language_code`, `diarize`, `timestamps_granularity`,
//!   `tag_audio_events` and `file`; the key in `xi-api-key`.
//! - `GET /v1/user`: the key check. It costs no credits. A key restricted to
//!   speech to text answers 403 there, which still proves ElevenLabs knows
//!   it; only 401 means the key is wrong.
//!
//! Statuses map to [`CloudError`] as the error reference documents them:
//! 401 a refused key, 402 no credits, 403 not permitted, 413 or
//! `request_too_large` too large, other 4xx rejected, 429 busy, 5xx
//! unavailable.

use std::sync::Arc;
use std::time::Duration;

use deino_core::final_transcription::CloudError;
use deino_net::{MAX_BODY_BYTES, NetClient, NetError, Request, Response};
use serde::de::DeserializeOwned;

use crate::multipart::Multipart;
use crate::wire::{ErrorBody, ScribeTranscript};

/// The only host Deino sends to (`deino_net::registry`).
pub const API_HOST: &str = "api.elevenlabs.io";
/// The transcription endpoint.
const CONVERT_URL: &str = "https://api.elevenlabs.io/v1/speech-to-text";
/// The key check's endpoint.
const USER_URL: &str = "https://api.elevenlabs.io/v1/user";
/// How long one upload and transcription may take: a long meeting uploads
/// tens of MB, and Scribe transcribes it in a few minutes.
const CONVERT_TIMEOUT: Duration = Duration::from_mins(30);
/// The header ElevenLabs reads the key from.
const KEY_HEADER: &str = "xi-api-key";
/// Deino transcribes English only (`config::TRANSCRIPTION_LANGUAGE`).
const LANGUAGE: &str = "en";

/// What to ask for one track.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScribeOptions<'a> {
    /// `scribe_v2` or `scribe_v1`.
    pub model: &'a str,
    /// Tell the voices apart (`diarize`).
    pub diarize: bool,
}

/// ElevenLabs' API with the user's key.
pub struct ScribeApi {
    client: Arc<dyn NetClient>,
    key: String,
}

impl ScribeApi {
    /// The API over `client` (the scoped client of `ServiceId::ElevenLabs`)
    /// with `key`.
    pub fn new(client: Arc<dyn NetClient>, key: String) -> Self {
        Self { client, key }
    }

    /// Transcribes `audio`, the bytes of the file named `file`.
    pub async fn transcribe(
        &self,
        (file, audio): (&str, &[u8]),
        options: ScribeOptions<'_>,
    ) -> Result<ScribeTranscript, CloudError> {
        let form = Multipart::new(format!("deino-{}", uuid::Uuid::new_v4().simple()));
        let content_type = form.content_type();
        let body = form
            .text("model_id", options.model)
            .text("language_code", LANGUAGE)
            .text("diarize", if options.diarize { "true" } else { "false" })
            .text("timestamps_granularity", "word")
            .text("tag_audio_events", "false")
            .file("file", (file, "audio/mp4"), audio)
            .finish();
        let request = Request::post(CONVERT_URL)
            .map_err(net)?
            .header("content-type", content_type)
            .body(body)
            .timeout(Some(CONVERT_TIMEOUT));
        self.json(request).await
    }

    /// Checks that ElevenLabs knows the key.
    pub async fn check(&self) -> Result<(), CloudError> {
        let request = Request::get(USER_URL).map_err(net)?;
        let response = self.send(request).await?;
        match response.status.as_u16() {
            403 => Ok(()),
            _ => refused(response).await.map(drop),
        }
    }

    async fn json<T: DeserializeOwned>(&self, request: Request) -> Result<T, CloudError> {
        let response = refused(self.send(request).await?).await?;
        let bytes = response.bytes(MAX_BODY_BYTES).await.map_err(net)?;
        serde_json::from_slice(&bytes).map_err(|error| {
            log::warn!(target: "cloud_transcription", "ElevenLabs answered JSON Deino cannot read: {error}");
            CloudError::Unreadable
        })
    }

    async fn send(&self, request: Request) -> Result<Response, CloudError> {
        let request = request
            .header(KEY_HEADER, self.key.clone())
            .header("accept", "application/json");
        self.client.send(request).await.map_err(net)
    }
}

/// `response` when it is a success, else the failure its status and body name.
async fn refused(response: Response) -> Result<Response, CloudError> {
    let status = response.status.as_u16();
    if response.status.is_success() {
        return Ok(response);
    }
    let body = response.bytes(MAX_BODY_BYTES).await.unwrap_or_default();
    let error: ErrorBody = serde_json::from_slice(&body).unwrap_or_default();
    let code = error.code().unwrap_or_default();
    log::warn!(target: "cloud_transcription", "ElevenLabs answered {status} ({code})");
    Err(match (status, code) {
        (401, _) => CloudError::KeyRefused,
        (402, _) => CloudError::NoCredits,
        (413, _) | (_, "request_too_large") => CloudError::TooLarge,
        (403, _) => CloudError::NotPermitted,
        (429, _) => CloudError::RateLimited,
        (500.., _) => CloudError::Unavailable,
        _ => CloudError::Rejected,
    })
}

/// A transport failure as the cloud error the stage understands.
fn net(error: NetError) -> CloudError {
    match error {
        error if error.is_blocked() => CloudError::Blocked,
        NetError::Decode(_) | NetError::TooLarge { .. } => CloudError::Unreadable,
        error => CloudError::Network(Box::new(error)),
    }
}

#[cfg(test)]
#[path = "api_tests.rs"]
mod tests;
