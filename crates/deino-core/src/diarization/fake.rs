//! A scripted diarization engine for tests.

use std::sync::Mutex;

use async_trait::async_trait;

use super::{TrackDiarization, TrackDiarizer, TrackError, TrackRequest};
use crate::testing::lock;

/// Answers every request with one scripted result (or `ModelsUnavailable`
/// when none is set) and records the requests it saw.
#[derive(Debug, Default)]
pub struct FakeTrackDiarizer {
    result: Mutex<Option<TrackDiarization>>,
    requests: Mutex<Vec<TrackRequest>>,
}

/// The embedding model the fake claims.
pub const FAKE_EMBEDDING_MODEL: &str = "fake-embedding@test";

impl FakeTrackDiarizer {
    /// A fake that returns `result` for every track.
    pub fn returning(result: TrackDiarization) -> Self {
        Self {
            result: Mutex::new(Some(result)),
            requests: Mutex::default(),
        }
    }

    /// Every request so far, in order.
    pub fn requests(&self) -> Vec<TrackRequest> {
        lock(&self.requests).clone()
    }
}

#[async_trait]
impl TrackDiarizer for FakeTrackDiarizer {
    fn embedding_model(&self) -> &str {
        FAKE_EMBEDDING_MODEL
    }

    async fn diarize(&self, request: TrackRequest) -> Result<TrackDiarization, TrackError> {
        lock(&self.requests).push(request);
        lock(&self.result)
            .clone()
            .ok_or_else(|| TrackError::ModelsUnavailable("no scripted result".into()))
    }
}
