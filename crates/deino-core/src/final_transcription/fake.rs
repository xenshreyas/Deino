//! [`ScriptedTracks`]: a [`TrackTranscriber`] for tests (behind
//! `test-support`), hearing whatever each file was scripted to say.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use async_trait::async_trait;

use super::{HeardSpan, TrackError, TrackRequest, TrackTranscriber};
use crate::testing::lock;

/// Hears the spans scripted for each file; an unscripted file is silent.
/// A file scripted to fail reports the engine as unavailable.
#[derive(Debug, Default)]
pub struct ScriptedTracks {
    spans: Mutex<HashMap<PathBuf, Vec<HeardSpan>>>,
    missing_model: Mutex<bool>,
    heard: Mutex<Vec<PathBuf>>,
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
        Ok(lock(&self.spans)
            .get(&request.audio)
            .cloned()
            .unwrap_or_default())
    }
}
