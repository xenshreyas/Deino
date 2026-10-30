//! Echo cancellation carried across passes over a growing recording.
//!
//! AEC3 runs one frame at a time from the start, so the cancelled samples
//! of a microphone track's first frames never depend on what comes after.
//! A pass during the meeting cancels the whole frames it has (short of the
//! unsettled end) and keeps the canceller in [`PassMemory`]; the next pass
//! checks that the start of the tracks is the same audio, reuses the
//! cancelled samples and goes on from there with that canceller. Anything
//! that does not match starts over from sample 0, so the result is always
//! what one pass over the whole track gives.

use super::cancel_echo;
use super::memory::{PassMemory, fingerprint};
use crate::echo::{FRAME, SonoraAec};

/// Samples at the end of a track still being recorded that the finished
/// file may decode differently (AAC's last packet lacks the next one's
/// overlap, and the resampler its tail): one second. A pass keeps its
/// canceller only before them.
const UNSETTLED: usize = 16_000;

/// Where a cancelled stretch of a microphone track stopped.
pub(super) struct Saved {
    /// Samples cancelled.
    samples: usize,
    /// Fingerprints of the microphone (before cancelling) and the reference
    /// over those samples.
    mic: u128,
    far: u128,
    cleaned: Vec<f32>,
    canceller: SonoraAec,
}

/// Cancels `far`'s echo from `mic` in place, resuming from `memory` when it
/// holds the same start; false when `progress` stopped it.
pub(super) fn cancel_resumed(
    memory: &PassMemory,
    (mic, far): (&mut [f32], &[f32]),
    progress: &dyn Fn(f32) -> bool,
) -> bool {
    let end = if memory.follows() {
        mic.len().min(far.len()).saturating_sub(UNSETTLED) / FRAME * FRAME
    } else {
        mic.len()
    };
    let original = memory
        .follows()
        .then(|| fingerprint(mic.get(..end).unwrap_or_default()));
    let (mut canceller, from) = resume(memory, mic, far);
    let rest = mic.get_mut(from..end.max(from)).unwrap_or_default();
    if !cancel_echo(
        rest,
        far.get(from..).unwrap_or_default(),
        &mut canceller,
        progress,
    ) {
        return false;
    }
    if let Some(original) = original {
        *memory.echo() = Some(Saved {
            samples: end,
            mic: original,
            far: fingerprint(far.get(..end).unwrap_or_default()),
            cleaned: mic.get(..end).unwrap_or_default().to_vec(),
            canceller,
        });
    }
    true
}

/// The canceller to go on with and the first sample it has not processed,
/// having copied the cancelled start into `mic` when it matches; a fresh
/// canceller from sample 0 otherwise.
fn resume(memory: &PassMemory, mic: &mut [f32], far: &[f32]) -> (SonoraAec, usize) {
    let saved = memory.echo().take();
    if let Some(saved) = saved
        && saved.samples <= mic.len().min(far.len())
        && fingerprint(mic.get(..saved.samples).unwrap_or_default()) == saved.mic
        && fingerprint(far.get(..saved.samples).unwrap_or_default()) == saved.far
        && let Some(start) = mic.get_mut(..saved.samples)
    {
        start.copy_from_slice(&saved.cleaned);
        log::info!(target: "pipeline", "echo cancellation resumed {} s into the microphone track", saved.samples / 16_000);
        return (saved.canceller, saved.samples);
    }
    (SonoraAec::new(), 0)
}

#[cfg(test)]
#[path = "echo_resume_tests.rs"]
mod tests;
