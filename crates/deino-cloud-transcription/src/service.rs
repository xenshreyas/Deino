//! [`ElevenLabs`]: the Settings → Advanced → ElevenLabs card's service and the
//! stage's [`CloudChoice`].
//!
//! - The key lives in the Keychain (`SecretName::ElevenLabsApiKey`, ADR
//!   0013): saved, checked and forgotten here, never read back by the UI.
//!   Saving switches `ServiceId::ElevenLabs` on, as connecting an import
//!   source does; forgetting switches it off. Offline mode still blocks it.
//! - A meeting goes to ElevenLabs when Settings say so and a key is saved,
//!   or when the user asked on that meeting ([`ElevenLabs::request`]); the
//!   request is spent when the stage finishes with it. Requests live in
//!   memory: one cut short by quitting falls back to the setting.

use std::collections::HashSet;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use async_trait::async_trait;
use deino_core::final_transcription::{CloudChoice, CloudError, CloudPass};
use deino_core::settings::{CloudTranscriptionModel, CloudTranscriptionProvider, SettingsService};
use deino_domain::{ErrorClass, ErrorCode, MeetingId, ServiceId};
use deino_net::{NetClient, ServiceSwitch, SwitchError};
use deino_secrets::{SecretError, SecretName, SecretStatus, SecretStore, SecretValue};

use crate::{ElevenLabsTranscriber, ScribeApi};

/// The key's Keychain item.
const KEY: SecretName = SecretName::ElevenLabsApiKey;

/// The transcription settings' cloud option (Settings → Advanced → ElevenLabs).
#[async_trait]
pub trait CloudSettings: Send + Sync {
    /// The chosen service and its model.
    async fn cloud(&self) -> (CloudTranscriptionProvider, CloudTranscriptionModel);
}

#[async_trait]
impl CloudSettings for SettingsService {
    async fn cloud(&self) -> (CloudTranscriptionProvider, CloudTranscriptionModel) {
        let transcription = self.get().await.transcription;
        (transcription.cloud_provider, transcription.cloud_model)
    }
}

/// What [`ElevenLabs`] is built from.
pub struct ElevenLabsParts {
    /// The scoped client of `ServiceId::ElevenLabs`.
    pub client: Arc<dyn NetClient>,
    /// The Keychain.
    pub secrets: Arc<dyn SecretStore>,
    /// The Network page's switches.
    pub switch: Arc<dyn ServiceSwitch>,
    /// The cloud option in the settings.
    pub settings: Arc<dyn CloudSettings>,
}

/// ElevenLabs transcription: the key and the choice per meeting.
pub struct ElevenLabs {
    parts: ElevenLabsParts,
    requests: Mutex<HashSet<MeetingId>>,
}

impl ElevenLabs {
    /// The service over its parts.
    pub fn new(parts: ElevenLabsParts) -> Self {
        Self {
            parts,
            requests: Mutex::new(HashSet::new()),
        }
    }

    /// Whether a key is saved: `missing`, `stored`, or `locked` when macOS
    /// refused access. Never the key.
    pub async fn key_status(&self) -> Result<SecretStatus, SecretError> {
        self.parts.secrets.status(KEY).await
    }

    /// Saves `key` and switches the service on.
    pub async fn save_key(&self, key: &str) -> Result<(), KeyError> {
        self.parts
            .secrets
            .set(KEY, SecretValue::parse(key)?)
            .await?;
        self.switch(true).await
    }

    /// Asks ElevenLabs whether it knows the saved key.
    pub async fn check_key(&self) -> Result<(), KeyError> {
        let key = self.parts.secrets.get(KEY).await?;
        let key = key.ok_or(CloudError::MissingKey)?;
        let client = Arc::clone(&self.parts.client);
        let api = ScribeApi::new(client, key.expose().to_owned());
        Ok(api.check().await?)
    }

    /// Forgets the key and switches the service off.
    pub async fn forget_key(&self) -> Result<(), KeyError> {
        self.parts.secrets.delete(KEY).await?;
        self.switch(false).await
    }

    /// Sends `meeting` to ElevenLabs on its next transcription, whatever the
    /// setting says; fails when no key is saved.
    pub async fn request(&self, meeting: &MeetingId) -> Result<(), KeyError> {
        if self.key_status().await? != SecretStatus::Stored {
            return Err(CloudError::MissingKey.into());
        }
        lock(&self.requests).insert(meeting.clone());
        Ok(())
    }

    async fn switch(&self, on: bool) -> Result<(), KeyError> {
        Ok(self
            .parts
            .switch
            .set_enabled(ServiceId::ElevenLabs, on)
            .await?)
    }

    fn transcriber(&self, model: CloudTranscriptionModel) -> ElevenLabsTranscriber {
        let client = Arc::clone(&self.parts.client);
        ElevenLabsTranscriber::new(client, Arc::clone(&self.parts.secrets), model)
    }
}

#[async_trait]
impl CloudChoice for ElevenLabs {
    async fn for_meeting(&self, meeting: &MeetingId) -> Option<CloudPass> {
        let (provider, model) = self.parts.settings.cloud().await;
        let requested = lock(&self.requests).contains(meeting);
        let chosen = provider == CloudTranscriptionProvider::ElevenLabs;
        let stored = matches!(self.key_status().await, Ok(SecretStatus::Stored));
        ((chosen || requested) && stored).then(|| CloudPass {
            transcriber: Arc::new(self.transcriber(model)),
            requested,
        })
    }

    fn finished(&self, meeting: &MeetingId) {
        lock(&self.requests).remove(meeting);
    }
}

/// Why a key could not be saved, checked or forgotten.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum KeyError {
    /// The Keychain refused, or the key is blank.
    #[error(transparent)]
    Secret(#[from] SecretError),
    /// ElevenLabs refused the key, or could not be asked.
    #[error(transparent)]
    Cloud(#[from] CloudError),
    /// The network switch could not be saved.
    #[error("The ElevenLabs network switch could not be saved.")]
    Switch(#[from] SwitchError),
}

impl ErrorClass for KeyError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::Secret(error) => error.code(),
            Self::Cloud(error) => error.code(),
            Self::Switch(_) => ErrorCode::Internal,
        }
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
#[path = "service_tests.rs"]
mod tests;
