//! Echo cancellation for a finished microphone track, before it is
//! transcribed after the meeting.
//!
//! The live Me lane hears the microphone through AEC3 ([`crate::echo`]);
//! the stored `mic.m4a` is never cancelled, so on laptop speakers it holds
//! the other side of the call. Before the offline pass transcribes it, the
//! decoded copy (never the file) is run through the same canceller with the
//! decoded system track as the far-end reference. Both tracks are anchored
//! to the recording epoch with leading silence and decoded with the AAC
//! priming trimmed (`deino-audio`'s `TrackTimeline` and finalize), so
//! sample *n* of one is sample *n* of the other: no offset to apply.
//!
//! Skipped when the user says they use headphones ([`EchoMode::Isolated`]).
//! The recording does not store its output route, so otherwise it runs, as
//! [`EchoMode::choose`] treats an unknown route: as speakers, the safe side.

use async_trait::async_trait;
use deino_core::settings::SettingsService;

#[cfg(doc)]
use crate::echo::EchoMode;
use crate::echo::{EchoCanceller, FRAME};

/// Frames between progress reports: one second of audio.
const REPORT_EVERY: usize = 100;

/// Settings → Audio → "I use headphones".
#[async_trait]
pub trait HeadphonesSetting: Send + Sync {
    /// True when the microphone never hears the call.
    async fn uses_headphones(&self) -> bool;
}

#[async_trait]
impl HeadphonesSetting for SettingsService {
    async fn uses_headphones(&self) -> bool {
        self.get().await.audio.uses_headphones
    }
}

/// The headphones setting fixed on or off (tests, tools).
#[derive(Clone, Copy, Debug)]
pub struct FixedHeadphones(pub bool);

#[async_trait]
impl HeadphonesSetting for FixedHeadphones {
    async fn uses_headphones(&self) -> bool {
        self.0
    }
}

/// Removes `reference`'s echo from `mic` in place, one [`FRAME`] at a time.
/// About once per second of audio it reports the fraction done to
/// `progress`, which answers whether to go on; it reports 1.0 at the end.
/// False when `progress` stopped it. A reference shorter than the
/// microphone is padded with silence.
pub fn cancel_echo(
    mic: &mut [f32],
    reference: &[f32],
    canceller: &mut dyn EchoCanceller,
    progress: &dyn Fn(f32) -> bool,
) -> bool {
    let frames = mic.len().div_ceil(FRAME);
    let mut far = [0.0_f32; FRAME];
    let mut near = [0.0_f32; FRAME];
    let mut references = reference.chunks(FRAME);
    for (index, capture) in mic.chunks_mut(FRAME).enumerate() {
        fill(&mut far, references.next().unwrap_or_default());
        if capture.len() == FRAME {
            canceller.process(&far, capture);
        } else {
            fill(&mut near, capture);
            canceller.process(&far, &mut near);
            capture
                .iter_mut()
                .zip(near)
                .for_each(|(sample, clean)| *sample = clean);
        }
        if index % REPORT_EVERY == 0 && !progress(fraction(index, frames)) {
            return false;
        }
    }
    progress(1.0)
}

/// Copies `from` into `frame`, padding with silence.
fn fill(frame: &mut [f32; FRAME], from: &[f32]) {
    let padded = from.iter().copied().chain(std::iter::repeat(0.0));
    frame
        .iter_mut()
        .zip(padded)
        .for_each(|(slot, sample)| *slot = sample);
}

/// `done` of `total` frames as a fraction.
fn fraction(done: usize, total: usize) -> f32 {
    let as_u64 = |n: usize| u64::try_from(n).unwrap_or(u64::MAX);
    deino_core::final_transcription::fraction_of(as_u64(done), as_u64(total))
}

#[cfg(test)]
#[path = "echo_tests.rs"]
mod tests;
