//! [`ScriptedTracks`] and [`ScriptedCloud`]: a [`TrackTranscriber`] and a
//! [`CloudTranscriber`] for tests (behind
//! `test-support`), hearing whatever each file was scripted to say.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use async_trait::async_trait;

use super::{
    CloudError, CloudRequest, CloudSpan, CloudTranscriber, HeardSpan, TrackError, TrackRequest,
    TrackTranscriber,
};
use crate::testing::lock;

/// The fractions [`ScriptedTracks`] reports for each file, in order.
pub const SCRIPTED_PROGRESS: [f32; 2] = [0.5, 1.0];

/// Hears the spans scripted for each file; an unscripted file is silent.
/// A file scripted to fail reports the engine as unavailable. Each file it
/// hears reports progress halfway, then done ([`SCRIPTED_PROGRESS`]).
#[derive(Debug, Default)]
pub struct ScriptedTracks {
    spans: Mutex<HashMap<PathBuf, Vec<HeardSpan>>>,
    missing_model: Mutex<bool>,
    heard: Mutex<Vec<PathBuf>>,
    references: Mutex<Vec<Option<PathBuf>>>,
}

impl ScriptedTracks {
    /// Scripts what `path` says.
    pub fn say(&self, path: impl AsRef<Path>, spans: Vec<HeardSpan>) {
        lock(&self.spans).insert(path.as_ref().to_path_buf(), spans);
    }

    /// Every following run fails as if no model were downloaded.
    pub fn without_model(&self) {
        *lock(&self.missing_model) = true;
    }

    /// The files transcribed so far, in order.
    pub fn heard(&self) -> Vec<PathBuf> {
        lock(&self.heard).clone()
    }

    /// The echo reference each file came with, in the same order.
    pub fn echo_references(&self) -> Vec<Option<PathBuf>> {
        lock(&self.references).clone()
    }
}

#[async_trait]
impl TrackTranscriber for ScriptedTracks {
    async fn transcribe(&self, request: TrackRequest) -> Result<Vec<HeardSpan>, TrackError> {
        if request.cancel.is_cancelled() {
            return Err(TrackError::Cancelled);
        }
        if *lock(&self.missing_model) {
            let missing = std::io::Error::new(std::io::ErrorKind::NotFound, "no model");
            return Err(TrackError::Unavailable(Box::new(missing)));
        }
        lock(&self.heard).push(request.audio.clone());
        lock(&self.references).push(request.echo_reference.clone());
        for fraction in SCRIPTED_PROGRESS {
            request.progress.report(fraction);
        }
        Ok(lock(&self.spans)
            .get(&request.audio)
            .cloned()
            .unwrap_or_default())
    }
}

/// A cloud service for tests: hears what each file was scripted to say
/// (with its voices), or fails every run with the scripted error.
#[derive(Debug, Default)]
pub struct ScriptedCloud {
    spans: Mutex<HashMap<PathBuf, Vec<CloudSpan>>>,
    failure: Mutex<Option<fn() -> CloudError>>,
    sent: Mutex<Vec<(PathBuf, bool)>>,
}

impl ScriptedCloud {
    /// Scripts what `path` says.
    pub fn say(&self, path: impl AsRef<Path>, spans: Vec<CloudSpan>) {
        lock(&self.spans).insert(path.as_ref().to_path_buf(), spans);
    }

    /// Every following run fails with `error()`.
    pub fn fail_with(&self, error: fn() -> CloudError) {
        *lock(&self.failure) = Some(error);
    }

    /// The files sent so far, each with whether it was to be diarized.
    pub fn sent(&self) -> Vec<(PathBuf, bool)> {
        lock(&self.sent).clone()
    }
}

#[async_trait]
impl CloudTranscriber for ScriptedCloud {
    async fn transcribe(&self, request: CloudRequest) -> Result<Vec<CloudSpan>, CloudError> {
        if request.cancel.is_cancelled() {
            return Err(CloudError::Cancelled);
        }
        if let Some(error) = *lock(&self.failure) {
            return Err(error());
        }
        lock(&self.sent).push((request.audio.clone(), request.diarize));
        Ok(lock(&self.spans)
            .get(&request.audio)
            .cloned()
            .unwrap_or_default())
    }
}
