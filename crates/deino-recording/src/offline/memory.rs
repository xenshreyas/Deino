//! [`PassMemory`]: what passes over one recording share, so the pass after
//! the meeting redoes only what the passes during it (`follow`) could not.
//!
//! Every answer is keyed by the exact samples it came from: a clip's
//! transcript by the clip and the engine's model. (The voice check is not
//! remembered: its model carries state from line to line, so a line's
//! answer depends on which lines were checked before it.) The pass after
//! the meeting reads the finished file,
//! whose samples equal those of the file being recorded except near the
//! end, so it reuses everything but the last moments; anything that differs
//! at all is a different key and is worked out again. The result is the
//! same as without the memory.
//!
//! The memory holds one recording: [`PassMemory::forget`] clears it when
//! the next one starts following.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};

use deino_stt::{BatchTranscriber, BatchTranscript, SttError};
use tokio::sync::{OwnedRwLockReadGuard, OwnedRwLockWriteGuard, RwLock};

use super::echo_resume::Saved;

/// A clip's key: its engine's model, its sample count and fingerprint.
type ClipKey = (String, usize, u128);

/// What the passes share.
#[derive(Default)]
struct Shared {
    clips: Mutex<HashMap<ClipKey, BatchTranscript>>,
    echo: Mutex<Option<Saved>>,
    /// One pass at a time: a pass during the meeting holds it whole, the
    /// pass after it per track.
    gate: Arc<RwLock<()>>,
}

/// The memory of the passes over the recording being followed.
#[derive(Clone, Default)]
pub struct PassMemory {
    shared: Arc<Shared>,
    /// A pass during the meeting: it keeps the echo canceller going.
    following: bool,
}

impl PassMemory {
    /// An empty memory, for the pass after the meeting.
    pub fn new() -> Self {
        Self::default()
    }

    /// The same memory for passes during the meeting.
    #[must_use]
    pub fn following(&self) -> Self {
        Self {
            shared: Arc::clone(&self.shared),
            following: true,
        }
    }

    /// Whether this is a pass during the meeting.
    pub fn follows(&self) -> bool {
        self.following
    }

    /// Drops everything remembered (a new recording is being followed).
    pub fn forget(&self) {
        lock(&self.shared.clips).clear();
        *lock(&self.shared.echo) = None;
    }

    /// Holds off the pass after the meeting until this pass is done.
    pub async fn exclusive(&self) -> OwnedRwLockWriteGuard<()> {
        Arc::clone(&self.shared.gate).write_owned().await
    }

    /// Waits for a pass during the meeting to finish.
    pub(super) async fn shared(&self) -> OwnedRwLockReadGuard<()> {
        Arc::clone(&self.shared.gate).read_owned().await
    }

    /// `engine` answering from memory for clips it was given before.
    pub(super) fn engine(
        &self,
        engine: Arc<dyn BatchTranscriber>,
        model: String,
    ) -> Arc<dyn BatchTranscriber> {
        Arc::new(Remembered {
            engine,
            model,
            memory: self.clone(),
        })
    }

    pub(super) fn echo(&self) -> std::sync::MutexGuard<'_, Option<Saved>> {
        lock(&self.shared.echo)
    }
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// A fingerprint of `samples`' exact values (two FNV-1a lanes).
pub(super) fn fingerprint(samples: &[f32]) -> u128 {
    let (mut low, mut high) = (0xcbf2_9ce4_8422_2325_u64, 0x8422_2325_cbf2_9ce4_u64);
    for sample in samples {
        let bits = u64::from(sample.to_bits());
        low = (low ^ bits).wrapping_mul(0x0000_0100_0000_01b3);
        high = (high ^ bits.rotate_left(17)).wrapping_mul(0x0000_0001_0000_0193);
    }
    (u128::from(high) << 64) | u128::from(low)
}

/// An engine that remembers its answers.
struct Remembered {
    engine: Arc<dyn BatchTranscriber>,
    model: String,
    memory: PassMemory,
}

impl BatchTranscriber for Remembered {
    fn transcribe(&self, pcm: Vec<f32>) -> Result<BatchTranscript, SttError> {
        let key = (self.model.clone(), pcm.len(), fingerprint(&pcm));
        if let Some(known) = lock(&self.memory.shared.clips).get(&key) {
            return Ok(known.clone());
        }
        let transcript = self.engine.transcribe(pcm)?;
        lock(&self.memory.shared.clips).insert(key, transcript.clone());
        Ok(transcript)
    }
}

#[cfg(test)]
#[path = "memory_tests.rs"]
mod tests;
